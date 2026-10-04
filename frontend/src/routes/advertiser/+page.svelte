<script lang="ts">
	import { resolve } from '$app/paths';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import { identity } from '$lib/dev/identity.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>Advertiser · WePush</title>
</svelte:head>

<header class="animate-rise">
	<h1 class="title-page">Which advertiser are you?</h1>
	<p class="mt-2 lede">Pick the brand you’re acting as to see and create its campaigns.</p>
</header>

{#if data.advertisers.length === 0}
	<EmptyState
		class="mt-8 animate-rise [--i:1]"
		title="No advertisers yet"
		message="Seed the database to get a few, then reload this page."
	/>
{:else}
	<ul class="mt-8 grid animate-rise gap-4 [--i:1] sm:grid-cols-2 sm:gap-6 lg:grid-cols-3">
		{#each data.advertisers as advertiser (advertiser.id)}
			<li>
				<a
					href={resolve('/advertiser/[advertiserId]', { advertiserId: String(advertiser.id) })}
					aria-label="{advertiser.name} campaigns"
					onclick={() =>
						identity.choose({
							role: 'advertiser',
							advertiserId: advertiser.id,
							name: advertiser.name
						})}
					class="group flex h-full lift items-center justify-between gap-4 surface"
				>
					<h2 class="truncate title-card">{advertiser.name}</h2>
					<span
						aria-hidden="true"
						class="text-neutral-400 transition-[translate,color] duration-500 ease-out-expo group-hover:translate-x-1 group-hover:text-black group-focus-visible:translate-x-1 group-focus-visible:text-black"
					>
						→
					</span>
				</a>
			</li>
		{/each}
	</ul>
{/if}
