import { skeleton } from '@skeletonlabs/skeleton/plugin';
import * as themes from '@skeletonlabs/skeleton/themes';

/** @type {import('tailwindcss').Config} */
export default {
	content: [
		'./src/**/*.{html,js,svelte,ts}',
		'./node_modules/@skeletonlabs/skeleton/**/*.{html,js,svelte,ts}'
	],
	theme: {
		extend: {},
	},
	plugins: [
		skeleton({
			// Use the 'wintry' theme as a base for macOS-like aesthetics
			themes: [themes.wintry, themes.cerberus]
		})
	]
}



