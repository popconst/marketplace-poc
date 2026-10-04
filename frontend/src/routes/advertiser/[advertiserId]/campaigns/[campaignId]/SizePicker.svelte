<script lang="ts">
	import CheckIcon from '$lib/components/CheckIcon.svelte';
	import FieldMessage from '$lib/components/FieldMessage.svelte';
	import { sizeColours } from '$lib/components/SizeBadge.svelte';
	import { formatCentsRounded, formatCompact, formatSizeViews, sizeGroupLabels } from '$lib/format';
	import { SIZE_GROUPS, type SizeEstimate, type SizeGroup } from '$lib/types';

	interface Props {
		name: string;
		legend: string;
		/** Never empty: the last size picked can't be unpicked. Kept smallest first. */
		selected: SizeGroup[];
		/** From the estimate, for each size's views and account count; null if it failed to load. */
		sizes: SizeEstimate[] | null;
		hint: string;
		error?: string | undefined;
	}

	let { name, legend, selected = $bindable(), sizes, hint, error }: Props = $props();

	/** The size toggled last. Phones can't hover, so they show its tooltip in the hint. */
	let toggled = $state<SizeGroup | null>(null);
	const toggledTip = $derived.by(() => {
		const size = sizes?.find((s) => s.group === toggled);
		return size ? `${sizeGroupLabels[size.group]}: ${describe(size)}` : null;
	});

	/** `500–3K views per post, about €99 per video here.` */
	function describe(size: SizeEstimate): string {
		const price =
			size.typicalPriceCents === null
				? ''
				: `, about ${formatCentsRounded(size.typicalPriceCents)} per video here`;
		return `${formatSizeViews(size)}${price}.`;
	}

	function toggle(group: SizeGroup, picked: boolean) {
		selected = SIZE_GROUPS.filter((g) => (g === group ? picked : selected.includes(g)));
		toggled = group;
	}
</script>

<fieldset
	class="flex flex-col gap-2"
	data-invalid={error ? '' : undefined}
	aria-describedby="{name}-message"
>
	<legend class="mb-2 text-sm font-semibold">{legend}</legend>
	<div class="flex flex-wrap gap-2">
		{#each SIZE_GROUPS as group (group)}
			{@const size = sizes?.find((s) => s.group === group)}
			{@const only = selected.length === 1 && selected[0] === group}
			<div class="tooltip">
				{#if size}
					<div
						id="{name}-{group}-tip"
						role="tooltip"
						class="tooltip-content max-w-64 rounded-xl px-3 py-2 text-xs"
					>
						{describe(size)}
					</div>
				{/if}
				<label class="group chip">
					<input
						type="checkbox"
						class="sr-only"
						{name}
						value={group}
						checked={selected.includes(group)}
						aria-disabled={only ? 'true' : undefined}
						aria-describedby={size ? `${name}-${group}-tip` : undefined}
						onclick={(event) => {
							if (only) event.preventDefault();
						}}
						onchange={(event) => toggle(group, event.currentTarget.checked)}
					/>
					<CheckIcon class="chip-tick" />
					<span
						aria-hidden="true"
						class={[
							'mr-1.5 size-2 shrink-0 rounded-full border border-black/15 group-has-checked:border-white/40',
							sizeColours[group].fill
						]}
					></span>
					{sizeGroupLabels[group]}
					{#if size && size.accounts !== null}
						<span
							class="ml-1.5 text-neutral-500 tabular-nums transition-colors duration-300 group-has-checked:text-white/60"
						>
							{formatCompact(size.accounts)}
							<span class="sr-only">matching accounts</span>
						</span>
					{/if}
				</label>
			</div>
		{/each}
	</div>
	<FieldMessage id="{name}-message" {error}>
		{#if toggledTip}
			<span class="sm:hidden">{toggledTip}</span>
			<span class="max-sm:hidden">{hint}</span>
		{:else}
			{hint}
		{/if}
	</FieldMessage>
</fieldset>
