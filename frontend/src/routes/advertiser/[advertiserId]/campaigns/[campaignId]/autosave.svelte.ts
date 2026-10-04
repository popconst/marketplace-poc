import { invalidate } from '$app/navigation';
import { errorMessage, fieldErrors, isConflict, updateCampaign } from '$lib/api';
import type { CampaignLimits } from '$lib/types';
import {
	checkEdits,
	keepEdits,
	toPatch,
	type DraftFields,
	type Edits,
	type FieldErrors
} from './edits';

const SAVE_DELAY_MS = 700;

/** `invalid`: the only unsaved edits have errors, so they wait for a change. */
export type SaveStatus = 'saving' | 'failed' | 'invalid' | 'saved';

/**
 * Saves a draft after a pause in typing, or when focus leaves a field. Saves run one at a time so
 * the server gets the edits in order. An edit with an error waits until it changes; the rest save.
 */
export class DraftAutosave {
	/** Edits the server doesn't have yet. */
	edits = $state.raw<Edits>({});
	/** From `checkEdits` or from the server. */
	errors = $state<FieldErrors>({});
	/** Why the last save failed, when the server named no field. */
	failure = $state<string | null>(null);

	#campaignId: () => number;
	#limits: () => CampaignLimits;
	#timer: ReturnType<typeof setTimeout> | undefined;
	/** Saves queued or under way. */
	#pending = $state(0);
	#queue: Promise<void> = Promise.resolve();

	/** Takes getters, so it always reads the current props. */
	constructor(campaignId: () => number, limits: () => CampaignLimits) {
		this.#campaignId = campaignId;
		this.#limits = limits;
	}

	get status(): SaveStatus {
		if (this.failure) return 'failed';
		if (this.#pending > 0 || this.hasUnsent) return 'saving';
		if (this.hasEdits) return 'invalid';
		return 'saved';
	}

	/** Whether any edit is unsaved, including those waiting on an error. */
	get hasEdits(): boolean {
		return Object.keys(this.edits).length > 0;
	}

	/** Whether an edit without an error is waiting to be sent. */
	get hasUnsent(): boolean {
		return Object.keys(this.edits).some((field) => !(field in this.errors));
	}

	edit<K extends keyof DraftFields>(field: K, value: DraftFields[K]) {
		const next = { ...this.edits };
		next[field] = value;
		this.edits = next;
		delete this.errors[field];
		this.failure = null;
		clearTimeout(this.#timer);
		this.#timer = setTimeout(() => this.save(), SAVE_DELAY_MS);
	}

	flush() {
		if (this.hasUnsent) this.save();
	}

	/** Queues a save behind any under way. Resolves when it is done. */
	save(): Promise<void> {
		clearTimeout(this.#timer);
		this.#pending += 1;
		this.#queue = this.#queue
			.then(() => this.#sendEdits())
			// Keep the queue alive: a rejected queue would skip every later save.
			.catch((error: unknown) => {
				console.error(error);
				this.failure = 'Something unexpected went wrong.';
			})
			.finally(() => (this.#pending -= 1));
		return this.#queue;
	}

	showErrors(fields: Record<string, string>) {
		// The server keys errors by request field, and each control is named after its field.
		this.errors = fields as FieldErrors;
	}

	/** Sends the error-free edits; if the server refuses some fields, resends the rest. */
	async #sendEdits() {
		for (let patch = this.#nextPatch(); Object.keys(patch).length > 0; patch = this.#nextPatch()) {
			this.failure = null;
			const goOn = await this.#send(patch);
			if (!goOn) return;
		}
	}

	#nextPatch(): Edits {
		Object.assign(this.errors, checkEdits(this.edits, this.#limits()));
		return keepEdits(this.edits, (field) => !(field in this.errors));
	}

	/** Resolves to whether the remaining edits can be sent next. */
	async #send(patch: Edits): Promise<boolean> {
		const result = await updateCampaign(fetch, this.#campaignId(), toPatch(patch));
		if (result.ok) {
			// Reload instead of using the response, since the estimate changes too. Reload first,
			// so the saved values are on screen before the edits over them are dropped.
			await invalidate('app:campaign');
			// Keep an edit that changed while it was being saved.
			this.edits = keepEdits(this.edits, (field, value) => value !== patch[field]);
			return true;
		}
		if (isConflict(result.error)) {
			// Published meanwhile from another tab.
			await invalidate('app:campaign');
			return false;
		}
		const fields = fieldErrors(result.error);
		if (fields) {
			// Nothing was saved: the named fields wait for a change, the others can go again.
			Object.assign(this.errors, fields);
			if (Object.keys(fields).some((field) => field in patch)) return true;
		}
		this.failure = errorMessage(result.error);
		return false;
	}
}
