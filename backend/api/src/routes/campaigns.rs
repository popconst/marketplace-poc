//! Campaigns: creating a draft, autosaving it, and publishing it, which freezes its deal.

use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use db::config::Settings;
use db::models::{Campaign, CampaignStatus, Phase, Platform};
use marketplace::{PLANNED_GROUPS, SizeGroup};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{PgConnection, PgExecutor, PgPool};

use crate::error::{ApiError, FieldErrors};
use crate::extract::{Json, Path};
use crate::format::euros;

const TITLE_MAX_CHARS: usize = 120;
const BRIEFING_MAX_CHARS: usize = 5000;
/// Both caps are far above any real campaign, and low enough that money arithmetic cannot overflow.
const MAX_BUDGET_CENTS: i64 = 1_000_000_000;
const MAX_TARGET_CPM_CENTS: i64 = 100_000;
const SLIDER_STEPS: [i16; 5] = [0, 25, 50, 75, 100];

/// A `campaign_details` row: a campaign with its targeting. An empty list means no restriction.
#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub(super) struct CampaignRow {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub(super) campaign: Campaign,
    pub(super) country_codes: Vec<String>,
    pub(super) language_codes: Vec<String>,
    pub(super) genre_ids: Vec<i16>,
}

/// A campaign as the API returns it: with its targeting and its current phase.
#[derive(Serialize)]
pub struct CampaignDetail {
    #[serde(flatten)]
    row: CampaignRow,
    phase: Phase,
}

impl CampaignDetail {
    fn new(row: CampaignRow, now: DateTime<Utc>) -> Self {
        let campaign = &row.campaign;
        let phase = Phase::new(campaign.status, campaign.bidding_deadline, now);
        Self { row, phase }
    }
}

/// The fields of a draft an advertiser edits, with the targeting lists sorted and deduplicated,
/// so that comparing two tells whether a save changes anything.
#[derive(Clone, PartialEq, sqlx::FromRow)]
struct DraftValues {
    title: Option<String>,
    briefing: Option<String>,
    platform: Option<Platform>,
    budget_cents: Option<i64>,
    target_cpm_cents: Option<i64>,
    size_groups: Vec<SizeGroup>,
    engagement_weight: i16,
    quality_weight: i16,
    reliability_weight: i16,
    submission_window_days: i16,
    country_codes: Vec<String>,
    language_codes: Vec<String>,
    genre_ids: Vec<i16>,
}

/// A partial update of a draft. An absent field is kept. `null` clears a [`Change`] field and is
/// ignored on the others, which cannot be empty. A targeting list replaces the current one.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CampaignChanges {
    title: Change<String>,
    briefing: Change<String>,
    platform: Change<Platform>,
    budget_cents: Change<i64>,
    target_cpm_cents: Change<i64>,
    size_groups: Option<Vec<SizeGroup>>,
    engagement_weight: Option<i16>,
    quality_weight: Option<i16>,
    reliability_weight: Option<i16>,
    submission_window_days: Option<i16>,
    country_codes: Option<Vec<String>>,
    language_codes: Option<Vec<String>>,
    genre_ids: Option<Vec<i16>>,
}

/// One field of a partial update: absent, `null`, or a value.
#[derive(Default)]
enum Change<T> {
    #[default]
    Keep,
    Clear,
    Set(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Change<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Serde only gets here for a field that is present; an absent one is `Keep`.
        Ok(Option::deserialize(deserializer)?.map_or(Self::Clear, Self::Set))
    }
}

impl<T: Clone> Change<T> {
    fn apply_to(&self, field: &mut Option<T>) {
        match self {
            Self::Keep => {}
            Self::Clear => *field = None,
            Self::Set(value) => *field = Some(value.clone()),
        }
    }
}

impl CampaignChanges {
    /// Applies the changes. Text is trimmed and cleared if blank; sizes and lists are sorted.
    fn apply(&self, draft: &mut DraftValues) {
        self.title.apply_to(&mut draft.title);
        trim(&mut draft.title);
        self.briefing.apply_to(&mut draft.briefing);
        trim(&mut draft.briefing);
        self.platform.apply_to(&mut draft.platform);
        self.budget_cents.apply_to(&mut draft.budget_cents);
        self.target_cpm_cents.apply_to(&mut draft.target_cpm_cents);
        if let Some(sizes) = &self.size_groups {
            draft.size_groups.clone_from(sizes);
            // Not deduplicated, so that validation can refuse a size picked twice.
            draft.size_groups.sort_unstable();
        }
        if let Some(weight) = self.engagement_weight {
            draft.engagement_weight = weight;
        }
        if let Some(weight) = self.quality_weight {
            draft.quality_weight = weight;
        }
        if let Some(weight) = self.reliability_weight {
            draft.reliability_weight = weight;
        }
        if let Some(days) = self.submission_window_days {
            draft.submission_window_days = days;
        }
        if let Some(codes) = &self.country_codes {
            draft.country_codes = sorted_once(codes);
        }
        if let Some(codes) = &self.language_codes {
            draft.language_codes = sorted_once(codes);
        }
        if let Some(ids) = &self.genre_ids {
            draft.genre_ids = sorted_once(ids);
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    /// Required. An `Option` so that a missing one is a field error, like a missing title.
    bidding_deadline: Option<DateTime<Utc>>,
}

/// What a draft may save and publish. GET `/api/options` sends it too, so that the campaign form
/// checks what the API checks.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    budget_cents: Bounds<i64>,
    target_cpm_cents: Bounds<i64>,
    /// How many hours after publishing the bidding deadline may be.
    bidding_hours: Bounds<i64>,
    posting_window_days: PostingWindowDays,
    title_max_chars: usize,
    briefing_max_chars: usize,
}

/// Inclusive bounds.
#[derive(Serialize)]
struct Bounds<T> {
    min: T,
    max: T,
}

impl<T: PartialOrd> Bounds<T> {
    fn contains(&self, value: &T) -> bool {
        self.min <= *value && *value <= self.max
    }
}

/// A new draft's posting window, and the longest allowed. The shortest is one day.
#[derive(Serialize)]
struct PostingWindowDays {
    default: i16,
    max: i16,
}

impl Limits {
    pub(super) fn new(settings: &Settings) -> Self {
        let limits = &settings.limits;
        Self {
            budget_cents: Bounds {
                min: settings.rules.min_budget_cents(),
                max: MAX_BUDGET_CENTS,
            },
            target_cpm_cents: Bounds {
                min: 1,
                max: MAX_TARGET_CPM_CENTS,
            },
            bidding_hours: Bounds {
                min: limits.min_bidding_hours,
                max: limits.max_bidding_hours,
            },
            posting_window_days: PostingWindowDays {
                default: limits.posting_window_default_days,
                max: limits.posting_window_max_days,
            },
            title_max_chars: TITLE_MAX_CHARS,
            briefing_max_chars: BRIEFING_MAX_CHARS,
        }
    }
}

/// Creates an empty draft with the default posting window; the editor fills it in by autosave.
pub async fn create(
    State(pool): State<PgPool>,
    State(settings): State<Settings>,
    Path(advertiser_id): Path<i64>,
) -> Result<(StatusCode, Json<CampaignDetail>), ApiError> {
    ensure_advertiser_exists(&pool, advertiser_id).await?;
    let campaign = sqlx::query_as(
        "INSERT INTO campaigns (advertiser_id, submission_window_days) VALUES ($1, $2)
         RETURNING *",
    )
    .bind(advertiser_id)
    .bind(settings.limits.posting_window_default_days)
    .fetch_one(&pool)
    .await?;
    let row = CampaignRow {
        campaign,
        country_codes: Vec::new(),
        language_codes: Vec::new(),
        genre_ids: Vec::new(),
    };
    Ok((
        StatusCode::CREATED,
        Json(CampaignDetail::new(row, Utc::now())),
    ))
}

/// The advertiser's campaigns, most recently updated first.
pub async fn list(
    State(pool): State<PgPool>,
    Path(advertiser_id): Path<i64>,
) -> Result<Json<Vec<CampaignDetail>>, ApiError> {
    ensure_advertiser_exists(&pool, advertiser_id).await?;
    let rows: Vec<CampaignRow> = sqlx::query_as(
        "SELECT * FROM campaign_details WHERE advertiser_id = $1
         ORDER BY updated_at DESC, id DESC",
    )
    .bind(advertiser_id)
    .fetch_all(&pool)
    .await?;
    let now = Utc::now();
    let campaigns = rows
        .into_iter()
        .map(|row| CampaignDetail::new(row, now))
        .collect();
    Ok(Json(campaigns))
}

pub async fn show(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<CampaignDetail>, ApiError> {
    let row = find_campaign(&pool, id).await?;
    Ok(Json(CampaignDetail::new(row, Utc::now())))
}

/// Autosave: applies all the changes, or none if any is invalid. A save that changes nothing
/// writes nothing, so that it does not move the draft up the advertiser's list.
pub async fn update(
    State(pool): State<PgPool>,
    State(settings): State<Settings>,
    Path(id): Path<i64>,
    Json(changes): Json<CampaignChanges>,
) -> Result<Json<CampaignDetail>, ApiError> {
    let mut tx = pool.begin().await?;
    let stored = lock_draft(&mut tx, id).await?;
    let mut edited = stored.clone();
    changes.apply(&mut edited);
    if edited != stored {
        let mut fields = invalid_values(&edited, &Limits::new(&settings));
        fields.extend(unknown_targeting(&mut tx, &stored, &edited).await?);
        if !fields.is_empty() {
            return Err(ApiError::Validation(fields));
        }
        write_values(&mut tx, id, &edited).await?;
        replace_targeting(&mut tx, id, &stored, &edited).await?;
    }
    let row = find_campaign(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(Json(CampaignDetail::new(row, Utc::now())))
}

/// Publishes a complete draft, which freezes its deal: the current rules and the offer they give
/// its target CPM.
pub async fn publish(
    State(pool): State<PgPool>,
    State(settings): State<Settings>,
    Path(id): Path<i64>,
    publication: Option<Json<Publication>>,
) -> Result<Json<CampaignDetail>, ApiError> {
    let deadline = publication.and_then(|Json(publication)| publication.bidding_deadline);

    let mut tx = pool.begin().await?;
    let draft = lock_draft(&mut tx, id).await?;
    let now = Utc::now();
    let limits = Limits::new(&settings);
    let mut fields = invalid_values(&draft, &limits);
    fields.extend(missing_fields(&draft));
    let deadline_problem = match deadline {
        Some(deadline) => deadline_error(deadline, now, &limits.bidding_hours),
        None => Some("Required.".to_owned()),
    };
    if let Some(problem) = deadline_problem {
        fields.insert("biddingDeadline", problem);
    }
    if !fields.is_empty() {
        return Err(ApiError::Validation(fields));
    }
    let offer_bps = draft
        .target_cpm_cents
        .and_then(|target| settings.offer_bps(target, &draft.size_groups));
    // Without field errors the draft has a deadline, a target CPM and a planned size, which give
    // it an offer, so the `else` cannot happen.
    let (Some(deadline), Some(offer_bps)) = (deadline, offer_bps) else {
        return Err(ApiError::Validation(fields));
    };

    let campaign =
        db::campaigns::publish(&mut *tx, id, deadline, now, &settings.rules, offer_bps).await?;
    tx.commit().await?;
    // The draft is locked, so the targeting read with it is still current.
    let row = CampaignRow {
        campaign,
        country_codes: draft.country_codes,
        language_codes: draft.language_codes,
        genre_ids: draft.genre_ids,
    };
    Ok(Json(CampaignDetail::new(row, now)))
}

pub(super) async fn find_campaign(
    executor: impl PgExecutor<'_>,
    id: i64,
) -> Result<CampaignRow, ApiError> {
    sqlx::query_as("SELECT * FROM campaign_details WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| ApiError::not_found("Campaign", id))
}

/// The error once a query for the campaign in a required state finds nothing: 404 if the
/// campaign does not exist, otherwise 409 with `conflict`.
pub(super) async fn missing_or_conflict(
    executor: impl PgExecutor<'_>,
    campaign_id: i64,
    conflict: String,
) -> ApiError {
    let exists: sqlx::Result<bool> =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM campaigns WHERE id = $1)")
            .bind(campaign_id)
            .fetch_one(executor)
            .await;
    match exists {
        Ok(true) => ApiError::Conflict(conflict),
        Ok(false) => ApiError::not_found("Campaign", campaign_id),
        Err(error) => error.into(),
    }
}

async fn ensure_advertiser_exists(executor: impl PgExecutor<'_>, id: i64) -> Result<(), ApiError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM advertisers WHERE id = $1)")
            .bind(id)
            .fetch_one(executor)
            .await?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::not_found("Advertiser", id))
    }
}

/// Locks the campaign, so that saves and publishing take turns, and returns it if still a draft.
async fn lock_draft(conn: &mut PgConnection, id: i64) -> Result<DraftValues, ApiError> {
    let status: CampaignStatus =
        sqlx::query_scalar("SELECT status FROM campaigns WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| ApiError::not_found("Campaign", id))?;
    if status != CampaignStatus::Draft {
        return Err(ApiError::Conflict(format!(
            "Campaign {id} is already published; only drafts can be changed."
        )));
    }
    let draft = sqlx::query_as("SELECT * FROM campaign_details WHERE id = $1")
        .bind(id)
        .fetch_one(conn)
        .await?;
    Ok(draft)
}

async fn write_values(conn: &mut PgConnection, id: i64, draft: &DraftValues) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE campaigns SET
             title = $2, briefing = $3, platform = $4, budget_cents = $5, target_cpm_cents = $6,
             size_groups = $7, engagement_weight = $8, quality_weight = $9,
             reliability_weight = $10, submission_window_days = $11, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&draft.title)
    .bind(&draft.briefing)
    .bind(draft.platform)
    .bind(draft.budget_cents)
    .bind(draft.target_cpm_cents)
    .bind(&draft.size_groups)
    .bind(draft.engagement_weight)
    .bind(draft.quality_weight)
    .bind(draft.reliability_weight)
    .bind(draft.submission_window_days)
    .execute(conn)
    .await?;
    Ok(())
}

/// Rewrites each targeting list the save changes. The values are already unique.
async fn replace_targeting(
    conn: &mut PgConnection,
    id: i64,
    stored: &DraftValues,
    edited: &DraftValues,
) -> sqlx::Result<()> {
    if edited.country_codes != stored.country_codes {
        sqlx::query("DELETE FROM campaign_countries WHERE campaign_id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "INSERT INTO campaign_countries (campaign_id, country_code)
             SELECT $1, unnest($2::text[])",
        )
        .bind(id)
        .bind(&edited.country_codes)
        .execute(&mut *conn)
        .await?;
    }
    if edited.language_codes != stored.language_codes {
        sqlx::query("DELETE FROM campaign_languages WHERE campaign_id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "INSERT INTO campaign_languages (campaign_id, language_code)
             SELECT $1, unnest($2::text[])",
        )
        .bind(id)
        .bind(&edited.language_codes)
        .execute(&mut *conn)
        .await?;
    }
    if edited.genre_ids != stored.genre_ids {
        sqlx::query("DELETE FROM campaign_genres WHERE campaign_id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "INSERT INTO campaign_genres (campaign_id, genre_id)
             SELECT $1, unnest($2::smallint[])",
        )
        .bind(id)
        .bind(&edited.genre_ids)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Field errors that apply to any draft, complete or not.
fn invalid_values(draft: &DraftValues, limits: &Limits) -> FieldErrors {
    let mut fields = FieldErrors::new();
    if longer_than(draft.title.as_deref(), limits.title_max_chars) {
        let message = format!("At most {} characters.", limits.title_max_chars);
        fields.insert("title", message);
    }
    if longer_than(draft.briefing.as_deref(), limits.briefing_max_chars) {
        let message = format!("At most {} characters.", limits.briefing_max_chars);
        fields.insert("briefing", message);
    }
    if draft
        .budget_cents
        .is_some_and(|budget| !limits.budget_cents.contains(&budget))
    {
        fields.insert("budgetCents", between_euros(&limits.budget_cents));
    }
    if draft
        .target_cpm_cents
        .is_some_and(|target| !limits.target_cpm_cents.contains(&target))
    {
        fields.insert("targetCpmCents", between_euros(&limits.target_cpm_cents));
    }
    if !one_or_more_planned_sizes_each_once(&draft.size_groups) {
        let message = "Pick one or more of nano, micro, macro and mega, each once.";
        fields.insert("sizeGroups", message.to_owned());
    }
    for (field, weight) in [
        ("engagementWeight", draft.engagement_weight),
        ("qualityWeight", draft.quality_weight),
        ("reliabilityWeight", draft.reliability_weight),
    ] {
        if !SLIDER_STEPS.contains(&weight) {
            fields.insert(field, "Must be 0, 25, 50, 75 or 100.".to_owned());
        }
    }
    let max_days = limits.posting_window_days.max;
    if !(1..=max_days).contains(&draft.submission_window_days) {
        let message = format!("Must be between 1 and {max_days} days.");
        fields.insert("submissionWindowDays", message);
    }
    fields
}

/// The fields a draft may leave empty but a published campaign may not.
fn missing_fields(draft: &DraftValues) -> FieldErrors {
    [
        ("title", draft.title.is_none()),
        ("briefing", draft.briefing.is_none()),
        ("platform", draft.platform.is_none()),
        ("budgetCents", draft.budget_cents.is_none()),
        ("targetCpmCents", draft.target_cpm_cents.is_none()),
    ]
    .into_iter()
    .filter(|&(_, missing)| missing)
    .map(|(field, _)| (field, "Required.".to_owned()))
    .collect()
}

/// The deadline must be within the bidding hours of `now`, when publishing opens bidding.
fn deadline_error(
    deadline: DateTime<Utc>,
    now: DateTime<Utc>,
    hours: &Bounds<i64>,
) -> Option<String> {
    let earliest = now + TimeDelta::hours(hours.min);
    let latest = now + TimeDelta::hours(hours.max);
    if (earliest..=latest).contains(&deadline) {
        None
    } else {
        Some(format!(
            "Must be between {} and {} hours from now.",
            hours.min, hours.max
        ))
    }
}

/// A field error for each changed targeting list naming an unknown country, language or genre.
async fn unknown_targeting(
    conn: &mut PgConnection,
    stored: &DraftValues,
    edited: &DraftValues,
) -> sqlx::Result<FieldErrors> {
    let mut fields = FieldErrors::new();
    if edited.country_codes != stored.country_codes {
        let unknown: Vec<String> = sqlx::query_scalar(
            "SELECT unnest($1::text[]) EXCEPT SELECT code FROM countries ORDER BY 1",
        )
        .bind(&edited.country_codes)
        .fetch_all(&mut *conn)
        .await?;
        insert_unknown(&mut fields, "countryCodes", &unknown);
    }
    if edited.language_codes != stored.language_codes {
        let unknown: Vec<String> = sqlx::query_scalar(
            "SELECT unnest($1::text[]) EXCEPT SELECT code FROM languages ORDER BY 1",
        )
        .bind(&edited.language_codes)
        .fetch_all(&mut *conn)
        .await?;
        insert_unknown(&mut fields, "languageCodes", &unknown);
    }
    if edited.genre_ids != stored.genre_ids {
        let unknown: Vec<i16> = sqlx::query_scalar(
            "SELECT unnest($1::smallint[]) EXCEPT SELECT id FROM genres ORDER BY 1",
        )
        .bind(&edited.genre_ids)
        .fetch_all(&mut *conn)
        .await?;
        insert_unknown(&mut fields, "genreIds", &unknown);
    }
    Ok(fields)
}

fn insert_unknown(fields: &mut FieldErrors, field: &'static str, unknown: &[impl ToString]) {
    if !unknown.is_empty() {
        let names: Vec<String> = unknown.iter().map(ToString::to_string).collect();
        fields.insert(field, format!("Unknown: {}.", names.join(", ")));
    }
}

/// Expects sorted sizes, so that a size picked twice shows up as two neighbours.
fn one_or_more_planned_sizes_each_once(sizes: &[SizeGroup]) -> bool {
    !sizes.is_empty()
        && sizes.iter().all(|size| PLANNED_GROUPS.contains(size))
        && sizes.windows(2).all(|pair| pair[0] != pair[1])
}

fn longer_than(text: Option<&str>, max_chars: usize) -> bool {
    text.is_some_and(|text| text.chars().count() > max_chars)
}

/// Trims the text, clearing it if blank.
fn trim(text: &mut Option<String>) {
    *text = text
        .take()
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());
}

fn sorted_once<T: Ord + Clone>(values: &[T]) -> Vec<T> {
    let mut values = values.to_vec();
    values.sort_unstable();
    values.dedup();
    values
}

fn between_euros(bounds: &Bounds<i64>) -> String {
    format!(
        "Must be between {} and {}.",
        euros(bounds.min),
        euros(bounds.max)
    )
}
