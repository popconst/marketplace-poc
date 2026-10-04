<script lang="ts">
	import type { Snippet } from 'svelte';
	import FieldMessage from './FieldMessage.svelte';

	interface Props {
		/** Id and name of the control. */
		name: string;
		label: string;
		icon?: Snippet | undefined;
		/** The control's current value in words, shown at the end of the label row. */
		value?: string | undefined;
		hint?: string | undefined;
		error?: string | undefined;
		/** Renders the control, spreading the attributes that link it to the label and message. */
		children: Snippet<
			[
				{
					id: string;
					name: string;
					'aria-invalid': 'true' | undefined;
					'aria-describedby': string | undefined;
				}
			]
		>;
	}

	let { name, label, icon, value, hint, error, children }: Props = $props();

	const messageId = $derived(`${name}-message`);
</script>

<div class="flex flex-col gap-2" data-invalid={error ? '' : undefined}>
	<div class="flex items-baseline justify-between gap-4">
		<label for={name} class="text-sm font-semibold">
			{#if icon}
				<span
					class="mr-1.5 inline-grid size-7 place-items-center rounded-full bg-neutral-100 align-middle text-black"
				>
					{@render icon()}
				</span>
			{/if}
			{label}
		</label>
		<!-- The control announces its own value; this is for the eye. -->
		{#if value}
			<span aria-hidden="true" class="text-sm text-neutral-600 tabular-nums">{value}</span>
		{/if}
	</div>
	{@render children({
		id: name,
		name,
		'aria-invalid': error ? 'true' : undefined,
		'aria-describedby': error || hint ? messageId : undefined
	})}
	<FieldMessage id={messageId} {error} {hint} />
</div>
