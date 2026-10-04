import { getCampaigns, unwrap } from '$lib/api';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => ({
	campaigns: unwrap(await getCampaigns(fetch, Number(params.advertiserId)))
});
