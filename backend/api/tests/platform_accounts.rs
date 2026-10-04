mod common;

use axum::http::StatusCode;
use common::{
    AccountSpec, RELIABILITY, add_genres, add_languages, get, insert_account, insert_account_with,
    insert_creator,
};
use db::models::Platform;
use serde_json::json;
use sqlx::PgPool;

/// The account ids of one page, and the cursor of the next.
async fn page(pool: &PgPool, query: &str) -> (Vec<i64>, Option<String>) {
    let (status, body) = get(pool, &format!("/api/platform-accounts?{query}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_i64().unwrap())
        .collect();
    (ids, body["nextCursor"].as_str().map(str::to_owned))
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn filters_by_platform_country_language_and_genre(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let dancer = insert_account(&pool, creator, "lena.moves", 12_500).await?;
    let cook = insert_account(&pool, creator, "lena.cooks", 9_000).await?;
    let in_austria = AccountSpec {
        view_score: 8_000,
        country_code: "AT",
        ..AccountSpec::default()
    };
    let austrian = insert_account_with(&pool, creator, "lena.vienna", in_austria).await?;
    let on_instagram = AccountSpec {
        platform: Platform::Instagram,
        view_score: 7_000,
        ..AccountSpec::default()
    };
    let instagram = insert_account_with(&pool, creator, "lena.insta", on_instagram).await?;
    add_genres(&pool, dancer, &[14, 5]).await?;
    add_genres(&pool, cook, &[6]).await?;
    add_genres(&pool, austrian, &[5]).await?;
    add_languages(&pool, dancer, &["en", "de"]).await?;
    add_languages(&pool, cook, &["de"]).await?;

    assert_eq!(page(&pool, "platform=instagram").await.0, [instagram]);
    assert_eq!(page(&pool, "country=AT").await.0, [austrian]);
    assert_eq!(page(&pool, "language=de").await.0, [dancer, cook]);
    assert_eq!(page(&pool, "genre=5").await.0, [dancer, austrian]);
    assert_eq!(page(&pool, "genre=5&country=DE").await.0, [dancer]);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn view_and_engagement_ranges_include_both_ends(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let big = insert_account(&pool, creator, "lena.moves", 12_500).await?;
    let mid = insert_account(&pool, creator, "lena.cooks", 9_000).await?;
    let small = insert_account(&pool, creator, "lena.vienna", 800).await?;
    sqlx::query("UPDATE platform_accounts SET engagement_rate = 0.12 WHERE id = $1")
        .bind(mid)
        .execute(&pool)
        .await?;

    assert_eq!(page(&pool, "min_views=9000").await.0, [big, mid]);
    assert_eq!(page(&pool, "max_views=9000").await.0, [mid, small]);
    assert_eq!(page(&pool, "min_views=1000&max_views=10000").await.0, [mid]);
    assert_eq!(page(&pool, "min_engagement=0.1").await.0, [mid]);
    assert_eq!(page(&pool, "max_engagement=0.064").await.0, [big, small]);
    let (status, _) = get(&pool, "/api/platform-accounts?min_views=lots").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_matches_the_handle_or_the_creator_name(pool: PgPool) -> sqlx::Result<()> {
    let lena = insert_creator(&pool, "Lena Vogt").await?;
    let max = insert_creator(&pool, "Max Fischer").await?;
    let by_name = insert_account(&pool, lena, "dance_daily", 12_500).await?;
    let by_handle = insert_account(&pool, max, "max.vogtland", 9_000).await?;
    let by_both = insert_account(&pool, lena, "lena.vogt", 8_000).await?;
    let neither = insert_account(&pool, max, "max.cooks", 7_000).await?;

    assert_eq!(page(&pool, "q=VOGT").await.0, [by_name, by_handle, by_both]);
    // Too short to search, so ignored.
    assert_eq!(
        page(&pool, "q=vo").await.0,
        [by_name, by_handle, by_both, neither]
    );
    // LIKE wildcards match literally: `_` is not "any character".
    assert!(page(&pool, "q=x_c").await.0.is_empty());

    let (first, cursor) = page(&pool, "q=vogt&limit=2").await;
    let (second, cursor) = page(&pool, &format!("q=vogt&limit=2&cursor={}", cursor.unwrap())).await;
    assert_eq!([first, second].concat(), [by_name, by_handle, by_both]);
    assert_eq!(cursor, None);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pages_follow_each_other_without_gaps_or_repeats(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let top = insert_account(&pool, creator, "a", 300).await?;
    let tied_first = insert_account(&pool, creator, "b", 200).await?;
    let tied_second = insert_account(&pool, creator, "c", 200).await?;
    let bottom = insert_account(&pool, creator, "d", 100).await?;

    let (first, cursor) = page(&pool, "limit=2").await;
    let (second, cursor) = page(&pool, &format!("limit=2&cursor={}", cursor.unwrap())).await;

    // Ties go to the higher id, so the tie can straddle the two pages.
    assert_eq!(first, [top, tied_second]);
    assert_eq!(second, [tied_first, bottom]);
    assert_eq!(cursor, None);

    let (status, body) = get(&pool, "/api/platform-accounts?cursor=nonsense").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "bad_request");
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn shows_one_account_as_the_list_does(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let account = insert_account(&pool, creator, "lena.moves", 12_500).await?;
    add_genres(&pool, account, &[14, 5]).await?;
    add_languages(&pool, account, &["en", "de"]).await?;

    let (status, shown) = get(&pool, &format!("/api/platform-accounts/{account}")).await;

    assert_eq!(status, StatusCode::OK, "{shown}");
    let (_, list) = get(&pool, "/api/platform-accounts").await;
    assert_eq!(shown, list["items"][0]);
    assert_eq!(shown["id"], account);
    assert_eq!(shown["handle"], "lena.moves");
    assert_eq!(shown["platform"], "tiktok");
    assert_eq!(shown["creatorName"], "Lena Vogt");
    assert_eq!(shown["reliabilityScore"], RELIABILITY);
    assert_eq!(shown["genreIds"], json!([5, 14]));
    assert_eq!(shown["languageCodes"], json!(["de", "en"]));

    let (status, error) = get(&pool, "/api/platform-accounts/999").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
    assert_eq!(error["error"], "not_found");
    Ok(())
}
