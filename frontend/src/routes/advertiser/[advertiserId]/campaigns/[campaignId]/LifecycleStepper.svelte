<script lang="ts">
	import CheckIcon from '$lib/components/CheckIcon.svelte';
	import { formatDateTime } from '$lib/format';
	import type { CampaignDetail, CampaignPhase, Timestamp } from '$lib/types';
	import { CLOSING_FAILED } from '../../phase';

	let { campaign }: { campaign: CampaignDetail } = $props();

	// Delay between steps' entrance animations, so the progress plays out in order.
	const STEP_MS = 400;

	const reachedByPhase: Record<CampaignPhase, number> = {
		draft: 0,
		in_review: 1,
		open: 2,
		closing: 2,
		closed: 3,
		failed: 3
	};
	const reached = $derived(reachedByPhase[campaign.phase]);

	type Step = { label: string; detail: string; date?: Timestamp | null };

	const openStep = $derived.by((): Step => {
		const label = 'Open for bids';
		if (campaign.phase === 'open') {
			return { label, detail: 'Bidding closes', date: campaign.biddingDeadline };
		}
		if (campaign.phase === 'closing') return { label, detail: 'Picking winners' };
		if (campaign.activatedAt) return { label, detail: 'Opened', date: campaign.activatedAt };
		return { label, detail: 'Once approved' };
	});

	const steps = $derived<Step[]>([
		{ label: 'Draft', detail: 'Started', date: campaign.createdAt },
		campaign.submittedAt
			? { label: 'In review', detail: 'Published', date: campaign.submittedAt }
			: { label: 'In review', detail: 'After publishing' },
		openStep,
		campaign.phase === 'failed'
			? { label: 'Failed', detail: CLOSING_FAILED }
			: campaign.closedAt
				? { label: 'Closed', detail: 'Winners picked', date: campaign.closedAt }
				: { label: 'Closed', detail: 'After the deadline' }
	]);

	function stateOf(step: number): 'done' | 'current' | 'failed' | 'upcoming' {
		if (step < reached) return 'done';
		if (step > reached) return 'upcoming';
		if (campaign.phase === 'closed') return 'done';
		return campaign.phase === 'failed' ? 'failed' : 'current';
	}
</script>

<!-- A column on phones, a row from sm. Each step draws the connector to the next one. -->
<ol aria-label="Campaign progress" class="flex flex-col sm:grid sm:grid-cols-4">
	{#each steps as step, i (step.label)}
		{@const state = stateOf(i)}
		<li
			class="relative grid grid-cols-[2rem_1fr] gap-x-4 pb-6 text-left last:pb-0 sm:flex sm:flex-col sm:items-center sm:px-1 sm:pb-0 sm:text-center"
			aria-current={state === 'current' ? 'step' : undefined}
		>
			{#if i < steps.length - 1}
				<span
					aria-hidden="true"
					class="absolute top-4 -bottom-4 left-4 w-0.5 -translate-x-1/2 bg-neutral-200 sm:bottom-auto sm:left-1/2 sm:h-0.5 sm:w-full sm:translate-x-0 sm:-translate-y-1/2"
				>
					{#if i < reached}
						<span
							class="fill block size-full origin-top bg-black sm:origin-left"
							style:animation-delay="{i * STEP_MS + 200}ms"
						></span>
					{/if}
				</span>
			{/if}
			<!-- z-10 keeps the dot above the connector that runs out of it. -->
			<span
				class={[
					'relative z-10 row-span-2 grid size-8 place-items-center rounded-full',
					state === 'upcoming' && 'bg-base-100',
					state === 'done' && 'pop bg-black text-white',
					state === 'current' && 'pop bg-primary',
					state === 'failed' && 'pop bg-error text-sm font-bold text-white'
				]}
				style:animation-delay="{i * STEP_MS + 200}ms"
			>
				{#if state === 'done'}
					<CheckIcon class="size-4" />
				{:else if state === 'current'}
					<span aria-hidden="true" class="size-2.5 rounded-full bg-white"></span>
				{:else if state === 'failed'}
					<span aria-hidden="true">!</span>
				{:else}
					<span aria-hidden="true" class="size-2 rounded-full bg-neutral-300"></span>
				{/if}
			</span>
			<!-- On phones the label sits level with the dot's centre. -->
			<span
				class={[
					'pt-1.5 text-sm font-semibold sm:mt-3 sm:pt-0',
					state === 'current' && 'text-primary-strong'
				]}
			>
				{step.label}
				{#if state === 'done'}<span class="sr-only">(done)</span>{/if}
			</span>
			<span class="mt-0.5 text-sm break-words text-neutral-500 sm:text-xs">
				{step.detail}
				{#if step.date}
					<span class="block whitespace-nowrap tabular-nums">{formatDateTime(step.date)}</span>
				{/if}
			</span>
		</li>
	{/each}
</ol>

<style>
	/* The connector fills down the column on phones and across the row from sm. */
	.fill {
		animation: fill-down 0.4s var(--ease-out-expo) both;
	}

	@media (width >= 40rem) {
		.fill {
			animation-name: fill-across;
		}
	}

	@keyframes fill-down {
		from {
			scale: 1 0;
		}
	}

	@keyframes fill-across {
		from {
			scale: 0 1;
		}
	}

	.pop {
		animation: pop 0.5s var(--ease-out-expo) both;
	}

	@keyframes pop {
		from {
			opacity: 0;
			scale: 0.5;
		}
	}
</style>
