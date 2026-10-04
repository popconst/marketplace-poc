import { formatDateTime } from '$lib/format';
import type { CampaignDetail } from '$lib/types';

/** The closing job's own error means nothing to an advertiser, so only the demo tools show it. */
export const CLOSING_FAILED = 'Winners couldn’t be picked.';

export function describePhase(campaign: CampaignDetail): string {
	switch (campaign.phase) {
		case 'draft':
			return `Edited ${formatDateTime(campaign.updatedAt)}`;
		case 'in_review':
			return 'Waiting for approval';
		case 'open':
			return campaign.biddingDeadline
				? `Bidding closes ${formatDateTime(campaign.biddingDeadline)}`
				: 'Open for bids';
		case 'closing':
			return 'Bidding closed, picking winners';
		case 'closed':
			return campaign.closedAt ? `Closed ${formatDateTime(campaign.closedAt)}` : 'Closed';
		case 'failed':
			return CLOSING_FAILED;
	}
}
