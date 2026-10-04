<script lang="ts">
	import type { ApiResult } from '$lib/api';
	import AlertIcon from '$lib/components/AlertIcon.svelte';
	import LoadError from '$lib/components/LoadError.svelte';
	import SizeBadge, { sizeColours } from '$lib/components/SizeBadge.svelte';
	import {
		formatCents,
		formatCentsRounded,
		formatCompact,
		formatList,
		formatNumber,
		sizeGroupLabels
	} from '$lib/format';
	import type { Estimate, SizeGroup } from '$lib/types';
	import TargetPill from './TargetPill.svelte';

	interface Props {
		estimate: ApiResult<Estimate>;
		/** The campaign's, as saved. */
		targetCpmCents: number | null;
	}

	let { estimate, targetCpmCents }: Props = $props();

	const needs: Record<Estimate['missing'][number], string> = {
		platform: 'a platform',
		budgetCents: 'a budget',
		targetCpmCents: 'a target CPM'
	};

	interface Metric {
		label: string;
		value: string;
		unit?: string;
		/** The picked sizes' matching accounts. */
		bySize?: { group: SizeGroup; accounts: number }[];
		compare?: { cpmCents: number; targetCpmCents: number };
	}

	const metrics = $derived.by(() => {
		if (!estimate.ok) return [];
		const { matchingAccounts, matchingAccountsCapped, sizes, videos, views, averageCpmCents } =
			estimate.data;
		const metrics: Metric[] = [];
		if (matchingAccounts !== null) {
			metrics.push({
				label: 'Matching accounts',
				value: matchingAccountsCapped
					? `${formatCompact(matchingAccounts)}+`
					: formatNumber(matchingAccounts),
				bySize: sizes.flatMap(({ group, picked, accounts }) =>
					picked && accounts !== null ? [{ group, accounts }] : []
				)
			});
		}
		if (videos !== null) metrics.push({ label: 'Videos', value: formatNumber(videos) });
		if (views !== null) metrics.push({ label: 'Views', value: formatCompact(views) });
		if (averageCpmCents !== null) {
			const metric: Metric = {
				label: 'Average CPM',
				value: formatCents(averageCpmCents),
				unit: 'per 1,000 views'
			};
			if (targetCpmCents !== null) {
				metric.compare = { cpmCents: averageCpmCents, targetCpmCents };
			}
			metrics.push(metric);
		}
		return metrics;
	});

	/** Notes say what the estimate lacks; warnings say what the campaign can't do. */
	const notices = $derived.by(() => {
		if (!estimate.ok) return [];
		const { missing, matchingAccounts, sizes, spentCents, fillsBudget } = estimate.data;
		const notices: { text: string; warning: boolean }[] = [];
		if (missing.length > 0) {
			const text = `Add ${formatList(missing.map((field) => needs[field]))} to see an estimate.`;
			notices.push({ text, warning: false });
		}
		if (matchingAccounts === 0) {
			notices.push({ text: 'No account matches this targeting yet.', warning: false });
		}
		for (const { group, picked, affordable, typicalPriceCents } of sizes) {
			if (picked && affordable === false && typicalPriceCents !== null) {
				const text =
					`${sizeGroupLabels[group]} needs about ${formatCentsRounded(typicalPriceCents)} per video, ` +
					'more than this campaign can pay one creator.';
				notices.push({ text, warning: true });
			}
		}
		if (fillsBudget === false && spentCents !== null) {
			const text =
				`Only about ${formatCentsRounded(spentCents)} of your budget would be spent with this ` +
				'targeting and these sizes.';
			notices.push({ text, warning: true });
		}
		return notices;
	});
</script>

<aside aria-labelledby="estimate-title" class="animate-rise surface [--i:1] lg:sticky lg:top-28">
	<h2 id="estimate-title" class="title-section">What to expect</h2>
	<p class="mt-2 text-sm text-neutral-600">Rough figures: the result depends on who bids.</p>

	{#if !estimate.ok}
		<div class="mt-6">
			<LoadError compact title="Couldn’t load the estimate" error={estimate.error} />
		</div>
	{:else}
		{#if metrics.length > 0}
			<dl class="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-1">
				{#each metrics as { label, value, unit, bySize, compare } (label)}
					<div>
						<dt class="text-sm text-neutral-500">{label}</dt>
						<dd class="mt-2 flex flex-wrap items-baseline gap-x-2">
							<span class="kpi">{value}</span>
							{#if unit}<span class="text-sm text-neutral-500">{unit}</span>{/if}
						</dd>
						{#if bySize && bySize.length > 0}
							<dd class="mt-3">
								<!-- Each size's share of the matching accounts. -->
								<div aria-hidden="true" class="flex h-1.5 gap-0.5 overflow-hidden rounded-full">
									{#each bySize as { group, accounts } (group)}
										<span class={sizeColours[group].fill} style:flex-grow={accounts}></span>
									{/each}
								</div>
								<ul class="mt-3 flex flex-col gap-1.5 text-sm">
									{#each bySize as { group, accounts } (group)}
										<li class="flex items-center justify-between gap-4">
											<SizeBadge {group} variant="dot" />
											<span class="text-neutral-600 tabular-nums">{formatCompact(accounts)}</span>
										</li>
									{/each}
								</ul>
							</dd>
						{/if}
						{#if compare}
							<dd class="mt-2 flex flex-wrap items-center gap-2 text-sm text-neutral-600">
								Your target {formatCents(compare.targetCpmCents)}
								<TargetPill {...compare} />
							</dd>
							{#if compare.cpmCents > compare.targetCpmCents}
								<dd class="mt-2 text-sm text-neutral-600">
									Creators in these sizes don’t go as low as {formatCents(compare.targetCpmCents)}.
								</dd>
							{/if}
						{/if}
					</div>
				{/each}
			</dl>
		{/if}

		{#if notices.length > 0}
			<div class="mt-6 flex flex-col gap-3">
				{#each notices as { text, warning } (text)}
					<p class="flex animate-rise gap-3 rounded-2xl bg-neutral-50 p-4 text-sm text-neutral-700">
						{#if warning}<AlertIcon class="size-5 text-warning-strong" />{/if}
						{text}
					</p>
				{/each}
			</div>
		{/if}
	{/if}
</aside>
