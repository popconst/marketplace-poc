<script lang="ts">
	import type { Snippet } from 'svelte';
	import Field from './Field.svelte';

	interface Props {
		name: string;
		label: string;
		/** The stops, in order, each with its label. */
		options: { value: number; label: string }[];
		value: number;
		icon?: Snippet | undefined;
		hint?: string | undefined;
		error?: string | undefined;
	}

	let { name, label, options, value = $bindable(), icon, hint, error }: Props = $props();

	const last = $derived(options.length - 1);
	const found = $derived(options.findIndex((option) => option.value === value));
	// The API accepts only the options' values; any other would show at the first stop.
	const index = $derived(found === -1 ? 0 : found);
	const selectedLabel = $derived(options[index]?.label ?? '');

	function moveTo(step: number) {
		const option = options[step];
		if (option) value = option.value;
	}
</script>

<Field {name} {label} {icon} value={selectedLabel} {hint} {error}>
	{#snippet children(control)}
		<div class="relative h-8">
			<!-- Inset by half a handle, so the fill and the ticks line up with the handle's centre. -->
			<div
				aria-hidden="true"
				class="absolute inset-x-2.5 top-1/2 h-1.5 -translate-y-1/2 rounded-full bg-neutral-200"
			>
				<div
					class="h-full rounded-full bg-primary transition-[width] duration-500 ease-out-expo"
					style:width="{(index / last) * 100}%"
				></div>
				{#each options as option, i (option.value)}
					<span
						class={[
							'absolute top-1/2 size-1 -translate-1/2 rounded-full transition-colors duration-300',
							i <= index ? 'bg-white/80' : 'bg-neutral-400'
						]}
						style:left="{(i / last) * 100}%"
					></span>
				{/each}
			</div>
			<!-- The input ranges over option indexes, not values. -->
			<input
				{...control}
				type="range"
				min="0"
				max={last}
				aria-valuetext={selectedLabel}
				class="absolute inset-0 slider w-full"
				bind:value={() => index, moveTo}
			/>
		</div>
	{/snippet}
</Field>
