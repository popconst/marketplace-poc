<script lang="ts">
	import { fade } from 'svelte/transition';
	import AlertIcon from '$lib/components/AlertIcon.svelte';
	import CheckIcon from '$lib/components/CheckIcon.svelte';
	import { formatDateTime } from '$lib/format';
	import type { Timestamp } from '$lib/types';
	import type { SaveStatus } from './autosave.svelte';

	interface Props {
		status: SaveStatus;
		failure: string | null;
		savedAt: Timestamp;
		publishError: string | null;
		onretry: () => void;
		onpublish: () => void;
	}

	let { status, failure, savedAt, publishError, onretry, onpublish }: Props = $props();

	/** The full status, as a tooltip for when the bar truncates it. */
	const message = $derived(
		{
			saving: 'Saving…',
			failed: `Couldn’t save: ${failure}`,
			invalid: 'Changes with errors aren’t saved.',
			saved: `Saved ${formatDateTime(savedAt)}`
		}[status]
	);
</script>

<!-- Fixed to the screen's bottom below lg, lined up with the page's edges; sticky from lg. -->
<div
	class="sticky bottom-4 z-40 flex items-center gap-4 rounded-3xl border border-white/60 bg-white/85 py-3 pr-3 pl-5 shadow-glass backdrop-blur-2xl backdrop-saturate-200 max-lg:fixed max-sm:inset-x-4 sm:max-lg:inset-x-6"
>
	<div class="min-w-0 flex-1 text-sm">
		<!-- Hidden when a refused publish already explains the fields with errors. -->
		{#if status !== 'invalid' || !publishError}
			{#key status}
				<p
					class="flex min-w-0 items-center gap-2 text-neutral-500"
					title={message}
					in:fade={{ duration: 150 }}
				>
					{#if status === 'saving'}
						<span class="loading loading-xs shrink-0 loading-spinner"></span>
						<span class="truncate">Saving…</span>
					{:else if status === 'failed'}
						<AlertIcon class="size-4 text-error" />
						<span role="alert" class="truncate text-error">{message}</span>
						<button type="button" class="btn shrink-0 btn-sm" onclick={onretry}>Retry</button>
					{:else if status === 'invalid'}
						<AlertIcon class="size-4 text-error" />
						<span class="truncate">{message}</span>
					{:else}
						<CheckIcon class="size-4 shrink-0 text-success" />
						<span class="truncate">
							Saved <span class="max-sm:hidden">{formatDateTime(savedAt)}</span>
						</span>
					{/if}
				</p>
			{/key}
		{/if}
		{#if publishError}
			<p role="alert" class="mt-0.5 animate-rise text-error">{publishError}</p>
		{/if}
	</div>
	<button type="button" class="btn btn-neutral" onclick={onpublish}>Publish</button>
</div>
