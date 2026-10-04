<script lang="ts">
	import { tick } from 'svelte';
	import { getAccount, type ApiResult } from '$lib/api';
	import LoadError from '$lib/components/LoadError.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import WhyItMatches from '$lib/components/WhyItMatches.svelte';
	import { flag, formatCents, formatNumber, optionNames } from '$lib/format';
	import type { Loser, Options, PlatformAccount, Winner } from '$lib/types';
	import AccountStats from './AccountStats.svelte';
	import { lossReasons } from './loss-reasons';

	let { options }: { options: Options } = $props();

	let dialog: HTMLDialogElement;
	let scroller = $state<HTMLDivElement>();
	// Kept after closing, so the drawer keeps its content while it slides out.
	let bid = $state.raw<Winner | Loser | null>(null);
	/** Null while it loads. */
	let account = $state.raw<ApiResult<PlatformAccount> | null>(null);
	/** The button that opened the drawer, which takes focus back when it closes. */
	let opener: HTMLElement | null = null;

	const names = $derived(optionNames(options));

	export async function open(next: Winner | Loser, button: HTMLElement) {
		bid = next;
		opener = button;
		load(next.accountId);
		// Renders this account first, so the drawer never opens on the last one.
		await tick();
		scroller?.scrollTo(0, 0);
		dialog.showModal();
	}

	async function load(accountId: number) {
		account = null;
		const result = await getAccount(fetch, accountId);
		// Dropped if another account was opened while this one loaded.
		if (bid?.accountId === accountId) account = result;
	}
</script>

{#snippet line(label: string, value: string)}
	<div class="flex items-baseline justify-between gap-4">
		<dt class="text-neutral-600">{label}</dt>
		<dd class="font-semibold tabular-nums">{value}</dd>
	</div>
{/snippet}

<!-- A click on the backdrop lands on the dialog itself; clicks inside land on its children. -->
<dialog
	bind:this={dialog}
	aria-labelledby="account-drawer-title"
	onclick={(event) => event.target === dialog && dialog.close()}
	onclose={() => opener?.focus()}
	class="sheet border border-white/60 bg-base-100 p-0 shadow-lift"
>
	{#if bid}
		{@const { accountId } = bid}
		{@const reason = 'lossReason' in bid ? lossReasons[bid.lossReason] : null}
		<div class="flex h-full max-h-[inherit] flex-col">
			<div
				aria-hidden="true"
				class="mx-auto mt-2 h-1.5 w-9 shrink-0 rounded-full bg-neutral-300 sm:hidden"
			></div>
			<header class="flex items-start gap-4 border-b border-base-300 p-6 max-sm:pt-4">
				<div class="min-w-0 flex-1">
					<h2 id="account-drawer-title" class="title-section break-words">@{bid.handle}</h2>
					<p class="mt-1 flex flex-wrap gap-x-3 text-neutral-600">
						<span class="break-words">{bid.creatorName}</span>
						<span class="whitespace-nowrap">
							<span aria-hidden="true">{flag(bid.countryCode)}</span>
							{names.country(bid.countryCode)}
						</span>
					</p>
				</div>
				<button
					type="button"
					class="btn btn-circle btn-ghost btn-sm"
					aria-label="Close"
					onclick={() => dialog.close()}
				>
					<svg aria-hidden="true" viewBox="0 0 16 16" class="size-4">
						<path
							d="M4 4l8 8M12 4l-8 8"
							fill="none"
							stroke="currentColor"
							stroke-width="1.75"
							stroke-linecap="round"
						/>
					</svg>
				</button>
			</header>

			<div
				bind:this={scroller}
				class="flex min-h-0 flex-1 flex-col gap-8 overflow-y-auto overscroll-contain p-6 pb-[max(1.5rem,env(safe-area-inset-bottom))]"
			>
				<section aria-labelledby="account-drawer-bid">
					<div class="flex items-center justify-between gap-4">
						<h3 id="account-drawer-bid" class="text-sm font-semibold text-neutral-500">
							In this campaign
						</h3>
						<StatusPill
							tone={reason ? 'neutral' : 'success'}
							label={reason ? 'Didn’t win' : 'Won'}
						/>
					</div>
					{#if reason}
						<p class="mt-2 text-sm text-neutral-600">
							<span class="font-semibold text-black">{reason.label}.</span>
							{reason.explanation}
						</p>
					{/if}
					<dl class="mt-3 flex flex-col gap-2 rounded-2xl bg-neutral-50 px-5 py-4 text-sm">
						{@render line(reason ? 'Bid' : 'You pay', formatCents(bid.amountCents))}
						{@render line('Avg. views', formatNumber(bid.viewScore))}
						{#if 'minPaidViews' in bid}
							{@render line('Min views to get paid', formatNumber(bid.minPaidViews))}
						{/if}
						{@render line('Cost per 1,000 views', formatCents(bid.expectedCpmCents))}
						<div class="mt-1 flex items-center justify-between gap-4 border-t border-base-300 pt-3">
							<dt class="text-neutral-600">Match</dt>
							<dd><WhyItMatches score={bid.match.score} factors={bid.match.factors} /></dd>
						</div>
					</dl>
				</section>

				<section aria-labelledby="account-drawer-stats" aria-busy={account === null}>
					<h3 id="account-drawer-stats" class="text-sm font-semibold text-neutral-500">
						Account stats
					</h3>
					<div class="mt-3">
						{#if account === null}
							<p role="status" class="sr-only">Loading the account’s stats…</p>
							<div aria-hidden="true" class="grid grid-cols-2 gap-2 sm:grid-cols-3">
								{#each { length: 6 }, i (i)}
									<div class="h-15 skeleton rounded-xl"></div>
								{/each}
							</div>
						{:else if account.ok}
							<AccountStats account={account.data} {options} />
						{:else}
							<LoadError
								compact
								title="Couldn’t load the account"
								error={account.error}
								onretry={() => load(accountId)}
							/>
						{/if}
					</div>
				</section>
			</div>
		</div>
	{/if}
</dialog>
