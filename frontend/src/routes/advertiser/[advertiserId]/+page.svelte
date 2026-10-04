<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createCampaign, errorMessage } from '$lib/api';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import PhaseBadge from '$lib/components/PhaseBadge.svelte';
	import PlatformBadge from '$lib/components/PlatformBadge.svelte';
	import { formatCents } from '$lib/format';
	import { describePhase } from './phase';

	let { data } = $props();

	let creating = $state(false);
	let createError = $state<string | null>(null);

	const campaignPath = (campaignId: number) =>
		resolve('/advertiser/[advertiserId]/campaigns/[campaignId]', {
			advertiserId: String(data.advertiser.id),
			campaignId: String(campaignId)
		});

	async function newCampaign() {
		creating = true;
		createError = null;
		const result = await createCampaign(fetch, data.advertiser.id);
		if (result.ok) return goto(campaignPath(result.data.id));
		creating = false;
		createError = errorMessage(result.error);
	}
</script>

<svelte:head>
	<title>{data.advertiser.name} · WePush</title>
</svelte:head>

{#snippet newCampaignButton()}
	<button type="button" class="btn btn-neutral" disabled={creating} onclick={newCampaign}>
		{#if creating}
			<span class="loading loading-sm loading-spinner"></span> Creating…
		{:else}
			New campaign
		{/if}
	</button>
{/snippet}

<header class="animate-rise">
	<a
		href={resolve('/advertiser')}
		class="inline-flex items-center gap-1.5 text-sm text-neutral-600 transition-colors duration-300 ease-out-expo hover:text-black"
	>
		<span aria-hidden="true">←</span> All advertisers
	</a>
	<div class="mt-6 flex flex-col items-start gap-6 sm:flex-row sm:items-end sm:justify-between">
		<h1 class="title-page">{data.advertiser.name}</h1>
		{@render newCampaignButton()}
	</div>
</header>
{#if createError}
	<p role="alert" class="mt-3 animate-rise text-sm text-error sm:text-right">{createError}</p>
{/if}

{#if data.campaigns.length === 0}
	<EmptyState
		class="mt-8 animate-rise [--i:1]"
		title="No campaigns yet"
		message="Start one: it saves as a draft while you fill it in, and creators see it once you publish."
	>
		{@render newCampaignButton()}
	</EmptyState>
{:else}
	<ul class="mt-8 grid animate-rise gap-4 [--i:1] sm:gap-6">
		{#each data.campaigns as campaign (campaign.id)}
			<li>
				<a href={campaignPath(campaign.id)} class="group flex lift items-center gap-6 surface">
					<div class="min-w-0 flex-1">
						<div class="flex flex-wrap items-center gap-2">
							<PhaseBadge phase={campaign.phase} />
							{#if campaign.platform}
								<PlatformBadge platform={campaign.platform} />
							{/if}
						</div>
						<h2 class="mt-2 truncate title-card">{campaign.title ?? 'Untitled campaign'}</h2>
						<p class="mt-0.5 text-sm text-neutral-500">
							{describePhase(campaign)}
							<!-- The budget gets its own column from sm. -->
							{#if campaign.budgetCents !== null}
								<span class="tabular-nums sm:hidden">· {formatCents(campaign.budgetCents)}</span>
							{/if}
						</p>
					</div>
					{#if campaign.budgetCents !== null}
						<p class="hidden text-right sm:block">
							<span class="block text-sm text-neutral-500">Budget</span>
							<span class="font-semibold tabular-nums">{formatCents(campaign.budgetCents)}</span>
						</p>
					{/if}
					<span
						aria-hidden="true"
						class="text-neutral-400 transition-[translate,color] duration-500 ease-out-expo group-hover:translate-x-1 group-hover:text-black group-focus-visible:translate-x-1 group-focus-visible:text-black"
					>
						→
					</span>
				</a>
			</li>
		{/each}
	</ul>
{/if}
