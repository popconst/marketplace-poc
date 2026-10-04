<script lang="ts">
	import SizeBadge from '$lib/components/SizeBadge.svelte';
	import WhyItMatches from '$lib/components/WhyItMatches.svelte';
	import { formatCents, formatCount, formatNumber } from '$lib/format';
	import type { Loser, LossReason } from '$lib/types';
	import BidderName from './BidderName.svelte';
	import { lossReasons } from './loss-reasons';
	import ResultRow from './ResultRow.svelte';
	import ResultsTable, { type Column } from './ResultsTable.svelte';

	interface Props {
		/** By loss reason, then the highest bid, as the API sends them. */
		losers: Loser[];
		onopenaccount: (loser: Loser, button: HTMLButtonElement) => void;
	}

	let { losers, onopenaccount }: Props = $props();

	const COLUMNS: Column[] = [
		{ label: 'Account' },
		{ label: 'Score' },
		{ label: 'Size' },
		{ label: 'Avg. views', numeric: true },
		{ label: 'Bid', numeric: true },
		{ label: 'Expected CPM', numeric: true },
		{ label: 'Reason' }
	];

	/** `70 outranked · 12 too expensive`, in the order of the list. */
	const reasonCounts = $derived.by(() => {
		const counts = new Map<LossReason, number>();
		for (const { lossReason } of losers) {
			counts.set(lossReason, (counts.get(lossReason) ?? 0) + 1);
		}
		return [...counts]
			.map(([reason, count]) => `${formatNumber(count)} ${lossReasons[reason].label.toLowerCase()}`)
			.join(' · ');
	});

	/** Rows rendered at a time: a big campaign can have thousands of losing bids. */
	const PAGE_SIZE = 50;

	let open = $state(false);
	let shown = $state(PAGE_SIZE);
	const visible = $derived(losers.slice(0, shown));
	const hidden = $derived(losers.length - visible.length);
</script>

<details class="group @container mt-10" bind:open>
	<summary
		class="flex w-full cursor-pointer list-none items-center justify-between gap-4 rounded-2xl border border-base-300 px-6 py-5 font-semibold transition-colors duration-300 ease-out-expo hover:bg-neutral-50 [&::-webkit-details-marker]:hidden"
	>
		{open ? 'Hide' : 'Show'}
		{formatCount(losers.length, 'bid')} that didn’t win
		<svg
			aria-hidden="true"
			viewBox="0 0 16 16"
			class="size-4 shrink-0 transition-[rotate,color] duration-500 ease-out-expo group-open:rotate-180 group-open:text-primary"
		>
			<path
				d="M4 6l4 4 4-4"
				fill="none"
				stroke="currentColor"
				stroke-width="1.75"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>
		</svg>
	</summary>

	<!-- Rendered only while open, so a closed list costs nothing. -->
	{#if open}
		<p class="mt-4 text-sm text-neutral-600 tabular-nums">{reasonCounts}</p>

		<!-- Padded by the focus ring's width, which the scroll box would otherwise clip. -->
		<div class="-mx-1 mt-4 hidden overflow-x-auto px-1 @min-[768px]:block">
			<ResultsTable caption="Bids that didn’t win" columns={COLUMNS}>
				<tbody>
					{#each visible as loser, i (loser.accountId)}
						<tr class="animate-rise hover:bg-neutral-50" style:--i={i}>
							<td>
								<BidderName
									bid={loser}
									class="max-w-48"
									onopen={(button) => onopenaccount(loser, button)}
								/>
							</td>
							<td><WhyItMatches score={loser.match.score} factors={loser.match.factors} /></td>
							<td><SizeBadge group={loser.sizeGroup} /></td>
							<td class="text-right tabular-nums">{formatNumber(loser.viewScore)}</td>
							<td class="text-right font-semibold tabular-nums">{formatCents(loser.amountCents)}</td
							>
							<td class="text-right tabular-nums">{formatCents(loser.expectedCpmCents)}</td>
							<td title={lossReasons[loser.lossReason].explanation}>
								<span class="pill border border-base-300 bg-base-100 text-neutral-700">
									{lossReasons[loser.lossReason].label}
								</span>
							</td>
						</tr>
					{/each}
				</tbody>
			</ResultsTable>
		</div>

		<ul class="mt-4 divide-y divide-base-300 @min-[768px]:hidden">
			{#each visible as loser, i (loser.accountId)}
				<li class="animate-rise py-4" style:--i={i}>
					<ResultRow
						bid={loser}
						amountLabel="Bid"
						amountCents={loser.amountCents}
						sizeGroup={loser.sizeGroup}
						facts={[
							['Avg. views', formatNumber(loser.viewScore)],
							['Expected CPM', formatCents(loser.expectedCpmCents)],
							['Reason', lossReasons[loser.lossReason].label]
						]}
						onopen={(button) => onopenaccount(loser, button)}
					/>
				</li>
			{/each}
		</ul>

		{#if hidden > 0}
			<button type="button" class="btn mt-4 w-full" onclick={() => (shown += PAGE_SIZE)}>
				Show {formatNumber(Math.min(PAGE_SIZE, hidden))} more of {formatNumber(hidden)}
			</button>
		{/if}
	{/if}
</details>
