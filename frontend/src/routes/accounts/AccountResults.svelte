<script lang="ts">
	import { errorMessage, listAccounts } from '$lib/api';
	import { formatCount } from '$lib/format';
	import type { Options, Page, PlatformAccount } from '$lib/types';
	import AccountTable from './AccountTable.svelte';
	import { toSearchParams, type AccountFilters } from './filters';

	interface Props {
		firstPage: Page<PlatformAccount>;
		/** The filters `firstPage` was loaded with; "Load more" reuses them. */
		filters: AccountFilters;
		options: Options;
	}

	let { firstPage, filters, options }: Props = $props();

	// Needs no reset: the parent's {#await} has a pending branch, so it remounts this component
	// on every new load.
	let morePages = $state.raw<Page<PlatformAccount>[]>([]);
	let loading = $state(false);
	let loadError = $state<string | null>(null);

	const pages = $derived([firstPage, ...morePages]);
	const nextCursor = $derived(pages.at(-1)?.nextCursor ?? null);
	const shown = $derived(pages.reduce((count, page) => count + page.items.length, 0));

	async function loadMore() {
		if (!nextCursor || loading) return;
		loading = true;
		loadError = null;
		const query = toSearchParams(filters);
		query.set('cursor', nextCursor);
		const result = await listAccounts(fetch, query);
		loading = false;
		if (result.ok) morePages = [...morePages, result.data];
		else loadError = errorMessage(result.error);
	}
</script>

<AccountTable
	pages={pages.map((page) => page.items)}
	skeletonRows={loading ? 3 : 0}
	sort={filters.sort}
	{options}
/>

<div class="mt-6 flex flex-col items-center gap-3">
	<p class="text-sm text-neutral-500 tabular-nums" aria-live="polite">
		{nextCursor
			? `Showing ${formatCount(shown, 'account')}`
			: `All ${formatCount(shown, 'matching account')} shown`}
	</p>
	{#if loadError}
		<p role="alert" class="animate-rise text-sm text-error">{loadError}</p>
	{/if}
	{#if nextCursor}
		<button type="button" class="btn w-full sm:w-auto" disabled={loading} onclick={loadMore}>
			{#if loading}<span class="loading loading-xs loading-spinner"></span>{/if}
			Load more
		</button>
	{/if}
</div>
