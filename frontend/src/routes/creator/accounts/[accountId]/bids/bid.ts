import type { BidListItem, LossReason } from '$lib/types';

/** Why a bid lost, in the creator's words. */
export const lossReasons: Record<LossReason, string> = {
	outranked: 'Other bids gave the advertiser better value for your size group',
	did_not_fit: 'Too high for the money left; a cheaper bid took it',
	over_limit: 'Your bid was above what this campaign allows'
};

/** Whether the creator can still edit the bid, or place it again after withdrawing it. */
export const canChange = (bid: BidListItem) =>
	bid.campaign.phase === 'open' && (bid.status === 'pending' || bid.status === 'withdrawn');
