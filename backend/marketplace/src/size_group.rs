//! Account sizes by views. The boundaries and typical views are our own assumptions.

use serde::{Deserialize, Serialize};

/// An account's size by its median views per post, smallest first. The database's `size_group`
/// enum has no starter, as no campaign can pick it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "size_group", rename_all = "lowercase")
)]
#[serde(rename_all = "snake_case")]
pub enum SizeGroup {
    /// Under 500 views.
    Starter,
    /// 500 to under 3,000.
    Nano,
    /// 3,000 to under 30,000.
    Micro,
    /// 30,000 to under 300,000.
    Macro,
    /// 300,000 and more.
    Mega,
}

/// The sizes a campaign can pick, smallest first. Not starters: below 500 views, the base fee
/// alone costs over €180 per 1,000 views.
pub const PLANNED_GROUPS: [SizeGroup; 4] = [
    SizeGroup::Nano,
    SizeGroup::Micro,
    SizeGroup::Macro,
    SizeGroup::Mega,
];

impl SizeGroup {
    #[must_use]
    pub fn of(view_score: i32) -> Self {
        [Self::Mega, Self::Macro, Self::Micro, Self::Nano]
            .into_iter()
            .find(|group| view_score >= group.min_views())
            .unwrap_or(Self::Starter)
    }

    #[must_use]
    pub fn min_views(self) -> i32 {
        match self {
            Self::Starter => 0,
            Self::Nano => 500,
            Self::Micro => 3_000,
            Self::Macro => 30_000,
            Self::Mega => 300_000,
        }
    }

    /// `None` for mega, which has no upper bound.
    #[must_use]
    pub fn max_views(self) -> Option<i32> {
        let next = match self {
            Self::Starter => Self::Nano,
            Self::Nano => Self::Micro,
            Self::Micro => Self::Macro,
            Self::Macro => Self::Mega,
            Self::Mega => return None,
        };
        Some(next.min_views() - 1)
    }

    /// The views of a typical account in the group. They set a size mix's usual CPM, and so the
    /// offer, and the estimate's typical prices. `None` for starters.
    #[must_use]
    pub fn typical_views(self) -> Option<i32> {
        match self {
            Self::Starter => None,
            Self::Nano => Some(1_500),
            Self::Micro => Some(10_000),
            Self::Macro => Some(90_000),
            Self::Mega => Some(1_000_000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_groups_change_at_their_lower_boundaries() {
        for (view_score, group) in [
            (0, SizeGroup::Starter),
            (499, SizeGroup::Starter),
            (500, SizeGroup::Nano),
            (2_999, SizeGroup::Nano),
            (3_000, SizeGroup::Micro),
            (29_999, SizeGroup::Micro),
            (30_000, SizeGroup::Macro),
            (299_999, SizeGroup::Macro),
            (300_000, SizeGroup::Mega),
        ] {
            assert_eq!(SizeGroup::of(view_score), group, "{view_score} views");
        }
        assert_eq!(SizeGroup::Micro.max_views(), Some(29_999));
        assert_eq!(SizeGroup::Mega.max_views(), None);
    }
}
