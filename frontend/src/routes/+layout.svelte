<script lang="ts">
	import '../app.css';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	// From wepush.com, unchanged; the image has its own transparent margin, like the site's header.
	import logo from '$lib/assets/wepush-logo.webp';
	import DebugMenu from '$lib/dev/DebugMenu.svelte';
	import { homePath, identity } from '$lib/dev/identity.svelte';
	import IdentitySwitcher from '$lib/dev/IdentitySwitcher.svelte';

	let { data, children } = $props();

	// The pages of the identity acted as; everything else is in the debug menu.
	const links = $derived.by(() => {
		const current = identity.current;
		if (!current) return [];
		if (current.role === 'advertiser') return [{ href: homePath(current), label: 'My campaigns' }];
		const accountId = String(current.accountId);
		return [
			{ href: resolve('/creator/accounts/[accountId]', { accountId }), label: 'Feed' },
			{ href: resolve('/creator/accounts/[accountId]/bids', { accountId }), label: 'My bids' }
		];
	});

	/** On the link's page or one below it. `/advertiser/12` is not below `/advertiser/1`. */
	function isOn(href: string): boolean {
		const here = page.url.pathname;
		return here === href || here.startsWith(`${href}/`);
	}

	// Links go from general to specific, so the last one the page is on is the current one: on
	// the bids page, My bids rather than Feed.
	const currentHref = $derived(links.filter(({ href }) => isOn(href)).at(-1)?.href);
</script>

<!-- One row at every width: below sm the buttons on the right shrink to icons. The glass box
     anchors the debug menu. -->
<header class="sticky top-4 z-50 container-app">
	<div
		style="anchor-name: --nav"
		class="flex items-center gap-1 rounded-3xl border border-white/60 bg-white/60 px-2 py-2 shadow-glass backdrop-blur-2xl backdrop-saturate-200 sm:gap-2 sm:px-3 sm:py-3"
	>
		<a href={resolve('/')} class="shrink-0 px-0.5 sm:px-2">
			<img src={logo} alt="WePush" width="2048" height="683" class="h-8 w-auto sm:h-10" />
		</a>
		{#if links.length > 0}
			<nav aria-label="Main" class="min-w-0">
				<ul class="flex gap-0.5 sm:gap-1">
					{#each links as { href, label } (href)}
						<li>
							<a {href} aria-current={href === currentHref ? 'page' : undefined} class="nav-link">
								{label}
							</a>
						</li>
					{/each}
				</ul>
			</nav>
		{/if}
		<div class="ml-auto flex shrink-0 items-center gap-2">
			<IdentitySwitcher />
			{#if data.options.devTools}
				<DebugMenu />
			{/if}
		</div>
	</div>
</header>

<main class="container-app pt-8 pb-16 sm:pt-12">
	{@render children()}
</main>
