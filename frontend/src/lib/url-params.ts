// Parsers for URL values, which may have been edited by hand. Each returns null for anything the
// API would reject, so the page drops that value rather than fail on it.

/** A whole number above 0, such as an id, or null. */
export function positiveInt(value: string | null): number | null {
	const n = Number(value);
	return Number.isSafeInteger(n) && n > 0 ? n : null;
}

/** A number from 0 to 1, such as a rate, or null. */
export function ratio(value: string | null): number | null {
	// `Number('')` is 0, which would pass.
	if (!value) return null;
	const n = Number(value);
	return n >= 0 && n <= 1 ? n : null;
}
