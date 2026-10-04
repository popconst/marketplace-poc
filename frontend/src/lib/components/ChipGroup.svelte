<script lang="ts" generics="T extends string | number">
	import CheckIcon from './CheckIcon.svelte';
	import FieldMessage from './FieldMessage.svelte';

	interface Props {
		name: string;
		legend: string;
		options: { value: T; label: string }[];
		selected: T[];
		hint?: string | undefined;
		error?: string | undefined;
	}

	let { name, legend, options, selected = $bindable(), hint, error }: Props = $props();
</script>

<fieldset
	class="flex flex-col gap-2"
	data-invalid={error ? '' : undefined}
	aria-describedby={error || hint ? `${name}-message` : undefined}
>
	<legend class="mb-2 flex w-full items-baseline justify-between gap-4 text-sm font-semibold">
		{legend}
		<span aria-hidden="true" class="font-normal text-neutral-600 tabular-nums">
			{selected.length === 0 ? 'Any' : `${selected.length} selected`}
		</span>
	</legend>
	<div class="flex flex-wrap gap-2">
		{#each options as option (option.value)}
			<label class="group chip">
				<input type="checkbox" class="sr-only" {name} value={option.value} bind:group={selected} />
				<CheckIcon class="chip-tick" />
				{option.label}
			</label>
		{/each}
	</div>
	<FieldMessage id="{name}-message" {error} {hint} />
</fieldset>
