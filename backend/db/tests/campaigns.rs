mod common;

use chrono::{TimeDelta, Utc};
use common::{insert_campaign, insert_complete_draft, insert_draft, update_fails_with};
use db::models::CampaignStatus;
use marketplace::{Rules, Terms};
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_draft_may_be_empty_but_a_published_campaign_is_complete(
    pool: PgPool,
) -> sqlx::Result<()> {
    let draft = insert_draft(&pool).await?;
    assert_eq!(draft.title, None);

    let now = Utc::now();
    let error = db::campaigns::publish(
        &pool,
        draft.id,
        now + TimeDelta::days(3),
        now,
        &Rules::default(),
        10_000,
    )
    .await
    .unwrap_err();

    assert_eq!(
        common::violated_constraint(&error),
        Some("campaigns_complete_unless_draft")
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publishing_freezes_the_deal_rules(pool: PgPool) -> sqlx::Result<()> {
    let draft = insert_complete_draft(&pool).await?;
    // Distinct from the defaults and from each other, so a rule in the wrong column shows.
    let rules = Rules {
        commission_bps: 2_000,
        usual_rate_base_cents: 8_000,
        usual_rate_per_1000_views_cents: 1_100,
        fair_pay_floor_bps: 4_000,
        max_bid_share_of_budget_bps: 2_200,
        views_to_get_paid_bps: 3_000,
        likely_views_limit_bps: 6_500,
        discovery_bps: 1_000,
    };
    let now = Utc::now();

    let published = db::campaigns::publish(
        &pool,
        draft.id,
        now + TimeDelta::days(3),
        now,
        &rules,
        12_345,
    )
    .await?;

    assert_eq!(published.status, CampaignStatus::Active);
    assert_eq!(published.commission_bps, Some(2_000));
    assert_eq!(
        db::selection::terms(&pool, draft.id).await?,
        Some(Terms {
            budget_cents: draft.budget_cents.unwrap(),
            size_groups: draft.size_groups,
            offer_bps: 12_345,
            rules,
        })
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_published_campaign_keeps_every_deal_rule(pool: PgPool) -> sqlx::Result<()> {
    let campaign = insert_campaign(&pool).await?;

    for column in [
        "offer_bps",
        "commission_bps",
        "usual_rate_base_cents",
        "usual_rate_per_1000_views_cents",
        "fair_pay_floor_bps",
        "max_bid_share_of_budget_bps",
        "views_to_get_paid_bps",
        "likely_views_limit_bps",
        "discovery_bps",
    ] {
        let violated =
            update_fails_with(&pool, "campaigns", campaign.id, &format!("{column} = NULL")).await;
        assert_eq!(
            violated.as_deref(),
            Some("campaigns_deal_frozen_unless_draft"),
            "{column}"
        );
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lifecycle_timestamps_follow_the_status(pool: PgPool) -> sqlx::Result<()> {
    let campaign = insert_campaign(&pool).await?;

    for (assignments, constraint) in [
        ("submitted_at = NULL", "campaigns_submitted_at_status"),
        ("activated_at = NULL", "campaigns_activated_at_status"),
        ("status = 'closed'", "campaigns_closed_at_status"),
        ("closed_at = now()", "campaigns_closed_at_status"),
    ] {
        let violated = update_fails_with(&pool, "campaigns", campaign.id, assignments).await;
        assert_eq!(violated.as_deref(), Some(constraint), "{assignments}");
    }

    sqlx::query("UPDATE campaigns SET status = 'closed', closed_at = now() WHERE id = $1")
        .bind(campaign.id)
        .execute(&pool)
        .await?;
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bidding_closes_after_it_opens(pool: PgPool) -> sqlx::Result<()> {
    let campaign = insert_campaign(&pool).await?;

    let assignments = "bidding_deadline = activated_at";
    let violated = update_fails_with(&pool, "campaigns", campaign.id, assignments).await;

    assert_eq!(violated.as_deref(), Some("campaigns_bidding_period"));
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sliders_move_in_steps_of_25(pool: PgPool) -> sqlx::Result<()> {
    let campaign = insert_campaign(&pool).await?;

    for (slider, constraint) in [
        ("engagement_weight", "campaigns_engagement_weight_step"),
        ("quality_weight", "campaigns_quality_weight_step"),
        ("reliability_weight", "campaigns_reliability_weight_step"),
    ] {
        let assignments = format!("{slider} = 60");
        let violated = update_fails_with(&pool, "campaigns", campaign.id, &assignments).await;
        assert_eq!(violated.as_deref(), Some(constraint), "{slider}");
    }

    sqlx::query(
        "UPDATE campaigns
         SET engagement_weight = 0, quality_weight = 75, reliability_weight = 100
         WHERE id = $1",
    )
    .bind(campaign.id)
    .execute(&pool)
    .await?;
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_campaign_picks_at_least_one_size(pool: PgPool) -> sqlx::Result<()> {
    let campaign = insert_campaign(&pool).await?;

    let violated = update_fails_with(&pool, "campaigns", campaign.id, "size_groups = '{}'").await;

    assert_eq!(violated.as_deref(), Some("campaigns_size_groups_not_empty"));
    Ok(())
}
