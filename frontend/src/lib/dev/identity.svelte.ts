// Demo only: with no sign-in, who the user acts as is picked in a dialog and kept in this browser.
// It only steers navigation; pages load their data by the ids in their URL. A real app would take
// it from the session (`grep -rnF '$lib/dev/' src` lists the code that would change).

import { resolve } from '$app/paths';
import type { PlatformAccount } from '$lib/types';

export type Role = 'advertiser' | 'creator';

export const roleLabels: Record<Role, string> = {
	advertiser: 'Advertiser',
	creator: 'Creator'
};

export type Identity =
	| { role: 'advertiser'; advertiserId: number; name: string }
	| {
			role: 'creator';
			name: string;
			/** The account they act with: campaigns are matched to accounts, not to people. */
			accountId: number;
			handle: string;
	  };

export function creatorIdentity(
	account: Pick<PlatformAccount, 'id' | 'handle' | 'creatorName'>
): Identity {
	return {
		role: 'creator',
		name: account.creatorName,
		accountId: account.id,
		handle: account.handle
	};
}

/** Where the identity works: the advertiser's campaigns, or the creator account's feed. */
export function homePath(identity: Identity): string {
	return identity.role === 'advertiser'
		? resolve('/advertiser/[advertiserId]', { advertiserId: String(identity.advertiserId) })
		: resolve('/creator/accounts/[accountId]', { accountId: String(identity.accountId) });
}

/** How the identity is shown: an advertiser by name, a creator by the account they act with. */
export function displayName(identity: Identity): string {
	return identity.role === 'advertiser' ? identity.name : `@${identity.handle}`;
}

const STORAGE_KEY = 'wepush:demo-identity';

class DemoIdentity {
	current = $state<Identity | null>(read());

	choose(identity: Identity) {
		this.current = identity;
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(identity));
		} catch {
			// Storage can be unavailable (private browsing); the choice still holds until reload.
		}
	}
}

function read(): Identity | null {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return stored ? (JSON.parse(stored) as Identity) : null;
	} catch {
		return null;
	}
}

export const identity = new DemoIdentity();
