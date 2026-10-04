mod common;

use axum::http::{Method, StatusCode};
use common::{add_languages, get, new_account, router_with, send, test_config};
use db::config::{CampaignLimits, Settings};
use marketplace::{Rules, VIEWS_COUNTING_DAYS};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn options_list_genres_and_the_countries_and_languages_in_use(
    pool: PgPool,
) -> sqlx::Result<()> {
    // Based in Germany.
    let account = new_account(&pool, "lena.moves", 12_500).await?;
    add_languages(&pool, account, &["de"]).await?;

    let (status, body) = get(&pool, "/api/options").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body["genres"].as_array().unwrap().is_empty());
    assert_eq!(
        body["countries"],
        json!([{ "code": "DE", "name": "Germany" }])
    );
    assert_eq!(
        body["languages"],
        json!([{ "code": "de", "name": "German" }])
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn options_send_the_campaign_limits_from_the_settings(pool: PgPool) {
    let settings = Settings {
        rules: Rules {
            usual_rate_base_cents: 8_000,
            ..test_config().rules
        },
        limits: CampaignLimits {
            min_bidding_hours: 12,
            max_bidding_hours: 240,
            posting_window_default_days: 2,
            posting_window_max_days: 5,
        },
        ..test_config()
    };
    let router = router_with(&pool, settings.clone(), false);

    let (status, body) = send(router, Method::GET, "/api/options", None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["limits"],
        json!({
            // Fixed by the API, not the settings: €10,000,000 and €1,000.
            "budgetCents": { "min": settings.rules.min_budget_cents(), "max": 1_000_000_000 },
            "targetCpmCents": { "min": 1, "max": 100_000 },
            "biddingHours": { "min": 12, "max": 240 },
            "postingWindowDays": { "default": 2, "max": 5 },
            "titleMaxChars": 120,
            "briefingMaxChars": 5000,
        })
    );
    assert_eq!(body["viewsCountingDays"], VIEWS_COUNTING_DAYS);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn options_send_the_views_of_each_account_size(pool: PgPool) {
    let (_, body) = get(&pool, "/api/options").await;

    assert_eq!(
        body["sizeGroups"],
        json!([
            { "group": "nano", "minViews": 500, "maxViews": 2_999 },
            { "group": "micro", "minViews": 3_000, "maxViews": 29_999 },
            { "group": "macro", "minViews": 30_000, "maxViews": 299_999 },
            { "group": "mega", "minViews": 300_000, "maxViews": null },
        ])
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn options_say_whether_the_demo_tools_are_on(pool: PgPool) {
    let (_, body) = send(common::router(&pool), Method::GET, "/api/options", None).await;
    assert_eq!(body["devTools"], false);

    let (_, body) = send(common::dev_router(&pool), Method::GET, "/api/options", None).await;
    assert_eq!(body["devTools"], true);
}
