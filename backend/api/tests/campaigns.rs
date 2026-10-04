mod common;

use axum::http::{Method, StatusCode};
use chrono::{SubsecRound as _, TimeDelta, Utc};
use common::{
    CampaignSpec, get, insert_advertiser, insert_campaign, new_account, patch, post, router_with,
    send, test_config,
};
use db::config::{CampaignLimits, Settings};
use marketplace::{Rules, SizeGroup};
use serde_json::{Value, json};
use sqlx::PgPool;

/// Every field publishing requires, plus some optional ones.
fn complete_campaign() -> Value {
    json!({
        "title": "Summer serum launch",
        "briefing": "Show your morning routine with the serum.",
        "platform": "tiktok",
        "budgetCents": 1_000_000,
        "targetCpmCents": 1200,
        "engagementWeight": 75,
        "countryCodes": ["DE", "IT"],
        "languageCodes": ["de"],
        "genreIds": [2, 7],
    })
}

async fn new_draft(pool: &PgPool) -> Value {
    let advertiser = insert_advertiser(pool, "Glow Labs").await.unwrap();
    let uri = format!("/api/advertisers/{advertiser}/campaigns");
    let (status, draft) = post(pool, &uri, None).await;
    assert_eq!(status, StatusCode::CREATED, "{draft}");
    draft
}

/// Creates a draft and autosaves the changes to it.
async fn draft_with(pool: &PgPool, changes: &Value) -> Value {
    let draft = new_draft(pool).await;
    let (status, saved) = patch(pool, &campaign_uri(&draft), changes).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    saved
}

fn campaign_uri(campaign: &Value) -> String {
    format!("/api/campaigns/{}", campaign["id"])
}

/// Publishes the draft with these settings and a bidding deadline `away` from now.
async fn publish_in(
    pool: &PgPool,
    draft: &Value,
    settings: Settings,
    away: TimeDelta,
) -> (StatusCode, Value) {
    let uri = format!("/api/campaigns/{}/publish", draft["id"]);
    let body = json!({ "biddingDeadline": Utc::now() + away });
    send(
        router_with(pool, settings, false),
        Method::POST,
        &uri,
        Some(&body),
    )
    .await
}

fn field_names(error: &Value) -> Vec<&str> {
    error["fields"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_new_draft_shows_the_defaults(pool: PgPool) {
    let draft = new_draft(&pool).await;

    assert_eq!(draft["status"], "draft");
    assert_eq!(draft["phase"], "draft");
    assert_eq!(draft["title"], Value::Null);
    assert_eq!(draft["sizeGroups"], json!(["nano", "micro", "macro"]));
    assert_eq!(
        draft["submissionWindowDays"],
        test_config().limits.posting_window_default_days
    );
    assert_eq!(draft["countryCodes"], json!([]));
    // The commission is part of the deal, which publishing sets.
    assert_eq!(draft["commissionBps"], Value::Null);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unknown_advertiser_is_not_found(pool: PgPool) {
    let (status, body) = post(&pool, "/api/advertisers/999/campaigns", None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn autosave_changes_only_the_given_fields_and_null_clears(pool: PgPool) {
    let draft = draft_with(&pool, &complete_campaign()).await;
    let uri = campaign_uri(&draft);

    let changes = json!({
        "budgetCents": null,
        "sizeGroups": ["mega", "micro"],
        "countryCodes": ["AT"],
    });
    let (status, saved) = patch(&pool, &uri, &changes).await;

    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["title"], "Summer serum launch");
    assert_eq!(saved["budgetCents"], Value::Null);
    assert_eq!(saved["sizeGroups"], json!(["micro", "mega"]));
    assert_eq!(saved["countryCodes"], json!(["AT"]));
    assert_eq!(saved["genreIds"], json!([2, 7]));
    assert_eq!(get(&pool, &uri).await.1, saved);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn autosave_trims_text_and_keeps_each_targeting_value_once(pool: PgPool) {
    let changes = json!({
        "title": "  Summer serum launch ",
        "briefing": "   ",
        "countryCodes": ["IT", "DE", "DE"],
        "genreIds": [7, 2, 7],
    });

    let saved = draft_with(&pool, &changes).await;

    assert_eq!(saved["title"], "Summer serum launch");
    assert_eq!(saved["briefing"], Value::Null);
    assert_eq!(saved["countryCodes"], json!(["DE", "IT"]));
    assert_eq!(saved["genreIds"], json!([2, 7]));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn an_autosave_that_changes_nothing_writes_nothing(pool: PgPool) {
    let draft = draft_with(&pool, &complete_campaign()).await;
    let uri = campaign_uri(&draft);

    // No change, or the same values: the draft keeps its place in the advertiser's list.
    for changes in [json!({}), complete_campaign()] {
        let (status, saved) = patch(&pool, &uri, &changes).await;
        assert_eq!(status, StatusCode::OK, "{saved}");
        assert_eq!(saved, draft);
    }

    let (_, saved) = patch(&pool, &uri, &json!({ "title": "Winter serum launch" })).await;
    assert_ne!(saved["updatedAt"], draft["updatedAt"]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn autosave_saves_nothing_if_any_field_is_invalid(pool: PgPool) {
    let draft = draft_with(&pool, &complete_campaign()).await;
    let uri = campaign_uri(&draft);

    let changes = json!({
        "title": "Winter serum launch",
        "qualityWeight": 60,
        "countryCodes": ["DE", "XX"],
    });
    let (status, error) = patch(&pool, &uri, &changes).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
    assert_eq!(error["error"], "validation");
    assert_eq!(field_names(&error), ["countryCodes", "qualityWeight"]);
    assert_eq!(get(&pool, &uri).await.1, draft);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn each_invalid_value_has_a_field_error(pool: PgPool) {
    let uri = campaign_uri(&new_draft(&pool).await);
    let min_budget = test_config().rules.min_budget_cents();
    // The API's highest budget, €10,000,000, and highest target CPM, €1,000.
    let (max_budget, max_target) = (1_000_000_000, 100_000);
    let budget_error = "Must be between €360 and €10,000,000.";
    let target_error = "Must be between €0.01 and €1,000.";
    let window_error = "Must be between 1 and 7 days.";

    for (field, value, message) in [
        ("title", json!("x".repeat(121)), "At most 120 characters."),
        (
            "briefing",
            json!("x".repeat(5001)),
            "At most 5000 characters.",
        ),
        ("budgetCents", json!(min_budget - 1), budget_error),
        ("budgetCents", json!(max_budget + 1), budget_error),
        ("targetCpmCents", json!(0), target_error),
        ("targetCpmCents", json!(max_target + 1), target_error),
        (
            "engagementWeight",
            json!(60),
            "Must be 0, 25, 50, 75 or 100.",
        ),
        ("submissionWindowDays", json!(0), window_error),
        ("submissionWindowDays", json!(8), window_error),
        ("countryCodes", json!(["DE", "XX"]), "Unknown: XX."),
        ("languageCodes", json!(["de", "xx"]), "Unknown: xx."),
        ("genreIds", json!([2, 999]), "Unknown: 999."),
    ] {
        let (status, error) = patch(&pool, &uri, &json!({ field: value })).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}: {error}");
        assert_eq!(error["fields"], json!({ field: message }), "{field}");
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_campaign_picks_one_or_more_planned_sizes_each_once(pool: PgPool) {
    let uri = campaign_uri(&new_draft(&pool).await);

    for sizes in [
        json!([]),
        json!(["starter", "nano"]),
        json!(["micro", "nano", "micro"]),
    ] {
        let (status, error) = patch(&pool, &uri, &json!({ "sizeGroups": sizes })).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{sizes}: {error}");
        assert_eq!(
            error["fields"],
            json!({ "sizeGroups": "Pick one or more of nano, micro, macro and mega, each once." })
        );
    }

    let (status, saved) = patch(&pool, &uri, &json!({ "sizeGroups": ["mega"] })).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["sizeGroups"], json!(["mega"]));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publishing_lists_every_missing_field(pool: PgPool) {
    let draft = draft_with(&pool, &json!({ "title": "Summer serum launch" })).await;

    let uri = format!("/api/campaigns/{}/publish", draft["id"]);
    let (status, error) = post(&pool, &uri, None).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
    assert_eq!(
        field_names(&error),
        [
            "biddingDeadline",
            "briefing",
            "budgetCents",
            "platform",
            "targetCpmCents"
        ]
    );
    assert_eq!(error["fields"]["briefing"], "Required.");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publishing_needs_a_deadline_within_the_bidding_hours(pool: PgPool) {
    let draft = draft_with(&pool, &complete_campaign()).await;
    // Bidding open for 2 to 48 hours, instead of the usual 24 to 720.
    let limits = CampaignLimits {
        min_bidding_hours: 2,
        max_bidding_hours: 48,
        ..test_config().limits
    };
    let settings = Settings {
        limits,
        ..test_config()
    };

    for away in [TimeDelta::minutes(90), TimeDelta::hours(49)] {
        let (status, error) = publish_in(&pool, &draft, settings.clone(), away).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{away}: {error}");
        assert_eq!(
            error["fields"],
            json!({ "biddingDeadline": "Must be between 2 and 48 hours from now." })
        );
    }
    assert_eq!(get(&pool, &campaign_uri(&draft)).await.1, draft);

    let (status, published) = publish_in(&pool, &draft, settings, TimeDelta::hours(3)).await;
    assert_eq!(status, StatusCode::OK, "{published}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publishing_opens_bidding_and_ends_editing(pool: PgPool) {
    let draft = draft_with(&pool, &complete_campaign()).await;
    let id = &draft["id"];
    let uri = format!("/api/campaigns/{id}/publish");
    assert_eq!(draft["biddingDeadline"], Value::Null);

    let deadline = (Utc::now() + TimeDelta::days(5)).trunc_subsecs(0);
    let body = json!({ "biddingDeadline": deadline });
    let (status, published) = post(&pool, &uri, Some(&body)).await;

    assert_eq!(status, StatusCode::OK, "{published}");
    assert_eq!(published["status"], "active");
    assert_eq!(published["phase"], "open");
    assert_eq!(published["biddingDeadline"], json!(deadline));
    assert!(published["submittedAt"].is_string());
    assert!(published["activatedAt"].is_string());
    assert_eq!(published["countryCodes"], json!(["DE", "IT"]));

    let changes = json!({ "title": "Too late" });
    let (status, error) = patch(&pool, &campaign_uri(&draft), &changes).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"], "conflict");
    assert_eq!(post(&pool, &uri, Some(&body)).await.0, StatusCode::CONFLICT);
}

/// [`test_config`] with every deal rule and the minimum offer changed.
fn changed_settings() -> Settings {
    Settings {
        rules: Rules {
            commission_bps: 2_000,
            usual_rate_base_cents: 8_000,
            usual_rate_per_1000_views_cents: 1_100,
            fair_pay_floor_bps: 4_000,
            max_bid_share_of_budget_bps: 2_200,
            views_to_get_paid_bps: 3_000,
            likely_views_limit_bps: 6_500,
            discovery_bps: 1_000,
        },
        min_offer_bps: 4_500,
        ..test_config()
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publishing_freezes_the_deal_rules(pool: PgPool) -> sqlx::Result<()> {
    let draft = draft_with(&pool, &complete_campaign()).await;
    let id = draft["id"].as_i64().unwrap();
    let settings = changed_settings();

    let away = TimeDelta::days(5);
    let (status, published) = publish_in(&pool, &draft, settings.clone(), away).await;

    assert_eq!(status, StatusCode::OK, "{published}");
    assert_eq!(published["commissionBps"], settings.rules.commission_bps);
    let sizes = [SizeGroup::Nano, SizeGroup::Micro, SizeGroup::Macro];
    assert_eq!(
        db::selection::terms(&pool, id).await?,
        settings.terms(1_000_000, 1200, &sizes)
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_config_change_leaves_live_campaigns_alone(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = CampaignSpec::default();
    let published_with = campaign.terms();
    let campaign = insert_campaign(&pool, advertiser, campaign).await?;
    let views = 12_500;
    let account = new_account(&pool, "lena.moves", views).await?;
    let changed = changed_settings();
    assert_ne!(
        changed.rules.min_bid_cents(views),
        published_with.rules.min_bid_cents(views)
    );

    let uri = format!("/api/platform-accounts/{account}/campaigns");
    let (status, feed) = send(router_with(&pool, changed, false), Method::GET, &uri, None).await;

    assert_eq!(status, StatusCode::OK, "{feed}");
    let item = &feed["items"][0];
    assert_eq!(item["campaign"]["id"], campaign);
    assert_eq!(
        item["campaign"]["commissionBps"],
        published_with.rules.commission_bps
    );
    assert_eq!(
        item["minBidCents"],
        published_with.rules.min_bid_cents(views)
    );
    assert_eq!(
        item["maxBidCents"],
        published_with.rules.max_bid_cents(views)
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lists_an_advertisers_campaigns_most_recently_updated_first(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let uri = format!("/api/advertisers/{advertiser}/campaigns");
    let (_, first) = post(&pool, &uri, None).await;
    let (_, second) = post(&pool, &uri, None).await;
    let changes = json!({ "title": "Summer serum launch" });
    patch(&pool, &campaign_uri(&first), &changes).await;

    let (status, campaigns) = get(&pool, &uri).await;

    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&Value> = campaigns
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["id"])
        .collect();
    assert_eq!(ids, [&first["id"], &second["id"]]);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unreadable_body_is_a_bad_request(pool: PgPool) {
    let uri = campaign_uri(&new_draft(&pool).await);

    let (status, error) = patch(&pool, &uri, &json!({ "budgetCents": "a lot" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "bad_request");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("budgetCents"), "{message}");
}
