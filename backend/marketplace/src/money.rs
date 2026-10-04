//! Shared arithmetic: basis-point shares and costs per 1,000 views. Amounts are euro cents.

/// 100% in basis points.
pub(crate) const BPS: i64 = 10_000;

/// `bps` basis points of `cents`, rounded down.
pub(crate) fn share_of(cents: i64, bps: i64) -> i64 {
    cents * bps / BPS
}

/// Cost per 1,000 views, rounded down. `None` without views.
#[must_use]
pub fn cpm_cents(cents: i64, views: i64) -> Option<i64> {
    (views > 0).then(|| cents * 1_000 / views)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpm_is_the_cost_of_1000_views() {
        // €1,070 for 71,000 views is €15.07 per 1,000, rounded down.
        assert_eq!(cpm_cents(107_000, 71_000), Some(1_507));
        assert_eq!(cpm_cents(107_000, 0), None);
    }
}
