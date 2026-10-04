<script lang="ts">
	import { formatDateTime } from '$lib/format';
	import type { Timestamp } from '$lib/types';

	/** A pill counting down to a bidding deadline, red in the last 24 hours. */
	let { deadline }: { deadline: Timestamp } = $props();

	let now = $state(Date.now());

	$effect(() => {
		const timer = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(timer);
	});

	// The lower unit is zero-padded, so the pill keeps its width as it ticks.
	const pad = (n: number) => String(n).padStart(2, '0');

	const seconds = $derived(Math.floor((new Date(deadline).getTime() - now) / 1000));
	const text = $derived.by(() => {
		if (seconds <= 0) return 'Bidding closed';
		const days = Math.floor(seconds / 86_400);
		const hours = Math.floor((seconds % 86_400) / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		if (days > 0) return `Closes in ${days}d ${hours}h`;
		if (hours > 0) return `Closes in ${hours}h ${pad(minutes)}m`;
		return `Closes in ${minutes}m ${pad(seconds % 60)}s`;
	});
	const lastDay = $derived(seconds > 0 && seconds < 86_400);
</script>

<time
	datetime={deadline}
	title={formatDateTime(deadline)}
	class={[
		'pill tabular-nums',
		lastDay ? 'bg-primary/10 text-primary-strong' : 'bg-neutral-100 text-neutral-700'
	]}
>
	<!-- Lucide (lucide.dev) clock, ISC licence. -->
	<svg
		aria-hidden="true"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="2.25"
		stroke-linecap="round"
		stroke-linejoin="round"
		class="size-3.5 shrink-0"
	>
		<circle cx="12" cy="12" r="10" />
		<path d="M12 6v6l4 2" />
	</svg>
	{text}
</time>
