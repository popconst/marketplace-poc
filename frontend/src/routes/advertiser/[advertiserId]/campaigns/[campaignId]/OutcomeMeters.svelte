<script lang="ts">
	import { formatCents, formatCompact } from '$lib/format';
	import type { CampaignProgress } from '$lib/types';
	import ProgressBar from './ProgressBar.svelte';
	import TargetPill from './TargetPill.svelte';

	interface Props {
		outcome: Pick<
			CampaignProgress,
			| 'budgetCents'
			| 'spentCents'
			| 'returnedCents'
			| 'minPaidViews'
			| 'expectedViews'
			| 'effectiveCpmCents'
			| 'targetCpmCents'
		>;
		/** The figures are what the campaign bought, not a projection. */
		closed: boolean;
	}

	let { outcome, closed }: Props = $props();

	const cpm = $derived(outcome.effectiveCpmCents);
	const aboveTarget = $derived(cpm !== null && cpm > outcome.targetCpmCents);
</script>

<ul class="mt-8 flex flex-col gap-6 text-sm">
	<li>
		<div class="flex flex-wrap items-baseline justify-between gap-x-4">
			<p class="font-semibold">Budget committed</p>
			<p class="text-neutral-600 tabular-nums">
				<span class="font-semibold text-black">{formatCents(outcome.spentCents)}</span>
				of {formatCents(outcome.budgetCents)}
			</p>
		</div>
		<ProgressBar value={outcome.spentCents} max={outcome.budgetCents} />
		<p class="mt-2 text-neutral-500">
			{formatCents(outcome.returnedCents)}
			{closed ? 'is' : 'would be'} returned to you.
		</p>
	</li>
	<li>
		<div class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
			<p class="font-semibold">Payout</p>
			<p class="text-neutral-600 tabular-nums">
				<span class="font-semibold text-black">{formatCents(outcome.spentCents)}</span>
				at
				<span class="font-semibold text-black">{formatCompact(outcome.minPaidViews)}</span>
				views
			</p>
		</div>
		<p class="mt-2 text-neutral-500">
			The full amount is paid only if every video reaches its minimum views; a video that falls
			short costs you nothing.
		</p>
	</li>
	<li>
		<div class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
			<p class="font-semibold">Expected views</p>
			<p class="font-semibold tabular-nums">
				{formatCompact(outcome.minPaidViews)} – {formatCompact(outcome.expectedViews)}
			</p>
		</div>
		<p class="mt-2 text-neutral-500">
			From every video just reaching its minimum to every video getting its creator’s usual views.
		</p>
	</li>
	<li>
		<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
			<p class="font-semibold">Cost per 1,000 views</p>
			<p class="flex items-center gap-2 text-neutral-600 tabular-nums">
				{#if cpm === null}
					No winners yet
				{:else}
					<TargetPill cpmCents={cpm} targetCpmCents={outcome.targetCpmCents} />
					<span class="font-semibold text-black">{formatCents(cpm)}</span>
				{/if}
			</p>
		</div>
		<!-- The bar runs to twice the target, so the target marks its middle. -->
		<ProgressBar
			value={cpm ?? 0}
			max={2 * outcome.targetCpmCents}
			warning={aboveTarget}
			marker={0.5}
			markerLabel="Target {formatCents(outcome.targetCpmCents)}"
		/>
		{#if aboveTarget}
			<p class="mt-2 text-neutral-500">
				Creators in these sizes don’t go as low as {formatCents(outcome.targetCpmCents)}.
			</p>
		{/if}
	</li>
</ul>
