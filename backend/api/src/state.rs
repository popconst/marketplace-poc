use axum::extract::FromRef;
use db::config::Settings;
use sqlx::PgPool;

/// The router's state. Handlers take only the field they need, as `State<PgPool>` or
/// `State<Settings>`. Both are cheap to clone: the pool is a handle, the settings plain numbers.
#[derive(Clone, FromRef)]
pub struct AppState {
    pub pool: PgPool,
    pub settings: Settings,
    /// Whether the demo-only routes under `/api/dev` are served. Not extractable on its own, as a
    /// bare `State<bool>` would not say what it is.
    #[from_ref(skip)]
    pub dev_tools: bool,
}
