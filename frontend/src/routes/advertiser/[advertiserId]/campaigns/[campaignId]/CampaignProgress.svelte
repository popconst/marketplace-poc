<script lang="ts">
	import { invalidate } from '$app/navigation';
	import { getCampaignProgress, isConflict, type ApiResult } from '$lib/api';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import LoadError from '$lib/components/LoadError.svelte';
	import RefreshAtDeadlines from '$lib/components/RefreshAtDeadlines.svelte';
	import SizeBadge from '$lib/components/SizeBadge.svelte';
	import { formatCents, formatCount } from '$lib/format';
	import type { CampaignProgress, Timestamp } from '$lib/types';
	import OutcomeMeters from './OutcomeMeters.svelte';
	import ProgressBar from './ProgressBar.svelte';

	interface Props {
		campaignId: number;
		biddingDeadline: Timestamp;
		progress: ApiResult<CampaignProgress>;
	}

	let { campaignId, biddingDeadline, progress }: Props = $props();

	const REFRESH_INTERVAL_MS = 15_000;

	// Polled here rather than through the page load, so the rest of the page doesn't reload. Each
	// refresh overwrites this writable $derived, and a new page load resets it.
	let latest = $derived(progress);
	let refreshFailed = $state(false);

	$effect(() => {
		const id = campaignId;
		const timer = setInterval(async () => {
			// Skip hidden tabs; the first tick after the tab is shown again catches up.
			if (document.hidden) return;
			const result = await getCampaignProgress(fetch, id);
			if (result.ok) {
				latest = result;
				refreshFailed = false;
			} else if (isConflict(result.error)) {
				// Bidding is over: reload the page.
				invalidate('app:campaign');
			} else {
				refreshFailed = true;
			}
		}, REFRESH_INTERVAL_MS);
		return () => clearInterval(timer);
	});
</script>

<!-- Reloads the page once the campaign has closed, which shows its results instead of this panel. -->
<RefreshAtDeadlines deadlines={[biddingDeadline]} refresh={() => invalidate('app:campaign')} />

<section aria-labelledby="progress-title" class="panel">
	<p class="tag gap-2">
		<span aria-hidden="true" class="relative flex size-1.5">
			<span class="absolute size-full animate-ping rounded-full bg-current opacity-75"></span>
			<span class="relative size-1.5 rounded-full bg-current"></span>
		</span>
		Live
	</p>
	<h2 id="progress-title" class="mt-3 title-section">If bidding closed now</h2>
	<p class="mt-2 text-sm text-neutral-600">
		What your bids would buy, picked by the rules that will pick the winners.
	</p>
	{#if refreshFailed && latest.ok}
		<p role="status" class="mt-2 text-sm text-neutral-500">Couldn’t refresh; retrying.</p>
	{/if}

	{#if !latest.ok}
		<div class="mt-6">
			<LoadError compact title="Couldn’t load your bids" error={latest.error} />
		</div>
	{:else if latest.data.pendingBids === 0}
		<EmptyState
			inset
			class="mt-6"
			title="No bids yet"
			message="Matching creators see your campaign in their feed. Their bids show up here as they come in."
		/>
	{:else}
		{@const projected = latest.data}
		<dl class="mt-6 grid grid-cols-2 gap-6">
			<div>
				<dt class="text-sm text-neutral-500">Bids received</dt>
				<dd class="mt-2 kpi">{projected.pendingBids}</dd>
			</div>
			<div>
				<dt class="text-sm text-neutral-500">Creators who’d win</dt>
				<dd class="mt-2 kpi">{projected.winners}</dd>
			</div>
		</dl>

		<OutcomeMeters outcome={projected} closed={false} />

		<h3 class="mt-10 text-sm font-semibold text-neutral-500">By account size</h3>
		<p class="mt-1 text-sm text-neutral-600">What one size leaves unspent goes to the next.</p>
		<ul class="mt-4 flex flex-col gap-5 text-sm">
			{#each projected.groups as group (group.group)}
				<li>
					<div class="flex flex-wrap items-baseline justify-between gap-x-4">
						<p class="flex flex-wrap items-center gap-2">
							<SizeBadge group={group.group} />
							<span class="text-neutral-600">
								{group.winners} of {formatCount(group.bids, 'bid')} would win
							</span>
						</p>
						<p class="text-neutral-600 tabular-nums">
							<span class="font-semibold text-black">{formatCents(group.spentCents)}</span>
							of {formatCents(group.budgetCents)}
						</p>
					</div>
					<ProgressBar value={group.spentCents} max={group.budgetCents} />
				</li>
			{/each}
		</ul>
	{/if}
</section>
