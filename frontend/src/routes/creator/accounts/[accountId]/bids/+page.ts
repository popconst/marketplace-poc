import { getBids } from '$lib/api';
import { bidStatusLabels } from '$lib/format';
import type { BidStatus } from '$lib/types';
import type { PageLoad } from './$types';

const isStatus = (value: string): value is BidStatus => Object.hasOwn(bidStatusLabels, value);

export const load: PageLoad = async ({ depends, fetch, parent, url }) => {
	// The page reloads the bids by this name when a campaign closes.
	depends('app:bids');
	const { account } = await parent();
	// An unknown status, as in a hand-edited URL, shows every bid.
	const param = url.searchParams.get('status') ?? '';
	const status = isStatus(param) ? param : null;
	// Awaited, so on a reload the cards on screen stay until the new list is in.
	return { status, bids: await getBids(fetch, account.id, status) };
};
