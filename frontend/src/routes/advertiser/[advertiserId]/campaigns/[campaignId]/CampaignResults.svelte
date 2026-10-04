<script lang="ts">
	import type { ApiResult } from '$lib/api';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import LoadError from '$lib/components/LoadError.svelte';
	import { formatCents } from '$lib/format';
	import type { CampaignResults, Loser, Options, Winner } from '$lib/types';
	import AccountDrawer from './AccountDrawer.svelte';
	import LosersList from './LosersList.svelte';
	import OutcomeMeters from './OutcomeMeters.svelte';
	import WinnersList from './WinnersList.svelte';

	interface Props {
		results: ApiResult<CampaignResults>;
		options: Options;
	}

	let { results, options }: Props = $props();

	let drawer: AccountDrawer;

	const openAccount = (bid: Winner | Loser, button: HTMLButtonElement) => drawer.open(bid, button);
</script>

<section aria-labelledby="results-title" class="panel">
	<p class="tag">Results</p>
	<h2 id="results-title" class="mt-3 title-section">What your budget bought</h2>

	{#if !results.ok}
		<div class="mt-6">
			<LoadError compact title="Couldn’t load the results" error={results.error} />
		</div>
	{:else}
		{@const { winners, losers, budgetCents } = results.data}
		{#if winners.length === 0}
			<EmptyState
				inset
				class="mt-6"
				title="No winners"
				message="{losers.length === 0
					? 'No creator bid before bidding closed.'
					: 'No bid fit the budget.'} All of your {formatCents(budgetCents)} is returned to you."
			/>
		{:else}
			<dl class="mt-6 grid grid-cols-2 gap-6">
				<div>
					<dt class="text-sm text-neutral-500">Creators who won</dt>
					<dd class="mt-2 kpi">{winners.length}</dd>
				</div>
				<div>
					<dt class="text-sm text-neutral-500">Bids that lost</dt>
					<dd class="mt-2 kpi">{losers.length}</dd>
				</div>
			</dl>
			<OutcomeMeters outcome={results.data} closed />
			<WinnersList
				{winners}
				viewsCountingDays={options.viewsCountingDays}
				onopenaccount={openAccount}
			/>
		{/if}
		{#if losers.length > 0}
			<LosersList {losers} onopenaccount={openAccount} />
		{/if}
	{/if}
</section>

<AccountDrawer bind:this={drawer} {options} />
