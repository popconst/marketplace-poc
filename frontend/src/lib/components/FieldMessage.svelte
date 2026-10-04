<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** For the control's `aria-describedby`. */
		id: string;
		error?: string | undefined;
		hint?: string | undefined;
		/** A hint that needs markup, in place of `hint`. */
		children?: Snippet;
	}

	/** The line under a control: what is wrong with its value, or else a hint. */
	let { id, error, hint, children }: Props = $props();
</script>

<!-- Only an error animates in, to catch the eye. -->
{#if error}
	<p {id} class="animate-rise text-sm text-error">{error}</p>
{:else if children}
	<p {id} class="text-sm text-neutral-500">{@render children()}</p>
{:else if hint}
	<p {id} class="text-sm text-neutral-500">{hint}</p>
{/if}
