import { platformNames } from '$lib/format';
import type { AccountSort, Platform } from '$lib/types';
import { positiveInt, ratio } from '$lib/url-params';

/** Kept in the URL under the API's parameter names, so one query string serves page and API. */
export interface AccountFilters {
	q: string;
	platform: Platform | null;
	country: string | null;
	genre: number | null;
	language: string | null;
	/** Inclusive bounds on views per post. */
	minViews: number | null;
	maxViews: number | null;
	/** Inclusive bounds on the engagement rate, from 0 to 1. */
	minEngagement: number | null;
	maxEngagement: number | null;
	sort: AccountSort;
}

export const NO_FILTERS: Omit<AccountFilters, 'sort'> = {
	q: '',
	platform: null,
	country: null,
	genre: null,
	language: null,
	minViews: null,
	maxViews: null,
	minEngagement: null,
	maxEngagement: null
};

/** The API ignores shorter searches (`MIN_SEARCH_CHARS` in api/src/routes/platform_accounts.rs). */
export const MIN_QUERY_LENGTH = 3;

const DEFAULT_SORT: AccountSort = 'view_score';

export const sortLabels: Record<AccountSort, string> = {
	view_score: 'Most views',
	followers: 'Most followers',
	engagement_rate: 'Highest engagement',
	quality_score: 'Best quality'
};

const isPlatform = (value: string): value is Platform => Object.hasOwn(platformNames, value);
const isSort = (value: string): value is AccountSort => Object.hasOwn(sortLabels, value);

/** Drops values the API would reject, so a hand-edited URL lists accounts instead of an error. */
export function parseFilters(params: URLSearchParams): AccountFilters {
	const platform = params.get('platform') ?? '';
	const sort = params.get('sort') ?? '';
	return {
		q: params.get('q') ?? '',
		platform: isPlatform(platform) ? platform : null,
		country: params.get('country') || null,
		genre: positiveInt(params.get('genre')),
		language: params.get('language') || null,
		minViews: positiveInt(params.get('min_views')),
		maxViews: positiveInt(params.get('max_views')),
		minEngagement: ratio(params.get('min_engagement')),
		maxEngagement: ratio(params.get('max_engagement')),
		sort: isSort(sort) ? sort : DEFAULT_SORT
	};
}

/** Leaves out what is unset or default, so every filter state has one URL. */
export function toSearchParams(filters: AccountFilters): URLSearchParams {
	const byName: Record<string, string | number | null> = {
		q: filters.q || null,
		platform: filters.platform,
		country: filters.country,
		genre: filters.genre,
		language: filters.language,
		min_views: filters.minViews,
		max_views: filters.maxViews,
		min_engagement: filters.minEngagement,
		max_engagement: filters.maxEngagement,
		sort: filters.sort === DEFAULT_SORT ? null : filters.sort
	};
	const params = new URLSearchParams();
	for (const [name, value] of Object.entries(byName)) {
		if (value !== null) params.set(name, String(value));
	}
	return params;
}

export function hasFilters(filters: AccountFilters): boolean {
	return toSearchParams({ ...filters, sort: DEFAULT_SORT }).size > 0;
}
