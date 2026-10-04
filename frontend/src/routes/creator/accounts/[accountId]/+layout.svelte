<script lang="ts">
	import PlatformBadge from '$lib/components/PlatformBadge.svelte';

	let { data, children } = $props();
</script>

<!-- Shared by the feed and the bids, so switching between them moves nothing above the content. -->
<header class="animate-rise">
	<h1 class="title-page">@{data.account.handle}</h1>
	<p class="mt-2 flex flex-wrap items-center gap-2 text-neutral-600">
		<span class="break-words">{data.account.creatorName}</span>
		<PlatformBadge platform={data.account.platform} />
	</p>
</header>

{@render children()}
