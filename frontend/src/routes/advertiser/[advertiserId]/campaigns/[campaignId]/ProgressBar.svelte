<script lang="ts">
	interface Props {
		value: number;
		max: number;
		warning?: boolean;
		/** Where to draw a mark, from 0 to 1 along the bar. */
		marker?: number;
		markerLabel?: string;
	}

	let { value, max, warning = false, marker, markerLabel }: Props = $props();

	const fill = $derived(max > 0 ? Math.min(value / max, 1) : 0);
</script>

<div class="mt-2">
	<div aria-hidden="true" class="relative h-2 rounded-full bg-neutral-200">
		<div
			class={['fill h-full rounded-full', warning ? 'bg-warning' : 'bg-black']}
			style:width="{fill * 100}%"
		></div>
		{#if marker !== undefined}
			<div
				class="absolute top-1/2 h-4 w-0.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-black"
				style:left="{marker * 100}%"
			></div>
		{/if}
	</div>
	{#if marker !== undefined && markerLabel}
		<!-- Centred under the mark. -->
		<p
			class="mt-1.5 w-max -translate-x-1/2 text-xs text-neutral-500 tabular-nums"
			style:margin-left="{marker * 100}%"
		>
			{markerLabel}
		</p>
	{/if}
</div>

<style>
	/* The bar grows in as the page enters, then glides to new values. */
	.fill {
		animation: grow 1s var(--ease-out-expo) 200ms backwards;
		transition:
			width 1s var(--ease-out-expo),
			background-color 0.3s ease;
	}

	@keyframes grow {
		from {
			width: 0;
		}
	}
</style>
