<script lang="ts">
	import { tick } from 'svelte';
	import { expoOut } from 'svelte/easing';
	import { prefersReducedMotion, Tween } from 'svelte/motion';
	import { invalidate } from '$app/navigation';
	import {
		errorMessage,
		fieldErrors,
		getBidChance,
		isConflict,
		placeBid,
		withdrawBid,
		type ApiResult
	} from '$lib/api';
	import BidStatusBadge from '$lib/components/BidStatusBadge.svelte';
	import Countdown from '$lib/components/Countdown.svelte';
	import EuroInput from '$lib/components/EuroInput.svelte';
	import Field from '$lib/components/Field.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { formatCents, formatCount, formatDateTime, formatNumber } from '$lib/format';
	import { amountProblem, commissionCents, minPaidViewsAt } from '$lib/pricing';
	import type { Bid, BidChance, FeedCampaign, FeedItem, PlatformAccount } from '$lib/types';
	import AccountStats from './AccountStats.svelte';

	interface Props {
		account: PlatformAccount;
		/** `Options.viewsCountingDays`, for the amount check's message. */
		viewsCountingDays: number;
	}

	let { account, viewsCountingDays }: Props = $props();

	const DAY_MS = 86_400_000;
	const CHANCE_DELAY_MS = 250;

	let dialog: HTMLDialogElement;
	let scroller = $state<HTMLDivElement>();
	let amountInput = $state<HTMLInputElement>();
	let closeButton = $state<HTMLButtonElement>();
	// Kept after closing, so the panel keeps its content while it slides out.
	let item = $state.raw<FeedItem | null>(null);
	/** The amount typed, in cents; null while the box is empty. */
	let amountCents = $state<number | null>(null);
	let busy = $state<'saving' | 'withdrawing' | null>(null);
	/** The server's error for the amount, cleared when the amount changes. */
	let serverAmountError = $state<string | null>(null);
	let formError = $state<string | null>(null);
	/** Set when the campaign stopped taking bids, or the bid was decided, since the panel opened. */
	let closedNotice = $state<string | null>(null);
	/** Whether the amount would win if bidding closed now. Kept while the next answer loads. */
	let chance = $state.raw<BidChance | null>(null);

	// `amountProblem` judges the amount typed now, so it wins; the server's error covers the rest.
	const amountError = $derived(
		item === null || amountCents === null
			? null
			: (amountProblem(item, amountCents, viewsCountingDays) ?? serverAmountError)
	);
	const canSubmit = $derived(
		amountCents !== null && amountError === null && busy === null && closedNotice === null
	);

	const preview = $derived.by(() => {
		if (item === null || amountCents === null || amountCents <= 0) return null;
		const commission = commissionCents(amountCents, item.campaign.commissionBps);
		return {
			bid: amountCents,
			commission,
			payout: amountCents - commission,
			minPaidViews: minPaidViewsAt(item, amountCents)
		};
	});
	const shownPayout = Tween.of(() => preview?.payout ?? 0, {
		duration: () => (prefersReducedMotion.current ? 0 : 500),
		easing: expoOut
	});
	const sliderCents = $derived(item === null ? 0 : clampBid(item, amountCents ?? 0));
	/** The slider's fill, from 0 to 1. */
	const sliderAt = $derived(
		item === null || item.maxBidCents <= item.minBidCents
			? 0
			: (sliderCents - item.minBidCents) / (item.maxBidCents - item.minBidCents)
	);

	// Asks a pause after the amount changes, and only for an amount the server accepts.
	$effect(() => {
		if (item === null || amountCents === null || amountError !== null) return;
		const campaignId = item.campaign.id;
		const amount = amountCents;
		// False once the amount changes again, so a late answer for this one is dropped.
		let current = true;
		const timer = setTimeout(async () => {
			const result = await getBidChance(fetch, campaignId, account.id, amount);
			if (current) chance = result.ok ? result.data : null;
		}, CHANCE_DELAY_MS);
		return () => {
			current = false;
			clearTimeout(timer);
		};
	});

	function clampBid(item: FeedItem, cents: number): number {
		return Math.min(Math.max(cents, item.minBidCents), item.maxBidCents);
	}

	function postBy(campaign: FeedCampaign): Date {
		return new Date(Date.parse(campaign.biddingDeadline) + campaign.submissionWindowDays * DAY_MS);
	}

	export async function open(next: FeedItem) {
		item = next;
		// The existing bid, or else the feed card's suggestion, so the two agree.
		amountCents = next.myBid?.amountCents ?? next.suggestedBidCents;
		busy = null;
		serverAmountError = null;
		formError = null;
		closedNotice = null;
		chance = null;
		// Renders this campaign first, so the amount box exists to be focused.
		await tick();
		// Opens on the briefing, not where the last campaign was scrolled to.
		scroller?.scrollTo(0, 0);
		dialog.showModal();
		// Selected, so typing replaces it; without scrolling, so the briefing stays in view.
		amountInput?.focus({ preventScroll: true });
		amountInput?.select();
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (item === null || amountCents === null || !canSubmit) return;
		busy = 'saving';
		formError = null;
		await settle(await placeBid(fetch, item.campaign.id, account.id, amountCents));
	}

	async function withdraw() {
		if (item === null) return;
		busy = 'withdrawing';
		formError = null;
		await settle(await withdrawBid(fetch, item.campaign.id, account.id));
	}

	async function settle(result: ApiResult<Bid>) {
		if (result.ok) {
			dialog.close();
			reloadPage();
			return;
		}
		busy = null;
		const { error } = result;
		if (isConflict(error)) {
			closedNotice = errorMessage(error);
			reloadPage();
			// The form is locked now, so focus moves to the way out.
			await tick();
			closeButton?.focus();
			return;
		}
		const message = fieldErrors(error)?.['amountCents'];
		if (message) {
			serverAmountError = message;
			amountInput?.focus();
		} else {
			formError = errorMessage(error);
		}
	}

	// The feed and the bids page both open the panel; only the page on screen reloads.
	const reloadPage = () => Promise.all([invalidate('app:feed'), invalidate('app:bids')]);
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
	aria-labelledby="bid-panel-title"
	onclick={(event) => event.target === dialog && dialog.close()}
	class="sheet border border-white/60 bg-base-100 p-0 shadow-lift"
>
	{#if item}
		{@const { campaign, myBid, minBidCents, maxBidCents } = item}
		<form class="flex h-full max-h-[inherit] flex-col" novalidate onsubmit={submit}>
			<div
				aria-hidden="true"
				class="mx-auto mt-2 h-1.5 w-9 shrink-0 rounded-full bg-neutral-300 sm:hidden"
			></div>
			<header class="flex items-start gap-4 border-b border-base-300 p-6 max-sm:pt-4">
				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-semibold text-neutral-500">{campaign.advertiserName}</p>
					<h2 id="bid-panel-title" class="mt-1 title-section break-words">
						{campaign.title}
					</h2>
					<p class="mt-3">
						<Countdown deadline={campaign.biddingDeadline} />
					</p>
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

			<!-- Only this part scrolls, so the title and the actions stay in view. -->
			<div
				bind:this={scroller}
				class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto overscroll-contain p-6"
			>
				<section aria-labelledby="bid-panel-briefing">
					<h3 id="bid-panel-briefing" class="text-sm font-semibold text-neutral-500">Briefing</h3>
					<p class="mt-2 leading-relaxed break-words whitespace-pre-line text-neutral-600">
						{campaign.briefing}
					</p>
					<p class="mt-3 text-sm text-neutral-600">
						If you win, post by
						<span class="font-semibold text-black">{formatDateTime(postBy(campaign))}</span>.
					</p>
				</section>

				<AccountStats {account} />

				{#if myBid?.status === 'withdrawn'}
					<p class="rounded-2xl bg-neutral-50 px-4 py-3 text-sm text-neutral-600">
						You withdrew your {formatCents(myBid.amountCents)} bid. Bid again before the deadline to reinstate
						it.
					</p>
				{:else if myBid}
					<p class="flex flex-wrap items-center gap-2 text-sm text-neutral-600">
						Current bid
						<span class="font-semibold text-black tabular-nums"
							>{formatCents(myBid.amountCents)}</span
						>
						<BidStatusBadge status={myBid.status} />
					</p>
				{/if}

				<Field name="bid-amount" label="Your bid" error={amountError ?? undefined}>
					{#snippet children(control)}
						<EuroInput
							{...control}
							disabled={closedNotice !== null}
							bind:input={amountInput}
							bind:cents={amountCents}
							oninput={() => (serverAmountError = null)}
						/>
						<div class="mt-2">
							<div class="relative h-8 has-disabled:opacity-50">
								<!-- Inset by half a handle, so the fill ends under the handle's centre. -->
								<div
									aria-hidden="true"
									class="absolute inset-x-2.5 top-1/2 h-1.5 -translate-y-1/2 rounded-full bg-neutral-200"
								>
									<div class="h-full rounded-full bg-primary" style:width="{sliderAt * 100}%"></div>
								</div>
								<input
									type="range"
									class="absolute inset-0 slider w-full"
									min={minBidCents}
									max={maxBidCents}
									step="100"
									aria-label="Your bid"
									aria-valuetext={formatCents(sliderCents)}
									disabled={closedNotice !== null}
									bind:value={() => sliderCents, (cents) => (amountCents = cents)}
									oninput={() => (serverAmountError = null)}
								/>
							</div>
							<div
								aria-hidden="true"
								class="flex justify-between gap-4 text-xs text-neutral-500 tabular-nums"
							>
								<span>{formatCents(minBidCents)}</span>
								<span>{formatCents(maxBidCents)}</span>
							</div>
						</div>
					{/snippet}
				</Field>

				<!-- Faded while the amount is invalid, as the figures don't apply then. -->
				<dl
					class={[
						'flex flex-col gap-2 rounded-2xl bg-neutral-50 px-5 py-4 text-sm transition-opacity duration-150 ease-out-expo',
						amountError && 'opacity-40'
					]}
				>
					{@render line('Your bid', preview ? formatCents(preview.bid) : '–')}
					{@render line('Fees', preview ? `−${formatCents(preview.commission)}` : '–')}
					{@render line(
						'Minimum views required',
						preview ? `≈ ${formatNumber(preview.minPaidViews)}` : '–'
					)}
					<div class="mt-1 flex items-baseline justify-between gap-4 border-t border-base-300 pt-3">
						<dt class="font-semibold">You receive</dt>
						<dd class="text-2xl font-extrabold tracking-tight tabular-nums">
							{preview ? formatCents(Math.round(shownPayout.current)) : '–'}
						</dd>
					</div>
					{#if chance}
						<div class="flex items-center justify-between gap-4">
							<dt class="text-neutral-600">
								If bidding closed now
								<span class="block text-xs text-neutral-500 tabular-nums">
									{formatCount(chance.pendingBids, 'bid')} so far
								</span>
							</dt>
							<dd aria-live="polite">
								<StatusPill
									tone={chance.wouldWin ? 'success' : 'neutral'}
									label={chance.wouldWin ? 'Would win' : 'Wouldn’t win'}
								/>
							</dd>
						</div>
					{/if}
				</dl>

				{#if closedNotice}
					<div
						id="bid-closed-notice"
						role="status"
						class="animate-rise rounded-2xl bg-neutral-50 px-4 py-3 text-sm"
					>
						<p class="font-semibold">{closedNotice}</p>
						<p class="mt-0.5 text-neutral-600">The page has been refreshed.</p>
					</div>
				{:else if formError}
					<p role="alert" class="animate-rise text-sm text-error">{formError}</p>
				{/if}
			</div>

			<footer
				class="flex items-center gap-3 border-t border-base-300 p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))]"
			>
				{#if closedNotice}
					<button
						bind:this={closeButton}
						type="button"
						aria-describedby="bid-closed-notice"
						class="btn ml-auto max-sm:flex-1"
						onclick={() => dialog.close()}
					>
						Close
					</button>
				{:else}
					{#if myBid?.status === 'pending'}
						<button
							type="button"
							class="btn btn-ghost text-error max-sm:flex-1"
							disabled={busy !== null}
							onclick={withdraw}
						>
							{#if busy === 'withdrawing'}
								<span class="loading loading-sm loading-spinner"></span> Withdrawing…
							{:else}
								Withdraw bid
							{/if}
						</button>
					{/if}
					<button type="submit" class="btn ml-auto btn-neutral max-sm:flex-1" disabled={!canSubmit}>
						{#if busy === 'saving'}
							<span class="loading loading-sm loading-spinner"></span> Saving…
						{:else if myBid?.status === 'pending'}
							Update bid
						{:else if myBid?.status === 'withdrawn'}
							Bid again
						{:else}
							Place bid
						{/if}
					</button>
				{/if}
			</footer>
		</form>
	{/if}
</dialog>
