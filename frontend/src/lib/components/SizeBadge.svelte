<script lang="ts" module>
	import type { SizeGroup } from '$lib/types';

	/**
	 * Grey to deep purple as accounts grow, with text that passes AA on each fill. Whole class names,
	 * so that Tailwind finds them.
	 */
	export const sizeColours: Record<SizeGroup, { fill: string; text: string }> = {
		nano: { fill: 'bg-neutral-200', text: 'text-neutral-800' },
		micro: { fill: 'bg-violet-300', text: 'text-violet-950' },
		macro: { fill: 'bg-violet-600', text: 'text-white' },
		mega: { fill: 'bg-violet-900', text: 'text-white' }
	};
</script>

<script lang="ts">
	import { page } from '$app/state';
	import { formatSizeViews, sizeGroupLabels } from '$lib/format';
	import type { Options } from '$lib/types';

	/** `dot` is a coloured dot and the name in plain text, for lists where pills would be loud. */
	let { group, variant = 'pill' }: { group: SizeGroup; variant?: 'pill' | 'dot' } = $props();

	// The root layout loads the options for every page.
	const options: Options = $derived(page.data['options']);
	const range = $derived(options.sizeGroups.find((size) => size.group === group));
	const label = $derived(sizeGroupLabels[group]);
</script>

<!-- The tooltip only shows on hover, so screen readers get its text from the sr-only span. -->
<span class="tooltip cursor-pointer">
	{#if range}
		<span aria-hidden="true" class="tooltip-content max-w-64 rounded-xl px-3 py-2 text-xs">
			{label}: {formatSizeViews(range)}
		</span>
	{/if}
	{#if variant === 'dot'}
		<span class="inline-flex items-center gap-2">
			<!-- Nano's light grey needs an edge to show on white. -->
			<span
				aria-hidden="true"
				class={['size-2.5 rounded-full ring-1 ring-black/10', sizeColours[group].fill]}
			></span>
			{label}{#if range}<span class="sr-only">: {formatSizeViews(range)}</span>{/if}
		</span>
	{:else}
		<span class={['pill', sizeColours[group].fill, sizeColours[group].text]}>
			{label}{#if range}<span class="sr-only">: {formatSizeViews(range)}</span>{/if}
		</span>
	{/if}
</span>
