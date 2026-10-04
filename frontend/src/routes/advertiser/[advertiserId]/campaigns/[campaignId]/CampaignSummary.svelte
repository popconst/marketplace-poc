<script lang="ts">
	import type { Snippet } from 'svelte';
	import PhaseBadge from '$lib/components/PhaseBadge.svelte';
	import PlatformBadge from '$lib/components/PlatformBadge.svelte';
	import { formatCount, formatDateTime } from '$lib/format';
	import type { Options, PublishedCampaign } from '$lib/types';
	import CampaignDetails from './CampaignDetails.svelte';
	import LifecycleStepper from './LifecycleStepper.svelte';

	interface Props {
		campaign: PublishedCampaign;
		options: Options;
		/** Live progress or results. */
		children?: Snippet;
	}

	let { campaign, options, children }: Props = $props();

	// Once bidding has closed, the stepper and the results say when.
	const biddingAhead = $derived(['in_review', 'open', 'closing'].includes(campaign.phase));
</script>

<header class="animate-rise">
	<div class="flex flex-wrap items-center gap-2">
		<PhaseBadge phase={campaign.phase} />
		<PlatformBadge platform={campaign.platform} />
	</div>
	<h1 class="mt-3 title-page">{campaign.title}</h1>
	{#if biddingAhead}
		<p class="mt-2 lede">
			Bidding deadline {formatDateTime(campaign.biddingDeadline)}. Winners post within
			{formatCount(campaign.submissionWindowDays, 'day')} of it.
		</p>
	{/if}
</header>

<!-- The results tables need the full width; until then the details sit beside the live panel. -->
<div
	class={[
		'mt-8 grid animate-rise grid-cols-1 items-start gap-6 [--i:1]',
		biddingAhead && 'lg:grid-cols-[minmax(0,1fr)_20rem] lg:gap-8'
	]}
>
	<div class="flex min-w-0 flex-col gap-6">
		<section aria-label="Progress" class="panel">
			<LifecycleStepper {campaign} />
		</section>

		{@render children?.()}
	</div>

	<CampaignDetails {campaign} {options} sidebar={biddingAhead} />
</div>
