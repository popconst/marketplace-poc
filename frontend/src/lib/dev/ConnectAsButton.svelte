<script lang="ts">
	import { goto } from '$app/navigation';
	import type { PlatformAccount } from '$lib/types';
	import { creatorIdentity, homePath, identity } from './identity.svelte';

	/** Demo only: acts as this account and opens its feed. */
	let { account }: { account: PlatformAccount } = $props();

	const connected = $derived(
		identity.current?.role === 'creator' && identity.current.accountId === account.id
	);

	async function connect() {
		const next = creatorIdentity(account);
		identity.choose(next);
		await goto(homePath(next));
	}
</script>

<!-- Dashed like the other demo tools; solid once connected. -->
<button
	type="button"
	class={[
		'btn whitespace-nowrap btn-sm',
		connected ? 'btn-neutral' : 'border-dashed border-neutral-300 btn-ghost hover:border-black'
	]}
	aria-label="Connect as @{account.handle}"
	onclick={connect}
>
	{connected ? 'Connected' : 'Connect as'}
</button>
