<script lang="ts">
	import { onDestroy } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { navigating } from '$app/state';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import EuroInput from '$lib/components/EuroInput.svelte';
	import LoadError from '$lib/components/LoadError.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import { formatCompact, formatPercent } from '$lib/format';
	import BidPanel from './BidPanel.svelte';
	import FeedCard from './FeedCard.svelte';
	import { sortLabels, toSearchParams, type FeedFilters } from './filters';

	let { data } = $props();

	const PAYOUT_DELAY_MS = 400;

	let panel: BidPanel;

	const sortOptions = (['match', 'payout', 'deadline'] as const).map((value) => ({
		value,
		label: sortLabels[value]
	}));

	const metrics = $derived([
		{ value: formatCompact(data.account.viewScore), label: 'avg. views/post' },
		{ value: formatCompact(data.account.followers), label: 'followers' },
		{ value: formatPercent(data.account.engagementRate), label: 'engagement' }
	]);

	const filtered = $derived(data.filters.genre !== null || data.filters.minPayoutCents !== null);
	const loading = $derived(navigating.to !== null);

	// Follows the URL, except when it already means the same minimum (0 and empty are both none).
	let minPayoutCents = $state<number | null>(null);
	const typedMinimum = $derived(
		minPayoutCents !== null && minPayoutCents > 0 ? minPayoutCents : null
	);
	let payoutTimer: ReturnType<typeof setTimeout> | undefined;

	afterNavigate(() => {
		if (typedMinimum !== data.filters.minPayoutCents) minPayoutCents = data.filters.minPayoutCents;
	});
	onDestroy(() => clearTimeout(payoutTimer));

	function applyFilters(changes: Partial<FeedFilters>) {
		clearTimeout(payoutTimer);
		const params = toSearchParams({ ...data.filters, minPayoutCents: typedMinimum, ...changes });
		const path = resolve('/creator/accounts/[accountId]', { accountId: String(data.account.id) });
		goto(params.size > 0 ? `${path}?${params}` : path, { keepFocus: true, noScroll: true });
	}

	function applyPayoutAfterPause() {
		clearTimeout(payoutTimer);
		payoutTimer = setTimeout(() => {
			if (typedMinimum !== data.filters.minPayoutCents) applyFilters({});
		}, PAYOUT_DELAY_MS);
	}
</script>

<svelte:head>
	<title>@{data.account.handle} · WePush</title>
</svelte:head>

<div class="mt-8 animate-rise [--i:1]">
	<ul aria-label="Account metrics" class="flex flex-wrap gap-2">
		{#each metrics as { value, label } (label)}
			<li class="pill border border-base-300 bg-base-100 font-normal text-neutral-600">
				<span class="font-semibold text-black tabular-nums">{value}</span>
				{label}
			</li>
		{/each}
	</ul>

	<search
		class="mt-4 grid grid-cols-2 items-end gap-3 surface sm:gap-4 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)_minmax(0,1fr)]"
	>
		<div class="col-span-2 lg:col-span-1">
			<SegmentedControl
				name="sort"
				legend="Sort by"
				options={sortOptions}
				bind:value={() => data.filters.sort, (sort) => sort && applyFilters({ sort })}
			/>
		</div>
		<label class="flex flex-col gap-2">
			<span class="text-sm font-semibold">Genre</span>
			<select
				class="select w-full"
				bind:value={() => data.filters.genre, (genre: number | null) => applyFilters({ genre })}
			>
				<option value={null}>All genres</option>
				{#each data.options.genres as { id, name } (id)}
					<option value={id}>{name}</option>
				{/each}
			</select>
		</label>
		<div class="flex flex-col gap-2">
			<!-- Balanced, so on phones it breaks between the words rather than after the hyphen. -->
			<label for="min-payout" class="text-sm font-semibold text-balance">Minimum take-home</label>
			<EuroInput
				id="min-payout"
				placeholder="Any"
				bind:cents={minPayoutCents}
				oninput={applyPayoutAfterPause}
			/>
		</div>
	</search>

	<div class="mt-6">
		{#if !data.feed.ok}
			<LoadError title="Couldn’t load campaigns" error={data.feed.error} />
		{:else if data.feed.data.items.length === 0 && filtered}
			<EmptyState
				title="No campaigns match these filters"
				message="Try another genre or a lower minimum."
			>
				<button
					type="button"
					class="btn"
					onclick={() => applyFilters({ genre: null, minPayoutCents: null })}
				>
					Clear filters
				</button>
			</EmptyState>
		{:else if data.feed.data.items.length === 0}
			<EmptyState
				title="No open campaigns match this account right now"
				message="New campaigns show up here as advertisers publish them."
			/>
		{:else}
			<ul
				aria-busy={loading}
				class={[
					'grid gap-4 transition-opacity duration-300 sm:gap-6 lg:grid-cols-2',
					loading && 'opacity-60'
				]}
			>
				{#each data.feed.data.items as item (item.campaign.id)}
					<li>
						<FeedCard {item} options={data.options} onbid={() => panel.open(item)} />
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

<BidPanel
	bind:this={panel}
	account={data.account}
	viewsCountingDays={data.options.viewsCountingDays}
/>
