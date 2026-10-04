<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		description?: string;
		children: Snippet;
	}

	let { title, description, children }: Props = $props();

	const id = $props.id();
</script>

<section aria-labelledby={id} class="panel">
	<h2 {id} class="title-section">{title}</h2>
	{#if description}
		<p class="mt-2 max-w-2xl text-sm text-neutral-600">{description}</p>
	{/if}
	<div class="mt-6 flex flex-col gap-6">
		{@render children()}
	</div>
</section>
