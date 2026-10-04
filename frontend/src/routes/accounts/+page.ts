import { listAccounts } from '$lib/api';
import type { PageLoad } from './$types';
import { parseFilters, toSearchParams } from './filters';

export const load: PageLoad = ({ fetch, url }) => {
	const filters = parseFilters(url.searchParams);
	// Not awaited, so the page renders at once with placeholder rows.
	return { filters, accounts: listAccounts(fetch, toSearchParams(filters)) };
};
