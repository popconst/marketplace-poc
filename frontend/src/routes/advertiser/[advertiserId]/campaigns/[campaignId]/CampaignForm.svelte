<script lang="ts">
	import { tick } from 'svelte';
	import { prefersReducedMotion } from 'svelte/motion';
	import ChipGroup from '$lib/components/ChipGroup.svelte';
	import EuroInput from '$lib/components/EuroInput.svelte';
	import Field from '$lib/components/Field.svelte';
	import PlatformIcon from '$lib/components/PlatformIcon.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import StepSlider from '$lib/components/StepSlider.svelte';
	import { flag, platformNames } from '$lib/format';
	import type { Options, SizeEstimate } from '$lib/types';
	import type { DraftAutosave } from './autosave.svelte';
	import type { DraftFields } from './edits';
	import FormSection from './FormSection.svelte';
	import PreferenceIcon from './PreferenceIcon.svelte';
	import SizePicker from './SizePicker.svelte';
	import { WEIGHT_OPTIONS } from './weights';

	interface Props {
		draft: DraftFields;
		autosave: DraftAutosave;
		options: Options;
		sizes: SizeEstimate[] | null;
	}

	let { draft, autosave, options, sizes }: Props = $props();

	const limits = $derived(options.limits);
	const platformOptions = (['tiktok', 'instagram'] as const).map((value) => ({
		value,
		label: platformNames[value]
	}));
	const countryOptions = $derived(
		options.countries.map(({ code, name }) => ({ value: code, label: `${flag(code)} ${name}` }))
	);
	const languageOptions = $derived(
		options.languages.map(({ code, name }) => ({ value: code, label: name }))
	);
	const genreOptions = $derived(options.genres.map(({ id, name }) => ({ value: id, label: name })));

	let form: HTMLFormElement;

	// Title and briefing as typed, shown while focused: the server trims text, and the trimmed
	// text mustn't replace what is being typed under the cursor.
	let keptWhileFocused = $state<{ title?: string; briefing?: string }>({});

	function typeText(field: 'title' | 'briefing', text: string) {
		keptWhileFocused[field] = text;
		autosave.edit(field, text || null);
	}

	function leaveField() {
		keptWhileFocused = {};
		autosave.flush();
	}

	/** Shakes every field with an error and brings the first into view, focused. */
	export async function revealErrors() {
		await tick();
		const fields = form.querySelectorAll<HTMLElement>('[data-invalid]');
		const reduceMotion = prefersReducedMotion.current;
		fields[0]?.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'center' });
		fields[0]?.querySelector<HTMLElement>('input, textarea')?.focus({ preventScroll: true });
		if (reduceMotion) return;
		for (const field of fields) {
			field.animate(
				{ translate: ['0', '-6px', '5px', '-3px', '0'] },
				{ duration: 400, easing: 'ease-out' }
			);
		}
	}
</script>

<form bind:this={form} class="flex animate-rise flex-col gap-6 [--i:1]" onfocusout={leaveField}>
	<FormSection title="Basics">
		<Field name="title" label="Title" error={autosave.errors.title}>
			{#snippet children(control)}
				<input
					{...control}
					class="input w-full"
					placeholder="Summer drop: our new running shoe"
					maxlength={limits.titleMaxChars}
					bind:value={
						() => keptWhileFocused.title ?? draft.title ?? '', (text) => typeText('title', text)
					}
				/>
			{/snippet}
		</Field>
		<Field
			name="briefing"
			label="Briefing"
			hint="What should creators post? Include must-haves, the tone, and anything to avoid."
			error={autosave.errors.briefing}
		>
			{#snippet children(control)}
				<textarea
					{...control}
					class="textarea w-full"
					rows="5"
					maxlength={limits.briefingMaxChars}
					bind:value={
						() => keptWhileFocused.briefing ?? draft.briefing ?? '',
						(text) => typeText('briefing', text)
					}></textarea>
			{/snippet}
		</Field>
	</FormSection>

	<FormSection
		title="Who should see it"
		description="Accounts outside these limits never see the campaign. Leave a list empty to allow any."
	>
		<SegmentedControl
			name="platform"
			legend="Platform"
			options={platformOptions}
			error={autosave.errors.platform}
			bind:value={() => draft.platform, (platform) => autosave.edit('platform', platform)}
		>
			{#snippet icon(platform)}<PlatformIcon {platform} />{/snippet}
		</SegmentedControl>
		<ChipGroup
			name="countryCodes"
			legend="Countries"
			options={countryOptions}
			hint="Where the account is based."
			error={autosave.errors.countryCodes}
			bind:selected={() => draft.countryCodes, (codes) => autosave.edit('countryCodes', codes)}
		/>
		<ChipGroup
			name="languageCodes"
			legend="Languages"
			options={languageOptions}
			hint="What the account posts in."
			error={autosave.errors.languageCodes}
			bind:selected={() => draft.languageCodes, (codes) => autosave.edit('languageCodes', codes)}
		/>
		<ChipGroup
			name="genreIds"
			legend="Genres"
			options={genreOptions}
			error={autosave.errors.genreIds}
			bind:selected={() => draft.genreIds, (ids) => autosave.edit('genreIds', ids)}
		/>
	</FormSection>

	<FormSection
		title="Budget and pricing"
		description="What you’ll spend, what a view is worth to you, and on which account sizes."
	>
		<div class="grid gap-6 sm:grid-cols-2">
			<Field name="budgetCents" label="Budget" error={autosave.errors.budgetCents}>
				{#snippet children(control)}
					<EuroInput
						{...control}
						placeholder="5000"
						bind:cents={() => draft.budgetCents, (cents) => autosave.edit('budgetCents', cents)}
					/>
				{/snippet}
			</Field>
			<Field name="targetCpmCents" label="Target CPM" error={autosave.errors.targetCpmCents}>
				{#snippet children(control)}
					<EuroInput
						{...control}
						placeholder="12"
						bind:cents={
							() => draft.targetCpmCents, (cents) => autosave.edit('targetCpmCents', cents)
						}
					/>
				{/snippet}
			</Field>
		</div>
		<SizePicker
			name="sizeGroups"
			legend="Account sizes"
			{sizes}
			hint="Your budget is split evenly across the sizes you pick."
			error={autosave.errors.sizeGroups}
			bind:selected={() => draft.sizeGroups, (groups) => autosave.edit('sizeGroups', groups)}
		/>
	</FormSection>

	<FormSection
		title="Matching preferences"
		description="These rank your campaign in creators’ feeds and let better matches win at a higher price."
	>
		<StepSlider
			name="engagementWeight"
			label="Engagement"
			options={WEIGHT_OPTIONS}
			error={autosave.errors.engagementWeight}
			bind:value={
				() => draft.engagementWeight, (weight) => autosave.edit('engagementWeight', weight)
			}
		>
			{#snippet icon()}<PreferenceIcon kind="engagement" />{/snippet}
		</StepSlider>
		<StepSlider
			name="qualityWeight"
			label="Content quality"
			options={WEIGHT_OPTIONS}
			error={autosave.errors.qualityWeight}
			bind:value={() => draft.qualityWeight, (weight) => autosave.edit('qualityWeight', weight)}
		>
			{#snippet icon()}<PreferenceIcon kind="quality" />{/snippet}
		</StepSlider>
		<StepSlider
			name="reliabilityWeight"
			label="Track record"
			options={WEIGHT_OPTIONS}
			hint="Posts on time and passes review."
			error={autosave.errors.reliabilityWeight}
			bind:value={
				() => draft.reliabilityWeight, (weight) => autosave.edit('reliabilityWeight', weight)
			}
		>
			{#snippet icon()}<PreferenceIcon kind="reliability" />{/snippet}
		</StepSlider>
	</FormSection>

	<FormSection title="Timing" description="You pick when bidding closes as you publish.">
		<Field
			name="submissionWindowDays"
			label="Posting window (days)"
			hint="Counted from the bidding deadline, up to {limits.postingWindowDays.max} days."
			error={autosave.errors.submissionWindowDays}
		>
			{#snippet children(control)}
				<!-- min and max only steer the arrow keys; checkEdits decides what is valid. -->
				<input
					{...control}
					type="number"
					inputmode="numeric"
					class="input w-full sm:w-48"
					min="1"
					max={limits.postingWindowDays.max}
					step="1"
					bind:value={
						() => draft.submissionWindowDays, (days) => autosave.edit('submissionWindowDays', days)
					}
				/>
			{/snippet}
		</Field>
	</FormSection>
</form>
