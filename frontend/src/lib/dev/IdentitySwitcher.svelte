<script lang="ts">
	import { resolve } from '$app/paths';
	import { displayName, identity, roleLabels } from './identity.svelte';

	const current = $derived(identity.current);

	function initials(name: string): string {
		return name
			.split(/\s+/)
			.slice(0, 2)
			.map((word) => word.charAt(0))
			.join('')
			.toUpperCase();
	}
</script>

<!-- The home page is where you pick who you act as. -->
{#if current}
	{@const roleLabel = roleLabels[current.role]}
	<a
		href={resolve('/')}
		aria-label="{roleLabel} {displayName(current)}: change who you are acting as"
		class="flex items-center gap-2.5 rounded-full border border-base-300 bg-white text-sm transition-colors duration-300 ease-out-expo hover:border-black max-sm:p-1 sm:h-10 sm:py-1 sm:pr-4 sm:pl-1"
	>
		<span
			aria-hidden="true"
			class="grid size-8 shrink-0 place-items-center rounded-full bg-black text-xs font-bold text-white"
		>
			{initials(current.name)}
		</span>
		<span class="flex min-w-0 flex-col items-start leading-tight max-sm:hidden">
			<span class="text-xs text-neutral-500">{roleLabel}</span>
			<span class="max-w-56 truncate font-semibold">{displayName(current)}</span>
		</span>
	</a>
{:else}
	<a
		href={resolve('/')}
		class="flex items-center gap-2.5 rounded-full border border-base-300 bg-white text-sm font-semibold transition-colors duration-300 ease-out-expo hover:border-black max-sm:p-1 sm:h-10 sm:py-1 sm:pr-4 sm:pl-1"
	>
		<!-- An empty avatar: Lucide (lucide.dev) user, ISC licence. -->
		<span
			aria-hidden="true"
			class="grid size-8 shrink-0 place-items-center rounded-full bg-neutral-100 text-neutral-600"
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
				<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" />
				<circle cx="12" cy="7" r="4" />
			</svg>
		</span>
		<span class="max-sm:sr-only">Choose who you are</span>
	</a>
{/if}
