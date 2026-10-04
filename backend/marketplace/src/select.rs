//! Winner selection. Closing, the advertiser's progress view and the estimate all run it, so
//! what the advertiser is shown is what closing does.

use std::cmp::Ordering;

use serde::Serialize;

use crate::money::share_of;
use crate::{GroupBudget, SizeGroup, Terms};

/// A bid, with the account's figures as of when it was placed or last edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct PendingBid {
    /// Unique per campaign. A lower id is an earlier bid, which wins a tie.
    pub id: i64,
    pub amount_cents: i64,
    pub view_score: i32,
    /// 0 to 100.
    pub match_score: i16,
    /// The creator's reliability, 0 to 100.
    pub reliability_score: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost(LossReason),
}

/// Why a bid lost. The database's `loss_reason` enum has the same values in the same order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "loss_reason", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum LossReason {
    /// Its size's budget went to bids of better value.
    Outranked,
    /// It was skipped for not fitting, and a bid of worse value won after it: it lost on price,
    /// not on value.
    DidNotFit,
    /// Above its account's max bid, or above the share of the budget one winning bid may take.
    /// Bidding refuses only the first, as creators are not shown the budget.
    OverLimit,
}

/// What one picked size spent, and on what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupResult {
    pub group: SizeGroup,
    /// The size's own share of the budget.
    pub budget_cents: i64,
    /// Can exceed `budget_cents`, as a size also spends what the bigger one before it left.
    pub spent_cents: i64,
    pub winners: usize,
    /// Sum of the winners' view scores.
    pub expected_views: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// One per bid, in input order.
    pub outcomes: Vec<Outcome>,
    /// One per picked size, smallest first.
    pub groups: Vec<GroupResult>,
    /// The budget no size spent.
    pub returned_cents: i64,
}

impl Selection {
    #[must_use]
    pub fn winner_count(&self) -> usize {
        self.groups.iter().map(|group| group.winners).sum()
    }

    #[must_use]
    pub fn spent_cents(&self) -> i64 {
        self.groups.iter().map(|group| group.spent_cents).sum()
    }

    #[must_use]
    pub fn expected_views(&self) -> i64 {
        self.groups.iter().map(|group| group.expected_views).sum()
    }
}

/// Picks the winning bids:
///
/// 1. Bids above their account's max bid or the budget share lose as over the limit.
/// 2. The picked sizes take turns from biggest to smallest, each spending its own share of the
///    budget plus what the bigger size before it left, so money big accounts leave buys more
///    videos from small ones. What the smallest leaves is returned.
/// 3. Within a size, the discovery share of its own budget goes first to bids scoring below the
///    size's median match score. The rest goes to all its remaining bids.
/// 4. Both fills take bids best value first, each only if it fits in what is left. A bid that
///    does not fit is skipped, and cheaper ones after it may still win.
/// 5. A loser did not fit if the main fill took a bid ranked below it, and was outranked
///    otherwise. Discovery winners do not count: that money was set aside for weaker matches.
///
/// Bids from sizes the campaign did not pick lose as outranked (the feed and bidding keep them
/// out anyway). The outcomes do not depend on the order of the bids.
#[must_use]
pub fn select_winners(terms: &Terms, bids: &[PendingBid]) -> Selection {
    // `None` until the bid is decided.
    let mut outcomes: Vec<Option<Outcome>> = bids
        .iter()
        .map(|bid| is_over_limit(terms, bid).then_some(Outcome::Lost(LossReason::OverLimit)))
        .collect();

    let mut groups = Vec::new();
    let mut left_over_cents = 0;
    for budget in terms.split_budget().into_iter().rev() {
        let ranked = rank_group(bids, &outcomes, budget.group);
        let discovery_cents = share_of(budget.budget_cents, terms.rules.discovery_bps);
        let available_cents = budget.budget_cents + left_over_cents;
        let fill = fill_group(bids, &ranked, discovery_cents, available_cents);
        for (place, &i) in ranked.iter().enumerate() {
            outcomes[i] = Some(fill.outcome_at(place));
        }
        left_over_cents = available_cents - fill.spent_cents;
        groups.push(fill.result(budget, bids, &ranked));
    }
    groups.reverse();

    Selection {
        outcomes: outcomes
            .into_iter()
            .map(|outcome| outcome.unwrap_or(Outcome::Lost(LossReason::Outranked)))
            .collect(),
        groups,
        returned_cents: left_over_cents,
    }
}

/// Step 1.
fn is_over_limit(terms: &Terms, bid: &PendingBid) -> bool {
    bid.amount_cents > terms.rules.max_bid_cents(bid.view_score)
        || bid.amount_cents > terms.rules.max_share_of_budget_cents(terms.budget_cents)
}

/// Indices of the size's undecided bids, best value first.
fn rank_group(bids: &[PendingBid], outcomes: &[Option<Outcome>], group: SizeGroup) -> Vec<usize> {
    let mut ranked: Vec<usize> = (0..bids.len())
        .filter(|&i| outcomes[i].is_none() && SizeGroup::of(bids[i].view_score) == group)
        .collect();
    ranked.sort_by(|&a, &b| best_value(&bids[a], &bids[b]));
    ranked
}

/// Steps 3 and 4: the discovery fill up to `discovery_cents`, then the main fill.
fn fill_group(
    bids: &[PendingBid],
    ranked: &[usize],
    discovery_cents: i64,
    available_cents: i64,
) -> GroupFill {
    let mut fill = GroupFill {
        taken: vec![false; ranked.len()],
        spent_cents: 0,
        last_main_place: None,
    };

    let weaker_than = median_match_score(bids, ranked);
    for (place, &i) in ranked.iter().enumerate() {
        let bid = &bids[i];
        if bid.match_score < weaker_than && fill.fits(bid, discovery_cents) {
            fill.take(place, bid);
        }
    }

    for (place, &i) in ranked.iter().enumerate() {
        let bid = &bids[i];
        if !fill.taken[place] && fill.fits(bid, available_cents) {
            fill.take(place, bid);
            fill.last_main_place = Some(place);
        }
    }
    fill
}

/// The upper median match score, so that the weaker matches (below it) are never more than
/// half. 0 without bids.
fn median_match_score(bids: &[PendingBid], ranked: &[usize]) -> i16 {
    let mut scores: Vec<i16> = ranked.iter().map(|&i| bids[i].match_score).collect();
    scores.sort_unstable();
    scores.get(scores.len() / 2).copied().unwrap_or(0)
}

/// What one size's two fills took, by place in the size's ranking.
struct GroupFill {
    taken: Vec<bool>,
    spent_cents: i64,
    last_main_place: Option<usize>,
}

impl GroupFill {
    fn fits(&self, bid: &PendingBid, limit_cents: i64) -> bool {
        self.spent_cents + bid.amount_cents <= limit_cents
    }

    fn take(&mut self, place: usize, bid: &PendingBid) {
        self.taken[place] = true;
        self.spent_cents += bid.amount_cents;
    }

    /// Step 5.
    fn outcome_at(&self, place: usize) -> Outcome {
        if self.taken[place] {
            Outcome::Won
        } else if self.last_main_place.is_some_and(|last| last > place) {
            Outcome::Lost(LossReason::DidNotFit)
        } else {
            Outcome::Lost(LossReason::Outranked)
        }
    }

    fn result(&self, budget: GroupBudget, bids: &[PendingBid], ranked: &[usize]) -> GroupResult {
        let winners: Vec<&PendingBid> = ranked
            .iter()
            .zip(&self.taken)
            .filter(|&(_, &taken)| taken)
            .map(|(&i, _)| &bids[i])
            .collect();
        GroupResult {
            group: budget.group,
            budget_cents: budget.budget_cents,
            spent_cents: self.spent_cents,
            winners: winners.len(),
            expected_views: winners.iter().map(|bid| i64::from(bid.view_score)).sum(),
        }
    }
}

/// Orders bids best value first: lowest amount / (views × match score), so a better match may
/// cost more per view. A score of 0 counts as 1. Ties go to the more reliable creator, then to
/// the earlier bid.
fn best_value(a: &PendingBid, b: &PendingBid) -> Ordering {
    // Cross-multiplied, so the comparison stays exact in whole numbers.
    let a_side =
        i128::from(a.amount_cents) * i128::from(b.view_score) * i128::from(b.match_score.max(1));
    let b_side =
        i128::from(b.amount_cents) * i128::from(a.view_score) * i128::from(a.match_score.max(1));
    a_side
        .cmp(&b_side)
        .then(b.reliability_score.cmp(&a.reliability_score))
        .then(a.id.cmp(&b.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PLANNED_GROUPS;
    use crate::SizeGroup::{Macro, Micro, Nano};
    use crate::rules::TEST_RULES;

    const WON: Outcome = Outcome::Won;
    const OUTRANKED: Outcome = Outcome::Lost(LossReason::Outranked);
    const DID_NOT_FIT: Outcome = Outcome::Lost(LossReason::DidNotFit);
    const OVER_LIMIT: Outcome = Outcome::Lost(LossReason::OverLimit);

    /// Terms offering the usual rate. Winner selection does not read the offer.
    fn terms(budget_cents: i64, size_groups: &[SizeGroup]) -> Terms {
        Terms {
            budget_cents,
            size_groups: size_groups.to_vec(),
            offer_bps: 10_000,
            rules: TEST_RULES,
        }
    }

    fn bid(id: i64, amount_cents: i64, view_score: i32, match_score: i16) -> PendingBid {
        PendingBid {
            id,
            amount_cents,
            view_score,
            match_score,
            reliability_score: 50,
        }
    }

    #[test]
    fn bids_above_the_max_bid_lose_as_over_limit() {
        // A nano account with a €100 usual rate may bid up to €171.42 by its views.
        let bids = [bid(1, 17_142, 1_000, 50), bid(2, 17_143, 1_000, 50)];

        let selection = select_winners(&terms(300_000, &[Nano]), &bids);

        assert_eq!(selection.outcomes, [WON, OVER_LIMIT]);
    }

    #[test]
    fn bids_above_the_budget_share_lose_as_over_limit() {
        // A macro account with a €590 usual rate may bid up to €1,011.42, but a winning bid may
        // take only €750 of €3,000.
        let bids = [bid(1, 75_000, 50_000, 50), bid(2, 75_001, 50_000, 50)];

        let selection = select_winners(&terms(300_000, &[Macro]), &bids);

        assert_eq!(selection.outcomes, [WON, OVER_LIMIT]);
    }

    #[test]
    fn bids_from_sizes_the_campaign_did_not_pick_lose_as_outranked() {
        // A starter, and a mega account.
        let bids = [bid(1, 9_000, 499, 50), bid(2, 100_000, 300_000, 50)];

        let selection = select_winners(&terms(1_000_000, &[Nano, Micro, Macro]), &bids);

        assert_eq!(selection.outcomes, [OUTRANKED, OUTRANKED]);
    }

    #[test]
    fn best_value_is_the_lowest_cost_per_match_point_then_reliability_then_age() {
        // €15 at a match of 90 beats €10 at 40 for the same views: 15 / 90 < 10 / 40.
        let better_match = bid(1, 15_000, 10_000, 90);
        let cheaper = bid(2, 10_000, 10_000, 40);
        assert_eq!(best_value(&better_match, &cheaper), Ordering::Less);
        let unmatched = bid(3, 10_000, 10_000, 0);
        assert_eq!(
            best_value(&unmatched, &bid(4, 100_000, 10_000, 1)),
            Ordering::Less
        );
        let reliable = PendingBid {
            reliability_score: 90,
            ..bid(8, 10_000, 10_000, 40)
        };
        assert_eq!(best_value(&reliable, &cheaper), Ordering::Less);
        assert_eq!(
            best_value(&cheaper, &bid(7, 10_000, 10_000, 40)),
            Ordering::Less
        );
    }

    #[test]
    fn discovery_takes_a_weaker_match_the_main_fill_would_not() {
        // Nano has €1,000, of which 15%, €150, is for discovery. The median score is 90, so only
        // the bid scoring 20 counts as a weaker match.
        let mut bids: Vec<PendingBid> = (1..=5).map(|id| bid(id, 20_000, 2_900, 90)).collect();
        bids.push(bid(6, 12_000, 2_900, 20));

        let selection = select_winners(&terms(100_000, &[Nano]), &bids);

        // Without discovery, the five best matches would take the €1,000. The fifth is
        // outranked, not did-not-fit: the discovery winner's money was set aside.
        assert_eq!(selection.outcomes, [WON, WON, WON, WON, OUTRANKED, WON]);
        assert_eq!(selection.spent_cents(), 92_000);
    }

    #[test]
    fn unspent_budget_rolls_from_the_biggest_size_to_the_smallest() {
        // Nano and macro get €400 each. Macro goes first and leaves €200, so nano has €600 for
        // three bids of €180; the other way round, only two of them would have won.
        let bids = [
            bid(1, 18_000, 2_000, 50),
            bid(2, 18_000, 2_000, 50),
            bid(3, 18_000, 2_000, 50),
            bid(4, 20_000, 30_000, 50),
        ];

        let selection = select_winners(&terms(80_000, &[Nano, Macro]), &bids);

        assert_eq!(selection.outcomes, [WON, WON, WON, WON]);
        let spent: Vec<(SizeGroup, i64, i64)> = selection
            .groups
            .iter()
            .map(|group| (group.group, group.budget_cents, group.spent_cents))
            .collect();
        assert_eq!(spent, [(Nano, 40_000, 54_000), (Macro, 40_000, 20_000)]);
        // What nano, the smallest, leaves is returned.
        assert_eq!(selection.returned_cents, 6_000);
    }

    #[test]
    fn a_loser_did_not_fit_only_if_a_worse_bid_won_after_it() {
        // Micro goes first with its €1,000. Bigger accounts give more views per euro, so the bids
        // rank as listed: €400 and €350 win, €300 is skipped with €250 left, €200 wins, and €150
        // is skipped with €50 left.
        let bids = [
            bid(1, 40_000, 29_000, 50),
            bid(2, 35_000, 25_000, 50),
            bid(3, 30_000, 20_000, 50),
            bid(4, 20_000, 12_000, 50),
            bid(5, 15_000, 8_000, 50),
        ];

        let selection = select_winners(&terms(200_000, &[Nano, Micro]), &bids);

        assert_eq!(selection.outcomes, [WON, WON, DID_NOT_FIT, WON, OUTRANKED]);
    }

    #[test]
    fn outcomes_do_not_depend_on_the_order_of_the_bids() {
        // 24 bids spread over every size, with amounts from €90 to €250 and match scores that
        // jump around (37 and 101 share no factor, so i × 37 % 101 does not repeat).
        let views = [800, 2_000, 6_000, 20_000, 60_000, 400_000];
        let mut bids: Vec<PendingBid> = (0..24_i16)
            .map(|i| {
                let view_score = views[usize::try_from(i % 6).unwrap()];
                bid(
                    i64::from(i),
                    9_000 + i64::from(i % 5) * 4_000,
                    view_score,
                    i * 37 % 101,
                )
            })
            .collect();
        let terms = terms(150_000, &PLANNED_GROUPS);
        let outcomes_by_id = |bids: &[PendingBid]| {
            let selection = select_winners(&terms, bids);
            let mut outcomes: Vec<(i64, Outcome)> = bids
                .iter()
                .map(|bid| bid.id)
                .zip(selection.outcomes)
                .collect();
            outcomes.sort_by_key(|&(id, _)| id);
            (outcomes, selection.groups, selection.returned_cents)
        };

        let first = outcomes_by_id(&bids);
        assert!(first.0.iter().any(|&(_, outcome)| outcome == WON));
        bids.reverse();
        assert_eq!(outcomes_by_id(&bids), first);
        bids.rotate_left(7);
        assert_eq!(outcomes_by_id(&bids), first);
    }

    #[test]
    fn a_selection_adds_up_what_its_sizes_bought() {
        // Macro's €1,000 buys the €400 bid, and micro's the two €100 ones.
        let bids = [
            bid(1, 40_000, 40_000, 50),
            bid(2, 10_000, 5_000, 50),
            bid(3, 10_000, 5_000, 50),
        ];

        let selection = select_winners(&terms(200_000, &[Micro, Macro]), &bids);

        assert_eq!(selection.winner_count(), 3);
        assert_eq!(selection.spent_cents(), 60_000);
        assert_eq!(selection.expected_views(), 50_000);
        assert_eq!(selection.returned_cents, 200_000 - 60_000);
    }
}
