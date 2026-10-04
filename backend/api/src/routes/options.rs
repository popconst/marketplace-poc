//! The choices and limits the frontend's forms use, loaded once by the app.

use axum::extract::State;
use db::models::{Country, Genre, Language};
use marketplace::{PLANNED_GROUPS, SizeGroup, VIEWS_COUNTING_DAYS};
use serde::Serialize;

use super::campaigns::Limits;
use crate::error::ApiError;
use crate::extract::Json;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    genres: Vec<Genre>,
    /// Only countries and languages some account has, so that targeting always matches someone.
    countries: Vec<Country>,
    languages: Vec<Language>,
    limits: Limits,
    /// The days after posting in which a video's views count towards its min paid views.
    views_counting_days: i64,
    /// The sizes a campaign can pick, smallest first.
    size_groups: Vec<SizeRange>,
    /// Whether `/api/dev` is served; the frontend hides the demo controls otherwise.
    dev_tools: bool,
}

/// An account size and the views per post it covers.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SizeRange {
    group: SizeGroup,
    min_views: i32,
    /// `None` for mega, which has no upper bound.
    max_views: Option<i32>,
}

pub async fn list(State(state): State<AppState>) -> Result<Json<Options>, ApiError> {
    let (genres, countries, languages) = tokio::try_join!(
        db::options::genres(&state.pool),
        db::options::countries_in_use(&state.pool),
        db::options::languages_in_use(&state.pool),
    )?;
    Ok(Json(Options {
        genres,
        countries,
        languages,
        limits: Limits::new(&state.settings),
        views_counting_days: VIEWS_COUNTING_DAYS,
        size_groups: PLANNED_GROUPS
            .into_iter()
            .map(|group| SizeRange {
                group,
                min_views: group.min_views(),
                max_views: group.max_views(),
            })
            .collect(),
        dev_tools: state.dev_tools,
    }))
}
