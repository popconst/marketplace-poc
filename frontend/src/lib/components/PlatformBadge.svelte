<script lang="ts">
	import { platformNames } from '$lib/format';
	import type { Platform } from '$lib/types';
	import PlatformIcon from './PlatformIcon.svelte';

	let { platform }: { platform: Platform } = $props();
</script>

<span class="pill border border-base-300 bg-base-100 align-middle text-neutral-700">
	<PlatformIcon {platform} class="size-3.5" />
	{platformNames[platform]}
</span>
