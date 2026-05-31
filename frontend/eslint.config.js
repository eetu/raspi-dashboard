import baseConfig from 'eslint-config';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import ts from 'typescript-eslint';

import svelteConfig from './svelte.config.js';

// House TS rules (eslint-config = typescript-eslint node base + import-sort +
// unused-imports + prettier) for .ts, layered with eslint-plugin-svelte for
// .svelte files — the shared config has no Svelte preset, so this is the
// documented pilot delta (see spa-frontend's Svelte section).
export default ts.config(
	...baseConfig,
	...svelte.configs.recommended,
	{
		languageOptions: {
			globals: { ...globals.browser }
		}
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
		languageOptions: {
			parserOptions: {
				// Parse <script lang="ts"> with the TS parser inside the Svelte parser.
				parser: ts.parser,
				extraFileExtensions: ['.svelte'],
				svelteConfig
			}
		}
	},
	{
		ignores: ['dist/', 'build/', '.svelte-kit/', 'node_modules/']
	}
);
