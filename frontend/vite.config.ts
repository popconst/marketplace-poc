import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Opt dependencies out: libraries may still ship legacy (non-runes) components.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// nginx serves the fallback page for every route that isn't a file on disk.
			adapter: adapter({ fallback: '200.html' })
		})
	],
	server: {
		// The API shares the app's origin in every environment, so there is no CORS to configure.
		proxy: { '/api': 'http://localhost:3000' }
	}
});
