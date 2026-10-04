<script lang="ts">
	import SizeBadge from '$lib/components/SizeBadge.svelte';
	import WhyItMatches from '$lib/components/WhyItMatches.svelte';
	import {
		formatCents,
		formatCompact,
		formatCount,
		formatNumber,
		formatPercent
	} from '$lib/format';
	import { SIZE_GROUPS, type SizeGroup, type Winner } from '$lib/types';
	import BidderName from './BidderName.svelte';
	import ResultRow from './ResultRow.svelte';
	import ResultsTable, { type Column } from './ResultsTable.svelte';

	interface Props {
		/** Smallest size first, then the highest bid, as the API sends them. */
		winners: Winner[];
		viewsCountingDays: number;
		onopenaccount: (winner: Winner, button: HTMLButtonElement) => void;
	}

	let { winners, viewsCountingDays, onopenaccount }: Props = $props();

	// Each size heads its own rows, so the table needs no column for it.
	const COLUMNS: Column[] = [
		{ label: 'Account' },
		{ label: 'Score' },
		{ label: 'Followers', numeric: true },
		{ label: 'Engagement', numeric: true },
		{ label: 'Avg. views', numeric: true },
		{ label: 'Min views', numeric: true },
		{ label: 'Expected CPM', numeric: true },
		{ label: 'You pay', numeric: true }
	];

	const bySize = $derived(
		SIZE_GROUPS.map((group) => ({
			group,
			winners: winners.filter((winner) => winner.sizeGroup === group)
		})).filter((size) => size.winners.length > 0)
	);
</script>

{#snippet sizeLabel(group: SizeGroup, count: number)}
	<span class="flex items-center gap-2 text-sm font-normal text-neutral-500">
		<SizeBadge {group} />
		{formatCount(count, 'creator')}
	</span>
{/snippet}

<!-- A table where the panel is wide enough for its columns, a list otherwise. -->
<div class="@container mt-10">
	<h3 class="text-sm font-semibold text-neutral-500">Winning creators</h3>
	<p class="mt-1 text-sm text-neutral-600">
		A creator is paid only if their video reaches the minimum views within
		{formatCount(viewsCountingDays, 'day')} of posting.
	</p>

	<!-- Padded by the focus ring's width, which the scroll box would otherwise clip. -->
	<div class="-mx-1 mt-4 hidden overflow-x-auto px-1 @min-[768px]:block">
		<ResultsTable caption="Winning creators" columns={COLUMNS}>
			{#each bySize as size (size.group)}
				<tbody>
					<tr>
						<th
							scope="rowgroup"
							colspan={COLUMNS.length}
							class="border-t border-b-0 border-base-300 pt-8 pb-2"
						>
							{@render sizeLabel(size.group, size.winners.length)}
						</th>
					</tr>
					{#each size.winners as winner (winner.accountId)}
						<tr class="hover:bg-neutral-50">
							<td>
								<BidderName
									bid={winner}
									class="max-w-48"
									onopen={(button) => onopenaccount(winner, button)}
								/>
							</td>
							<td>
								<WhyItMatches score={winner.match.score} factors={winner.match.factors} />
							</td>
							<td class="text-right tabular-nums">{formatCompact(winner.followers)}</td>
							<td class="text-right tabular-nums">{formatPercent(winner.engagementRate)}</td>
							<td class="text-right tabular-nums">{formatNumber(winner.viewScore)}</td>
							<td class="text-right tabular-nums">{formatNumber(winner.minPaidViews)}</td>
							<td class="text-right tabular-nums">{formatCents(winner.expectedCpmCents)}</td>
							<td class="text-right font-semibold tabular-nums">
								{formatCents(winner.amountCents)}
							</td>
						</tr>
					{/each}
				</tbody>
			{/each}
		</ResultsTable>
	</div>

	<div class="mt-4 @min-[768px]:hidden">
		{#each bySize as size (size.group)}
			<h4 class="border-t border-base-300 pt-8 pb-2 first:border-t-0 first:pt-0">
				{@render sizeLabel(size.group, size.winners.length)}
			</h4>
			<ul class="divide-y divide-base-300">
				{#each size.winners as winner (winner.accountId)}
					<li class="py-4">
						<ResultRow
							bid={winner}
							amountLabel="You pay"
							amountCents={winner.amountCents}
							facts={[
								['Followers', formatCompact(winner.followers)],
								['Engagement', formatPercent(winner.engagementRate)],
								['Avg. views', formatNumber(winner.viewScore)],
								['Min views', formatNumber(winner.minPaidViews)],
								['Expected CPM', formatCents(winner.expectedCpmCents)]
							]}
							onopen={(button) => onopenaccount(winner, button)}
						/>
					</li>
				{/each}
			</ul>
		{/each}
	</div>
</div>
