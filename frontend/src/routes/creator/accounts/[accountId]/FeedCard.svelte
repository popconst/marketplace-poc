<script lang="ts">
	import BidStatusBadge from '$lib/components/BidStatusBadge.svelte';
	import Countdown from '$lib/components/Countdown.svelte';
	import WhyItMatches from '$lib/components/WhyItMatches.svelte';
	import { flag, formatCents, formatNumber, optionNames } from '$lib/format';
	import type { FeedItem, Options } from '$lib/types';

	interface Props {
		item: FeedItem;
		options: Options;
		onbid: () => void;
	}

	let { item, options, onbid }: Props = $props();

	const campaign = $derived(item.campaign);
	// The feed lists only open campaigns, so a bid here is pending or withdrawn, and editable.
	const bid = $derived(item.myBid);
	const score = $derived(Math.round(item.match.score));
	const names = $derived(optionNames(options));
</script>

<!-- Targeting is metadata, so outline pills. -->
{#snippet target(text: string, title?: string)}
	<li {title} class="pill border border-base-300 bg-base-100 text-neutral-700">{text}</li>
{/snippet}

{#snippet anyTarget(text: string)}
	<li class="pill border border-base-300 bg-base-100 text-neutral-500">{text}</li>
{/snippet}

<article class="flex h-full flex-col gap-5 surface">
	<div>
		<p class="truncate text-sm font-semibold text-neutral-500">{campaign.advertiserName}</p>
		<h2 class="mt-1 title-card">{campaign.title}</h2>
		<div class="mt-3 flex flex-wrap items-center gap-2">
			<Countdown deadline={campaign.biddingDeadline} />
			<WhyItMatches {score} factors={item.match.factors} />
		</div>
	</div>

	<ul aria-label="Target countries, languages and genres" class="flex flex-wrap gap-2">
		{#each campaign.countryCodes as code (code)}
			{@render target(`${flag(code)} ${code}`, names.country(code))}
		{:else}
			{@render anyTarget('Any country')}
		{/each}
		{#each campaign.languageCodes as code (code)}
			{@render target(names.language(code))}
		{:else}
			{@render anyTarget('Any language')}
		{/each}
		{#each campaign.genreIds as id (id)}
			{@render target(names.genre(id))}
		{:else}
			{@render anyTarget('Any genre')}
		{/each}
	</ul>

	<div
		class="mt-auto flex items-center gap-3 border-t border-base-300 pt-4 text-sm text-neutral-600"
	>
		{#if bid}
			<p class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
				Your bid
				<span class="font-semibold text-black tabular-nums">{formatCents(bid.amountCents)}</span>
				<BidStatusBadge status={bid.status} />
			</p>
		{:else}
			<!-- The views are what the video must reach to get paid at that bid. -->
			<p class="min-w-0 text-neutral-500 tabular-nums">
				<span class="block text-xs">Suggested bid</span>
				<span class="font-bold text-black">{formatCents(item.suggestedBidCents)}</span>
				for {formatNumber(item.suggestedMinPaidViews)} views
			</p>
		{/if}
		<button type="button" class={['btn ml-auto shrink-0', !bid && 'btn-neutral']} onclick={onbid}>
			{bid ? 'Edit bid' : 'Bid'}
		</button>
	</div>
</article>
