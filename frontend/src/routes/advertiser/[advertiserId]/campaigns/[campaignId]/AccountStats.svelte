<script lang="ts">
	import PlatformBadge from '$lib/components/PlatformBadge.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { formatCompact, formatDateTime, formatPercent, optionNames } from '$lib/format';
	import type { Options, PlatformAccount } from '$lib/types';

	interface Props {
		account: PlatformAccount;
		options: Options;
	}

	let { account, options }: Props = $props();

	const names = $derived(optionNames(options));
</script>

{#snippet figure(label: string, value: string)}
	<div class="rounded-xl bg-neutral-50 px-3 py-2.5">
		<dt class="text-xs whitespace-nowrap text-neutral-500">{label}</dt>
		<dd class="mt-0.5 font-semibold tabular-nums">{value}</dd>
	</div>
{/snippet}

<!-- A score from 0 to 100, with a bar for where it sits on that scale. -->
{#snippet score(label: string, value: number)}
	<div class="rounded-xl bg-neutral-50 px-3 py-2.5">
		<dt class="text-xs whitespace-nowrap text-neutral-500">{label}</dt>
		<dd class="mt-0.5 font-semibold tabular-nums">
			{value}<span class="sr-only"> out of 100</span>
			<span aria-hidden="true" class="mt-1 block h-1 overflow-hidden rounded-full bg-neutral-200">
				<span class="block h-full rounded-full bg-black" style:width="{value}%"></span>
			</span>
		</dd>
	</div>
{/snippet}

{#snippet pills(label: string, items: string[])}
	<div>
		<dt class="text-xs text-neutral-500">{label}</dt>
		<dd class="mt-1.5 flex flex-wrap gap-1.5">
			{#each items as item (item)}
				<span class="pill border border-base-300 bg-base-100 text-neutral-700">{item}</span>
			{:else}
				<span class="text-sm text-neutral-500">None</span>
			{/each}
		</dd>
	</div>
{/snippet}

<div class="flex flex-wrap items-center gap-2">
	<PlatformBadge platform={account.platform} />
	<StatusPill
		tone={account.brandSafe ? 'success' : 'error'}
		label={account.brandSafe ? 'Brand safe' : 'Not brand safe'}
	/>
</div>

<!-- Equal columns, but one widens rather than wrapping or cutting its label. -->
<dl class="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-[repeat(3,minmax(max-content,1fr))]">
	{@render figure('Followers', formatCompact(account.followers))}
	{@render figure('Avg. views/post', formatCompact(account.viewScore))}
	{@render figure('Engagement', formatPercent(account.engagementRate))}
	{@render figure('Posts per week', account.postsPerWeek.toFixed(1))}
	{@render score('Content quality', account.qualityScore)}
	{@render score('Track record', account.reliabilityScore)}
</dl>

<dl class="mt-5 flex flex-col gap-4">
	{@render pills('Genres', account.genreIds.map(names.genre))}
	{@render pills('Languages', account.languageCodes.map(names.language))}
</dl>

<p class="mt-5 text-xs text-neutral-500">Stats updated {formatDateTime(account.statsUpdatedAt)}</p>
