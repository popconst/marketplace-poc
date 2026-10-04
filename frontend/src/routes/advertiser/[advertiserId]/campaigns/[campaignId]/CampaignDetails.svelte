<script lang="ts">
	import type { Snippet } from 'svelte';
	import SizeBadge from '$lib/components/SizeBadge.svelte';
	import { flag, formatBps, formatCents, optionNames } from '$lib/format';
	import type { Options, PublishedCampaign } from '$lib/types';
	import { weightLabel } from './weights';

	interface Props {
		campaign: PublishedCampaign;
		options: Options;
		/** In the column beside the live panel rather than full width below the results. */
		sidebar: boolean;
	}

	let { campaign, options, sidebar }: Props = $props();

	const names = $derived(optionNames(options));

	function listNames<Code>(codes: Code[], name: (code: Code) => string): string {
		return codes.length === 0 ? 'Any' : codes.map(name).join(', ');
	}

	const groups = $derived<{ title: string; rows: [term: string, detail: string | Snippet][] }[]>([
		{
			title: 'Money',
			rows: [
				['Budget', formatCents(campaign.budgetCents)],
				['Target CPM', formatCents(campaign.targetCpmCents)],
				['Account sizes', sizes],
				['WePush fee', `${formatBps(campaign.commissionBps)} of each winning bid`]
			]
		},
		{
			title: 'Who sees it',
			rows: [
				[
					'Countries',
					listNames(campaign.countryCodes, (code) => `${flag(code)} ${names.country(code)}`)
				],
				['Languages', listNames(campaign.languageCodes, names.language)],
				['Genres', listNames(campaign.genreIds, names.genre)]
			]
		},
		{
			title: 'Preferences',
			rows: [
				['Engagement', weightLabel(campaign.engagementWeight)],
				['Content quality', weightLabel(campaign.qualityWeight)],
				['Track record', weightLabel(campaign.reliabilityWeight)]
			]
		}
	]);
</script>

{#snippet sizes()}
	<span class="inline-flex flex-wrap justify-end gap-1.5 sm:justify-start">
		{#each campaign.sizeGroups as group (group)}
			<SizeBadge {group} />
		{/each}
	</span>
{/snippet}

<section aria-labelledby="details-title" class={sidebar ? 'surface lg:sticky lg:top-28' : 'panel'}>
	<h2 id="details-title" class="title-section">Campaign details</h2>

	<div class={['mt-6 grid gap-8 sm:grid-cols-3', sidebar && 'lg:grid-cols-1']}>
		{#each groups as group (group.title)}
			<div>
				<h3 class="text-sm font-semibold text-neutral-500">{group.title}</h3>
				<!-- One row per fact on phones, the term above its value from sm. -->
				<dl class="mt-3 flex flex-col gap-3 text-sm">
					{#each group.rows as [term, detail] (term)}
						<div class="flex justify-between gap-4 sm:block">
							<dt class="shrink-0 text-neutral-500">{term}</dt>
							<dd class="text-right font-semibold sm:text-left">
								{#if typeof detail === 'string'}{detail}{:else}{@render detail()}{/if}
							</dd>
						</div>
					{/each}
				</dl>
			</div>
		{/each}
	</div>

	<h3 class="mt-8 text-sm font-semibold text-neutral-500">Briefing</h3>
	<p
		class={[
			'mt-2 max-w-2xl leading-relaxed break-words whitespace-pre-line text-neutral-700',
			sidebar && 'text-sm'
		]}
	>
		{campaign.briefing}
	</p>
</section>
