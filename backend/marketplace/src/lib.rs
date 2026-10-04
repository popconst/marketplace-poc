//! The marketplace's rules, free of I/O: callers pass plain values read from the database.
//!
//! - **Usual rate**: what an account usually charges for a post, by its expected views.
//! - **Offer**: what a campaign pays relative to usual rates, from its target CPM.
//! - **Campaign rate**: the usual rate times the offer.
//! - **Min / max bid**: the range an account may bid. **Suggested bid**: the campaign rate,
//!   capped at the max bid and at the share of the budget one winning bid may take.
//! - **Min paid views**: the views a winning video must reach for its creator to be paid.
//! - **Size group**: an account's size by its views. A campaign picks sizes and splits its budget
//!   evenly across them.
//! - **Match score**: how well an account fits a campaign's sliders and genres, 0 to 100.
//!
//! Reading order: `size_group`, `rules` (every limit, in one table), `terms`, `matching`,
//! `select`, `estimate`.

mod estimate;
mod matching;
mod money;
mod rules;
mod select;
mod size_group;
mod terms;

pub use estimate::{Candidate, Estimate, SizeEstimate, estimate, round_to_two_figures, sizes};
pub use matching::{Account, Factor, FactorKind, Match, Preferences, score};
pub use money::cpm_cents;
pub use rules::{DEFAULT_MIN_OFFER_BPS, Rules, VIEWS_COUNTING_DAYS};
pub use select::{GroupResult, LossReason, Outcome, PendingBid, Selection, select_winners};
pub use size_group::{PLANNED_GROUPS, SizeGroup};
pub use terms::{GroupBudget, Terms};
