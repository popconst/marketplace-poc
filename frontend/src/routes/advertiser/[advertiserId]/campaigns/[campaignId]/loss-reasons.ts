import type { LossReason } from '$lib/types';

export const lossReasons: Record<LossReason, { label: string; explanation: string }> = {
	outranked: {
		label: 'Outranked',
		explanation: 'Other bids gave better value for fit.'
	},
	did_not_fit: {
		label: 'Too expensive',
		explanation: 'Too expensive for the money left; a cheaper bid took it.'
	},
	over_limit: {
		label: 'Over the limit',
		explanation: 'Above the highest bid allowed.'
	}
};
