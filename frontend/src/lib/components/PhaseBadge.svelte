<script lang="ts">
	import type { CampaignPhase } from '$lib/types';
	import StatusPill, { type StatusTone } from './StatusPill.svelte';

	let { phase }: { phase: CampaignPhase } = $props();

	const badges: Record<CampaignPhase, { label: string; tone: StatusTone }> = {
		draft: { label: 'Draft', tone: 'neutral' },
		in_review: { label: 'In review', tone: 'waiting' },
		open: { label: 'Open for bids', tone: 'success' },
		closing: { label: 'Picking winners', tone: 'waiting' },
		closed: { label: 'Closed', tone: 'final' },
		failed: { label: 'Closing failed', tone: 'error' }
	};
</script>

<StatusPill {...badges[phase]} />
