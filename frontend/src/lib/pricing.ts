// What the bid panel shows while the creator types, computed as the server does so the preview
// matches what is paid. The limits come from the feed; only these formulas are copied from Rust.

import { formatCents, formatCount, formatNumber } from './format';
import type { FeedItem } from './types';

/** WePush's commission on a bid, rounded half up to the cent, as `Rules::commission_cents` does. */
export function commissionCents(amountCents: number, commissionBps: number): number {
	return Math.round((amountCents * commissionBps) / 10_000);
}

/**
 * The views the video must reach to get paid at this bid, rounded up, as `Rules::min_paid_views`
 * does. The feed's per-euro figure is a float, so this can come out one above the exact number.
 */
export function minPaidViewsAt(item: FeedItem, amountCents: number): number {
	return Math.ceil((item.minPaidViewsPerEuro * amountCents) / 100);
}

/** Mirrors `check_amount` in api/src/routes/bids.rs, in its order and words; null if valid. */
export function amountProblem(
	item: FeedItem,
	amountCents: number,
	viewsCountingDays: number
): string | null {
	if (amountCents < item.minBidCents) {
		return `Must be at least ${formatCents(item.minBidCents)}.`;
	}
	if (amountCents > item.maxBidCents) {
		const views = formatNumber(minPaidViewsAt(item, amountCents));
		const days = formatCount(viewsCountingDays, 'day');
		return (
			`At ${formatCents(amountCents)} your video would need ${views} views within ${days}, ` +
			`more than it is likely to reach. Bid at most ${formatCents(item.maxBidCents)}.`
		);
	}
	return null;
}
