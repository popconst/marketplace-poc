<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';

	const notFound = $derived(page.status === 404);
	// Only a server error can clear up on its own.
	const retryable = $derived(page.status >= 500);
	// SvelteKit's own message for a missing page says no more than the heading.
	const message = $derived(page.error?.message === 'Not Found' ? null : page.error?.message);
</script>

<svelte:head>
	<title>{notFound ? 'Not found' : 'Error'} · WePush</title>
</svelte:head>

<section class="mx-auto max-w-xl animate-rise py-16 text-center">
	<h1 class="title-page">{notFound ? 'Nothing here' : 'Something went wrong'}</h1>
	<p class="mt-2 text-sm text-neutral-500 tabular-nums">Error {page.status}</p>
	{#if message}
		<p class="mt-3 text-pretty text-neutral-600">{message}</p>
	{/if}
	<div class="mt-8 flex justify-center gap-3">
		{#if retryable}
			<button type="button" class="btn btn-neutral" onclick={() => invalidateAll()}>
				Try again
			</button>
		{/if}
		<a href={resolve('/')} class={['btn', !retryable && 'btn-neutral']}>Home</a>
	</div>
</section>
