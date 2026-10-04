<script lang="ts" generics="T extends string">
	import type { Snippet } from 'svelte';
	import FieldMessage from './FieldMessage.svelte';

	interface Props {
		name: string;
		legend: string;
		/** Keeps the legend for screen readers only, where a heading nearby already names the choice. */
		hideLegend?: boolean;
		options: { value: T; label: string }[];
		/** Null selects nothing. */
		value: T | null;
		/** An option's icon, before its label; may render nothing. */
		icon?: Snippet<[T]>;
		error?: string | undefined;
	}

	let {
		name,
		legend,
		hideLegend = false,
		options,
		value = $bindable(),
		icon,
		error
	}: Props = $props();

	const selectedIndex = $derived(options.findIndex((option) => option.value === value));
</script>

<fieldset
	class="flex flex-col gap-2"
	data-invalid={error ? '' : undefined}
	aria-describedby={error ? `${name}-message` : undefined}
>
	<legend class={['mb-2 text-sm font-semibold', hideLegend && 'sr-only']}>{legend}</legend>
	<!-- As tall as a field, so the two line up in a row of filters. -->
	<div
		class="relative grid h-11 rounded-full border border-base-300 bg-white p-1"
		style:grid-template-columns="repeat({options.length}, minmax(0, 1fr))"
	>
		<span
			aria-hidden="true"
			class={[
				'absolute inset-y-1 left-1 rounded-full bg-black transition-[translate,opacity] duration-500 ease-out-expo',
				selectedIndex === -1 && 'opacity-0'
			]}
			style:width="calc((100% - 0.5rem) / {options.length})"
			style:translate="{Math.max(selectedIndex, 0) * 100}% 0"
		></span>
		{#each options as option (option.value)}
			<label
				class="relative flex h-full cursor-pointer items-center justify-center gap-1.5 rounded-full px-1.5 text-[13px] font-semibold whitespace-nowrap text-neutral-600 transition-colors duration-300 ease-out-expo select-none hover:not-has-checked:text-black has-checked:text-white has-focus-visible:outline-2 has-focus-visible:outline-offset-2 has-focus-visible:outline-black sm:px-4 sm:text-sm"
			>
				<input type="radio" class="sr-only" {name} value={option.value} bind:group={value} />
				{@render icon?.(option.value)}
				{option.label}
			</label>
		{/each}
	</div>
	<FieldMessage id="{name}-message" {error} />
</fieldset>
