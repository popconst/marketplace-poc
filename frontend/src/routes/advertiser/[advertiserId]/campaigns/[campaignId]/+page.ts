import { error } from '@sveltejs/kit';
import {
	getCampaign,
	getCampaignProgress,
	getCampaignResults,
	getEstimate,
	unwrap
} from '$lib/api';
import { positiveInt } from '$lib/url-params';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params, depends }) => {
	// The editor invalidates this after each save, to refresh the estimate, and after publishing.
	depends('app:campaign');

	const campaignId = positiveInt(params.campaignId);
	// A non-numeric id 404s like any unknown route.
	if (campaignId === null) error(404, 'Not Found');
	const campaign = unwrap(await getCampaign(fetch, campaignId));
	if (String(campaign.advertiserId) !== params.advertiserId) {
		error(404, 'This advertiser has no campaign with this id.');
	}
	// Not unwrapped, so a panel whose data fails to load shows the error instead of failing the page.
	return {
		campaign,
		estimate: campaign.status === 'draft' ? await getEstimate(fetch, campaignId) : null,
		progress: campaign.status === 'active' ? await getCampaignProgress(fetch, campaignId) : null,
		results: campaign.status === 'closed' ? await getCampaignResults(fetch, campaignId) : null
	};
};
