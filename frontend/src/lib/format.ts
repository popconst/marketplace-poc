import type { BidStatus, Options, Platform, SizeGroup, SizeRange } from './types';

// English as written where the euro is used: €2,500, 48.2K, 24-hour times.
const LOCALE = 'en-IE';

export const platformNames: Record<Platform, string> = {
	tiktok: 'TikTok',
	instagram: 'Instagram'
};

export const bidStatusLabels: Record<BidStatus, string> = {
	pending: 'Pending',
	won: 'Won',
	lost: 'Lost',
	withdrawn: 'Withdrawn'
};

export const sizeGroupLabels: Record<SizeGroup, string> = {
	nano: 'Nano',
	micro: 'Micro',
	macro: 'Macro',
	mega: 'Mega'
};

/** Names for country, language and genre codes. A code the options leave out shows as itself. */
export function optionNames(options: Pick<Options, 'countries' | 'languages' | 'genres'>) {
	return {
		country: (code: string) => options.countries.find((c) => c.code === code)?.name ?? code,
		language: (code: string) => options.languages.find((l) => l.code === code)?.name ?? code,
		genre: (id: number) => options.genres.find((g) => g.id === id)?.name ?? String(id)
	};
}

const euros = new Intl.NumberFormat(LOCALE, { style: 'currency', currency: 'EUR' });
const wholeEuros = new Intl.NumberFormat(LOCALE, {
	style: 'currency',
	currency: 'EUR',
	maximumFractionDigits: 0
});

/** `€2,500` for whole euros, `€12.50` otherwise. */
export function formatCents(cents: number): string {
	return (cents % 100 === 0 ? wholeEuros : euros).format(cents / 100);
}

/** Rounded to whole euros: `€11,500`. */
export function formatCentsRounded(cents: number): string {
	return wholeEuros.format(Math.round(cents / 100));
}

const whole = new Intl.NumberFormat(LOCALE, { maximumFractionDigits: 0 });

export function formatNumber(n: number): string {
	return whole.format(n);
}

/** `1 bid`, `3,158 bids`: adds an `s` unless the count is one. */
export function formatCount(count: number, noun: string): string {
	return `${formatNumber(count)} ${count === 1 ? noun : `${noun}s`}`;
}

const compact = new Intl.NumberFormat(LOCALE, { notation: 'compact', maximumFractionDigits: 1 });

/** `48.2K`, `1.3M`. */
export function formatCompact(n: number): string {
	return compact.format(n);
}

/** `3K–30K views per post`, or `300K+ views per post` for mega. */
export function formatSizeViews({ minViews, maxViews }: SizeRange): string {
	const views =
		maxViews === null
			? `${formatCompact(minViews)}+`
			: `${formatCompact(minViews)}–${formatCompact(maxViews)}`;
	return `${views} views per post`;
}

const list = new Intl.ListFormat(LOCALE, { type: 'conjunction' });

/** `a, b and c`. */
export function formatList(items: string[]): string {
	return list.format(items);
}

const percent = new Intl.NumberFormat(LOCALE, { style: 'percent', maximumFractionDigits: 1 });

/** A ratio from 0 to 1 as `6.4%`. */
export function formatPercent(ratio: number): string {
	return percent.format(ratio);
}

/** Basis points as `25%`. */
export function formatBps(bps: number): string {
	return percent.format(bps / 10_000);
}

const dateTime = new Intl.DateTimeFormat(LOCALE, {
	weekday: 'short',
	day: 'numeric',
	month: 'short',
	hour: '2-digit',
	minute: '2-digit'
});

/** `Fri, 3 Oct, 14:00` in the viewer's time zone. */
export function formatDateTime(date: Date | string): string {
	return dateTime.format(new Date(date));
}

const time = new Intl.DateTimeFormat(LOCALE, {
	hour: '2-digit',
	minute: '2-digit',
	second: '2-digit'
});

/** `14:00:05` in the viewer's time zone. */
export function formatTime(date: Date): string {
	return time.format(date);
}

const relative = new Intl.RelativeTimeFormat(LOCALE, { numeric: 'auto' });
const HOUR_MS = 3_600_000;

/** `in 5 hours`, `in 3 days`. */
export function formatFromNow(date: Date, now = Date.now()): string {
	const hours = Math.round((date.getTime() - now) / HOUR_MS);
	return Math.abs(hours) < 48
		? relative.format(hours, 'hour')
		: relative.format(Math.round(hours / 24), 'day');
}

/** Regional indicator symbol A; B to Z follow it. */
const REGIONAL_INDICATOR_A = 0x1f1e6;

/** The flag emoji of an ISO 3166-1 alpha-2 code: its two letters as regional indicators. */
export function flag(countryCode: string): string {
	const A = 'A'.charCodeAt(0);
	return String.fromCodePoint(
		...[...countryCode.toUpperCase()].map(
			(letter) => REGIONAL_INDICATOR_A + letter.charCodeAt(0) - A
		)
	);
}
