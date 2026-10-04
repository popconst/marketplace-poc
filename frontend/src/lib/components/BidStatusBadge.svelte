<script lang="ts">
	import { bidStatusLabels } from '$lib/format';
	import type { BidStatus } from '$lib/types';
	import StatusPill, { type StatusTone } from './StatusPill.svelte';

	let { status }: { status: BidStatus } = $props();

	const tones: Record<BidStatus, StatusTone> = {
		pending: 'waiting',
		won: 'success',
		lost: 'error',
		withdrawn: 'neutral'
	};
</script>

<StatusPill tone={tones[status]} label={bidStatusLabels[status]} />
