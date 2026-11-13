import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	server: {
		host: '127.0.0.1',
		port: 5173,
		strictPort: true,
		warmup: {
			clientFiles: [
				'./src/routes/+layout.ts',
				'./src/routes/(app)/+layout.svelte',
				'./src/routes/(app)/+page.svelte',
				'./src/routes/overlay/+page.svelte'
			]
		}
	},
	optimizeDeps: {
		include: [
			'@tauri-apps/api/core',
			'@tauri-apps/api/event',
			'@tauri-apps/api/window',
			'@tauri-apps/api/dpi',
			'@tauri-apps/api/path',
			'@tauri-apps/plugin-store',
			'@tauri-apps/plugin-shell',
			'@tauri-apps/plugin-dialog',
			'@tauri-apps/plugin-updater',
			'@tauri-apps/plugin-process',
			'@tauri-apps/plugin-autostart'
		]
	},
	build: {
		target: 'es2022',
		modulePreload: {
			polyfill: false
		}
	}
});
