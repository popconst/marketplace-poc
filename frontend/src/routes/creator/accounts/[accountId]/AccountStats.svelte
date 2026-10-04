<script lang="ts">
	import { formatCompact, formatPercent } from '$lib/format';
	import type { PlatformAccount } from '$lib/types';

	interface Props {
		account: PlatformAccount;
	}

	let { account }: Props = $props();
</script>

{#snippet stat(label: string, value: string)}
	<div class="rounded-xl bg-neutral-50 px-3 py-2.5">
		<dt class="text-xs whitespace-nowrap text-neutral-500">{label}</dt>
		<dd class="mt-0.5 font-semibold tabular-nums">{value}</dd>
	</div>
{/snippet}

<!-- Equal columns, but one widens rather than wrapping or cutting its label. -->
<dl class="grid grid-cols-[repeat(3,minmax(max-content,1fr))] gap-2">
	{@render stat('Avg. views/post', formatCompact(account.viewScore))}
	{@render stat('Followers', formatCompact(account.followers))}
	{@render stat('Engagement', formatPercent(account.engagementRate))}
</dl>
