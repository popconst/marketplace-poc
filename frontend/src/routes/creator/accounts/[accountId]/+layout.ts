import { error } from '@sveltejs/kit';
import { getAccount, unwrap } from '$lib/api';
import { positiveInt } from '$lib/url-params';
import type { LayoutLoad } from './$types';

// Loads the account once for both tabs. They take its checked id from here, not from the URL.
export const load: LayoutLoad = async ({ fetch, params }) => {
	const accountId = positiveInt(params.accountId);
	if (accountId === null) error(404, 'Not Found');
	return { account: unwrap(await getAccount(fetch, accountId)) };
};
