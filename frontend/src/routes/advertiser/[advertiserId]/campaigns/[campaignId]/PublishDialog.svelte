<script lang="ts">
	import { tick } from 'svelte';
	import Field from '$lib/components/Field.svelte';
	import { formatDateTime, formatFromNow } from '$lib/format';
	import type { CampaignLimits, Timestamp } from '$lib/types';
	import {
		deadlineBounds,
		deadlineIn,
		deadlineInMinutes,
		formatHours,
		postBy,
		toLocalInput
	} from './deadline';

	interface Props {
		biddingHours: CampaignLimits['biddingHours'];
		/** Winners post within this many days of the deadline; null while unset. */
		submissionWindowDays: number | null;
		/** Resolves to a deadline error for the dialog to show; the caller shows any other error. */
		onpublish: (biddingDeadline: Timestamp) => Promise<string | null>;
	}

	let { biddingHours, submissionWindowDays, onpublish }: Props = $props();

	const QUICK_PICKS = [
		{ label: '3 days', days: 3 },
		{ label: '1 week', days: 7 },
		{ label: '2 weeks', days: 14 }
	];
	const DEFAULT_DAYS = 7;
	/** Offered only when bidding has no minimum length, as under the demo's dev tools. */
	const DEMO_PICK_MINUTES = 5;

	let dialog: HTMLDialogElement;
	let isOpen = $state(false);
	// The bounds are relative to now, so it is refreshed on open, on a quick pick and on submit.
	let now = $state(Date.now());
	/** In milliseconds; null while the input is empty. */
	let deadline = $state<number | null>(null);
	let publishing = $state(false);
	let serverError = $state<string | null>(null);

	const bounds = $derived(deadlineBounds(now, biddingHours));
	// A pick outside the bidding window would be clamped to its edge, possibly onto another pick.
	const quickPicks = $derived(
		QUICK_PICKS.filter(({ days }) => biddingHours.min <= days * 24 && days * 24 <= biddingHours.max)
	);
	const localError = $derived.by(() => {
		if (deadline === null) return 'Pick when bidding closes.';
		if (deadline < bounds.min || deadline > bounds.max) {
			const latest = formatHours(biddingHours.max);
			if (biddingHours.min === 0) return `Pick a time in the next ${latest}.`;
			return `Pick a time between ${formatHours(biddingHours.min)} and ${latest} from now.`;
		}
		return null;
	});
	const hint = $derived.by(() => {
		if (deadline === null) return undefined;
		const closes = new Date(deadline);
		const closing = `Bidding closes ${formatDateTime(closes)}, ${formatFromNow(closes, now)}.`;
		if (submissionWindowDays === null) return closing;
		const postByDate = new Date(postBy(deadline, submissionWindowDays));
		return `${closing} Winners must post by ${formatDateTime(postByDate)}.`;
	});

	export async function open() {
		now = Date.now();
		deadline = deadlineIn(DEFAULT_DAYS, now, biddingHours);
		serverError = null;
		publishing = false;
		isOpen = true;
		await tick();
		dialog.showModal();
		// Focus the selected quick pick: a focus ring on another would read as a second selection.
		dialog.querySelector<HTMLElement>('[aria-pressed="true"]')?.focus();
	}

	export function close() {
		dialog.close();
	}

	function setDeadline(next: number | null) {
		deadline = next;
		serverError = null;
	}

	function pick(days: number) {
		now = Date.now();
		setDeadline(deadlineIn(days, now, biddingHours));
	}

	function pickMinutes(minutes: number) {
		now = Date.now();
		setDeadline(deadlineInMinutes(minutes, now));
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		now = Date.now();
		if (deadline === null || localError) return;
		publishing = true;
		try {
			serverError = await onpublish(new Date(deadline).toISOString());
		} finally {
			publishing = false;
		}
	}
</script>

<!-- A click on the backdrop lands on the dialog itself; clicks inside land on its children. -->
<dialog
	bind:this={dialog}
	aria-labelledby="publish-title"
	onclose={() => (isOpen = false)}
	onclick={(event) => event.target === dialog && dialog.close()}
	class="dialog-card"
>
	{#if isOpen}
		<form
			class="flex flex-col gap-6 p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))] sm:p-8"
			novalidate
			onsubmit={submit}
		>
			<header>
				<h2 id="publish-title" class="title-section">When should bidding close?</h2>
				<p class="mt-2 text-sm text-neutral-600">
					Creators can bid until then. Winners are picked as soon as it closes.
				</p>
			</header>

			<Field
				name="biddingDeadline"
				label="Bidding deadline"
				{hint}
				error={localError ?? serverError ?? undefined}
			>
				{#snippet children(control)}
					<div role="group" aria-label="Quick picks" class="flex flex-wrap gap-2">
						{#if biddingHours.min === 0}
							<button
								type="button"
								aria-pressed={deadline === deadlineInMinutes(DEMO_PICK_MINUTES, now)}
								class="chip"
								onclick={() => pickMinutes(DEMO_PICK_MINUTES)}
							>
								{DEMO_PICK_MINUTES} minutes
							</button>
						{/if}
						{#each quickPicks as { label, days } (days)}
							<button
								type="button"
								aria-pressed={deadline === deadlineIn(days, now, biddingHours)}
								class="chip"
								onclick={() => pick(days)}
							>
								{label}
							</button>
						{/each}
					</div>
					<input
						{...control}
						type="datetime-local"
						class="input w-full"
						min={toLocalInput(bounds.min)}
						max={toLocalInput(bounds.max)}
						bind:value={
							() => (deadline === null ? '' : toLocalInput(deadline)),
							(local) => setDeadline(local ? new Date(local).getTime() : null)
						}
					/>
				{/snippet}
			</Field>

			<!-- Full width on phones, where the dialog is a bottom sheet. -->
			<div class="flex flex-col-reverse gap-3 sm:flex-row sm:justify-end">
				<button type="button" class="btn" onclick={close}>Cancel</button>
				<button type="submit" class="btn btn-neutral" disabled={publishing || localError !== null}>
					{#if publishing}
						<span class="loading loading-sm loading-spinner"></span> Publishing…
					{:else}
						Publish
					{/if}
				</button>
			</div>
		</form>
	{/if}
</dialog>
