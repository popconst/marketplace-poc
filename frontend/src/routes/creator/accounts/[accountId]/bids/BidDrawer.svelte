<script lang="ts">
	import { tick } from 'svelte';
	import BidStatusBadge from '$lib/components/BidStatusBadge.svelte';
	import Countdown from '$lib/components/Countdown.svelte';
	import PlatformBadge from '$lib/components/PlatformBadge.svelte';
	import { formatCents, formatCount, formatDateTime, formatNumber } from '$lib/format';
	import type { BidListItem, PlatformAccount } from '$lib/types';
	import AccountStats from '../AccountStats.svelte';
	import { canChange, lossReasons } from './bid';

	interface Props {
		/** The account the bids were placed with. */
		account: PlatformAccount;
		/** `Options.viewsCountingDays`: how long after posting the video's views count. */
		viewsCountingDays: number;
		/** Opens the bid panel on the bid. */
		onedit: (bid: BidListItem) => void;
	}

	let { account, viewsCountingDays, onedit }: Props = $props();

	const DAY_MS = 86_400_000;

	let dialog: HTMLDialogElement;
	let scroller = $state<HTMLDivElement>();
	// Kept after closing, so the drawer keeps its content while it slides out.
	let bid = $state.raw<BidListItem | null>(null);
	/** The card's button that opened the drawer, which takes focus back when it closes. */
	let opener: HTMLElement | null = null;

	export async function open(next: BidListItem, button: HTMLElement) {
		bid = next;
		opener = button;
		// Renders this bid first, so the drawer never opens on the last one.
		await tick();
		scroller?.scrollTo(0, 0);
		dialog.showModal();
	}

	function edit() {
		dialog.close();
		if (bid) onedit(bid);
	}

	// Counted from the bidding deadline, which closing early doesn't move.
	function postBy(campaign: BidListItem['campaign']): Date {
		return new Date(Date.parse(campaign.biddingDeadline) + campaign.submissionWindowDays * DAY_MS);
	}

	/**
	 * When bidding closed before its deadline, as the demo's Debug menu allows; null otherwise.
	 * Closing decides every bid at once, so the bid's `decidedAt` is when it closed.
	 */
	function closedEarlyAt({ decidedAt, campaign }: BidListItem): string | null {
		return decidedAt !== null && Date.parse(decidedAt) < Date.parse(campaign.biddingDeadline)
			? decidedAt
			: null;
	}
</script>

{#snippet line(label: string, value: string)}
	<div class="flex items-baseline justify-between gap-4">
		<dt class="text-neutral-600">{label}</dt>
		<dd class="font-semibold tabular-nums">{value}</dd>
	</div>
{/snippet}

<!-- A click on the backdrop lands on the dialog itself; clicks inside land on its children. -->
<dialog
	bind:this={dialog}
	aria-labelledby="bid-drawer-title"
	onclick={(event) => event.target === dialog && dialog.close()}
	onclose={() => opener?.focus()}
	class="sheet border border-white/60 bg-base-100 p-0 shadow-lift"
>
	{#if bid}
		{@const { campaign } = bid}
		{@const biddingOpen = campaign.phase === 'open'}
		{@const wonOrPending = bid.status === 'won' || bid.status === 'pending'}
		{@const closedAt = closedEarlyAt(bid)}
		{@const editable = canChange(bid)}
		<div class="flex h-full max-h-[inherit] flex-col">
			<div
				aria-hidden="true"
				class="mx-auto mt-2 h-1.5 w-9 shrink-0 rounded-full bg-neutral-300 sm:hidden"
			></div>
			<header class="flex items-start gap-4 border-b border-base-300 p-6 max-sm:pt-4">
				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-semibold text-neutral-500">{campaign.advertiserName}</p>
					<h2 id="bid-drawer-title" class="mt-1 title-section break-words">{campaign.title}</h2>
					<div class="mt-3 flex flex-wrap items-center gap-2">
						<BidStatusBadge status={bid.status} />
						<PlatformBadge platform={campaign.platform} />
						{#if biddingOpen}
							<Countdown deadline={campaign.biddingDeadline} />
						{/if}
					</div>
					{#if bid.status === 'pending' && !biddingOpen}
						<p class="mt-3 text-sm text-neutral-600">Bidding closed, winners are being picked</p>
					{:else if bid.status === 'lost' && bid.lossReason}
						<p class="mt-3 text-sm text-neutral-600">{lossReasons[bid.lossReason]}</p>
					{/if}
				</div>
				<button
					type="button"
					class="btn btn-circle btn-ghost btn-sm"
					aria-label="Close"
					onclick={() => dialog.close()}
				>
					<svg aria-hidden="true" viewBox="0 0 16 16" class="size-4">
						<path
							d="M4 4l8 8M12 4l-8 8"
							fill="none"
							stroke="currentColor"
							stroke-width="1.75"
							stroke-linecap="round"
						/>
					</svg>
				</button>
			</header>

			<!-- Only this part scrolls, so the title and the action stay in view. -->
			<div
				bind:this={scroller}
				class={[
					'flex min-h-0 flex-1 flex-col gap-8 overflow-y-auto overscroll-contain p-6',
					!editable && 'pb-[max(1.5rem,env(safe-area-inset-bottom))]'
				]}
			>
				<!-- Together, so the views to get paid read against the account's usual views. -->
				<div class="flex flex-col gap-2">
					<AccountStats {account} />
					<dl class="flex flex-col gap-2 rounded-2xl bg-neutral-50 px-5 py-4 text-sm">
						{@render line('Your bid', formatCents(bid.amountCents))}
						{#if wonOrPending}
							<div class="flex items-baseline justify-between gap-4">
								<dt class="text-neutral-600">
									Views to get paid
									<span class="block text-xs text-neutral-500">
										within {formatCount(viewsCountingDays, 'day')} of posting
									</span>
								</dt>
								<dd class="font-semibold tabular-nums">{formatNumber(bid.minPaidViews)}</dd>
							</div>
							<div
								class="mt-1 flex items-baseline justify-between gap-4 border-t border-base-300 pt-3"
							>
								<dt class="font-semibold">
									{bid.status === 'won' ? 'You receive' : 'Take-home if you win'}
								</dt>
								<dd class="text-2xl font-extrabold tracking-tight tabular-nums">
									{formatCents(bid.payoutCents)}
								</dd>
							</div>
						{:else}
							{@render line('Views it needed to get paid', formatNumber(bid.minPaidViews))}
						{/if}
					</dl>
				</div>

				<section aria-labelledby="bid-drawer-timing">
					<h3 id="bid-drawer-timing" class="text-sm font-semibold text-neutral-500">Timing</h3>
					<!-- In the order things happen. -->
					<dl class="mt-3 flex flex-col gap-2 text-sm">
						{#if bid.status === 'withdrawn'}
							{@render line('Withdrawn', formatDateTime(bid.updatedAt))}
						{/if}
						{#if closedAt}
							{@render line('Bidding closed early', formatDateTime(closedAt))}
						{:else}
							{@render line(
								biddingOpen ? 'Bidding closes' : 'Bidding closed',
								formatDateTime(campaign.biddingDeadline)
							)}
							{#if bid.decidedAt}
								{@render line('Winners picked', formatDateTime(bid.decidedAt))}
							{/if}
						{/if}
						{#if wonOrPending}
							{@render line(
								bid.status === 'won' ? 'Post by' : 'Post by, if you win',
								formatDateTime(postBy(campaign))
							)}
						{/if}
					</dl>
				</section>

				<section aria-labelledby="bid-drawer-briefing">
					<h3 id="bid-drawer-briefing" class="text-sm font-semibold text-neutral-500">Briefing</h3>
					<p class="mt-2 leading-relaxed break-words whitespace-pre-line text-neutral-600">
						{campaign.briefing}
					</p>
				</section>
			</div>

			{#if editable}
				<footer
					class="flex border-t border-base-300 p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))]"
				>
					<button type="button" class="btn ml-auto btn-neutral max-sm:flex-1" onclick={edit}>
						{bid.status === 'withdrawn' ? 'Bid again' : 'Edit bid'}
					</button>
				</footer>
			{/if}
		</div>
	{/if}
</dialog>
