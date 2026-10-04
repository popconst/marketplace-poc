<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ClassValue } from 'svelte/elements';

	interface Props {
		title: string;
		/** One line on why it is empty and what to do about it. */
		message?: string | undefined;
		/** For use inside a panel. */
		inset?: boolean;
		/** Placement, such as a top margin or an entrance. */
		class?: ClassValue;
		/** A large, muted icon of what is missing, above the title. */
		icon?: Snippet;
		/** The next action: a button or a link. */
		children?: Snippet;
	}

	let { title, message, inset = false, class: className, icon, children }: Props = $props();
</script>

<div class={[inset ? 'empty-state-inset' : 'empty-state', className]}>
	{@render icon?.()}
	<p class={['text-base font-bold', icon && 'mt-3']}>{title}</p>
	{#if message}
		<p class="mx-auto mt-1 max-w-md text-sm text-neutral-600">{message}</p>
	{/if}
	{#if children}
		<div class="mt-6">{@render children()}</div>
	{/if}
</div>
