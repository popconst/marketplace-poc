<script lang="ts">
	import type { ClassValue } from 'svelte/elements';

	let { class: className }: { class: ClassValue } = $props();
</script>

<svg aria-hidden="true" viewBox="0 0 16 16" class={className}>
	<path
		d="M3 8.5l3 3 7-7"
		fill="none"
		stroke="currentColor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
</svg>
