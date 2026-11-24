import js from '@eslint/js';
import ts from 'typescript-eslint';
import vue from 'eslint-plugin-vue';
export default ts.config(
  { ignores: ['dist/**', 'node_modules/**', 'src-tauri/target/**', 'src-tauri/gen/**', 'diagnostics/target/**', 'diagnostics/minimal-tauri/gen/**'] },
  js.configs.recommended,
  { files: ['scripts/*.mjs'], languageOptions: { globals: { process: 'readonly', console: 'readonly' } } },
  ...ts.configs.recommended,
  ...vue.configs['flat/essential'],
  { files: ['**/*.vue'], languageOptions: { parserOptions: { parser: ts.parser } } },
  { files: ['src/**/*.{ts,vue}', 'public/*.js', 'tests/**/*.ts', 'src-tauri/diagnostics/*.js'], languageOptions: { globals: { window: 'readonly', document: 'readonly', localStorage: 'readonly', indexedDB: 'readonly', navigator: 'readonly', location: 'readonly', caches: 'readonly', crypto: 'readonly', Blob: 'readonly', TextEncoder: 'readonly', TextDecoder: 'readonly', HTMLElement: 'readonly', HTMLInputElement: 'readonly', HTMLSelectElement: 'readonly', ResizeObserver: 'readonly', KeyboardEvent: 'readonly', setTimeout: 'readonly', clearTimeout: 'readonly' } } }
);
