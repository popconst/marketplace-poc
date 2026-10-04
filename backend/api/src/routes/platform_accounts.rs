//! The account list advertisers browse, with search, filters and keyset paging.

use axum::extract::State;
use db::models::{Platform, PlatformAccount};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::error::ApiError;
use crate::extract::{Json, Path, Query};

const DEFAULT_LIMIT: u16 = 50;
const MAX_LIMIT: u16 = 100;
/// Shorter searches are ignored: the trigram indexes on the handle and the creator's name only
/// help from three characters, and shorter ones would match most accounts anyway.
const MIN_SEARCH_CHARS: usize = 3;

/// The columns of an [`AccountListItem`], from account `pa` and its creator `cr`.
const ITEM_COLUMNS: &str = "pa.*, cr.name AS creator_name, cr.reliability_score,
     ARRAY(SELECT genre_id FROM platform_account_genres
           WHERE platform_account_id = pa.id ORDER BY genre_id) AS genre_ids,
     ARRAY(SELECT language_code FROM platform_account_languages
           WHERE platform_account_id = pa.id ORDER BY language_code) AS language_codes";

/// All optional; the filters combine with AND.
#[derive(Deserialize)]
pub struct ListParams {
    /// Case-insensitive substring of the handle or the creator's name.
    q: Option<String>,
    platform: Option<Platform>,
    country: Option<String>,
    genre: Option<i16>,
    language: Option<String>,
    /// Bounds on views per post, both included.
    min_views: Option<i32>,
    max_views: Option<i32>,
    /// Bounds on the engagement rate, 0 to 1, both included.
    min_engagement: Option<f64>,
    max_engagement: Option<f64>,
    #[serde(default)]
    sort: Sort,
    /// `nextCursor` of the previous page, with the same sort.
    cursor: Option<String>,
    limit: Option<u16>,
}

/// Always highest first, ties broken by the higher id.
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Sort {
    #[default]
    ViewScore,
    Followers,
    EngagementRate,
    QualityScore,
}

impl Sort {
    /// Safe to splice into SQL: it comes from this enum, never from the request's text.
    fn column(self) -> &'static str {
        match self {
            Self::ViewScore => "view_score",
            Self::Followers => "followers",
            Self::EngagementRate => "engagement_rate",
            Self::QualityScore => "quality_score",
        }
    }
}

/// A cursor's sort value, bound with its column's type so that the comparison can use its index.
#[derive(Clone, Copy)]
enum SortValue {
    Integer(i64),
    Real(f64),
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AccountListItem {
    #[serde(flatten)]
    #[sqlx(flatten)]
    account: PlatformAccount,
    creator_name: String,
    reliability_score: i16,
    genre_ids: Vec<i16>,
    language_codes: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    items: Vec<AccountListItem>,
    /// `None` on the last page.
    next_cursor: Option<String>,
}

#[derive(Clone, Copy)]
enum Search<'a> {
    Handle(&'a str),
    CreatorName(&'a str),
}

/// A page of accounts. Keyset paging, not offset, keeps every page as fast as the first across
/// tens of millions of accounts. The cursor holds the last sort value and id, not the sort, so a
/// client drops it when it changes the sort.
///
/// The page is picked first, and only its rows get their creator, genres and languages:
///
/// ```sql
/// SELECT <ITEM_COLUMNS>
/// FROM (<page>) pa
/// JOIN creators cr ON cr.id = pa.creator_id
/// ORDER BY pa.<sort> DESC, pa.id DESC  -- again, as a join does not keep the page's order
/// ```
///
/// Without a search, `<page>` is one [`push_page`]. With one, the handle and the creator's name
/// are in different tables and no index serves an `OR` across both, so each gets its own page
/// from its own index. `UNION` keeps the best of the two and lists an account matching both once:
///
/// ```sql
/// (<page of handle matches>) UNION (<page of creator name matches>)
/// ORDER BY <sort> DESC, id DESC LIMIT $fetch
/// ```
pub async fn list(
    State(pool): State<PgPool>,
    Query(params): Query<ListParams>,
) -> Result<Json<Page>, ApiError> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let after = params
        .cursor
        .as_deref()
        .map(|cursor| {
            decode_cursor(params.sort, cursor)
                .ok_or_else(|| ApiError::BadRequest("Invalid cursor.".to_owned()))
        })
        .transpose()?;
    let pattern = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| q.chars().count() >= MIN_SEARCH_CHARS)
        .map(like_pattern);
    // One more than a page tells whether another page follows.
    let fetch = i64::from(limit) + 1;
    let sort = params.sort.column();

    let mut query = QueryBuilder::new("SELECT ");
    query.push(ITEM_COLUMNS).push(" FROM (");
    match &pattern {
        Some(pattern) => {
            let (handle, name) = (Search::Handle(pattern), Search::CreatorName(pattern));
            query.push("(");
            push_page(&mut query, &params, Some(handle), after, fetch);
            query.push(") UNION (");
            push_page(&mut query, &params, Some(name), after, fetch);
            query.push(format_args!(") ORDER BY {sort} DESC, id DESC LIMIT "));
            query.push_bind(fetch);
        }
        None => push_page(&mut query, &params, None, after, fetch),
    }
    query.push(format_args!(
        ") pa JOIN creators cr ON cr.id = pa.creator_id ORDER BY pa.{sort} DESC, pa.id DESC"
    ));

    let mut items: Vec<AccountListItem> = query.build_query_as().fetch_all(&pool).await?;
    let next_cursor = if items.len() > usize::from(limit) {
        items.truncate(usize::from(limit));
        items
            .last()
            .map(|item| encode_cursor(params.sort, &item.account))
    } else {
        None
    };
    Ok(Json(Page { items, next_cursor }))
}

/// One account, as the list shows it.
pub async fn show(
    State(pool): State<PgPool>,
    Path(account_id): Path<i64>,
) -> Result<Json<AccountListItem>, ApiError> {
    let mut query = QueryBuilder::new("SELECT ");
    query
        .push(ITEM_COLUMNS)
        .push(" FROM platform_accounts pa JOIN creators cr ON cr.id = pa.creator_id WHERE pa.id = ")
        .push_bind(account_id);
    query
        .build_query_as::<AccountListItem>()
        .fetch_optional(&pool)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("Platform account", account_id))
}

/// Pushes the query for the first `fetch` matching accounts after the cursor, in sort order:
///
/// ```sql
/// SELECT pa.* FROM platform_accounts pa [JOIN creators cr ON cr.id = pa.creator_id]
/// WHERE <search, or TRUE> AND <each filter given>
///   AND (pa.<sort>, pa.id) < ($cursor_value, $cursor_id)
/// ORDER BY pa.<sort> DESC, pa.id DESC LIMIT $fetch
/// ```
fn push_page(
    query: &mut QueryBuilder<Postgres>,
    params: &ListParams,
    search: Option<Search>,
    after: Option<(SortValue, i64)>,
    fetch: i64,
) {
    query.push("SELECT pa.* FROM platform_accounts pa");
    match search {
        Some(Search::Handle(pattern)) => {
            query.push(" WHERE pa.handle ILIKE ").push_bind(pattern);
        }
        Some(Search::CreatorName(pattern)) => {
            query
                .push(" JOIN creators cr ON cr.id = pa.creator_id WHERE cr.name ILIKE ")
                .push_bind(pattern);
        }
        // So that every filter below can start with `AND`.
        None => {
            query.push(" WHERE TRUE");
        }
    }
    if let Some(platform) = params.platform {
        query.push(" AND pa.platform = ").push_bind(platform);
    }
    if let Some(country) = &params.country {
        query.push(" AND pa.country_code = ").push_bind(country);
    }
    // `EXISTS`, not a join, so that an account is listed once however many rows match.
    if let Some(genre) = params.genre {
        query
            .push(
                " AND EXISTS (SELECT 1 FROM platform_account_genres
                              WHERE platform_account_id = pa.id AND genre_id = ",
            )
            .push_bind(genre)
            .push(")");
    }
    if let Some(language) = &params.language {
        query
            .push(
                " AND EXISTS (SELECT 1 FROM platform_account_languages
                              WHERE platform_account_id = pa.id AND language_code = ",
            )
            .push_bind(language)
            .push(")");
    }
    if let Some(min) = params.min_views {
        query.push(" AND pa.view_score >= ").push_bind(min);
    }
    if let Some(max) = params.max_views {
        query.push(" AND pa.view_score <= ").push_bind(max);
    }
    if let Some(min) = params.min_engagement {
        query.push(" AND pa.engagement_rate >= ").push_bind(min);
    }
    if let Some(max) = params.max_engagement {
        query.push(" AND pa.engagement_rate <= ").push_bind(max);
    }
    let sort = params.sort.column();
    if let Some((value, id)) = after {
        query.push(format_args!(" AND (pa.{sort}, pa.id) < ("));
        match value {
            SortValue::Integer(value) => query.push_bind(value),
            SortValue::Real(value) => query.push_bind(value),
        };
        query.push(", ").push_bind(id).push(")");
    }
    query.push(format_args!(" ORDER BY pa.{sort} DESC, pa.id DESC LIMIT "));
    query.push_bind(fetch);
}

/// `%q%`, with LIKE's own wildcards in `q` escaped so that they match literally.
fn like_pattern(q: &str) -> String {
    let escaped = q
        .replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_");
    format!("%{escaped}%")
}

/// `<sort value>_<id>` of the page's last account. Opaque to clients.
fn encode_cursor(sort: Sort, last: &PlatformAccount) -> String {
    match sort {
        Sort::ViewScore => format!("{}_{}", last.view_score, last.id),
        Sort::Followers => format!("{}_{}", last.followers, last.id),
        Sort::EngagementRate => format!("{}_{}", last.engagement_rate, last.id),
        Sort::QualityScore => format!("{}_{}", last.quality_score, last.id),
    }
}

fn decode_cursor(sort: Sort, cursor: &str) -> Option<(SortValue, i64)> {
    let (value, id) = cursor.split_once('_')?;
    let value = match sort {
        Sort::EngagementRate => SortValue::Real(value.parse().ok()?),
        _ => SortValue::Integer(value.parse().ok()?),
    };
    Some((value, id.parse().ok()?))
}
