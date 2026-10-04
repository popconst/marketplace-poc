//! The deal rules: what a campaign pays, what a creator may bid and what WePush keeps. A
//! campaign freezes them at publish, so a later change never alters a live or closed deal. Amounts
//! are euro cents; shares are basis points (2,500 is 25%).
//!
//! | Term | Rule | Default |
//! |---|---|---|
//! | Usual rate | a base fee plus a rate per 1,000 expected views | €90 + €10 |
//! | Min bid | the fair-pay floor of the usual rate, at least the base fee | 50%, €90 |
//! | Offer | the target CPM over the usual CPM of the picked sizes | at least 0.5× |
//! | Campaign rate | the usual rate times the offer, at least the min bid | |
//! | Max bid | where the min paid views reach the likely-views limit | 60 / 35 ≈ 1.71× |
//! | Budget share | the most one winning bid may take of the budget | 25% |
//! | Min paid views | a share of the expected views at the usual rate, growing with the bid | 35% |
//! | Commission | WePush's share of a winning bid | 15% |
//! | Discovery | the share of each size's budget kept for weaker matches | 15% |
//! | Min budget | the lowest bid over the budget share | €360 |
//!
//! The min and max bid depend only on the account, never on the budget, which creators are not
//! shown. So bidding accepts a bid above the budget share, and winner selection rejects it.

use crate::SizeGroup;
use crate::money::{BPS, share_of};

/// Days after posting in which a video's views count towards its min paid views. Display only.
pub const VIEWS_COUNTING_DAYS: i64 = 5;

/// The default lowest offer, half the usual rate. Not in [`Rules`], as only the offer it yields
/// is frozen.
pub const DEFAULT_MIN_OFFER_BPS: i64 = 5_000;

/// The deal rules frozen on a campaign. [`Default`] holds today's values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Rules {
    /// WePush's commission on each winning bid.
    pub commission_bps: i64,
    /// The fixed part of the usual rate: the cost of making the video. Also the lowest bid.
    pub usual_rate_base_cents: i64,
    pub usual_rate_per_1000_views_cents: i64,
    /// The lowest bid, as a share of the account's usual rate.
    pub fair_pay_floor_bps: i64,
    /// The most one winning bid may take of the budget, so that no single creator takes it all.
    pub max_bid_share_of_budget_bps: i64,
    /// Min paid views at the usual rate, as a share of the expected views. Set so that only a
    /// video that flops goes unpaid.
    pub views_to_get_paid_bps: i64,
    /// The most min paid views a bid may promise, as a share of the expected views. Above it, an
    /// ordinary video is unlikely to reach them.
    pub likely_views_limit_bps: i64,
    /// The share of each size's budget that goes first to its weaker matches, so that the sliders
    /// do not shut out every account they rank low.
    pub discovery_bps: i64,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            commission_bps: 1_500,
            usual_rate_base_cents: 9_000,
            usual_rate_per_1000_views_cents: 1_000,
            fair_pay_floor_bps: 5_000,
            max_bid_share_of_budget_bps: 2_500,
            views_to_get_paid_bps: 3_500,
            likely_views_limit_bps: 6_000,
            discovery_bps: 1_500,
        }
    }
}

impl Rules {
    /// Checks that every rule is in range and consistent with the others.
    ///
    /// # Errors
    ///
    /// Returns the first problem, naming the field.
    pub fn validate(&self) -> Result<(), String> {
        let shares = [
            ("commission_bps", self.commission_bps, 0),
            ("fair_pay_floor_bps", self.fair_pay_floor_bps, 0),
            (
                "max_bid_share_of_budget_bps",
                self.max_bid_share_of_budget_bps,
                1,
            ),
            ("views_to_get_paid_bps", self.views_to_get_paid_bps, 1),
            ("likely_views_limit_bps", self.likely_views_limit_bps, 1),
            ("discovery_bps", self.discovery_bps, 0),
        ];
        for (name, bps, least) in shares {
            let problem = format!("{name} must be from {least} to {BPS}, not {bps}");
            ensure((least..=BPS).contains(&bps), &problem)?;
        }
        ensure(
            self.usual_rate_base_cents > 0,
            "usual_rate_base_cents must be above 0",
        )?;
        ensure(
            self.usual_rate_per_1000_views_cents > 0,
            "usual_rate_per_1000_views_cents must be above 0",
        )?;
        // So that the max bid is above the usual rate.
        ensure(
            self.likely_views_limit_bps > self.views_to_get_paid_bps,
            "likely_views_limit_bps must be above views_to_get_paid_bps",
        )
    }

    /// Checks that the lowest offer is at least the fair-pay floor.
    ///
    /// # Errors
    ///
    /// Returns the problem if it is below.
    pub fn validate_min_offer(&self, min_offer_bps: i64) -> Result<(), String> {
        let least = self.fair_pay_floor_bps;
        let problem = format!(
            "min_offer_bps must be at least fair_pay_floor_bps ({least}), not {min_offer_bps}"
        );
        ensure(min_offer_bps >= least, &problem)
    }

    /// The base fee plus the rate for the expected views, rounded down.
    #[must_use]
    pub fn usual_rate_cents(&self, view_score: i32) -> i64 {
        self.usual_rate_base_cents
            + i64::from(view_score) * self.usual_rate_per_1000_views_cents / 1_000
    }

    /// The fair-pay floor of the usual rate, rounded down, and at least the base fee: no creator
    /// posts for less than the video costs to make.
    #[must_use]
    pub fn min_bid_cents(&self, view_score: i32) -> i64 {
        share_of(self.usual_rate_cents(view_score), self.fair_pay_floor_bps)
            .max(self.usual_rate_base_cents)
    }

    /// The views a bid's video must reach for the creator to be paid, rounded up. They grow in
    /// proportion to the bid, so asking more promises more.
    #[must_use]
    pub fn min_paid_views(&self, view_score: i32, amount_cents: i64) -> i64 {
        let (numerator, denominator) = self.min_paid_views_fraction(view_score, amount_cents);
        i64::try_from((numerator + denominator - 1) / denominator).unwrap_or(i64::MAX)
    }

    /// Min paid views per euro bid, unrounded, for the preview while a bid is typed.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "both terms are far below 2^53 for any real account"
    )]
    pub fn min_paid_views_per_euro(&self, view_score: i32) -> f64 {
        let (numerator, denominator) = self.min_paid_views_fraction(view_score, 100);
        numerator as f64 / denominator as f64
    }

    /// The highest bid whose min paid views stay within the likely-views limit, rounded down.
    #[must_use]
    pub fn max_bid_cents(&self, view_score: i32) -> i64 {
        self.usual_rate_cents(view_score) * self.likely_views_limit_bps / self.views_to_get_paid_bps
    }

    /// The most one winning bid may take of this budget, rounded down.
    #[must_use]
    pub fn max_share_of_budget_cents(&self, budget_cents: i64) -> i64 {
        share_of(budget_cents, self.max_bid_share_of_budget_bps)
    }

    /// The smallest budget whose share for one bid still covers the base fee, rounded up.
    #[must_use]
    pub fn min_budget_cents(&self) -> i64 {
        let share_bps = self.max_bid_share_of_budget_bps;
        (self.usual_rate_base_cents * BPS + share_bps - 1) / share_bps
    }

    /// The most views an account may have and still bid with this budget: above them, its min bid
    /// is more than one bid may take. `None` below [`min_budget_cents`](Self::min_budget_cents).
    /// The matching queries filter on it; it agrees with [`Terms::can_bid`](crate::Terms::can_bid).
    #[must_use]
    pub fn max_view_score_for_budget(&self, budget_cents: i64) -> Option<i32> {
        let share_cents = self.max_share_of_budget_cents(budget_cents);
        let fits = |view_score: i32| self.min_bid_cents(view_score) <= share_cents;
        if !fits(0) {
            return None;
        }
        if fits(i32::MAX) {
            return Some(i32::MAX);
        }
        // The min bid grows with the views, so binary search for the last view score that fits.
        // `fitting` always fits and `too_many` never does.
        let (mut fitting, mut too_many) = (0, i32::MAX);
        while too_many - fitting > 1 {
            let middle = fitting + (too_many - fitting) / 2;
            if fits(middle) {
                fitting = middle;
            } else {
                too_many = middle;
            }
        }
        Some(fitting)
    }

    /// WePush's commission on a winning bid, rounded half up to the cent.
    #[must_use]
    pub fn commission_cents(&self, bid_cents: i64) -> i64 {
        (bid_cents * self.commission_bps + BPS / 2) / BPS
    }

    /// What the creator receives: the bid less the commission.
    #[must_use]
    pub fn payout_cents(&self, bid_cents: i64) -> i64 {
        bid_cents - self.commission_cents(bid_cents)
    }

    /// The target CPM over the size mix's usual CPM, rounded down, and at least `min_offer_bps`.
    /// Computed at publish and frozen. `None` if no size is picked.
    #[must_use]
    pub fn offer_bps(
        &self,
        target_cpm_cents: i64,
        size_groups: &[SizeGroup],
        min_offer_bps: i64,
    ) -> Option<i64> {
        let usual_cpm_cents = self.usual_cpm_cents(size_groups)?;
        let offer_bps = target_cpm_cents * BPS / usual_cpm_cents;
        Some(offer_bps.max(min_offer_bps))
    }

    /// What a size mix usually costs per 1,000 views, rounded down. Each picked size spends an
    /// equal share of the budget on typical accounts at their usual rate; the budget cancels out,
    /// leaving the harmonic mean of the typical CPMs, n × 1,000 / Σ (views / rate). `None` if no
    /// size is picked.
    #[must_use]
    pub fn usual_cpm_cents(&self, size_groups: &[SizeGroup]) -> Option<i64> {
        // Σ (views / rate) as one exact fraction.
        let (mut numerator, mut denominator, mut count) = (0_i128, 1_i128, 0_i128);
        for views in size_groups.iter().filter_map(|group| group.typical_views()) {
            let rate = i128::from(self.usual_rate_cents(views));
            numerator = numerator * rate + i128::from(views) * denominator;
            denominator *= rate;
            count += 1;
        }
        if count == 0 {
            return None;
        }
        i64::try_from(count * 1_000 * denominator / numerator).ok()
    }

    /// Min paid views as a fraction, in `i128` as the amount may come straight from a request.
    fn min_paid_views_fraction(&self, view_score: i32, amount_cents: i64) -> (i128, i128) {
        let numerator = i128::from(view_score)
            * i128::from(self.views_to_get_paid_bps)
            * i128::from(amount_cents);
        let denominator = i128::from(BPS) * i128::from(self.usual_rate_cents(view_score));
        (numerator, denominator)
    }
}

fn ensure(holds: bool, problem: &str) -> Result<(), String> {
    if holds {
        Ok(())
    } else {
        Err(problem.to_owned())
    }
}

/// Today's rules spelled out, so the tests do not change with the defaults.
#[cfg(test)]
pub(crate) const TEST_RULES: Rules = Rules {
    commission_bps: 1_500,
    usual_rate_base_cents: 9_000,
    usual_rate_per_1000_views_cents: 1_000,
    fair_pay_floor_bps: 5_000,
    max_bid_share_of_budget_bps: 2_500,
    views_to_get_paid_bps: 3_500,
    likely_views_limit_bps: 6_000,
    discovery_bps: 1_500,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SizeGroup::{Macro, Mega, Micro, Nano};

    #[test]
    fn rules_default_to_todays_values() {
        assert_eq!(Rules::default(), TEST_RULES);
        assert_eq!(TEST_RULES.validate(), Ok(()));
        assert_eq!(TEST_RULES.validate_min_offer(DEFAULT_MIN_OFFER_BPS), Ok(()));
        // €90 is the lowest bid and a bid may take 25%, so a budget needs €360.
        assert_eq!(TEST_RULES.min_budget_cents(), 36_000);
    }

    #[test]
    fn validate_rejects_inconsistent_rules() {
        let broken = [
            Rules {
                commission_bps: 10_001,
                ..TEST_RULES
            },
            Rules {
                max_bid_share_of_budget_bps: 0,
                ..TEST_RULES
            },
            Rules {
                usual_rate_base_cents: 0,
                ..TEST_RULES
            },
            Rules {
                likely_views_limit_bps: 3_500,
                ..TEST_RULES
            },
        ];
        for rules in broken {
            assert!(rules.validate().is_err(), "{rules:?}");
        }
        // One below the fair-pay floor.
        assert!(TEST_RULES.validate_min_offer(4_999).is_err());
    }

    #[test]
    fn usual_rate_is_the_base_fee_plus_the_rate_for_the_views() {
        // €90 plus €10 for each 1,000 views.
        assert_eq!(TEST_RULES.usual_rate_cents(0), 9_000);
        assert_eq!(TEST_RULES.usual_rate_cents(1_750), 10_750);
        assert_eq!(TEST_RULES.usual_rate_cents(1_000_000), 1_009_000);
    }

    #[test]
    fn min_bid_is_the_fair_pay_floor_of_the_usual_rate_but_at_least_the_base_fee() {
        // Half of €190 and of €10,090.
        assert_eq!(TEST_RULES.min_bid_cents(10_000), 9_500);
        assert_eq!(TEST_RULES.min_bid_cents(1_000_000), 504_500);
        // Half of €180.01, rounded down, and half of €105, raised to the €90 base fee.
        assert_eq!(TEST_RULES.min_bid_cents(9_001), 9_000);
        assert_eq!(TEST_RULES.min_bid_cents(1_500), 9_000);
    }

    #[test]
    fn usual_cpm_is_the_harmonic_mean_of_the_picked_sizes_typical_cpms() {
        // Typical accounts cost €105 for 1,500 views (€70 per 1,000), €190 for 10,000 (€19),
        // €990 for 90,000 (€11) and €10,090 for 1,000,000 (€10.09).
        assert_eq!(TEST_RULES.usual_cpm_cents(&[Nano]), Some(7_000));
        // 3 / (1/70 + 1/19 + 1/11) is €19.008.
        assert_eq!(
            TEST_RULES.usual_cpm_cents(&[Nano, Micro, Macro]),
            Some(1_900)
        );
        // 2 / (1/11 + 1/10.09) is €10.525.
        assert_eq!(TEST_RULES.usual_cpm_cents(&[Macro, Mega]), Some(1_052));
        assert_eq!(TEST_RULES.usual_cpm_cents(&[]), None);
    }

    #[test]
    fn offer_is_the_target_over_the_usual_cpm_but_at_least_the_lowest_offer() {
        let sizes = [Nano, Micro, Macro];
        // €12 against the €19 usual CPM is 63.15%, rounded down.
        assert_eq!(TEST_RULES.offer_bps(1_200, &sizes, 5_000), Some(6_315));
        // €9 would be 47.36%, raised to the lowest offer. €40 is 210.52%, with no cap.
        assert_eq!(TEST_RULES.offer_bps(900, &sizes, 5_000), Some(5_000));
        assert_eq!(TEST_RULES.offer_bps(4_000, &sizes, 5_000), Some(21_052));
        assert_eq!(TEST_RULES.offer_bps(1_200, &[], 5_000), None);
    }

    #[test]
    fn min_paid_views_grow_with_the_bid_and_round_up() {
        // 35% of 10,000 views at the €190 usual rate, and 1.5 times that at 1.5 times the rate.
        assert_eq!(TEST_RULES.min_paid_views(10_000, 19_000), 3_500);
        assert_eq!(TEST_RULES.min_paid_views(10_000, 28_500), 5_250);
        // 3,500.18 views, rounded up.
        assert_eq!(TEST_RULES.min_paid_views(10_000, 19_001), 3_501);
        // Unrounded: 35% of 10,000 views per €190.
        let per_euro = TEST_RULES.min_paid_views_per_euro(10_000);
        assert!((per_euro - 3_500.0 / 190.0).abs() < 1e-9, "{per_euro}");
    }

    #[test]
    fn max_bid_is_the_last_bid_within_the_likely_views_limit() {
        // 60 / 35 of the €190 usual rate is €325.714, rounded down.
        let max_cents = TEST_RULES.max_bid_cents(10_000);
        assert_eq!(max_cents, 32_571);
        // 60% of 10,000 views is 6,000.
        assert_eq!(TEST_RULES.min_paid_views(10_000, max_cents), 6_000);
        assert_eq!(TEST_RULES.min_paid_views(10_000, max_cents + 1), 6_001);
    }

    #[test]
    fn min_budget_is_the_least_whose_bid_share_allows_the_lowest_bid() {
        // 70% does not divide €90 evenly, so the min budget is rounded up.
        let rules = Rules {
            max_bid_share_of_budget_bps: 7_000,
            ..TEST_RULES
        };
        let min_cents = rules.min_budget_cents();
        assert!(rules.max_share_of_budget_cents(min_cents) >= rules.usual_rate_base_cents);
        assert!(rules.max_share_of_budget_cents(min_cents - 1) < rules.usual_rate_base_cents);
    }

    #[test]
    fn commission_rounds_half_up_and_payout_is_the_rest() {
        // 15% of €100.03 is €15.0045, and of €100.10 is €15.015.
        assert_eq!(TEST_RULES.commission_cents(10_003), 1_500);
        assert_eq!(TEST_RULES.commission_cents(10_010), 1_502);
        assert_eq!(TEST_RULES.payout_cents(10_010), 8_508);
    }
}
