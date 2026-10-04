<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { errorMessage, type ApiError } from '$lib/api';
	import AlertIcon from './AlertIcon.svelte';

	interface Props {
		title: string;
		error: ApiError;
		/** For use inside a panel: an inset tile with a small button. */
		compact?: boolean;
		/** What "Try again" does; reloads the page's data by default. */
		onretry?: () => unknown;
	}

	let { title, error, compact = false, onretry = invalidateAll }: Props = $props();
</script>

<div role="alert" class={['animate-rise', compact ? 'empty-state-inset' : 'empty-state']}>
	<AlertIcon class="mx-auto size-8 text-error" />
	<p class={['mt-3 font-bold', compact ? 'text-base' : 'text-lg']}>{title}</p>
	<p class={['mx-auto mt-1 max-w-md text-neutral-600', compact && 'text-sm']}>
		{errorMessage(error)}
	</p>
	<!-- Network and server failures can clear up on their own; a 4xx will not. -->
	{#if error.kind !== 'http' || error.status >= 500}
		<button
			type="button"
			class={['btn mt-6 btn-neutral', compact && 'btn-sm']}
			onclick={() => onretry()}
		>
			Try again
		</button>
	{/if}
</div>
