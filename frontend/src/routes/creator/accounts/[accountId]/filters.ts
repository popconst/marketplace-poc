import type { FeedSort } from '$lib/types';
import { positiveInt } from '$lib/url-params';

/** Kept in the URL under the API's parameter names, so one query string serves page and API. */
export interface FeedFilters {
	sort: FeedSort;
	/** A genre id, or null for any. */
	genre: number | null;
	/** The least take-home the creator accepts: above 0, or null for no minimum. */
	minPayoutCents: number | null;
}

const DEFAULT_SORT: FeedSort = 'match';

export const sortLabels: Record<FeedSort, string> = {
	match: 'Best match',
	payout: 'Highest pay',
	deadline: 'Deadline soon'
};

const isSort = (value: string): value is FeedSort => Object.hasOwn(sortLabels, value);

/** Drops values the API would reject, so a hand-edited URL loads the feed instead of an error. */
export function parseFilters(params: URLSearchParams): FeedFilters {
	const sort = params.get('sort') ?? '';
	return {
		sort: isSort(sort) ? sort : DEFAULT_SORT,
		genre: positiveInt(params.get('genre')),
		minPayoutCents: positiveInt(params.get('min_payout_cents'))
	};
}

/** Leaves out what is unset or default, so every filter state has one URL. */
export function toSearchParams({ sort, genre, minPayoutCents }: FeedFilters): URLSearchParams {
	const params = new URLSearchParams();
	if (sort !== DEFAULT_SORT) params.set('sort', sort);
	if (genre !== null) params.set('genre', String(genre));
	if (minPayoutCents !== null) params.set('min_payout_cents', String(minPayoutCents));
	return params;
}
