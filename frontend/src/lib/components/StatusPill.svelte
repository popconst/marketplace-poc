<script lang="ts" module>
	/** Grey at rest, green when good, amber while waiting on the system, red when it went wrong. */
	export type StatusTone = 'neutral' | 'success' | 'waiting' | 'error' | 'final';
</script>

<script lang="ts">
	interface Props {
		tone: StatusTone;
		label: string;
	}

	let { tone, label }: Props = $props();

	const toneClasses: Record<StatusTone, string> = {
		neutral: 'bg-neutral-100 text-neutral-700',
		success: 'bg-success/10 text-success-strong',
		waiting: 'bg-warning/20 text-warning-strong',
		error: 'bg-error/10 text-error',
		// Outlined: nothing more will happen.
		final: 'border border-black text-black'
	};
</script>

<!-- A status is a tinted pill with a dot; metadata, such as a platform, is an outlined pill. -->
<span class={['pill', toneClasses[tone]]}>
	<span aria-hidden="true" class="size-1.5 shrink-0 rounded-full bg-current"></span>
	{label}
</span>
