<script lang="ts">
	import StatusPill from '$lib/components/StatusPill.svelte';

	interface Props {
		cpmCents: number;
		targetCpmCents: number;
	}

	let { cpmCents, targetCpmCents }: Props = $props();

	const above = $derived(cpmCents > targetCpmCents);
</script>

<!-- Above the target is a warning to watch, not a failure. -->
<StatusPill tone={above ? 'waiting' : 'success'} label={above ? 'Above target' : 'Below target'} />
