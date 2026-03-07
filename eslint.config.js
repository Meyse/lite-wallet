// ESLint configuration for lite-wallet crypto project
// Security-focused linting rules for cryptocurrency wallet development
// Enforces patterns defined in Cursor Rules for safe crypto development

import js from '@eslint/js';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';

export default [
  js.configs.recommended,

  // TypeScript files
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        project: './tsconfig.json',
        extraFileExtensions: ['.svelte'],
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/ban-ts-comment': [
        'error',
        {
          'ts-expect-error': 'allow-with-description',
        },
      ],
      '@typescript-eslint/no-non-null-assertion': 'error',
    },
  },

  // All JavaScript/TypeScript files
  {
    files: ['**/*.{js,mjs,cjs,ts,tsx,svelte}'],
    languageOptions: {
      globals: {
        // Browser globals
        console: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        document: 'readonly',
        window: 'readonly',

        // Node.js globals
        process: 'readonly',
        Buffer: 'readonly',

        // Tauri globals
        __TAURI__: 'readonly',
        __TAURI_METADATA__: 'readonly',

        // Svelte 5 rune globals used in .svelte.ts helpers
        $state: 'readonly',
        $derived: 'readonly',
        $effect: 'readonly',
        $props: 'readonly',
        $bindable: 'readonly',
        $inspect: 'readonly',
        $host: 'readonly',
      },
    },
    rules: {
      // Security: Prevent console.log with sensitive data
      'no-console': [
        'warn',
        {
          allow: ['warn', 'error', 'info'],
        },
      ],

      // Code quality: Prefer const for immutability (disabled for Svelte files)
      'prefer-const': 'error',

      // Security: No eval (code injection prevention)
      'no-eval': 'error',
      'no-implied-eval': 'error',

      // Security: Prevent prototype pollution
      'no-prototype-builtins': 'error',

      // Code quality: Consistent return values
      'consistent-return': 'error',

      // Security: Require proper error handling
      'no-empty': ['error', { allowEmptyCatch: false }],

      // Import organization (matches our naming conventions)
      'sort-imports': [
        'warn',
        {
          ignoreCase: true,
          ignoreDeclarationSort: true,
        },
      ],
    },
  },

  // Feature code should consume local wrappers, not Bits UI primitives directly.
  {
    files: ['src/**/*.{ts,svelte}'],
    ignores: ['src/lib/components/ui/**'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'bits-ui',
              message:
                'Import Bits UI primitives only inside src/lib/components/ui. Feature code should use local wrappers.',
            },
          ],
        },
      ],
    },
  },

  // Svelte files
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tsparser,
        project: './tsconfig.json',
        extraFileExtensions: ['.svelte'],
      },
    },
    plugins: {
      svelte,
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...svelte.configs.recommended.rules,
      // Core no-unused-vars does not understand Svelte 5 runes and snippets.
      'no-unused-vars': 'off',
      // Allow let for Svelte props because they can participate in bindings and runes.
      'prefer-const': 'off',
      'svelte/no-unused-svelte-ignore': 'error',
      'svelte/no-useless-mustaches': 'warn',
    },
  },

  // Extra strict rules for sensitive files
  {
    files: ['**/machines/**', '**/stores/**', '**/utils/crypto*'],
    rules: {
      'no-console': ['error', { allow: ['error'] }],
      'prefer-const': 'error',
      'no-var': 'error',
    },
  },

  // Test files - more relaxed rules
  {
    files: ['**/*.test.{js,ts,svelte}', '**/*.spec.{js,ts,svelte}'],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
      'no-console': 'off',
    },
  },

  // Ignore patterns
  {
    ignores: [
      'build/',
      '.svelte-kit/',
      'dist/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/gen/',
      '*.env*',
      'static/',
      'playwright-report/',
      'test-results/',
      'src/lib/components/ui/**', // Ignore shadcn-svelte generated components
    ],
  },
];
