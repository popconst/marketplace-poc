//! The route table: JSON under `/api`, and the demo-only routes under `/api/dev` when asked for.

mod advertisers;
mod bids;
mod campaigns;
mod dev;
mod matching;
mod options;
mod platform_accounts;
mod progress;
mod results;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let mut api = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/options", get(options::list))
        .route("/advertisers", get(advertisers::list))
        .route(
            "/advertisers/{advertiser_id}/campaigns",
            get(campaigns::list).post(campaigns::create),
        )
        .route(
            "/campaigns/{campaign_id}",
            get(campaigns::show).patch(campaigns::update),
        )
        .route("/campaigns/{campaign_id}/publish", post(campaigns::publish))
        .route("/campaigns/{campaign_id}/estimate", get(matching::estimate))
        .route("/campaigns/{campaign_id}/progress", get(progress::show))
        .route("/campaigns/{campaign_id}/results", get(results::show))
        .route(
            "/campaigns/{campaign_id}/bids/{account_id}",
            put(bids::place).delete(bids::withdraw),
        )
        .route(
            "/campaigns/{campaign_id}/bids/{account_id}/chance",
            get(bids::chance),
        )
        .route("/platform-accounts", get(platform_accounts::list))
        .route(
            "/platform-accounts/{account_id}",
            get(platform_accounts::show),
        )
        .route(
            "/platform-accounts/{account_id}/campaigns",
            get(matching::feed),
        )
        .route("/platform-accounts/{account_id}/bids", get(bids::list));
    if state.dev_tools {
        api = api.nest("/dev", dev::routes());
    }

    Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Liveness. Never touches the database, so a database outage marks instances
/// unready instead of getting them restarted.
async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Readiness: this instance can reach the database.
async fn readyz(State(pool): State<PgPool>) -> (StatusCode, Json<Value>) {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(error) => {
            tracing::warn!(%error, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "unavailable" })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use db::config::Settings;
    use sqlx::PgPool;
    use tower::ServiceExt;

    use crate::AppState;

    #[tokio::test]
    async fn healthz_responds_without_a_database() {
        let state = AppState {
            pool: PgPool::connect_lazy("postgres://localhost/unused").unwrap(),
            settings: Settings::default(),
            dev_tools: false,
        };

        let response = super::router(state)
            .oneshot(Request::get("/api/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], br#"{"status":"ok"}"#);
    }
}
