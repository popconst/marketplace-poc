<script lang="ts">
	import { goto, invalidate } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { getFeed } from '$lib/api';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import LoadError from '$lib/components/LoadError.svelte';
	import RefreshAtDeadlines from '$lib/components/RefreshAtDeadlines.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import { bidStatusLabels } from '$lib/format';
	import type { BidListItem, BidStatus } from '$lib/types';
	import BidPanel from '../BidPanel.svelte';
	import { canChange } from './bid';
	import BidCard from './BidCard.svelte';
	import BidDrawer from './BidDrawer.svelte';

	let { data } = $props();

	let drawer: BidDrawer;
	let panel: BidPanel;
	/** The bid whose panel is loading. */
	let openingId = $state<number | null>(null);

	/** Opens the bid panel with the campaign's feed item, which holds the prices it needs. */
	async function edit(bid: BidListItem) {
		openingId = bid.id;
		const feed = await getFeed(fetch, data.account.id, new URLSearchParams());
		openingId = null;
		const item = feed.ok && feed.data.items.find((i) => i.campaign.id === bid.campaign.id);
		if (item) {
			panel.open(item);
		} else {
			// The campaign stopped taking bids since the page loaded.
			invalidate('app:bids');
		}
	}

	type Filter = BidStatus | 'all';
	const filterOptions: { value: Filter; label: string }[] = [
		{ value: 'all', label: 'All' },
		...(['pending', 'won', 'lost', 'withdrawn'] as const).map((value) => ({
			value,
			label: bidStatusLabels[value]
		}))
	];

	const accountId = $derived(String(data.account.id));
	const feedPath = $derived(resolve('/creator/accounts/[accountId]', { accountId }));
	const bidsPath = $derived(resolve('/creator/accounts/[accountId]/bids', { accountId }));

	function showStatus(status: BidStatus | null) {
		const query = status ? `?${new URLSearchParams({ status })}` : '';
		goto(`${bidsPath}${query}`, { keepFocus: true, noScroll: true });
	}

	const getFilter = (): Filter => data.status ?? 'all';
	const setFilter = (filter: Filter | null) => showStatus(filter === 'all' ? null : filter);

	// A pending bid is decided when its campaign closes.
	const pendingDeadlines = $derived(
		data.bids.ok
			? data.bids.data.items
					.filter((bid) => bid.status === 'pending')
					.map((bid) => bid.campaign.biddingDeadline)
			: []
	);
</script>

<RefreshAtDeadlines deadlines={pendingDeadlines} refresh={() => invalidate('app:bids')} />

<svelte:head>
	<title>My bids · @{data.account.handle} · WePush</title>
</svelte:head>

<div class="mt-8 animate-rise [--i:1]">
	<div class="max-w-2xl">
		<!-- Five options don't fit side by side on a phone. -->
		<label class="flex flex-col gap-2 sm:hidden">
			<span class="text-sm font-semibold">Status</span>
			<select class="select w-full" bind:value={getFilter, setFilter}>
				{#each filterOptions as { value, label } (value)}
					<option {value}>{label}</option>
				{/each}
			</select>
		</label>
		<div class="hidden sm:block">
			<SegmentedControl
				name="status"
				legend="Status"
				options={filterOptions}
				bind:value={getFilter, setFilter}
			/>
		</div>
	</div>

	<div class="mt-6">
		{#if !data.bids.ok}
			<LoadError title="Couldn’t load your bids" error={data.bids.error} />
		{:else if data.bids.data.items.length === 0 && data.status}
			<EmptyState title="No {bidStatusLabels[data.status].toLowerCase()} bids">
				<button type="button" class="btn" onclick={() => showStatus(null)}>Show all bids</button>
			</EmptyState>
		{:else if data.bids.data.items.length === 0}
			<EmptyState
				title="No bids yet"
				message="Bids you place on open campaigns show up here, so you can follow them until the campaign closes."
			>
				<a href={feedPath} class="btn btn-neutral">Browse open campaigns</a>
			</EmptyState>
		{:else}
			<ul class="flex flex-col gap-4">
				{#each data.bids.data.items as bid (bid.id)}
					<li>
						<BidCard
							{bid}
							viewsCountingDays={data.options.viewsCountingDays}
							onopen={(button) => drawer.open(bid, button)}
							onedit={canChange(bid) ? () => edit(bid) : undefined}
							opening={openingId === bid.id}
						/>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

<BidDrawer
	bind:this={drawer}
	account={data.account}
	viewsCountingDays={data.options.viewsCountingDays}
	onedit={edit}
/>

<BidPanel
	bind:this={panel}
	account={data.account}
	viewsCountingDays={data.options.viewsCountingDays}
/>
