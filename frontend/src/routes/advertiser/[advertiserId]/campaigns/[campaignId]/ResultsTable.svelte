<script lang="ts" module>
	export interface Column {
		label: string;
		/** Right-aligned, so the figures line up. */
		numeric?: boolean;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		caption: string;
		columns: Column[];
		/** The `<tbody>` elements. */
		children: Snippet;
	}

	let { caption, columns, children }: Props = $props();
</script>

<!-- The outer cells lose their outer padding, so the table lines up with the panel's text. -->
<table
	class="table text-sm [&_:is(th,td)]:px-1.5 [&_:is(th,td):first-child]:pl-0 [&_:is(th,td):last-child]:pr-0"
>
	<caption class="sr-only">{caption}</caption>
	<thead>
		<tr>
			{#each columns as column (column.label)}
				<th
					scope="col"
					class={[
						'border-b-0 text-sm font-semibold whitespace-normal text-neutral-500',
						column.numeric && 'text-right'
					]}
				>
					{column.label}
				</th>
			{/each}
		</tr>
	</thead>
	{@render children()}
</table>
