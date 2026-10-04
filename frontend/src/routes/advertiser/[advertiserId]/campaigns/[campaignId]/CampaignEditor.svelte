<script lang="ts">
	import { prefersReducedMotion } from 'svelte/motion';
	import { beforeNavigate, invalidate } from '$app/navigation';
	import { errorMessage, fieldErrors, isConflict, publishCampaign, type ApiResult } from '$lib/api';
	import PhaseBadge from '$lib/components/PhaseBadge.svelte';
	import type { CampaignDetail, Estimate, Options, Timestamp } from '$lib/types';
	import { DraftAutosave } from './autosave.svelte';
	import CampaignForm from './CampaignForm.svelte';
	import EstimatePanel from './EstimatePanel.svelte';
	import PublishDialog from './PublishDialog.svelte';
	import SaveBar from './SaveBar.svelte';

	interface Props {
		/** The draft as the server has it. */
		campaign: CampaignDetail;
		options: Options;
		estimate: ApiResult<Estimate>;
	}

	let { campaign, options, estimate }: Props = $props();

	const autosave = new DraftAutosave(
		() => campaign.id,
		() => options.limits
	);
	/** The saved draft with the unsaved edits over it. */
	const draft = $derived({ ...campaign, ...autosave.edits });

	let form: CampaignForm;
	let publishDialog: PublishDialog;
	let publishError = $state<string | null>(null);

	async function startPublishing() {
		publishError = null;
		await autosave.save();
		if (autosave.hasEdits) {
			publishError = autosave.failure
				? 'Your latest changes aren’t saved yet.'
				: 'Fix the highlighted fields first.';
			form.revealErrors();
			return;
		}
		publishDialog.open();
	}

	async function publish(biddingDeadline: Timestamp): Promise<string | null> {
		const result = await publishCampaign(fetch, campaign.id, biddingDeadline);
		if (result.ok || isConflict(result.error)) {
			// Published now or meanwhile from another tab: the reload shows the summary.
			await invalidate('app:campaign');
			window.scrollTo({ top: 0, behavior: prefersReducedMotion.current ? 'auto' : 'smooth' });
			return null;
		}
		const { biddingDeadline: deadlineError, ...fields } = fieldErrors(result.error) ?? {};
		// A deadline error alone stays in the dialog, so another deadline can be picked.
		if (deadlineError && Object.keys(fields).length === 0) return deadlineError;
		publishDialog.close();
		if (Object.keys(fields).length > 0) {
			autosave.showErrors(fields);
			publishError = 'A few things are missing before you can publish.';
			form.revealErrors();
		} else {
			publishError = errorMessage(result.error);
		}
		return null;
	}

	beforeNavigate(({ willUnload, cancel }) => {
		// Unloading would cut the save off, so let the browser ask first.
		if (willUnload && autosave.hasUnsent) cancel();
		else autosave.flush();
	});
</script>

<header class="animate-rise">
	<PhaseBadge phase={campaign.phase} />
	<h1 class="mt-3 title-page">{draft.title || 'Untitled campaign'}</h1>
</header>

<div class="mt-8 grid grid-cols-1 items-start gap-6 lg:grid-cols-[minmax(0,1fr)_20rem] lg:gap-8">
	<!-- The save bar stays out of the form's entrance: a moving ancestor would unpin it on phones. -->
	<div class="flex min-w-0 flex-col gap-6">
		<CampaignForm
			bind:this={form}
			{draft}
			{autosave}
			{options}
			sizes={estimate.ok ? estimate.data.sizes : null}
		/>
		<SaveBar
			status={autosave.status}
			failure={autosave.failure}
			savedAt={campaign.updatedAt}
			{publishError}
			onretry={() => autosave.save()}
			onpublish={startPublishing}
		/>
	</div>

	<EstimatePanel {estimate} targetCpmCents={campaign.targetCpmCents} />
</div>

<PublishDialog
	bind:this={publishDialog}
	biddingHours={options.limits.biddingHours}
	submissionWindowDays={draft.submissionWindowDays}
	onpublish={publish}
/>
