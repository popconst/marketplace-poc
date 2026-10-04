<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { MatchFactor } from '$lib/types';

	/** The match score, opening a card that rates the account in words on each weighed factor. */
	let { score, factors }: { score: number; factors: MatchFactor[] } = $props();

	const HOVER_CLOSE_DELAY_MS = 150;

	const id = $props.id();

	const labels: Record<MatchFactor['kind'], string> = {
		engagement: 'Engagement',
		quality: 'Content quality',
		reliability: 'Track record',
		genre: 'Genre fit'
	};

	/** Always the same order, so that cards side by side compare line by line. */
	const ORDER: MatchFactor['kind'][] = ['genre', 'engagement', 'quality', 'reliability'];

	/** A factor's value, from 0 to 1, as one word per quarter of the scale. */
	function band(value: number): string {
		if (value < 0.25) return 'Low';
		if (value < 0.5) return 'Fair';
		if (value < 0.75) return 'Good';
		return 'Great';
	}

	const weighed = $derived(
		factors
			.filter((factor) => factor.weight > 0)
			.toSorted((a, b) => ORDER.indexOf(a.kind) - ORDER.indexOf(b.kind))
	);

	let card = $state<HTMLDivElement>();
	let closeTimer: ReturnType<typeof setTimeout> | undefined;
	onDestroy(() => clearTimeout(closeTimer));

	// A mouse opens the card by hovering; touch and keyboard use the button's popovertarget.
	function openOnHover(event: PointerEvent) {
		if (event.pointerType !== 'mouse') return;
		clearTimeout(closeTimer);
		card?.togglePopover(true);
	}

	/** Closes after a moment, so the mouse can cross from the button to the card, or back. */
	function closeSoon(event: PointerEvent) {
		if (event.pointerType !== 'mouse') return;
		clearTimeout(closeTimer);
		closeTimer = setTimeout(() => card?.togglePopover(false), HOVER_CLOSE_DELAY_MS);
	}
</script>

{#snippet scoreText()}
	<span class="text-base leading-none font-extrabold text-black tabular-nums">{score}</span> match
{/snippet}

{#if weighed.length === 0}
	<p class="shrink-0 text-sm font-semibold text-neutral-600">{@render scoreText()}</p>
{:else}
	<div class="shrink-0">
		<!-- Shows rather than toggles, so a click while hovering doesn't close it again. -->
		<button
			type="button"
			popovertarget="why-{id}"
			popovertargetaction="show"
			style="anchor-name: --why-{id}"
			onpointerenter={openOnHover}
			onpointerleave={closeSoon}
			aria-label="Match score {score}: why it matches"
			class="inline-flex items-baseline gap-1.5 rounded-full border border-base-300 px-3 py-1 text-sm font-semibold text-neutral-600 transition-colors duration-300 ease-out-expo hover:border-black hover:text-black"
		>
			{@render scoreText()}
			<!-- Lucide (lucide.dev) info, ISC licence. -->
			<svg
				aria-hidden="true"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.25"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="size-3.5 shrink-0 self-center"
			>
				<circle cx="12" cy="12" r="10" />
				<path d="M12 16v-4M12 8h.01" />
			</svg>
		</button>

		<div
			bind:this={card}
			id="why-{id}"
			role="group"
			aria-labelledby="why-{id}-title"
			popover="auto"
			style="position-anchor: --why-{id}"
			onpointerenter={openOnHover}
			onpointerleave={closeSoon}
			class="why dropdown my-2 w-64 rounded-3xl border border-base-300 bg-base-100 p-5 shadow-lift"
		>
			<p id="why-{id}-title" class="mb-3 text-sm font-bold">Why it matches</p>
			<ul class="flex flex-col gap-4">
				{#each weighed as { kind, value } (kind)}
					<li>
						<p class="flex items-baseline justify-between gap-4 text-sm">
							<span class="text-neutral-600">{labels[kind]}<span class="sr-only">:</span></span>
							<span class="font-semibold">{band(value)}</span>
						</p>
						<div aria-hidden="true" class="relative mt-2 h-1.5 rounded-full bg-neutral-200">
							<span class="marker" style:--value={value}></span>
						</div>
					</li>
				{/each}
			</ul>
		</div>
	</div>
{/if}

<style>
	.why {
		position-try-fallbacks: flip-block, flip-inline;
	}

	.marker {
		position: absolute;
		top: 50%;
		left: calc(var(--value) * 100%);
		translate: -50% -50%;
		width: 0.875rem;
		height: 0.875rem;
		border: 2px solid var(--color-base-100);
		border-radius: 9999px;
		background: var(--color-primary);
		box-shadow: 0 2px 6px rgb(254 56 66 / 0.35);
	}
</style>
