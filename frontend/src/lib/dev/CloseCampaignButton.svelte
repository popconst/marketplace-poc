<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { closeCampaign, errorMessage } from '$lib/api';

	/** Demo only: render it for an active campaign, and only while `Options.devTools` is on. */
	let { campaignId }: { campaignId: number } = $props();

	// A second click confirms. A browser dialog would block whatever drives the demo.
	let step = $state<'idle' | 'confirming' | 'closing'>('idle');
	let failure = $state<string | null>(null);

	async function onclick() {
		if (step === 'idle') {
			step = 'confirming';
			failure = null;
			return;
		}
		step = 'closing';
		const result = await closeCampaign(fetch, campaignId);
		if (result.ok) {
			await invalidateAll();
		} else {
			failure = errorMessage(result.error);
		}
		step = 'idle';
	}
</script>

<button
	type="button"
	class={['btn mt-4 w-full btn-sm', step === 'confirming' && 'btn-primary']}
	disabled={step === 'closing'}
	{onclick}
	onblur={() => {
		if (step === 'confirming') step = 'idle';
	}}
>
	{#if step === 'idle'}
		Close campaign now
	{:else if step === 'confirming'}
		Click again to close
	{:else}
		<span class="loading loading-xs loading-spinner"></span> Closing…
	{/if}
</button>
{#if failure}
	<p role="status" class="mt-3 animate-rise text-sm text-error">{failure}</p>
{/if}
