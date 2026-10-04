//! The HTTP API the frontend talks to: JSON under `/api`.

mod error;
mod extract;
mod format;
mod routes;
mod state;

pub use routes::router;
pub use state::AppState;
