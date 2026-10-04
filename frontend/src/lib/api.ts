import { error } from '@sveltejs/kit';
import type {
	Advertiser,
	Bid,
	BidChance,
	BidList,
	BidStatus,
	CampaignDetail,
	CampaignPatch,
	CampaignProgress,
	CampaignResults,
	Closing,
	ErrorBody,
	Estimate,
	Feed,
	Options,
	Page,
	PlatformAccount,
	Simulation,
	Timestamp
} from './types';

export type ApiError =
	/** No response arrived: server unreachable, connection refused, request aborted. */
	| { kind: 'network'; cause: unknown }
	| { kind: 'http'; status: number; body: unknown }
	/** The server answered 2xx, but the body could not be read as JSON. */
	| { kind: 'invalid-body'; status: number; cause: unknown };

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** A load function's own `fetch`, so `invalidate` tracks the request, or else the global one. */
type Fetch = typeof fetch;

async function request<T>(fetch: Fetch, path: string, init?: RequestInit): Promise<ApiResult<T>> {
	let response: Response;
	try {
		response = await fetch(`/api${path}`, init);
	} catch (cause) {
		return { ok: false, error: { kind: 'network', cause } };
	}

	if (!response.ok) {
		// Error bodies are best-effort: a proxy in front of the API may answer with HTML.
		const body: unknown = await response.json().catch(() => undefined);
		return { ok: false, error: { kind: 'http', status: response.status, body } };
	}

	try {
		// Trusts the server's shape; validate at the call site where it matters.
		return { ok: true, data: (await response.json()) as T };
	} catch (cause) {
		return { ok: false, error: { kind: 'invalid-body', status: response.status, cause } };
	}
}

function withJsonBody(method: string, body: unknown): RequestInit {
	return { method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) };
}

/** A sentence to show the user. */
export function errorMessage(error: ApiError): string {
	switch (error.kind) {
		case 'network':
			return 'Can’t reach the server. Check that the API is running, then try again.';
		case 'http':
			return (
				(error.body as ErrorBody | undefined)?.message ??
				`The server answered with status ${error.status}.`
			);
		case 'invalid-body':
			return 'The server sent a response that could not be read.';
	}
}

/** Per-field messages from a 422, or null. A rule spanning fields has none; see `errorMessage`. */
export function fieldErrors(error: ApiError): Record<string, string> | null {
	if (error.kind !== 'http' || error.status !== 422) return null;
	const fields = (error.body as ErrorBody | undefined)?.fields;
	return fields && Object.keys(fields).length > 0 ? fields : null;
}

/** A 409: the page is out of date, as when a campaign stopped taking bids after it loaded. */
export function isConflict(error: ApiError): boolean {
	return error.kind === 'http' && error.status === 409;
}

/** For load functions: the data, or else SvelteKit's error page with the reason. */
export function unwrap<T>(result: ApiResult<T>): T {
	if (result.ok) return result.data;
	// No usable response at all means the API is unavailable.
	error(result.error.kind === 'http' ? result.error.status : 503, errorMessage(result.error));
}

export const getOptions = (fetch: Fetch) => request<Options>(fetch, '/options');

export const getAdvertisers = (fetch: Fetch) => request<Advertiser[]>(fetch, '/advertisers');

/** The advertiser's campaigns, most recently updated first. */
export const getCampaigns = (fetch: Fetch, advertiserId: number) =>
	request<CampaignDetail[]>(fetch, `/advertisers/${advertiserId}/campaigns`);

/** Starts an empty draft. */
export const createCampaign = (fetch: Fetch, advertiserId: number) =>
	request<CampaignDetail>(fetch, `/advertisers/${advertiserId}/campaigns`, { method: 'POST' });

export const getCampaign = (fetch: Fetch, campaignId: number) =>
	request<CampaignDetail>(fetch, `/campaigns/${campaignId}`);

export const updateCampaign = (fetch: Fetch, campaignId: number, patch: CampaignPatch) =>
	request<CampaignDetail>(fetch, `/campaigns/${campaignId}`, withJsonBody('PATCH', patch));

/** Publishes a draft. The deadline must be within `Options.limits.biddingHours` from now. */
export const publishCampaign = (fetch: Fetch, campaignId: number, biddingDeadline: Timestamp) =>
	request<CampaignDetail>(
		fetch,
		`/campaigns/${campaignId}/publish`,
		withJsonBody('POST', { biddingDeadline })
	);

export const getEstimate = (fetch: Fetch, campaignId: number) =>
	request<Estimate>(fetch, `/campaigns/${campaignId}/estimate`);

/** What an active campaign's bids would buy if bidding closed now. Other campaigns get a 409. */
export const getCampaignProgress = (fetch: Fetch, campaignId: number) =>
	request<CampaignProgress>(fetch, `/campaigns/${campaignId}/progress`);

/** What a closed campaign's winning bids bought. Other campaigns get a 409. */
export const getCampaignResults = (fetch: Fetch, campaignId: number) =>
	request<CampaignResults>(fetch, `/campaigns/${campaignId}/results`);

export const getAccount = (fetch: Fetch, accountId: number) =>
	request<PlatformAccount>(fetch, `/platform-accounts/${accountId}`);

/**
 * `query` takes `q`, `platform`, `country`, `genre`, `language`, `min_views`, `max_views`,
 * `min_engagement`, `max_engagement`, `sort`, `cursor` and `limit`. The API rejects empty values
 * here and in `getFeed`, so leave unset ones out. It ignores a `q` too short to search by.
 */
export const listAccounts = (fetch: Fetch, query: URLSearchParams) =>
	request<Page<PlatformAccount>>(fetch, `/platform-accounts?${query}`);

/** Open campaigns the account can bid on. `query` takes `genre`, `sort` and `min_payout_cents`. */
export const getFeed = (fetch: Fetch, accountId: number, query: URLSearchParams) =>
	request<Feed>(fetch, `/platform-accounts/${accountId}/campaigns?${query}`);

/** The account's bids, newest first: all of them, or those with `status`. */
export function getBids(fetch: Fetch, accountId: number, status: BidStatus | null) {
	const query = new URLSearchParams(status ? { status } : {});
	return request<BidList>(fetch, `/platform-accounts/${accountId}/bids?${query}`);
}

/** Places a bid, changes a pending one, or reinstates a withdrawn one. */
export const placeBid = (
	fetch: Fetch,
	campaignId: number,
	accountId: number,
	amountCents: number
) =>
	request<Bid>(
		fetch,
		`/campaigns/${campaignId}/bids/${accountId}`,
		withJsonBody('PUT', { amountCents })
	);

export const withdrawBid = (fetch: Fetch, campaignId: number, accountId: number) =>
	request<Bid>(fetch, `/campaigns/${campaignId}/bids/${accountId}`, { method: 'DELETE' });

/**
 * Whether a bid of this amount would win if bidding closed now. The amount is checked as for
 * `placeBid`, with the same errors.
 */
export const getBidChance = (
	fetch: Fetch,
	campaignId: number,
	accountId: number,
	amountCents: number
) =>
	request<BidChance>(
		fetch,
		`/campaigns/${campaignId}/bids/${accountId}/chance?amount_cents=${amountCents}`
	);

// Demo only: served when the API runs with `DEV_TOOLS=true`, as `Options.devTools` reports.

/** Simulated creators bid on every open campaign, by the same rules as real ones. */
export const simulateBids = (fetch: Fetch) =>
	request<Simulation>(fetch, '/dev/simulate-bids', { method: 'POST' });

/** Closes an active campaign now, whatever its deadline, as the worker does once it passes. */
export const closeCampaign = (fetch: Fetch, campaignId: number) =>
	request<Closing>(fetch, `/dev/campaigns/${campaignId}/close`, { method: 'POST' });

/** Demo only: moves an active campaign's deadline to 10 seconds from now, for the worker. */
export const closeCampaignSoon = (fetch: Fetch, campaignId: number) =>
	request<{ biddingDeadline: Timestamp }>(fetch, `/dev/campaigns/${campaignId}/close-soon`, {
		method: 'POST'
	});
