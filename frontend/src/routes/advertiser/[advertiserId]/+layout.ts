import { error } from '@sveltejs/kit';
import { getAdvertisers, unwrap } from '$lib/api';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch, params }) => {
	// The API has no single-advertiser endpoint; the list is only a handful of demo brands.
	const advertisers = unwrap(await getAdvertisers(fetch));
	const advertiser = advertisers.find(({ id }) => String(id) === params.advertiserId);
	if (!advertiser) error(404, 'There is no advertiser with this id.');
	return { advertiser };
};
