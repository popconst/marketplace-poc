<script lang="ts">
	import SizeBadge from '$lib/components/SizeBadge.svelte';
	import WhyItMatches from '$lib/components/WhyItMatches.svelte';
	import { formatCents } from '$lib/format';
	import type { SizeGroup, Winner } from '$lib/types';
	import BidderName from './BidderName.svelte';

	interface Props {
		bid: Pick<Winner, 'handle' | 'creatorName' | 'countryCode' | 'match'>;
		/** Shown beside the name, such as `You pay`. */
		amountLabel: string;
		amountCents: number;
		/** Shown after the score, in a list that isn't grouped by size. */
		sizeGroup?: SizeGroup | undefined;
		facts: [term: string, detail: string][];
		onopen: (button: HTMLButtonElement) => void;
	}

	let { bid, amountLabel, amountCents, sizeGroup, facts, onopen }: Props = $props();
</script>

<!-- A bid in the narrow list, where a table's columns don't fit. -->
<div class="flex items-baseline justify-between gap-4">
	<BidderName {bid} {onopen} />
	<p class="shrink-0 tabular-nums">
		<span class="text-sm text-neutral-500">{amountLabel}</span>
		<span class="font-bold">{formatCents(amountCents)}</span>
	</p>
</div>
<dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-sm sm:grid-cols-4">
	<div>
		<dt class="text-neutral-500">Score</dt>
		<dd class="mt-0.5"><WhyItMatches score={bid.match.score} factors={bid.match.factors} /></dd>
	</div>
	{#if sizeGroup}
		<div>
			<dt class="text-neutral-500">Size</dt>
			<dd class="mt-0.5"><SizeBadge group={sizeGroup} /></dd>
		</div>
	{/if}
	{#each facts as [term, detail] (term)}
		<div>
			<dt class="text-neutral-500">{term}</dt>
			<dd class="font-semibold tabular-nums">{detail}</dd>
		</div>
	{/each}
</dl>
