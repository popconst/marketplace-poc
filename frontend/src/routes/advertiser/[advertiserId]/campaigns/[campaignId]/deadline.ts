// The bidding deadline picked on publishing. Times are in milliseconds, as `Date.now()` gives them.

import { formatCount } from '$lib/format';
import type { CampaignLimits } from '$lib/types';

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
/** Keeps a deadline picked at the edge valid if publishing takes a few minutes. */
const MARGIN_MS = 10 * MINUTE_MS;

type BiddingHours = CampaignLimits['biddingHours'];

/** The earliest and latest deadline if published now, in whole minutes like datetime-local. */
export function deadlineBounds(now: number, hours: BiddingHours) {
	// With no minimum, as under the demo's dev tools, the earliest deadline is a minute away.
	const earliest = hours.min === 0 ? now + MINUTE_MS : now + hours.min * HOUR_MS + MARGIN_MS;
	return {
		min: Math.ceil(earliest / MINUTE_MS) * MINUTE_MS,
		max: Math.floor((now + hours.max * HOUR_MS - MARGIN_MS) / MINUTE_MS) * MINUTE_MS
	};
}

/** A deadline `minutes` from now, on a whole minute. */
export function deadlineInMinutes(minutes: number, now: number): number {
	return Math.ceil((now + minutes * MINUTE_MS) / MINUTE_MS) * MINUTE_MS;
}

/** A deadline `days` from now on the next full hour, kept within the bounds. */
export function deadlineIn(days: number, now: number, hours: BiddingHours): number {
	const { min, max } = deadlineBounds(now, hours);
	const onTheHour = Math.ceil((now + days * DAY_MS) / HOUR_MS) * HOUR_MS;
	return Math.min(Math.max(onTheHour, min), max);
}

export function postBy(deadline: number, days: number): number {
	return deadline + days * DAY_MS;
}

/** `30 days` for whole days from two days up, `24 hours` otherwise. */
export function formatHours(hours: number): string {
	return hours >= 48 && hours % 24 === 0
		? formatCount(hours / 24, 'day')
		: formatCount(hours, 'hour');
}

/** `YYYY-MM-DDTHH:mm` in local time: the value of a datetime-local input. */
export function toLocalInput(ms: number): string {
	const date = new Date(ms);
	const pad = (n: number) => String(n).padStart(2, '0');
	return (
		`${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
		`T${pad(date.getHours())}:${pad(date.getMinutes())}`
	);
}
