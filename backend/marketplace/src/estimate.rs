//! What a campaign's budget can buy, estimated before anyone bids, and what each size holds.
//!
//! The method is deterministic and explainable rather than precise:
//!
//! 1. **Likely bidders**: in each picked size, the candidates that can bid and score in the top
//!    quarter of the size, so the advertiser's sliders decide who counts.
//! 2. **One run**: every likely bidder bids its suggested bid, as the feed proposes, and
//!    [`select_winners`] picks the winners as closing does.
//!
//! Real bids will differ, so counts of videos, views and accounts are rounded to two significant
//! figures. The candidates may be a sample of the matching accounts; each then also bids for the
//! accounts like it that were left out.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::money::{cpm_cents, share_of};
use crate::{PLANNED_GROUPS, PendingBid, SizeGroup, Terms, select_winners};

/// Likely bidders score at or above this percentile of their size.
const LIKELY_BIDDER_PERCENTILE: usize = 75;

/// The winners fill the budget if they spend at least this share of it.
const FILLS_BUDGET_BPS: i64 = 9_500;

#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    pub view_score: i32,
    /// 0 to 100.
    pub match_score: i16,
    /// The creator's reliability, 0 to 100.
    pub reliability_score: i16,
}

/// What the budget buys from the likely bidders at their suggested bids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    /// The number of winners, rounded.
    pub videos: i64,
    /// Sum of the winners' view scores, rounded.
    pub views: i64,
    pub average_cpm_cents: i64,
    pub spent_cents: i64,
    pub fills_budget: bool,
}

/// What a planned size holds for a campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeEstimate {
    pub group: SizeGroup,
    pub min_views: i32,
    /// `None` for mega.
    pub max_views: Option<i32>,
    pub picked: bool,
    /// Accounts of the size that meet the campaign's requirements and, once it has a budget,
    /// whose min bid that allows, rounded. `None` while they cannot be counted.
    pub accounts: Option<i64>,
    /// What a typical account of the size would bid on the campaign; `None` without terms.
    pub typical_price_cents: Option<i64>,
    /// Whether such an account can bid and its price fits in the size's share of the budget;
    /// `None` without terms or for a size the campaign did not pick.
    pub affordable: Option<bool>,
}

/// Runs the method above on the candidates. `None` if no likely bidder wins.
#[must_use]
pub fn estimate(
    candidates: &[Candidate],
    terms: &Terms,
    matching_accounts: i64,
) -> Option<Estimate> {
    if candidates.is_empty() {
        return None;
    }
    let copies = copies_per_candidate(matching_accounts, candidates.len());
    let bids = likely_bids(candidates, terms, copies);
    if bids.is_empty() {
        return None;
    }
    let selection = select_winners(terms, &bids);
    let spent_cents = selection.spent_cents();
    let views = selection.expected_views();
    #[expect(clippy::cast_possible_wrap, reason = "far fewer winners than i64::MAX")]
    let videos = selection.winner_count() as i64;

    Some(Estimate {
        videos: round_to_two_figures(videos),
        views: round_to_two_figures(views),
        // `None` without winners.
        average_cpm_cents: cpm_cents(spent_cents, views)?,
        spent_cents,
        fills_budget: spent_cents >= share_of(terms.budget_cents, FILLS_BUDGET_BPS),
    })
}

/// Every planned size, smallest first. `terms` exist once the campaign has a budget and a target
/// CPM, and `accounts` once it has a platform.
#[must_use]
pub fn sizes(
    size_groups: &[SizeGroup],
    terms: Option<&Terms>,
    accounts: Option<&BTreeMap<SizeGroup, i64>>,
) -> Vec<SizeEstimate> {
    let budgets = terms.map(Terms::split_budget).unwrap_or_default();
    PLANNED_GROUPS
        .into_iter()
        .map(|group| {
            let share_cents = budgets
                .iter()
                .find(|budget| budget.group == group)
                .map(|budget| budget.budget_cents);
            let (typical_price_cents, affordable) = match (terms, group.typical_views()) {
                (Some(terms), Some(views)) => {
                    let price_cents = typical_price_cents(terms, views);
                    // Only a picked size has a share of the budget to fit in.
                    let affordable =
                        share_cents.map(|share| terms.can_bid(views) && price_cents <= share);
                    (Some(price_cents), affordable)
                }
                _ => (None, None),
            };
            SizeEstimate {
                group,
                min_views: group.min_views(),
                max_views: group.max_views(),
                picked: size_groups.contains(&group),
                accounts: accounts
                    .and_then(|counts| counts.get(&group))
                    .map(|&count| round_to_two_figures(count)),
                typical_price_cents,
                affordable,
            }
        })
        .collect()
}

/// Rounds half up to two significant figures, so it does not pass for exact. Exact below 100.
#[must_use]
pub fn round_to_two_figures(n: i64) -> i64 {
    // 10 to the power of (digits - 2): 1 below 100, 10 for three digits, 100 for four.
    let step = 10_i64.pow(n.checked_ilog10().unwrap_or(0).saturating_sub(1));
    (n + step / 2) / step * step
}

/// The suggested bid, as likely bidders bid. An account that cannot bid would need at least its
/// campaign rate, so that is shown instead.
fn typical_price_cents(terms: &Terms, typical_views: i32) -> i64 {
    if terms.can_bid(typical_views) {
        terms.suggested_bid_cents(typical_views)
    } else {
        terms.campaign_rate_cents(typical_views)
    }
}

/// How many matching accounts each sampled candidate stands for, rounded half up, at least 1.
fn copies_per_candidate(matching_accounts: i64, sampled: usize) -> usize {
    let matching = usize::try_from(matching_accounts).unwrap_or(0);
    ((matching + sampled / 2) / sampled).max(1)
}

/// One bid at its suggested bid from each copy of each likely bidder.
fn likely_bids(candidates: &[Candidate], terms: &Terms, copies: usize) -> Vec<PendingBid> {
    let mut bidders: Vec<&Candidate> = Vec::new();
    for group in terms.picked_groups() {
        let in_group: Vec<&Candidate> = candidates
            .iter()
            .filter(|c| SizeGroup::of(c.view_score) == group && terms.can_bid(c.view_score))
            .collect();
        if in_group.is_empty() {
            continue;
        }
        let mut scores: Vec<i16> = in_group.iter().map(|c| c.match_score).collect();
        scores.sort_unstable();
        let threshold = percentile(&scores, LIKELY_BIDDER_PERCENTILE);
        bidders.extend(in_group.into_iter().filter(|c| c.match_score >= threshold));
    }
    bidders
        .into_iter()
        .flat_map(|c| std::iter::repeat_n(c, copies))
        .zip(0..)
        .map(|(c, id)| PendingBid {
            id,
            amount_cents: terms.suggested_bid_cents(c.view_score),
            view_score: c.view_score,
            match_score: c.match_score,
            reliability_score: c.reliability_score,
        })
        .collect()
}

/// The nearest-rank percentile of sorted, non-empty scores.
fn percentile(sorted: &[i16], percent: usize) -> i16 {
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SizeGroup::{Macro, Mega, Micro, Nano};
    use crate::rules::{DEFAULT_MIN_OFFER_BPS, TEST_RULES};

    fn candidate(view_score: i32, match_score: i16) -> Candidate {
        Candidate {
            view_score,
            match_score,
            reliability_score: 50,
        }
    }

    /// Terms that offer 100%, so that campaign rates are usual rates.
    fn terms(budget_cents: i64, size_groups: &[SizeGroup]) -> Terms {
        Terms {
            budget_cents,
            size_groups: size_groups.to_vec(),
            offer_bps: 10_000,
            rules: TEST_RULES,
        }
    }

    #[test]
    fn buys_from_the_likely_bidders_at_their_suggested_bids() {
        // Usual rates of €100, €190, €190 and €590. Nano, micro and macro get €1,000 each of the
        // €3,000. Macro goes first and leaves €410 to micro, which leaves €1,030 to nano.
        let candidates = [
            candidate(1_000, 50),
            candidate(10_000, 50),
            candidate(10_000, 50),
            candidate(50_000, 50),
        ];

        let bought = estimate(&candidates, &terms(300_000, &[Nano, Micro, Macro]), 4);

        // €1,070 for 71,000 views.
        assert_eq!(
            bought,
            Some(Estimate {
                videos: 4,
                views: 71_000,
                average_cpm_cents: 1_507,
                spent_cents: 107_000,
                fills_budget: false,
            })
        );
        // A bid may take only €175 of €700, less than the €190 rate, so the account bids €175.
        let small = estimate(&[candidate(10_000, 50)], &terms(70_000, &[Micro]), 1);
        assert_eq!(small.unwrap().spent_cents, 17_500);
    }

    #[test]
    fn a_target_above_the_max_bid_still_gives_an_estimate() {
        // A €50 target is far above the €19 usual CPM of nano, micro and macro, so the campaign
        // rates are above the max bid, and likely bidders bid their max bid.
        let sizes = [Nano, Micro, Macro];
        let offer_bps = TEST_RULES.offer_bps(5_000, &sizes, DEFAULT_MIN_OFFER_BPS);
        let terms = Terms {
            offer_bps: offer_bps.unwrap(),
            ..terms(1_000_000, &sizes)
        };

        let estimate = estimate(&[candidate(10_000, 50)], &terms, 1).unwrap();

        assert_eq!(estimate.spent_cents, TEST_RULES.max_bid_cents(10_000));
    }

    #[test]
    fn likely_bidders_are_the_top_quarter_by_match_of_each_size() {
        // The nano account matches worst of all but is the best of its size. Of the micro ones,
        // only those at or above the size's 75th percentile, 30, bid.
        let candidates = [
            candidate(1_000, 5),
            candidate(10_000, 90),
            candidate(10_000, 30),
            candidate(10_000, 20),
            candidate(10_000, 10),
        ];

        let estimate = estimate(&candidates, &terms(300_000, &[Nano, Micro, Macro]), 5).unwrap();

        // Micro's €1,000 would buy all four.
        assert_eq!(estimate.videos, 3);
        assert_eq!(estimate.views, 21_000);
    }

    #[test]
    fn each_sampled_candidate_stands_for_the_accounts_left_out() {
        // Two candidates sampled from 20 matching accounts, so each bids ten times. The starter
        // is in no picked size, so only the micro one bids.
        let candidates = [candidate(10_000, 50), candidate(100, 50)];

        let estimate = estimate(&candidates, &terms(200_000, &[Micro]), 20).unwrap();

        // Ten bids of €190 take 95% of €2,000.
        assert_eq!(estimate.videos, 10);
        assert_eq!(estimate.views, 100_000);
        assert_eq!(estimate.spent_cents, 190_000);
        assert!(estimate.fills_budget);
    }

    #[test]
    fn no_likely_bidder_gives_no_estimate() {
        let terms = terms(100_000, &[Micro]);
        assert_eq!(estimate(&[], &terms, 0), None);
        // A starter, in no size a campaign can pick.
        assert_eq!(estimate(&[candidate(100, 50)], &terms, 1), None);
        // A bid may take €90 of €360, less than the €95 min bid of a €190 account.
        let smallest = Terms {
            budget_cents: 36_000,
            ..terms
        };
        assert_eq!(estimate(&[candidate(10_000, 50)], &smallest, 1), None);
    }

    #[test]
    fn sizes_show_what_each_picked_size_can_afford() {
        // Typical accounts bid their usual rates, all within the €2,500 a bid may take of
        // €10,000. Mega is not picked, and its €10,090 is shown for comparison.
        let counts = BTreeMap::from([(Nano, 4_123), (Micro, 1_000), (Macro, 55), (Mega, 0)]);

        let known = sizes(
            &[Nano, Micro, Macro],
            Some(&terms(1_000_000, &[Nano, Micro, Macro])),
            Some(&counts),
        );

        let size = |group: SizeGroup, accounts, price, affordable| SizeEstimate {
            group,
            min_views: group.min_views(),
            max_views: group.max_views(),
            picked: group != Mega,
            accounts: Some(accounts),
            typical_price_cents: Some(price),
            affordable,
        };
        assert_eq!(
            known,
            [
                size(Nano, 4_100, 10_500, Some(true)),
                size(Micro, 1_000, 19_000, Some(true)),
                size(Macro, 55, 99_000, Some(true)),
                size(Mega, 0, 1_009_000, None),
            ]
        );
    }

    #[test]
    fn a_size_whose_min_bid_the_budget_does_not_allow_is_not_affordable() {
        // A bid may take €250 of €1,000, less than the €495 min bid of a typical macro account.
        let sizes = sizes(&[Macro], Some(&terms(100_000, &[Macro])), None);

        assert_eq!(sizes[2].group, Macro);
        assert_eq!(sizes[2].typical_price_cents, Some(99_000));
        assert_eq!(sizes[2].affordable, Some(false));
    }

    #[test]
    fn sizes_without_terms_or_counts_show_only_the_sizes() {
        let unknown = sizes(&[Mega], None, None);

        assert_eq!(unknown[3].max_views, None);
        assert!(unknown[3].picked);
        assert!(unknown.iter().all(|size| size.accounts.is_none()
            && size.typical_price_cents.is_none()
            && size.affordable.is_none()));
    }

    #[test]
    fn rounds_to_two_significant_figures() {
        assert_eq!(round_to_two_figures(0), 0);
        assert_eq!(round_to_two_figures(99), 99);
        assert_eq!(round_to_two_figures(1_234), 1_200);
        assert_eq!(round_to_two_figures(1_250), 1_300);
        assert_eq!(round_to_two_figures(999), 1_000);
        assert_eq!(round_to_two_figures(123_456_789), 120_000_000);
    }
}
