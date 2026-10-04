<script lang="ts">
	import { resolve } from '$app/paths';
	import { formatCount } from '$lib/format';
	import { isPublished } from '$lib/types';
	import CampaignEditor from './CampaignEditor.svelte';
	import CampaignProgress from './CampaignProgress.svelte';
	import CampaignResults from './CampaignResults.svelte';
	import CampaignSummary from './CampaignSummary.svelte';

	let { data } = $props();

	const isDraft = $derived(data.campaign.status === 'draft');
</script>

<svelte:head>
	<title>{data.campaign.title ?? 'Untitled campaign'} · {data.advertiser.name} · WePush</title>
</svelte:head>

<!-- A draft leaves room for the save bar, which is pinned to the bottom on phones. -->
<div class={['mx-auto max-w-3xl lg:max-w-none', isDraft && 'pb-28 lg:pb-0']}>
	<div class="mb-6 flex animate-rise items-center justify-between gap-4">
		<a
			href={resolve('/advertiser/[advertiserId]', { advertiserId: String(data.advertiser.id) })}
			class="inline-flex items-center gap-1.5 text-sm text-neutral-600 transition-colors duration-300 ease-out-expo hover:text-black"
		>
			<span aria-hidden="true">←</span>
			{data.advertiser.name}
		</a>
		{#if data.options.devTools && data.campaign.status === 'failed'}
			<p
				class="max-w-md rounded-2xl border border-dashed border-neutral-300 px-3 py-1.5 text-xs text-neutral-500"
			>
				Closing failed {formatCount(data.campaign.closeAttempts, 'time')}:
				{data.campaign.lastCloseError ?? 'no error recorded'}
			</p>
		{/if}
	</div>

	{#if isPublished(data.campaign)}
		<CampaignSummary campaign={data.campaign} options={data.options}>
			{#if data.progress}
				<CampaignProgress
					campaignId={data.campaign.id}
					biddingDeadline={data.campaign.biddingDeadline}
					progress={data.progress}
				/>
			{:else if data.results}
				<CampaignResults results={data.results} options={data.options} />
			{/if}
		</CampaignSummary>
	{:else if data.estimate}
		<!-- Keyed so a pending edit to one draft is never saved to another. -->
		{#key data.campaign.id}
			<CampaignEditor campaign={data.campaign} options={data.options} estimate={data.estimate} />
		{/key}
	{/if}
</div>
