<script lang="ts">
	import BidStatusBadge from '$lib/components/BidStatusBadge.svelte';
	import Countdown from '$lib/components/Countdown.svelte';
	import { formatCents, formatCount, formatDateTime, formatNumber } from '$lib/format';
	import type { BidListItem, BidStatus } from '$lib/types';
	import { lossReasons } from './bid';

	interface Props {
		bid: BidListItem;
		/** `Options.viewsCountingDays`: how long after posting the video's views count. */
		viewsCountingDays: number;
		/** Opens the bid's details. Gets the button, to take focus back when they close. */
		onopen: (button: HTMLButtonElement) => void;
		/** Opens the bid panel; undefined once the bid can no longer change. */
		onedit?: (() => void) | undefined;
		/** While the panel's data loads. */
		opening?: boolean;
	}

	let { bid, viewsCountingDays, onopen, onedit, opening = false }: Props = $props();

	const takeHomeLabels: Record<BidStatus, string> = {
		pending: 'Take-home if you win',
		won: 'You receive',
		lost: 'Take-home',
		withdrawn: 'Take-home'
	};

	const campaign = $derived(bid.campaign);
</script>

<!-- The title's button stretches over the card, so a click anywhere opens the bid. The edit
     button comes later and is positioned, as every .btn is, so it stays on top and clickable. -->
<article class="relative flex lift flex-col gap-4 surface sm:flex-row sm:items-center sm:gap-6">
	<div class="min-w-0 flex-1">
		<!-- From sm the amounts hold the card's edge, so the status moves next to the advertiser. -->
		<div class="flex items-center justify-between gap-3 sm:justify-start">
			<p class="min-w-0 truncate text-sm font-semibold text-neutral-500">
				{campaign.advertiserName}
			</p>
			<BidStatusBadge status={bid.status} />
		</div>
		<h2 class="mt-1 title-card">
			<button
				type="button"
				aria-haspopup="dialog"
				class="text-left after:absolute after:inset-0 after:rounded-3xl focus-visible:outline-none focus-visible:after:outline-2 focus-visible:after:outline-offset-2 focus-visible:after:outline-base-content"
				onclick={(event) => onopen(event.currentTarget)}
			>
				{campaign.title}
			</button>
		</h2>
		<p class="mt-2 text-sm text-neutral-600">
			{#if bid.status === 'pending'}
				{#if campaign.phase === 'open'}
					<Countdown deadline={campaign.biddingDeadline} />
				{:else}
					Bidding closed, winners are being picked
				{/if}
			{:else if bid.status === 'lost' && bid.lossReason}
				{lossReasons[bid.lossReason]}
			{:else if bid.status === 'won' && bid.decidedAt}
				Won {formatDateTime(bid.decidedAt)}
			{:else if bid.status === 'withdrawn'}
				Withdrawn {formatDateTime(bid.updatedAt)}
			{/if}
		</p>
		{#if bid.status === 'won' || bid.status === 'pending'}
			<p class="mt-2 text-sm text-neutral-600">
				Your video must reach
				<span class="font-semibold text-black tabular-nums">{formatNumber(bid.minPaidViews)}</span>
				views within {formatCount(viewsCountingDays, 'day')} of posting to get paid.
			</p>
		{/if}
	</div>
	<!-- Fixed width, so amounts line up across cards. Labels fill the first row and amounts the
	     second, each row on one baseline. -->
	<dl
		class="grid grid-flow-col grid-cols-2 grid-rows-[auto_auto] items-baseline gap-x-8 sm:w-60 sm:text-right"
	>
		<dt class="text-xs text-neutral-500">Your bid</dt>
		<dd class="font-semibold tabular-nums">{formatCents(bid.amountCents)}</dd>
		<dt class="text-xs text-neutral-500">{takeHomeLabels[bid.status]}</dt>
		{#if bid.status === 'won' || bid.status === 'pending'}
			<dd class="text-xl font-extrabold tracking-tight tabular-nums">
				{formatCents(bid.payoutCents)}
			</dd>
		{:else}
			<dd class="text-xl font-extrabold text-neutral-300">
				<span aria-hidden="true">—</span><span class="sr-only">Nothing</span>
			</dd>
		{/if}
	</dl>
	{#if onedit}
		<button
			type="button"
			class="btn shrink-0 max-sm:w-full"
			disabled={opening}
			aria-busy={opening}
			onclick={onedit}
		>
			{#if opening}<span class="loading loading-sm loading-spinner"></span>{/if}
			{bid.status === 'withdrawn' ? 'Bid again' : 'Edit bid'}
		</button>
	{/if}
</article>
