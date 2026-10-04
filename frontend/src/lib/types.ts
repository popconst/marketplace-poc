// The API's JSON, mirroring the Rust structs in backend/api/src/routes and
// backend/db/src/models.rs. Money is in euro cents; `*Bps` fields are basis points (2500 = 25%).

/** RFC 3339, e.g. `2026-10-01T12:00:00Z`. */
export type Timestamp = string;

export type Platform = 'tiktok' | 'instagram';

export type BidStatus = 'pending' | 'won' | 'lost' | 'withdrawn';

export type LossReason =
	/** Its size group's budget went to bids of better value. */
	| 'outranked'
	/** It cost more than its group had left, and a cheaper, worse-value bid won after it. */
	| 'did_not_fit'
	/** It was above the account's max bid, or above the most one winning bid may take. */
	| 'over_limit';

/** Account sizes by views per post: nano 500 to 3K, micro to 30K, macro to 300K, mega above. */
export type SizeGroup = 'nano' | 'micro' | 'macro' | 'mega';

export const SIZE_GROUPS: SizeGroup[] = ['nano', 'micro', 'macro', 'mega'];

export interface Genre {
	id: number;
	name: string;
}

export interface Country {
	/** ISO 3166-1 alpha-2, e.g. `DE`. */
	code: string;
	name: string;
}

export interface Language {
	/** ISO 639-1, e.g. `de`. */
	code: string;
	name: string;
}

/** `GET /api/options`: the choices the forms and filters offer, and the rules they follow. */
export interface Options {
	genres: Genre[];
	/** Only countries with a brand-safe account, since no other account can be matched. */
	countries: Country[];
	/** Only languages at least one account speaks. */
	languages: Language[];
	limits: CampaignLimits;
	/** How many days after posting a video's views count toward the minimum it must reach. */
	viewsCountingDays: number;
	/** The sizes a campaign can pick, smallest first. */
	sizeGroups: SizeRange[];
	/** Whether the API serves the demo tools under `/api/dev`. */
	devTools: boolean;
}

/** An account size and the views per post it covers. */
export interface SizeRange {
	group: SizeGroup;
	minViews: number;
	/** Null for mega, which has no upper bound. */
	maxViews: number | null;
}

/** Both ends included. */
export interface Bounds {
	min: number;
	max: number;
}

/** `Limits` in api/src/routes/campaigns.rs: what a campaign may be saved and published with. */
export interface CampaignLimits {
	budgetCents: Bounds;
	targetCpmCents: Bounds;
	/** How long bidding may stay open after publishing, in hours. */
	biddingHours: Bounds;
	/** Days after the bidding deadline that winners have to post: from 1 to `max`. */
	postingWindowDays: { default: number; max: number };
	titleMaxChars: number;
	briefingMaxChars: number;
}

/** An item of `GET /api/advertisers`. */
export interface Advertiser {
	id: number;
	name: string;
	createdAt: Timestamp;
}

/** A creator's account with its latest stats: `GET /api/platform-accounts[/{accountId}]`. */
export interface PlatformAccount {
	id: number;
	creatorId: number;
	creatorName: string;
	platform: Platform;
	handle: string;
	countryCode: string;
	followers: number;
	/** Median views per post over posts 2 to 90 days old. */
	viewScore: number;
	/** (likes + comments + shares + saves) / views, from 0 to 1. */
	engagementRate: number;
	postsPerWeek: number;
	/** Content quality from 0 to 100. */
	qualityScore: number;
	/** How dependably the creator delivers, from 0 to 100; shared by all their accounts. */
	reliabilityScore: number;
	/** Accounts that are not brand safe are never matched to a campaign. */
	brandSafe: boolean;
	genreIds: number[];
	languageCodes: string[];
	statsUpdatedAt: Timestamp;
	createdAt: Timestamp;
}

/** `in_review` is never stored: a real review step would stop there, this demo approves at once. */
export type CampaignStatus = 'draft' | 'in_review' | 'active' | 'closed' | 'failed';

/** The status as the UI shows it: an active campaign is `open` until its deadline, then `closing`. */
export type CampaignPhase = 'draft' | 'in_review' | 'open' | 'closing' | 'closed' | 'failed';

/** A draft until published; publishing requires the fields `PublishedCampaign` makes non-null. */
export interface Campaign {
	id: number;
	advertiserId: number;
	status: CampaignStatus;
	title: string | null;
	/** What creators are asked to post. */
	briefing: string | null;
	platform: Platform | null;

	budgetCents: number | null;
	/** What 1,000 expected views are worth to the advertiser. */
	targetCpmCents: number | null;
	/** The account sizes it hires from, at least one. The budget is split evenly across them. */
	sizeGroups: SizeGroup[];
	/** WePush's commission on each winning bid, frozen on publishing; null on a draft. */
	commissionBps: number | null;

	/**
	 * Matching weights: 0, 25, 50, 75 or 100. They make up the match score, which ranks the campaign
	 * in creators' feeds and is weighed against a bid's price when winners are picked.
	 */
	engagementWeight: number;
	qualityWeight: number;
	reliabilityWeight: number;

	/** When bidding closes and winners are picked; null on a draft. */
	biddingDeadline: Timestamp | null;
	/** Winners must post within this many days of the bidding deadline. */
	submissionWindowDays: number;

	createdAt: Timestamp;
	updatedAt: Timestamp;
	/** When it was published. */
	submittedAt: Timestamp | null;
	/** When bidding opened. */
	activatedAt: Timestamp | null;
	closedAt: Timestamp | null;
	/** How often the closing job has failed, and its last error. */
	closeAttempts: number;
	lastCloseError: string | null;
}

/** Who a campaign is for. An empty list means any. */
export interface CampaignTargeting {
	countryCodes: string[];
	languageCodes: string[];
	genreIds: number[];
}

/** `GET /api/campaigns/{campaignId}`, and each item of `GET /api/advertisers/{id}/campaigns`. */
export type CampaignDetail = Campaign & CampaignTargeting & { phase: CampaignPhase };

/** A campaign past its draft, with the fields publishing requires. */
export type PublishedCampaign = CampaignDetail & {
	title: string;
	briefing: string;
	platform: Platform;
	budgetCents: number;
	targetCpmCents: number;
	commissionBps: number;
	biddingDeadline: Timestamp;
};

export function isPublished(campaign: CampaignDetail): campaign is PublishedCampaign {
	return campaign.status !== 'draft';
}

/** What the advertiser edits. Null clears a nullable field. */
export type CampaignFields = Pick<
	Campaign,
	| 'title'
	| 'briefing'
	| 'platform'
	| 'budgetCents'
	| 'targetCpmCents'
	| 'sizeGroups'
	| 'engagementWeight'
	| 'qualityWeight'
	| 'reliabilityWeight'
	| 'submissionWindowDays'
> &
	CampaignTargeting;

/** `PATCH /api/campaigns/{campaignId}`. Lists replace the set; publishing checks completeness. */
export type CampaignPatch = Partial<CampaignFields>;

/** One platform account's offer to post for a campaign (`db::models::Bid`). */
export interface Bid {
	id: number;
	campaignId: number;
	platformAccountId: number;
	platform: Platform;
	/** What the advertiser pays. The creator receives this minus the commission. */
	amountCents: number;
	// The next three are copied when the bid is placed or last edited.
	viewScore: number;
	reliabilityScore: number;
	matchScore: number;
	status: BidStatus;
	/** Set only on lost bids. */
	lossReason: LossReason | null;
	/** Set when the bid is won or lost. */
	decidedAt: Timestamp | null;
	createdAt: Timestamp;
	updatedAt: Timestamp;
}

/** `GET /api/campaigns/{campaignId}/estimate`. Anything that needs a `missing` field is null. */
export interface Estimate {
	missing: ('platform' | 'budgetCents' | 'targetCpmCents')[];
	/** Accounts of the picked sizes that match the targeting, rounded. */
	matchingAccounts: number | null;
	/** True when there are more than `matchingAccounts`, which is then the cap. */
	matchingAccountsCapped: boolean;
	/** All four sizes, smallest first, picked or not. */
	sizes: SizeEstimate[];
	// The five figures below are null together.
	/** How many videos the budget buys. */
	videos: number | null;
	views: number | null;
	/** What the videos cost per 1,000 of their views. */
	averageCpmCents: number | null;
	spentCents: number | null;
	/** False when the targeting and sizes can't take the whole budget. */
	fillsBudget: boolean | null;
}

export interface SizeEstimate {
	group: SizeGroup;
	minViews: number;
	/** Null for mega, which has no upper bound. */
	maxViews: number | null;
	picked: boolean;
	/** Matching accounts of this size, rounded. */
	accounts: number | null;
	/** What a video from an account this size usually costs at the target CPM. */
	typicalPriceCents: number | null;
	/** Whether the campaign can pay one creator this size that much; null when unknown. */
	affordable: boolean | null;
}

/** `GET /api/campaigns/{campaignId}/progress`, active only: the winners if bidding closed now. */
export interface CampaignProgress {
	pendingBids: number;
	budgetCents: number;
	spentCents: number;
	/** The budget no size group would spend. */
	returnedCents: number;
	winners: number;
	/** The winners' min paid views, added up: what their videos must reach for all to be paid. */
	minPaidViews: number;
	/** The winners' view scores when they bid, added up. */
	expectedViews: number;
	/** What the winners cost per 1,000 of their views; null without winners. */
	effectiveCpmCents: number | null;
	targetCpmCents: number;
	/** One for each size group the plan gives budget to, smallest first. */
	groups: GroupProgress[];
}

export interface GroupProgress {
	group: SizeGroup;
	budgetCents: number;
	/** Can exceed the group's budget, as a group also spends what the one before it left. */
	spentCents: number;
	winners: number;
	/** Pending bids from accounts in the group. */
	bids: number;
}

/** `GET /api/campaigns/{campaignId}/results`, closed only. Figures as in `CampaignProgress`. */
export interface CampaignResults {
	closedAt: Timestamp;
	budgetCents: number;
	spentCents: number;
	returnedCents: number;
	minPaidViews: number;
	expectedViews: number;
	effectiveCpmCents: number | null;
	targetCpmCents: number;
	/** Smallest size group first, then the highest bid. */
	winners: Winner[];
	/** By loss reason, in `LossReason`'s order, then the highest bid. Withdrawn bids are left out. */
	losers: Loser[];
}

export interface Winner {
	accountId: number;
	handle: string;
	creatorName: string;
	countryCode: string;
	/** The account's stats now, not when it bid. */
	followers: number;
	engagementRate: number;
	/** The views the bid was priced on: the account's views per post when it bid. */
	viewScore: number;
	/** The score winner selection used, explained by factors from the account's stats now. */
	match: Match;
	amountCents: number;
	/** By the bid's views, as the winners were picked. */
	sizeGroup: SizeGroup;
	minPaidViews: number;
	/** What the advertiser pays per 1,000 of the views the account expected when it bid. */
	expectedCpmCents: number;
}

export interface Loser {
	accountId: number;
	handle: string;
	creatorName: string;
	countryCode: string;
	viewScore: number;
	match: Match;
	amountCents: number;
	lossReason: LossReason;
	sizeGroup: SizeGroup;
	expectedCpmCents: number;
}

export interface Match {
	/** From 0 to 100. */
	score: number;
	factors: MatchFactor[];
}

export interface MatchFactor {
	kind: 'engagement' | 'quality' | 'reliability' | 'genre';
	/** How much it counts in this campaign's score, from 0 to 1. */
	weight: number;
	/** How well the account does on it, from 0 to 1. */
	value: number;
}

/** An open campaign as a creator sees it. Everything publishing requires is set. */
export interface FeedCampaign extends CampaignTargeting {
	id: number;
	advertiserId: number;
	advertiserName: string;
	title: string;
	briefing: string;
	platform: Platform;
	targetCpmCents: number;
	commissionBps: number;
	biddingDeadline: Timestamp;
	submissionWindowDays: number;
}

/** `GET /api/platform-accounts/{accountId}/campaigns`, without the account (see `getAccount`). */
export interface Feed {
	items: FeedItem[];
}

/** A campaign in the feed, and what this account may bid on it. */
export interface FeedItem {
	campaign: FeedCampaign;
	match: Match;
	/** The least this account may bid on any campaign. */
	minBidCents: number;
	/** The most the account may bid: above it, the video is unlikely to reach its views. */
	maxBidCents: number;
	/** The account's usual rate scaled by the campaign's target CPM, capped so that it can win. */
	suggestedBidCents: number;
	suggestedMinPaidViews: number;
	/** The views needed to get paid per euro bid, for previews. A placed bid has the exact number. */
	minPaidViewsPerEuro: number;
	/** This account's bid on the campaign, whatever its status. */
	myBid: Bid | null;
}

/** Best match, highest payout, or soonest deadline first. */
export type FeedSort = 'match' | 'payout' | 'deadline';

/** `GET /api/campaigns/{campaignId}/bids/{accountId}/chance`: a bid against the bids so far. */
export interface BidChance {
	/** Whether the bid would win if bidding closed now. */
	wouldWin: boolean;
	/** The campaign's pending bids, this one included. */
	pendingBids: number;
}

/** `GET /api/platform-accounts/{accountId}/bids`. */
export interface BidList {
	items: BidListItem[];
}

export interface BidListItem extends Bid {
	campaign: {
		id: number;
		title: string;
		advertiserName: string;
		platform: Platform;
		briefing: string;
		biddingDeadline: Timestamp;
		/** Winners must post within this many days of the bidding deadline. */
		submissionWindowDays: number;
		phase: CampaignPhase;
	};
	/** What the creator receives for this bid, after the commission. */
	payoutCents: number;
	/** The views the video must reach within `Options.viewsCountingDays` for the creator to be paid. */
	minPaidViews: number;
}

/** `GET /api/platform-accounts` sorts by one of these, descending. */
export type AccountSort = 'view_score' | 'followers' | 'engagement_rate' | 'quality_score';

/** A keyset-paginated list. There is no total count. */
export interface Page<T> {
	items: T[];
	/** Pass as `cursor` to get the next page; null on the last one. */
	nextCursor: string | null;
}

/** Every error body. `error` is a code like `validation`; a 422 may add per-field messages. */
export interface ErrorBody {
	error: string;
	message: string;
	fields?: Record<string, string>;
}

// Demo only: served under `/api/dev`.

/** `POST /api/dev/simulate-bids`: what one round of bidding by bots did. */
export interface Simulation {
	/** How many campaigns were open. */
	campaigns: number;
	placed: number;
	/** Bids a rule refused, mostly because another account of the creator has a pending bid. */
	skipped: number;
}

/** `POST /api/dev/campaigns/{campaignId}/close`: what closing the campaign decided. */
export interface Closing {
	winners: number;
	losers: number;
	spentCents: number;
	/** The budget no size group spent. */
	returnedCents: number;
}
