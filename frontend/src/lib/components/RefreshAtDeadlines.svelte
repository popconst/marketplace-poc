<script lang="ts">
	import type { Timestamp } from '$lib/types';

	/**
	 * Calls `refresh` just after the soonest bidding deadline, then again until the page has moved
	 * on, so it shows the close as soon as the worker has made it. Pass the deadlines of campaigns
	 * that are not closed yet. Renders nothing.
	 */
	let { deadlines, refresh }: { deadlines: Timestamp[]; refresh: () => void } = $props();

	/** The worker closes a campaign within milliseconds of its deadline. */
	const AFTER_DEADLINE_MS = 500;
	/** Looks again every second at first, as a clock a bit off makes the first look early. */
	const QUICK_RETRY_MS = 1_000;
	const QUICK_RETRIES_FOR_MS = 10_000;
	/** Then every five seconds, while the worker is late or a close has failed. */
	const SLOW_RETRY_MS = 5_000;
	/** setTimeout can't wait much beyond 24 days, and a page is rarely open for a whole day. */
	const MAX_WAIT_MS = 24 * 60 * 60 * 1000;

	function waitMs(sinceDeadline: number): number {
		if (sinceDeadline < 0) return -sinceDeadline + AFTER_DEADLINE_MS;
		return sinceDeadline < QUICK_RETRIES_FOR_MS ? QUICK_RETRY_MS : SLOW_RETRY_MS;
	}

	// Runs again, from scratch, whenever the deadlines change: once the campaign shows as closed,
	// the page passes it no more, or removes this component.
	$effect(() => {
		if (deadlines.length === 0) return;
		const soonest = Math.min(...deadlines.map((deadline) => new Date(deadline).getTime()));
		let timer: ReturnType<typeof setTimeout> | undefined;
		function schedule() {
			const wait = waitMs(Date.now() - soonest);
			if (wait > MAX_WAIT_MS) return;
			timer = setTimeout(() => {
				refresh();
				schedule();
			}, wait);
		}
		schedule();
		return () => clearTimeout(timer);
	});
</script>
