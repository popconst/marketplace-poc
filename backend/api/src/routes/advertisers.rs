use axum::extract::State;
use db::models::Advertiser;
use sqlx::PgPool;

use crate::error::ApiError;
use crate::extract::Json;

/// Every advertiser, by name. There are few, so the list is not paged.
pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Advertiser>>, ApiError> {
    let advertisers =
        sqlx::query_as("SELECT id, name, created_at FROM advertisers ORDER BY name, id")
            .fetch_all(&pool)
            .await?;
    Ok(Json(advertisers))
}
