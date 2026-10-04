import { getFeed } from '$lib/api';
import type { PageLoad } from './$types';
import { parseFilters, toSearchParams } from './filters';

export const load: PageLoad = async ({ depends, fetch, parent, url }) => {
	// The bid panel reloads the feed by this name after a bid.
	depends('app:feed');
	const { account } = await parent();
	const filters = parseFilters(url.searchParams);
	// Awaited, so after a filter change or a bid the cards on screen stay until the new feed is in.
	return { filters, feed: await getFeed(fetch, account.id, toSearchParams(filters)) };
};
