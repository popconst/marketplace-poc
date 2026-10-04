<script lang="ts">
	import type { ClassValue } from 'svelte/elements';

	let { class: className }: { class: ClassValue } = $props();
</script>

<!-- Lucide (lucide.dev) circle-alert, ISC licence. -->
<svg
	aria-hidden="true"
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={['shrink-0', className]}
>
	<circle cx="12" cy="12" r="10" />
	<path d="M12 8v4M12 16h.01" />
</svg>
