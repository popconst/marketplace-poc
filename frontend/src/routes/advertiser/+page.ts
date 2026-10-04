import { getAdvertisers, unwrap } from '$lib/api';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => ({
	advertisers: unwrap(await getAdvertisers(fetch))
});
