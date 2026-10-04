<script lang="ts">
	import { onDestroy } from 'svelte';
	import { afterNavigate, goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import LoadError from '$lib/components/LoadError.svelte';
	import PlatformIcon from '$lib/components/PlatformIcon.svelte';
	import RangeSlider from '$lib/components/RangeSlider.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import { flag, formatCompact, formatPercent, platformNames } from '$lib/format';
	import type { AccountSort, Platform } from '$lib/types';
	import AccountResults from './AccountResults.svelte';
	import AccountTable from './AccountTable.svelte';
	import {
		hasFilters,
		MIN_QUERY_LENGTH,
		NO_FILTERS,
		sortLabels,
		toSearchParams,
		type AccountFilters
	} from './filters';

	let { data } = $props();

	const SEARCH_DELAY_MS = 250;
	// Includes the size group boundaries (`SizeGroup::min_views` in marketplace/src/size_group.rs).
	const VIEW_STOPS = [0, 500, 1_000, 3_000, 10_000, 30_000, 100_000, 300_000, 1_000_000, 3_000_000];
	const ENGAGEMENT_STOPS = [0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.08, 0.1, 0.12, 0.15, 1];

	// A radio needs a value, so "all" stands for no platform filter.
	type PlatformChoice = Platform | 'all';
	const platformOptions: { value: PlatformChoice; label: string }[] = [
		{ value: 'all', label: 'All' },
		...(['tiktok', 'instagram'] as const).map((value) => ({ value, label: platformNames[value] }))
	];

	function effectiveQuery(input: string): string {
		const query = input.trim();
		return query.length >= MIN_QUERY_LENGTH ? query : '';
	}

	const filtered = $derived(hasFilters(data.filters));

	// Follows the URL, except when it already means the same search (a trailing space, say).
	let search = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	const tooShort = $derived(search.trim() !== '' && effectiveQuery(search) === '');

	afterNavigate(() => {
		if (effectiveQuery(search) !== data.filters.q) search = data.filters.q;
	});
	onDestroy(() => clearTimeout(searchTimer));

	function applyFilters(changes: Partial<AccountFilters>, { replaceState = false } = {}) {
		clearTimeout(searchTimer);
		const params = toSearchParams({ ...data.filters, q: effectiveQuery(search), ...changes });
		const query = params.size > 0 ? `?${params}` : '';
		goto(`${resolve('/accounts')}${query}`, { keepFocus: true, noScroll: true, replaceState });
	}

	function searchAfterPause() {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			if (effectiveQuery(search) === data.filters.q) return;
			// Refining a search replaces its history entry, so going back skips every prefix typed.
			applyFilters({}, { replaceState: data.filters.q !== '' });
		}, SEARCH_DELAY_MS);
	}

	function clearFilters() {
		search = '';
		applyFilters(NO_FILTERS);
	}
</script>

<svelte:head>
	<title>Creator accounts · WePush</title>
</svelte:head>

{#snippet searchIcon(className: string)}
	<svg aria-hidden="true" viewBox="0 0 16 16" class={className}>
		<circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5" />
		<path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
	</svg>
{/snippet}

<div class="animate-rise">
	<h1 class="title-page">Creator accounts</h1>
	<p class="mt-2 lede">
		Every connected TikTok and Instagram account, with the stats matching uses.
	</p>
</div>

<search class="mt-8 grid animate-rise grid-cols-2 gap-3 surface [--i:1] sm:gap-4 lg:grid-cols-4">
	<div class="col-span-2 flex flex-col gap-2">
		<div class="flex items-baseline justify-between gap-4">
			<label for="search" class="text-sm font-semibold">Search</label>
			<!-- In the label row, so it takes no space until it shows. -->
			<p id="search-hint" aria-live="polite" class="text-right text-sm text-neutral-500">
				{#if tooShort}Type at least {MIN_QUERY_LENGTH} characters{/if}
			</p>
		</div>
		<div class="input w-full">
			{@render searchIcon('size-4 text-neutral-500')}
			<input
				id="search"
				type="search"
				placeholder="Handle or creator name"
				autocomplete="off"
				aria-describedby="search-hint"
				bind:value={search}
				oninput={searchAfterPause}
			/>
		</div>
	</div>
	<div class="col-span-2">
		<SegmentedControl
			name="platform"
			legend="Platform"
			options={platformOptions}
			bind:value={
				() => data.filters.platform ?? 'all',
				(choice) => applyFilters({ platform: choice === 'all' ? null : choice })
			}
		>
			{#snippet icon(choice)}
				{#if choice !== 'all'}<PlatformIcon platform={choice} />{/if}
			{/snippet}
		</SegmentedControl>
	</div>
	<label class="flex flex-col gap-2">
		<span class="text-sm font-semibold">Country</span>
		<select
			class="select w-full"
			bind:value={() => data.filters.country, (country: string | null) => applyFilters({ country })}
		>
			<option value={null}>All countries</option>
			{#each data.options.countries as { code, name } (code)}
				<option value={code}>{flag(code)} {name}</option>
			{/each}
		</select>
	</label>
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
	<label class="flex flex-col gap-2">
		<span class="text-sm font-semibold">Language</span>
		<select
			class="select w-full"
			bind:value={
				() => data.filters.language, (language: string | null) => applyFilters({ language })
			}
		>
			<option value={null}>All languages</option>
			{#each data.options.languages as { code, name } (code)}
				<option value={code}>{name}</option>
			{/each}
		</select>
	</label>
	<label class="flex flex-col gap-2">
		<span class="text-sm font-semibold">Sort by</span>
		<select
			class="select w-full"
			bind:value={() => data.filters.sort, (sort: AccountSort) => applyFilters({ sort })}
		>
			{#each Object.entries(sortLabels) as [value, label] (value)}
				<option {value}>{label}</option>
			{/each}
		</select>
	</label>
	<div class="col-span-2">
		<RangeSlider
			legend="Views per post"
			stops={VIEW_STOPS}
			low={data.filters.minViews}
			high={data.filters.maxViews}
			format={formatCompact}
			onchange={(minViews, maxViews) => applyFilters({ minViews, maxViews })}
		/>
	</div>
	<div class="col-span-2">
		<RangeSlider
			legend="Engagement"
			stops={ENGAGEMENT_STOPS}
			low={data.filters.minEngagement}
			high={data.filters.maxEngagement}
			format={formatPercent}
			onchange={(minEngagement, maxEngagement) => applyFilters({ minEngagement, maxEngagement })}
		/>
	</div>
</search>

<div class="mt-6">
	{#await data.accounts}
		<AccountTable pages={[]} skeletonRows={8} sort={data.filters.sort} options={data.options} />
	{:then result}
		{#if !result.ok}
			<LoadError title="Couldn’t load accounts" error={result.error} />
		{:else if result.data.items.length === 0 && filtered}
			<EmptyState title="No accounts match" message="Try a shorter search or fewer filters.">
				{#snippet icon()}{@render searchIcon('mx-auto size-8 text-neutral-400')}{/snippet}
				<button type="button" class="btn" onclick={clearFilters}>Clear filters</button>
			</EmptyState>
		{:else if result.data.items.length === 0}
			<EmptyState title="No accounts yet" message="No creator has connected an account yet.">
				{#snippet icon()}{@render searchIcon('mx-auto size-8 text-neutral-400')}{/snippet}
			</EmptyState>
		{:else}
			<AccountResults firstPage={result.data} filters={data.filters} options={data.options} />
		{/if}
	{/await}
</div>
