/** The weights the API accepts for a matching preference. */
export const WEIGHT_OPTIONS = [
	{ value: 0, label: 'Not important' },
	{ value: 25, label: 'Slightly' },
	{ value: 50, label: 'Somewhat' },
	{ value: 75, label: 'Important' },
	{ value: 100, label: 'Very important' }
];

export function weightLabel(value: number): string {
	return WEIGHT_OPTIONS.find((option) => option.value === value)?.label ?? String(value);
}
