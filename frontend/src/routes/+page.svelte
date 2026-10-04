<script lang="ts">
	import { resolve } from '$app/paths';
	import { displayName, identity, roleLabels, type Role } from '$lib/dev/identity.svelte';

	// Each leads to the list to pick from: an advertiser, or a creator account to connect as.
	const roles: { role: Role; description: string; href: string }[] = [
		{
			role: 'advertiser',
			description: 'Create campaigns and track the bids they receive.',
			href: resolve('/advertiser')
		},
		{
			role: 'creator',
			description: 'Find campaigns that fit your profile and place bids.',
			href: resolve('/accounts')
		}
	];

	const current = $derived(identity.current);
</script>

<svelte:head>
	<title>WePush</title>
</svelte:head>

<section class="pt-6 sm:pt-16">
	<div class="animate-rise">
		<h1 class="text-4xl font-black tracking-[-0.03em] text-balance sm:text-5xl">
			Who are you acting as?
		</h1>
		<p class="mt-4 lede text-lg">No sign-in in this demo: your choice stays in this browser.</p>
	</div>

	<ul class="mt-10 grid animate-rise gap-4 [--i:1] sm:grid-cols-2 sm:gap-6">
		{#each roles as { role, description, href } (role)}
			<li>
				<a
					{href}
					class={[
						'group flex h-full w-full lift flex-col panel text-left',
						current?.role === role && 'border-black hover:border-black focus-visible:border-black'
					]}
				>
					<h2 class="title-section">{roleLabels[role]}</h2>
					<p class="mt-2 leading-relaxed text-neutral-600">{description}</p>
					<span class="mt-auto flex items-end justify-between gap-4 pt-8">
						<span class="min-w-0 truncate text-sm text-neutral-500">
							{#if current?.role === role}Acting as {displayName(current)}{/if}
						</span>
						<!-- Lucide (lucide.dev) arrow-right, ISC licence. -->
						<span
							aria-hidden="true"
							class="grid size-10 shrink-0 place-items-center rounded-full border border-base-300 transition-colors duration-300 ease-out-expo group-hover:border-black group-hover:bg-black group-hover:text-white group-focus-visible:border-black group-focus-visible:bg-black group-focus-visible:text-white"
						>
							<svg
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
								class="size-4"
							>
								<path d="M5 12h14M12 5l7 7-7 7" />
							</svg>
						</span>
					</span>
				</a>
			</li>
		{/each}
	</ul>
</section>
