<script lang="ts">
	import type { ClassValue } from 'svelte/elements';
	import { flag } from '$lib/format';
	import type { Winner } from '$lib/types';

	interface Props {
		bid: Pick<Winner, 'handle' | 'creatorName' | 'countryCode'>;
		/** Opens the account's details. Gets the button, to take focus back when they close. */
		onopen: (button: HTMLButtonElement) => void;
		class?: ClassValue;
	}

	let { bid, onopen, class: className }: Props = $props();
</script>

<!-- Both lines truncate, so a long name can't push a table past its panel. -->
<button
	type="button"
	aria-haspopup="dialog"
	onclick={(event) => onopen(event.currentTarget)}
	class={['group block max-w-full min-w-0 cursor-pointer rounded-md text-left', className]}
>
	<span
		title="@{bid.handle}"
		class="block truncate font-semibold underline decoration-transparent underline-offset-2 transition-[text-decoration-color] duration-300 ease-out-expo group-hover:decoration-current"
	>
		@{bid.handle}
	</span>
	<span class="mt-0.5 block truncate text-sm text-neutral-500"
		>{bid.creatorName} {flag(bid.countryCode)}</span
	>
</button>
