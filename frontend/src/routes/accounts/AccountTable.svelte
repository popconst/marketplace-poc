<script lang="ts">
	import { MediaQuery } from 'svelte/reactivity';
	import PlatformBadge from '$lib/components/PlatformBadge.svelte';
	import PlatformIcon from '$lib/components/PlatformIcon.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import ConnectAsButton from '$lib/dev/ConnectAsButton.svelte';
	import { identity } from '$lib/dev/identity.svelte';
	import { flag, optionNames, platformNames } from '$lib/format';
	import type { AccountSort, Options, PlatformAccount } from '$lib/types';

	interface Props {
		/** One list per page loaded, so a page added with "Load more" rises in on its own. */
		pages: PlatformAccount[][];
		skeletonRows?: number;
		sort: AccountSort;
		options: Options;
	}

	let { pages, skeletonRows = 0, sort, options }: Props = $props();

	// Narrower than this, the columns don't fit and the table becomes a list of cards.
	const wide = new MediaQuery('min-width: 80rem');

	// Demo only: highlights the account the demo identity acts as.
	const connectedId = $derived(
		identity.current?.role === 'creator' ? identity.current.accountId : null
	);

	const names = $derived(optionNames(options));
	const genreList = (ids: number[]) => ids.map(names.genre).join(', ');
	const languageList = (codes: string[]) => codes.map(names.language).join(', ');

	// Always one decimal (48.0K, not 48K) so a column's figures line up. Locale as in $lib/format.
	const oneDecimal = { minimumFractionDigits: 1, maximumFractionDigits: 1 };
	const compact = new Intl.NumberFormat('en-IE', { notation: 'compact', ...oneDecimal });
	const percent = new Intl.NumberFormat('en-IE', { style: 'percent', ...oneDecimal });

	const formatFigure = (n: number) => (n < 1000 ? String(n) : compact.format(n));
	const formatRate = (ratio: number) => percent.format(ratio);

	// Each width fits its header (which may wrap), sort arrow and widest value, so the table fits
	// from 1280px without scrolling. The genres and languages column takes the rest.
	const columns: {
		label: string;
		sort?: AccountSort;
		numeric?: boolean;
		hidden?: boolean;
		width?: string;
	}[] = [
		{ label: 'Account', width: 'w-56' },
		{ label: 'Followers', sort: 'followers', numeric: true, width: 'w-25' },
		{ label: 'Views per post', sort: 'view_score', numeric: true, width: 'w-22' },
		{ label: 'Engagement', sort: 'engagement_rate', numeric: true, width: 'w-31' },
		{ label: 'Posts per week', numeric: true, width: 'w-22' },
		{ label: 'Quality', sort: 'quality_score', width: 'w-25' },
		{ label: 'Brand safe', width: 'w-25' },
		{ label: 'Genres and languages' },
		// Demo only, so it goes last, after the stats an account is picked by.
		{ label: 'Connect as', hidden: true, width: 'w-32' }
	];
</script>

{#snippet country(code: string)}
	<span class="shrink-0 whitespace-nowrap" title={names.country(code)}>
		<span aria-hidden="true">{flag(code)}</span>
		{code}
	</span>
{/snippet}

{#snippet quality(score: number)}
	<span class="flex items-center gap-2">
		<span
			aria-hidden="true"
			class="h-1.5 w-12 shrink-0 overflow-hidden rounded-full bg-neutral-200"
		>
			<span class="block h-full rounded-full bg-black" style:width="{score}%"></span>
		</span>
		<span class="w-7 text-right tabular-nums">{score}</span>
	</span>
{/snippet}

<!-- Safe is the norm, so it stays quiet; only the exception is a status pill. -->
{#snippet brandSafe(safe: boolean)}
	{#if safe}
		<span class="inline-flex items-center gap-1.5 text-xs whitespace-nowrap text-neutral-500">
			<span aria-hidden="true" class="size-1.5 rounded-full bg-success"></span>
			Safe
		</span>
	{:else}
		<StatusPill tone="error" label="Not safe" />
	{/if}
{/snippet}

{#snippet twoLineSkeleton(first: string, second: string)}
	<div class="flex h-10 flex-col justify-center gap-1.5">
		<div class={['h-4 skeleton', first]}></div>
		<div class={['h-3 skeleton', second]}></div>
	</div>
{/snippet}

{#if wide.current}
	<div class="overflow-clip rounded-box border border-base-300 bg-base-100">
		<!-- Tight cells so the columns fit; the outer ones line up with the filter panel's padding. -->
		<table
			class="table table-fixed text-sm [&_:is(th,td)]:px-2 [&_:is(th,td):first-child]:pl-6 [&_:is(th,td):last-child]:pr-6"
		>
			<caption class="sr-only">Creator accounts</caption>
			<colgroup>
				{#each columns as column (column.label)}
					<col class={column.width} />
				{/each}
			</colgroup>
			<thead>
				<tr>
					{#each columns as column (column.label)}
						<th
							scope="col"
							aria-sort={column.sort === sort ? 'descending' : undefined}
							class={[
								'sticky top-24 z-10 bg-base-100/90 align-bottom text-sm font-semibold whitespace-normal text-neutral-500 backdrop-blur',
								column.numeric && 'text-right',
								column.sort === sort && 'text-black'
							]}
						>
							<span class={[column.hidden && 'sr-only']}>{column.label}</span
							>{#if column.sort === sort}<span aria-hidden="true">&nbsp;↓</span>{/if}
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each pages as items, pageIndex (pageIndex)}
					{#each items as account (account.id)}
						<tr
							class={[
								'transition-colors duration-300 ease-out-expo hover:bg-neutral-50',
								account.id === connectedId && 'bg-neutral-50',
								pageIndex > 0 && 'animate-rise'
							]}
						>
							<td>
								<span class="block truncate font-semibold" title="@{account.handle}">
									@{account.handle}
								</span>
								<span class="flex items-center gap-1.5 text-neutral-500">
									<!-- The icon, not the badge: the badge leaves no room for the name. -->
									<span class="flex shrink-0" title={platformNames[account.platform]}>
										<PlatformIcon platform={account.platform} class="size-3.5" />
										<span class="sr-only">{platformNames[account.platform]}</span>
									</span>
									<span class="min-w-0 truncate" title={account.creatorName}>
										{account.creatorName}
									</span>
									{@render country(account.countryCode)}
								</span>
							</td>
							<td class="text-right tabular-nums">{formatFigure(account.followers)}</td>
							<td class="text-right tabular-nums">{formatFigure(account.viewScore)}</td>
							<td class="text-right tabular-nums">{formatRate(account.engagementRate)}</td>
							<td class="text-right tabular-nums">{account.postsPerWeek.toFixed(1)}</td>
							<td>{@render quality(account.qualityScore)}</td>
							<td>{@render brandSafe(account.brandSafe)}</td>
							<td>
								<span class="block truncate" title={genreList(account.genreIds)}>
									{genreList(account.genreIds)}
								</span>
								<span
									class="block truncate text-neutral-500"
									title={languageList(account.languageCodes)}
								>
									{languageList(account.languageCodes)}
								</span>
							</td>
							<td class="text-right"><ConnectAsButton {account} /></td>
						</tr>
					{/each}
				{/each}
				{#each { length: skeletonRows }, i (i)}
					<tr aria-hidden="true">
						<td>{@render twoLineSkeleton('w-40', 'w-28')}</td>
						{#each { length: 4 }, cell (cell)}
							<td><div class="ml-auto h-4 w-12 skeleton"></div></td>
						{/each}
						<td><div class="h-4 w-20 skeleton"></div></td>
						<td><div class="h-4 w-12 skeleton"></div></td>
						<td>{@render twoLineSkeleton('w-32', 'w-20')}</td>
						<td><div class="ml-auto h-8 w-24 skeleton"></div></td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{:else}
	<ul class="grid grid-cols-1 gap-4 sm:gap-6 md:grid-cols-2">
		{#each pages as items, pageIndex (pageIndex)}
			{#each items as account (account.id)}
				<li class={['flex flex-col surface', pageIndex > 0 && 'animate-rise']}>
					<div class="flex items-baseline justify-between gap-3">
						<div class="min-w-0">
							<p class="truncate title-card" title="@{account.handle}">@{account.handle}</p>
							<p class="mt-1 flex items-center gap-2 text-sm text-neutral-500">
								<PlatformBadge platform={account.platform} />
								<span class="min-w-0 truncate" title={account.creatorName}>
									{account.creatorName}
								</span>
								{@render country(account.countryCode)}
							</p>
						</div>
						{@render brandSafe(account.brandSafe)}
					</div>
					<dl class="mt-5 grid grid-cols-3 gap-4">
						<div>
							<dt class="text-xs text-neutral-500">Followers</dt>
							<dd class="text-base font-bold tabular-nums">{formatFigure(account.followers)}</dd>
						</div>
						<div>
							<dt class="text-xs text-neutral-500">Views per post</dt>
							<dd class="text-base font-bold tabular-nums">{formatFigure(account.viewScore)}</dd>
						</div>
						<div>
							<dt class="text-xs text-neutral-500">Engagement</dt>
							<dd class="text-base font-bold tabular-nums">
								{formatRate(account.engagementRate)}
							</dd>
						</div>
						<div>
							<dt class="text-xs text-neutral-500">Posts per week</dt>
							<dd class="text-base font-bold tabular-nums">{account.postsPerWeek.toFixed(1)}</dd>
						</div>
						<div>
							<dt class="text-xs text-neutral-500">Quality</dt>
							<dd class="text-base font-bold">{@render quality(account.qualityScore)}</dd>
						</div>
						<div>
							<dt class="text-xs text-neutral-500">Languages</dt>
							<dd class="text-base font-bold" title={languageList(account.languageCodes)}>
								{account.languageCodes.map((code) => code.toUpperCase()).join(', ')}
							</dd>
						</div>
					</dl>
					<!-- The genres wrap above the button rather than squeezing it. -->
					<div class="mt-auto flex flex-wrap items-center justify-end gap-3 pt-5">
						<span class="mr-auto flex flex-wrap gap-2">
							{#each account.genreIds as id (id)}
								<span class="pill border border-base-300 bg-base-100 text-neutral-700">
									{names.genre(id)}
								</span>
							{/each}
						</span>
						<ConnectAsButton {account} />
					</div>
				</li>
			{/each}
		{/each}
		{#each { length: skeletonRows }, i (i)}
			<li aria-hidden="true" class="flex flex-col surface">
				<div class="h-6 w-44 skeleton"></div>
				<div class="mt-1 h-6 w-56 skeleton"></div>
				<div class="mt-5 grid grid-cols-3 gap-4">
					{#each { length: 6 }, cell (cell)}
						<div>
							<div class="h-3 w-16 skeleton"></div>
							<div class="mt-2 h-5 w-12 skeleton"></div>
						</div>
					{/each}
				</div>
				<div class="mt-auto flex items-center justify-between gap-3 pt-5">
					<div class="flex gap-2">
						<div class="h-6 w-16 skeleton"></div>
						<div class="h-6 w-20 skeleton"></div>
					</div>
					<div class="h-8 w-24 skeleton"></div>
				</div>
			</li>
		{/each}
	</ul>
{/if}
