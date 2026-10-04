<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	// No min or max: they would be in euros, and the browser enforces them only on form submit.
	// Callers check the cents against the API's limits instead.
	type Props = Omit<HTMLInputAttributes, 'type' | 'value' | 'min' | 'max' | 'step'> & {
		/** In cents; null while the input is empty. */
		cents: number | null;
		input?: HTMLInputElement | undefined;
	};

	let { cents = $bindable(), input = $bindable(), ...attributes }: Props = $props();

	// Typed in euros, kept in cents. A third decimal rounds to the nearest cent.
	const toEuros = (amount: number | null) => (amount === null ? null : amount / 100);
	const toCents = (euros: number | null) => (euros === null ? null : Math.round(euros * 100));
</script>

<!-- app.css turns the border red when the input is aria-invalid. -->
<div class="input w-full">
	<span aria-hidden="true" class="text-neutral-500">€</span>
	<input
		{...attributes}
		bind:this={input}
		type="number"
		inputmode="decimal"
		step="0.01"
		bind:value={() => toEuros(cents), (euros) => (cents = toCents(euros))}
	/>
</div>
