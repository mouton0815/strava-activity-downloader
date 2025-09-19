// eslint.config.js
import js from '@eslint/js';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import reactPlugin from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import jsxA11y from 'eslint-plugin-jsx-a11y';
import globals from 'globals';

/** @type {import("eslint").Linter.FlatConfig[]} */
export default [
    js.configs.recommended,
    {
        files: ['**/*.ts', '**/*.tsx'],
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                project: './tsconfig.json',
                ecmaVersion: 'latest',
                sourceType: 'module',
                ecmaFeatures: { jsx: true }
            },
            globals: {
                ...globals.browser
            }
        },
        plugins: {
            '@typescript-eslint': tseslint,
            react: reactPlugin,
            'react-hooks': reactHooks,
            'jsx-a11y': jsxA11y
        },
        rules: {
            ...tseslint.configs.recommended.rules,
            ...reactPlugin.configs.recommended.rules,
            ...reactHooks.configs.recommended.rules,
            ...jsxA11y.configs.recommended.rules,

            // Optional: disable this for modern React setups
            'react/react-in-jsx-scope': 'off'
        },
        settings: {
            react: {
                version: 'detect'
            }
        }
    }
];
