import { getOptions, unwrap } from '$lib/api';
import type { LayoutLoad } from './$types';

// Single-page app: every route renders in the browser, served by the adapter's fallback page.
export const ssr = false;

// Every page reads this as `data.options`. Nothing can render without it, so a failure shows the
// static page in src/error.html.
export const load: LayoutLoad = async ({ fetch }) => ({
	options: unwrap(await getOptions(fetch))
});
