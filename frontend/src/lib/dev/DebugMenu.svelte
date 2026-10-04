<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { closeCampaignSoon, errorMessage, simulateBids } from '$lib/api';
	import { formatCount, formatNumber, formatTime } from '$lib/format';
	import type { Campaign } from '$lib/types';
	import CloseCampaignButton from './CloseCampaignButton.svelte';

	// Pages for looking around as anyone, outside the chosen identity.
	const links = [
		{ href: resolve('/advertiser'), label: 'All advertisers' },
		{ href: resolve('/accounts'), label: 'All creator accounts' }
	];

	// The campaign page loads its campaign into the page data; an active one can be closed early.
	const campaign: Campaign | undefined = $derived(page.data['campaign']);
	const closable = $derived(campaign?.status === 'active' ? campaign : null);

	let menu: HTMLDivElement;
	let busy = $state(false);
	let outcome = $state<{ ok: boolean; message: string } | null>(null);
	let closingSoon = $state(false);
	let closeSoonOutcome = $state<{ ok: boolean; message: string } | null>(null);

	async function simulate() {
		busy = true;
		outcome = null;
		const result = await simulateBids(fetch);
		if (result.ok) {
			const { campaigns, placed, skipped } = result.data;
			await invalidateAll();
			const bids = `${formatCount(placed, 'bid')} on ${formatCount(campaigns, 'campaign')}`;
			outcome = { ok: true, message: `Bots placed ${bids} (${formatNumber(skipped)} skipped).` };
		} else {
			outcome = { ok: false, message: errorMessage(result.error) };
		}
		busy = false;
	}

	/** Moves the deadline 10 seconds ahead; the worker then closes the campaign as at any deadline. */
	async function closeSoon(campaignId: number) {
		closingSoon = true;
		closeSoonOutcome = null;
		const result = await closeCampaignSoon(fetch, campaignId);
		if (result.ok) {
			// Reloaded, so the page counts down to the new deadline and refreshes when it passes.
			await invalidateAll();
			const at = formatTime(new Date(result.data.biddingDeadline));
			closeSoonOutcome = { ok: true, message: `Closes at ${at}.` };
		} else {
			closeSoonOutcome = { ok: false, message: errorMessage(result.error) };
		}
		closingSoon = false;
	}
</script>

<!-- A popover closes on Escape and on an outside click by itself. -->
<button
	type="button"
	popovertarget="debug-menu"
	class="grid place-items-center rounded-full border border-dashed border-neutral-300 text-xs font-semibold text-neutral-500 transition-colors duration-300 ease-out-expo hover:border-black hover:text-black max-sm:size-10.5 sm:h-8 sm:px-3"
>
	<!-- Lucide (lucide.dev) bug, ISC licence. -->
	<svg
		aria-hidden="true"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
		class="size-4 sm:hidden"
	>
		<path d="m8 2 1.88 1.88" />
		<path d="M14.12 3.88 16 2" />
		<path d="M9 7.13v-1a3.003 3.003 0 1 1 6 0v1" />
		<path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6" />
		<path d="M12 20v-9" />
		<path d="M6.53 9C4.6 8.8 3 7.1 3 5" />
		<path d="M6 13H2" />
		<path d="M3 21c0-2.1 1.7-3.9 3.8-4" />
		<path d="M20.97 5c0 2.1-1.6 3.8-3.5 4" />
		<path d="M22 13h-4" />
		<path d="M17.2 17c2.1.1 3.8 1.9 3.8 4" />
	</svg>
	<span class="max-sm:sr-only">Debug</span>
</button>

<!-- Hangs below the nav box (`--nav`, set in the layout), flush with its right edge. -->
<div
	bind:this={menu}
	id="debug-menu"
	popover="auto"
	style="position-anchor: --nav"
	class="dropdown dropdown-end mt-2 w-72 rounded-3xl border border-base-300 bg-base-100 p-5 shadow-lift"
>
	<span class="tag">Demo</span>
	<nav aria-label="Debug" class="mt-4">
		<ul class="flex flex-col gap-2">
			{#each links as { href, label } (href)}
				<li>
					<!-- Client-side navigation leaves the popover open, so close it here. -->
					<a
						{href}
						onclick={() => menu.hidePopover()}
						class="group flex items-center justify-between rounded-2xl border border-dashed border-neutral-300 px-4 py-2.5 text-sm font-medium text-neutral-600 transition-colors duration-300 ease-out-expo hover:border-black hover:text-black"
					>
						{label}
						<span
							aria-hidden="true"
							class="transition-transform duration-500 ease-out-expo group-hover:translate-x-1"
						>
							→
						</span>
					</a>
				</li>
			{/each}
		</ul>
	</nav>
	<p class="mt-5 border-t border-dashed border-neutral-300 pt-5 text-sm text-neutral-600">
		Simulated creators bid on every open campaign, by the same rules as real ones.
	</p>
	<button
		type="button"
		class="btn mt-4 w-full btn-neutral btn-sm"
		disabled={busy}
		onclick={simulate}
	>
		{#if busy}
			<span class="loading loading-xs loading-spinner"></span> Simulating…
		{:else}
			Simulate creator bids
		{/if}
	</button>
	{#if outcome}
		<p
			role="status"
			class={['mt-3 animate-rise text-sm', outcome.ok ? 'text-neutral-600' : 'text-error']}
		>
			{outcome.message}
		</p>
	{/if}
	{#if closable}
		<p class="mt-5 border-t border-dashed border-neutral-300 pt-5 text-sm text-neutral-600">
			Closes this campaign and picks its winners now, before the deadline.
		</p>
		<!-- Keyed, so a half-confirmed close never carries over to another campaign. -->
		{#key closable.id}
			<CloseCampaignButton campaignId={closable.id} />
		{/key}
		<button
			type="button"
			class="btn mt-2 w-full btn-sm"
			disabled={closingSoon}
			onclick={() => closeSoon(closable.id)}
		>
			{#if closingSoon}<span class="loading loading-xs loading-spinner"></span>{/if}
			Close in 10 seconds
		</button>
		{#if closeSoonOutcome}
			<p
				role="status"
				class={[
					'mt-3 animate-rise text-sm',
					closeSoonOutcome.ok ? 'text-neutral-600' : 'text-error'
				]}
			>
				{closeSoonOutcome.message}
			</p>
		{/if}
	{/if}
</div>
