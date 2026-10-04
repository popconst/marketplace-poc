import { formatCents } from '$lib/format';
import type { Bounds, CampaignFields, CampaignLimits, CampaignPatch } from '$lib/types';

/** The editable fields as the controls hold them: the posting window is null while empty. */
export type DraftFields = Omit<CampaignFields, 'submissionWindowDays'> & {
	submissionWindowDays: number | null;
};

export type Edits = Partial<DraftFields>;

export type FieldErrors = Partial<Record<keyof DraftFields, string>>;

/**
 * Catches mistakes before a round trip, worded as the server words them; the server checks again.
 * Also flags an empty posting window, which can't be sent.
 */
export function checkEdits(edits: Edits, limits: CampaignLimits): FieldErrors {
	const errors: FieldErrors = {};
	// Null clears the field, which a draft may do.
	if (typeof edits.budgetCents === 'number' && !within(edits.budgetCents, limits.budgetCents)) {
		errors.budgetCents = betweenEuros(limits.budgetCents);
	}
	if (
		typeof edits.targetCpmCents === 'number' &&
		!within(edits.targetCpmCents, limits.targetCpmCents)
	) {
		errors.targetCpmCents = betweenEuros(limits.targetCpmCents);
	}
	if ('submissionWindowDays' in edits) {
		const days = edits.submissionWindowDays;
		const max = limits.postingWindowDays.max;
		if (typeof days !== 'number' || !Number.isInteger(days) || !within(days, { min: 1, max })) {
			errors.submissionWindowDays = `Must be between 1 and ${max} days.`;
		}
	}
	return errors;
}

function within(value: number, { min, max }: Bounds): boolean {
	return min <= value && value <= max;
}

/** `Must be between €360 and €10,000,000.`, as `between_euros` in campaigns.rs words it. */
function betweenEuros({ min, max }: Bounds): string {
	return `Must be between ${formatCents(min)} and ${formatCents(max)}.`;
}

/** An empty posting window never gets here: `checkEdits` gives it an error. */
export function toPatch(edits: Edits): CampaignPatch {
	const { submissionWindowDays, ...rest } = edits;
	return typeof submissionWindowDays === 'number' ? { ...rest, submissionWindowDays } : rest;
}

export function keepEdits(
	edits: Edits,
	keep: (field: keyof Edits, value: unknown) => boolean
): Edits {
	return Object.fromEntries(
		Object.entries(edits).filter(([field, value]) => keep(field as keyof Edits, value))
	);
}
