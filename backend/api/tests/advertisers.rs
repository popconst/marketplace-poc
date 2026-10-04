mod common;

use axum::http::StatusCode;
use common::{get, insert_advertiser};
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lists_advertisers_by_name(pool: PgPool) -> sqlx::Result<()> {
    insert_advertiser(&pool, "Glow Labs").await?;
    insert_advertiser(&pool, "Alpine Outdoor").await?;

    let (status, body) = get(&pool, "/api/advertisers").await;

    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|advertiser| advertiser["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Alpine Outdoor", "Glow Labs"]);
    assert!(body[0]["createdAt"].is_string());
    Ok(())
}
