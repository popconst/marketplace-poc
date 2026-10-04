mod common;

use std::collections::BTreeMap;

use axum::http::StatusCode;
use chrono::TimeDelta;
use common::{
    AccountSpec, CampaignSpec, CampaignState, ENGAGEMENT_RATE, FOLLOWERS, add_genres,
    add_languages, get, insert_account_with, insert_advertiser, insert_campaign, insert_creator,
    new_account, patch, place_bid, post, set_state, test_config,
};
use db::models::Platform;
use marketplace::SizeGroup::{Macro, Mega, Micro, Nano};
use serde_json::{Value, json};
use sqlx::PgPool;

/// Views of the account whose feed the tests read: a micro account.
const VIEWS: i32 = 12_500;

/// Genre ids from the genres migration.
const MUSIC: i16 = 1;
const GAMING: i16 = 2;
const FOOD: i16 = 6;

fn closing_in_days(days: i64) -> CampaignSpec {
    CampaignSpec {
        closing_in: TimeDelta::days(days),
        ..CampaignSpec::default()
    }
}

/// Restricts the campaign to these countries and languages.
async fn target(
    pool: &PgPool,
    campaign: i64,
    countries: &[&str],
    languages: &[&str],
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO campaign_countries (campaign_id, country_code)
         SELECT $1, unnest($2::text[])",
    )
    .bind(campaign)
    .bind(countries)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO campaign_languages (campaign_id, language_code)
         SELECT $1, unnest($2::text[])",
    )
    .bind(campaign)
    .bind(languages)
    .execute(pool)
    .await?;
    Ok(())
}

async fn look_for(pool: &PgPool, campaign: i64, genre: i16) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO campaign_genres (campaign_id, genre_id) VALUES ($1, $2)")
        .bind(campaign)
        .bind(genre)
        .execute(pool)
        .await?;
    Ok(())
}

async fn feed_body(pool: &PgPool, account: i64, query: &str) -> Value {
    let uri = format!("/api/platform-accounts/{account}/campaigns?{query}");
    let (status, body) = get(pool, &uri).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn feed(pool: &PgPool, account: i64, query: &str) -> Vec<i64> {
    feed_body(pool, account, query).await["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["campaign"]["id"].as_i64().unwrap())
        .collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feed_shows_only_open_campaigns_the_account_matches(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    // A TikTok account in Germany, posting in German.
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    add_languages(&pool, account, &["de"]).await?;
    let anyone = insert_campaign(&pool, advertiser, closing_in_days(3)).await?;
    let targeted = insert_campaign(&pool, advertiser, closing_in_days(4)).await?;
    target(&pool, targeted, &["AT", "DE"], &["de", "en"]).await?;

    let elsewhere = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    target(&pool, elsewhere, &["AT"], &[]).await?;
    let in_french = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    target(&pool, in_french, &[], &["fr"]).await?;
    let on_instagram = CampaignSpec {
        platform: Platform::Instagram,
        ..CampaignSpec::default()
    };
    let no_micro = CampaignSpec {
        size_groups: vec![Nano, Macro, Mega],
        ..CampaignSpec::default()
    };
    // A bid may take 25% of €400, less than the account's min bid.
    let too_small = CampaignSpec {
        budget_cents: 40_000,
        ..CampaignSpec::default()
    };
    assert!(!too_small.terms().can_bid(VIEWS));
    for campaign in [on_instagram, no_micro, too_small] {
        insert_campaign(&pool, advertiser, campaign).await?;
    }
    for state in [
        CampaignState::Draft,
        CampaignState::Closed,
        CampaignState::PastDeadline,
    ] {
        let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
        set_state(&pool, campaign, state).await?;
    }

    // Equal matches, so the nearer deadline comes first.
    assert_eq!(feed(&pool, account, "").await, [anyone, targeted]);
    // No campaign can pick an account under 500 views.
    let starter = new_account(&pool, "lena.starts", 499).await?;
    assert!(feed(&pool, starter, "").await.is_empty());
    let creator = insert_creator(&pool, "Max Fischer").await?;
    let not_brand_safe = AccountSpec {
        brand_safe: false,
        ..AccountSpec::default()
    };
    let unsafe_account = insert_account_with(&pool, creator, "max.moves", not_brand_safe).await?;
    assert!(feed(&pool, unsafe_account, "").await.is_empty());
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_campaign_with_genres_takes_only_accounts_covering_one(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let any_genre = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let music_or_gaming = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    look_for(&pool, music_or_gaming, MUSIC).await?;
    look_for(&pool, music_or_gaming, GAMING).await?;
    let gamer = new_account(&pool, "lena.plays", VIEWS).await?;
    add_genres(&pool, gamer, &[GAMING, FOOD]).await?;
    let cook = new_account(&pool, "max.cooks", VIEWS).await?;
    add_genres(&pool, cook, &[FOOD]).await?;
    let min_bid_cents = test_config().rules.min_bid_cents(VIEWS);

    // One of the campaign's genres is enough.
    assert!(feed(&pool, gamer, "").await.contains(&music_or_gaming));
    let (status, bid) = place_bid(&pool, music_or_gaming, gamer, min_bid_cents).await;
    assert_eq!(status, StatusCode::OK, "{bid}");
    // The cook covers neither, so only the campaign without genres takes it.
    assert_eq!(feed(&pool, cook, "").await, [any_genre]);
    let (status, error) = place_bid(&pool, music_or_gaming, cook, min_bid_cents).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
    let (status, bid) = place_bid(&pool, any_genre, cook, min_bid_cents).await;
    assert_eq!(status, StatusCode::OK, "{bid}");
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feed_ranks_by_match_then_deadline(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    // Each campaign weighs one measure only, and reliability (88) beats quality (72).
    let weighing = |quality_weight, reliability_weight, days| CampaignSpec {
        engagement_weight: 0,
        quality_weight,
        reliability_weight,
        ..closing_in_days(days)
    };
    let quality_later = insert_campaign(&pool, advertiser, weighing(100, 0, 5)).await?;
    let reliability = insert_campaign(&pool, advertiser, weighing(0, 100, 4)).await?;
    let quality_sooner = insert_campaign(&pool, advertiser, weighing(100, 0, 3)).await?;

    assert_eq!(
        feed(&pool, account, "").await,
        [reliability, quality_sooner, quality_later]
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feed_sorts_and_filters_on_request(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    // Both genres, so that every campaign below is in the feed.
    add_genres(&pool, account, &[MUSIC, GAMING]).await?;
    // A higher target CPM pays the account more.
    let paying = |target_cpm_cents, days| CampaignSpec {
        target_cpm_cents,
        ..closing_in_days(days)
    };
    let (music, gaming, any) = (paying(1_200, 5), paying(1_800, 3), paying(2_400, 4));
    let gaming_terms = gaming.terms();
    let gaming_payout = gaming_terms
        .rules
        .payout_cents(gaming_terms.suggested_bid_cents(VIEWS));
    let music = insert_campaign(&pool, advertiser, music).await?;
    look_for(&pool, music, MUSIC).await?;
    let gaming = insert_campaign(&pool, advertiser, gaming).await?;
    look_for(&pool, gaming, GAMING).await?;
    let any = insert_campaign(&pool, advertiser, any).await?;

    assert_eq!(
        feed(&pool, account, "sort=payout").await,
        [any, gaming, music]
    );
    assert_eq!(
        feed(&pool, account, "sort=deadline").await,
        [gaming, any, music]
    );
    let at_least_gaming = format!("sort=deadline&min_payout_cents={gaming_payout}");
    assert_eq!(feed(&pool, account, &at_least_gaming).await, [gaming, any]);
    // A campaign that looks for no genre takes any.
    let music_only = format!("sort=deadline&genre={MUSIC}");
    assert_eq!(feed(&pool, account, &music_only).await, [any, music]);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feed_item_shows_the_campaigns_limits_and_the_accounts_own_bid(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let other_account = new_account(&pool, "max.moves", VIEWS).await?;
    // Different targets, so the two campaigns suggest different bids. The max bid depends only on
    // the account, so it is the same on both, whatever their budgets.
    let modest = CampaignSpec {
        target_cpm_cents: 1_200,
        ..closing_in_days(3)
    };
    let generous = CampaignSpec {
        target_cpm_cents: 2_800,
        budget_cents: 100_000,
        ..closing_in_days(4)
    };
    let terms = [modest.terms(), generous.terms()];
    let campaigns = [
        insert_campaign(&pool, advertiser, modest).await?,
        insert_campaign(&pool, advertiser, generous).await?,
    ];
    let amount = terms[0].suggested_bid_cents(VIEWS);
    let (status, bid) = place_bid(&pool, campaigns[0], account, amount).await;
    assert_eq!(status, StatusCode::OK, "{bid}");
    let (status, _) = place_bid(&pool, campaigns[1], other_account, amount).await;
    assert_eq!(status, StatusCode::OK);

    let body = feed_body(&pool, account, "").await;

    assert_eq!(
        body["account"],
        json!({ "viewScore": VIEWS, "followers": FOLLOWERS, "engagementRate": ENGAGEMENT_RATE })
    );
    let items = body["items"].as_array().unwrap();
    for ((item, campaign), terms) in items.iter().zip(campaigns).zip(&terms) {
        let suggested = terms.suggested_bid_cents(VIEWS);
        assert_eq!(item["campaign"]["id"], campaign);
        assert_eq!(item["minBidCents"], terms.rules.min_bid_cents(VIEWS));
        assert_eq!(item["maxBidCents"], terms.rules.max_bid_cents(VIEWS));
        assert_eq!(item["suggestedBidCents"], suggested);
        assert_eq!(
            item["suggestedMinPaidViews"],
            terms.rules.min_paid_views(VIEWS, suggested)
        );
        // Parsing JSON may round the last digit.
        let per_euro = item["minPaidViewsPerEuro"].as_f64().unwrap();
        assert!((per_euro - terms.rules.min_paid_views_per_euro(VIEWS)).abs() < 1e-9);
        // The payout sorts and filters the feed, but is not sent.
        assert!(item.get("payoutCents").is_none(), "{item}");
    }
    assert_eq!(items[0]["maxBidCents"], items[1]["maxBidCents"]);
    assert_ne!(items[0]["suggestedBidCents"], items[1]["suggestedBidCents"]);
    assert_eq!(items[0]["myBid"], bid);
    assert_eq!(items[1]["myBid"], Value::Null);
    assert_eq!(items[0]["campaign"]["advertiserName"], "Glow Labs");
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unknown_account_feed_is_not_found(pool: PgPool) {
    let (status, body) = get(&pool, "/api/platform-accounts/999/campaigns").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

/// Creates a draft of the advertiser with these changes saved, and returns its id.
async fn draft_with(pool: &PgPool, advertiser: i64, changes: &Value) -> i64 {
    let (status, draft) = post(
        pool,
        &format!("/api/advertisers/{advertiser}/campaigns"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{draft}");
    let id = draft["id"].as_i64().unwrap();
    let (status, saved) = patch(pool, &format!("/api/campaigns/{id}"), changes).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    id
}

async fn get_estimate(pool: &PgPool, campaign: i64) -> Value {
    let (status, estimate) = get(pool, &format!("/api/campaigns/{campaign}/estimate")).await;
    assert_eq!(status, StatusCode::OK, "{estimate}");
    estimate
}

/// An estimate's size entry with only the size fields filled in.
fn size(group: &str, min_views: i32, max_views: Option<i32>, picked: bool) -> Value {
    json!({
        "group": group,
        "minViews": min_views,
        "maxViews": max_views,
        "picked": picked,
        "accounts": null,
        "typicalPriceCents": null,
        "affordable": null,
    })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn estimate_needs_a_platform_then_a_budget_and_target_cpm(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    new_account(&pool, "lena.moves", VIEWS).await?;
    let draft = draft_with(&pool, advertiser, &json!({})).await;

    let mut sizes = [
        size("nano", 500, Some(2_999), true),
        size("micro", 3_000, Some(29_999), true),
        size("macro", 30_000, Some(299_999), true),
        size("mega", 300_000, None, false),
    ];
    let mut expected = json!({
        "missing": ["platform", "budgetCents", "targetCpmCents"],
        "matchingAccounts": null,
        "matchingAccountsCapped": false,
        "sizes": sizes,
        "videos": null,
        "views": null,
        "averageCpmCents": null,
        "spentCents": null,
        "fillsBudget": null,
    });
    assert_eq!(get_estimate(&pool, draft).await, expected);

    // With a platform, accounts can be counted: one micro account.
    let changes = json!({ "platform": "tiktok" });
    let (status, _) = patch(&pool, &format!("/api/campaigns/{draft}"), &changes).await;
    assert_eq!(status, StatusCode::OK);
    for (size, accounts) in sizes.iter_mut().zip([0, 1, 0, 0]) {
        size["accounts"] = json!(accounts);
    }
    expected["missing"] = json!(["budgetCents", "targetCpmCents"]);
    expected["matchingAccounts"] = json!(1);
    expected["sizes"] = json!(sizes);
    assert_eq!(get_estimate(&pool, draft).await, expected);

    let (status, _) = get(&pool, "/api/campaigns/999/estimate").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn estimate_counts_only_matching_accounts(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    for (handle, views) in [
        ("nano.a", 1_000),
        ("nano.b", 2_000),
        ("micro.a", 10_000),
        ("micro.b", 20_000),
        ("macro.a", 50_000),
        ("mega.a", 400_000),
    ] {
        new_account(&pool, handle, views).await?;
    }
    // Bigger still, but on Instagram or not brand safe.
    let creator = insert_creator(&pool, "Big Creator").await?;
    let big = AccountSpec {
        view_score: 1_000_000,
        ..AccountSpec::default()
    };
    let on_instagram = AccountSpec {
        platform: Platform::Instagram,
        ..big
    };
    let not_brand_safe = AccountSpec {
        brand_safe: false,
        ..big
    };
    insert_account_with(&pool, creator, "big.instagram", on_instagram).await?;
    insert_account_with(&pool, creator, "big.unsafe", not_brand_safe).await?;
    // A bid may take 25% of the budget: enough for the macro account's min bid, not the mega's.
    let (budget_cents, target_cpm_cents) = (150_000, 1_200);
    let rules = test_config().rules;
    let budget_share = rules.max_share_of_budget_cents(budget_cents);
    assert!(rules.min_bid_cents(50_000) <= budget_share);
    assert!(rules.min_bid_cents(400_000) > budget_share);
    let changes = json!({
        "platform": "tiktok",
        "budgetCents": budget_cents,
        "targetCpmCents": target_cpm_cents,
    });
    let draft = draft_with(&pool, advertiser, &changes).await;

    let estimate = get_estimate(&pool, draft).await;

    assert_eq!(estimate["matchingAccounts"], 5);
    let picked = [Nano, Micro, Macro];
    let terms = test_config().terms(budget_cents, target_cpm_cents, &picked);
    let counts = BTreeMap::from([(Nano, 2), (Micro, 2), (Macro, 1), (Mega, 0)]);
    let sizes = marketplace::sizes(&picked, terms.as_ref(), Some(&counts));
    assert_eq!(estimate["sizes"], serde_json::to_value(sizes).unwrap());
    assert!(estimate["spentCents"].as_i64().unwrap() <= budget_cents);

    // Of macro and mega, only the macro account matches.
    let changes = json!({ "sizeGroups": ["macro", "mega"] });
    let (status, _) = patch(&pool, &format!("/api/campaigns/{draft}"), &changes).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(get_estimate(&pool, draft).await["matchingAccounts"], 1);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn estimate_is_empty_when_no_account_matches(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let on_instagram = AccountSpec {
        platform: Platform::Instagram,
        ..AccountSpec::default()
    };
    insert_account_with(&pool, creator, "lena.moves", on_instagram).await?;
    let changes = json!({ "platform": "tiktok", "budgetCents": 150_000, "targetCpmCents": 1_200 });
    let draft = draft_with(&pool, advertiser, &changes).await;

    let estimate = get_estimate(&pool, draft).await;

    assert_eq!(estimate["missing"], json!([]));
    assert_eq!(estimate["matchingAccounts"], 0);
    for field in [
        "videos",
        "views",
        "averageCpmCents",
        "spentCents",
        "fillsBudget",
    ] {
        assert_eq!(estimate[field], Value::Null, "{field}");
    }
    Ok(())
}
