//! A campaign's terms, and the prices, limits and budget split that follow from them.

use crate::money::share_of;
use crate::{PLANNED_GROUPS, Rules, SizeGroup};

/// What a campaign's bids are judged by: frozen at publish, or built from the current settings
/// for a draft's estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Terms {
    pub budget_cents: i64,
    /// The planned sizes the campaign picked, each once. Never empty.
    pub size_groups: Vec<SizeGroup>,
    /// See [`Rules::offer_bps`].
    pub offer_bps: i64,
    #[cfg_attr(feature = "sqlx", sqlx(flatten))]
    pub rules: Rules,
}

/// A picked size's share of the budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupBudget {
    pub group: SizeGroup,
    pub budget_cents: i64,
}

impl Terms {
    /// The usual rate times the offer, rounded down, and at least the min bid.
    #[must_use]
    pub fn campaign_rate_cents(&self, view_score: i32) -> i64 {
        share_of(self.rules.usual_rate_cents(view_score), self.offer_bps)
            .max(self.rules.min_bid_cents(view_score))
    }

    /// The campaign rate, capped at the max bid and at the share of the budget one winning bid
    /// may take, so that the suggestion can win.
    #[must_use]
    pub fn suggested_bid_cents(&self, view_score: i32) -> i64 {
        self.campaign_rate_cents(view_score)
            .min(self.rules.max_bid_cents(view_score))
            .min(self.rules.max_share_of_budget_cents(self.budget_cents))
    }

    /// Whether the campaign picked the account's size and its min bid is within the share of the
    /// budget one winning bid may take; if not, none of its bids could win. SQL checks the second
    /// part through [`Rules::max_view_score_for_budget`].
    #[must_use]
    pub fn can_bid(&self, view_score: i32) -> bool {
        self.size_groups.contains(&SizeGroup::of(view_score))
            && self.rules.min_bid_cents(view_score)
                <= self.rules.max_share_of_budget_cents(self.budget_cents)
    }

    /// An equal share of the budget per picked size, smallest first. The smallest also gets the
    /// cents left over, so the shares add up to the budget.
    #[must_use]
    pub fn split_budget(&self) -> Vec<GroupBudget> {
        let picked = self.picked_groups();
        if picked.is_empty() {
            return Vec::new();
        }
        #[expect(clippy::cast_possible_wrap, reason = "at most four sizes")]
        let count = picked.len() as i64;
        let share_cents = self.budget_cents / count;
        let left_over_cents = self.budget_cents % count;
        picked
            .into_iter()
            .enumerate()
            .map(|(i, group)| GroupBudget {
                group,
                budget_cents: if i == 0 {
                    share_cents + left_over_cents
                } else {
                    share_cents
                },
            })
            .collect()
    }

    /// `size_groups`, smallest first.
    pub(crate) fn picked_groups(&self) -> Vec<SizeGroup> {
        PLANNED_GROUPS
            .into_iter()
            .filter(|group| self.size_groups.contains(group))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SizeGroup::{Macro, Mega, Micro, Nano};
    use crate::rules::TEST_RULES;

    /// A micro account whose usual rate is €190.
    const MICRO_VIEWS: i32 = 10_000;

    fn terms(budget_cents: i64, offer_bps: i64, size_groups: &[SizeGroup]) -> Terms {
        Terms {
            budget_cents,
            size_groups: size_groups.to_vec(),
            offer_bps,
            rules: TEST_RULES,
        }
    }

    #[test]
    fn campaign_rate_is_the_usual_rate_times_the_offer_but_at_least_the_min_bid() {
        // 63.15% of €190 is €119.985, rounded down.
        assert_eq!(
            terms(1_000_000, 6_315, &[Micro]).campaign_rate_cents(MICRO_VIEWS),
            11_998
        );
        // 63.15% of a nano account's €105 is €66.30, below its €90 min bid.
        assert_eq!(
            terms(1_000_000, 6_315, &[Nano]).campaign_rate_cents(1_500),
            9_000
        );
    }

    #[test]
    fn suggested_bid_is_the_campaign_rate_capped_at_the_max_bid_and_the_budget_share() {
        // The €190 rate fits in the €250 a bid may take of €1,000, but not in the €150 of €600.
        assert_eq!(
            terms(100_000, 10_000, &[Micro]).suggested_bid_cents(MICRO_VIEWS),
            19_000
        );
        assert_eq!(
            terms(60_000, 10_000, &[Micro]).suggested_bid_cents(MICRO_VIEWS),
            15_000
        );
        // At 200% the rate is €380, above the €325.71 max bid.
        assert_eq!(
            terms(1_000_000, 20_000, &[Micro]).suggested_bid_cents(MICRO_VIEWS),
            32_571
        );
    }

    #[test]
    fn can_bid_needs_a_picked_size_and_a_min_bid_within_the_budget_share() {
        let micro_only = terms(100_000, 10_000, &[Micro]);
        assert!(micro_only.can_bid(MICRO_VIEWS));
        // A macro account, of a size the campaign did not pick.
        assert!(!micro_only.can_bid(50_000));
        // A bid may take €90 of €360, less than the €95 min bid of a €190 account.
        assert!(!terms(36_000, 10_000, &[Micro]).can_bid(MICRO_VIEWS));
    }

    #[test]
    fn max_view_score_for_budget_agrees_with_can_bid() {
        for budget_cents in [36_000, 36_003, 100_000, 1_000_000, 3_000_001, 100_000_000] {
            let max_views = TEST_RULES.max_view_score_for_budget(budget_cents).unwrap();
            let terms = terms(budget_cents, 10_000, &PLANNED_GROUPS);
            let near_the_bound = max_views - 3..=max_views + 3;
            for view_score in (500..10_000_000).step_by(997).chain(near_the_bound) {
                assert_eq!(
                    terms.can_bid(view_score),
                    view_score <= max_views,
                    "{budget_cents} {view_score}"
                );
            }
        }
        // Below the €360 min budget, no account can bid.
        assert_eq!(TEST_RULES.max_view_score_for_budget(35_999), None);
    }

    #[test]
    fn split_budget_gives_each_picked_size_an_equal_share_and_leftover_cents_to_the_smallest() {
        // €10,000.01 over three sizes, picked in any order.
        let split = terms(1_000_001, 10_000, &[Mega, Nano, Macro]).split_budget();

        let budget = |group, budget_cents| GroupBudget {
            group,
            budget_cents,
        };
        assert_eq!(
            split,
            [
                budget(Nano, 333_335),
                budget(Macro, 333_333),
                budget(Mega, 333_333)
            ]
        );
    }
}
