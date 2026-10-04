<script lang="ts">
	interface Props {
		legend: string;
		/** Ascending. A handle on the first or last stop leaves that end open. */
		stops: readonly number[];
		/** The bounds as set, or null for none. Values between stops show at the nearest one. */
		low: number | null;
		high: number | null;
		format: (value: number) => string;
		/** Called when a handle is let go, with null for an open end. */
		onchange: (low: number | null, high: number | null) => void;
	}

	let { legend, stops, low, high, format, onchange }: Props = $props();

	const last = $derived(stops.length - 1);

	/** The index of the stop closest to `value`. */
	function nearest(value: number): number {
		const distances = stops.map((stop) => Math.abs(stop - value));
		return distances.indexOf(Math.min(...distances));
	}

	// Handle positions as stop indexes. A drag assigns them so the handle moves at once; the next
	// change of `low` or `high` derives them again.
	let lowIndex = $derived(low === null ? 0 : nearest(low));
	let highIndex = $derived(high === null ? last : nearest(high));

	const lowValue = $derived(lowIndex === 0 ? null : (stops[lowIndex] ?? null));
	const highValue = $derived(highIndex === last ? null : (stops[highIndex] ?? null));

	const readout = $derived.by(() => {
		const min = lowValue === null ? null : format(lowValue);
		const max = highValue === null ? null : format(highValue);
		if (min && max) return min === max ? min : `${min} – ${max}`;
		if (min) return `${min} or more`;
		if (max) return `Up to ${max}`;
		return 'Any';
	});
	// Where the handles meet, the one that can still move must be on top.
	const lowOnTop = $derived(lowIndex === highIndex && lowIndex > last / 2);
	// With both ends open nothing is filtered out, so there is no range to fill.
	const open = $derived(lowValue === null && highValue === null);
</script>

<fieldset>
	<legend class="flex w-full items-baseline justify-between gap-4 text-sm font-semibold">
		{legend}
		<span class="font-normal text-neutral-600 tabular-nums">{readout}</span>
	</legend>
	<div class="relative mt-1 h-8">
		<!-- Inset by half a handle, so the fill ends under the handles' centres. -->
		<div aria-hidden="true" class="absolute inset-x-2.5 top-1/2 h-1.5 -translate-y-1/2">
			<div class="h-full rounded-full bg-neutral-200"></div>
			<div
				class={['fill absolute inset-y-0 rounded-full bg-primary', open && 'opacity-0']}
				style:left="{(lowIndex / last) * 100}%"
				style:right="{100 - (highIndex / last) * 100}%"
			></div>
		</div>
		<input
			type="range"
			min="0"
			max={last}
			class="slider"
			aria-label="Lowest {legend.toLowerCase()}"
			aria-valuetext={lowValue === null ? 'No minimum' : format(lowValue)}
			style:z-index={lowOnTop ? 2 : 1}
			bind:value={() => lowIndex, (index) => (lowIndex = Math.min(index, highIndex))}
			onchange={() => onchange(lowValue, highValue)}
		/>
		<input
			type="range"
			min="0"
			max={last}
			class="slider"
			aria-label="Highest {legend.toLowerCase()}"
			aria-valuetext={highValue === null ? 'No maximum' : format(highValue)}
			bind:value={() => highIndex, (index) => (highIndex = Math.max(index, lowIndex))}
			onchange={() => onchange(lowValue, highValue)}
		/>
	</div>
</fieldset>

<style>
	/* Two inputs share one track: only their handles take the pointer. */
	input {
		position: absolute;
		inset: 0;
		z-index: 1;
		width: 100%;
		pointer-events: none;
	}

	input::-webkit-slider-thumb {
		pointer-events: auto;
	}

	input::-moz-range-thumb {
		pointer-events: auto;
	}

	.fill {
		transition:
			left 0.3s var(--ease-out-expo),
			right 0.3s var(--ease-out-expo),
			opacity 0.3s var(--ease-out-expo);
	}
</style>
