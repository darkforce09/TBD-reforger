import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import jsdoc from 'eslint-plugin-jsdoc'
import importX from 'eslint-plugin-import-x'
import eslintConfigPrettier from 'eslint-config-prettier'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  // Generated contract projections (make schema-codegen) + the wasm-pack outputs
  // (make wasm / make wasm-render) are not hand-linted.
  globalIgnores(['dist', 'src/types/contract/**', 'src/wasm/pkg/**', 'src/wasm/render/**']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    plugins: { 'import-x': importX },
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      // CODING_STANDARDS §10 — T-125.3 code gates.
      // TS-3 (De): no `any`, no unsafe non-null `!` on contract data.
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-non-null-assertion': 'error',
      // TS-4/TS-7 (Us): a catch (or handler) must surface/recover, never swallow.
      'no-empty': ['error', { allowEmptyCatch: false }],
      'no-empty-function': 'error',
      // LOG-2 (De): no committed console.log; dev diagnostics use warn/error or an inline opt-out.
      'no-console': ['error', { allow: ['warn', 'error'] }],
      // COMP-1 (Re): cyclomatic complexity ≤ 15 — inline `// eslint-disable-next-line complexity`
      // (with a reason) is the only sanctioned escape; no file-level disable.
      complexity: ['error', { max: 15 }],
      // TS-2 (Sc): layer boundaries — a page is composed FROM features/ui, never imported BY them.
      // Catches relative imports (`../pages/...`). The `@/pages` alias form is covered by the
      // no-restricted-imports block below (import-x cannot resolve the `@/` alias without a resolver dep).
      'import-x/no-restricted-paths': [
        'error',
        {
          zones: [
            {
              target: './src/features',
              from: './src/pages',
              message: 'TS-2: features/ must not import pages/ (pages compose features, not the reverse).',
            },
            {
              target: './src/components',
              from: './src/pages',
              message: 'TS-2: components/ must not import pages/.',
            },
          ],
        },
      ],
    },
  },
  {
    // TS-2 (alias form): forbid the `@/pages` path alias from features/ and components/.
    files: ['src/features/**/*.{ts,tsx}', 'src/components/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['@/pages', '@/pages/*'],
              message: 'TS-2: features/ and components/ must not import pages/.',
            },
          ],
        },
      ],
    },
  },
  {
    // Contract layer (DOCUMENTATION_STANDARDS §5/§10): every exported symbol must carry a
    // TSDoc/JSDoc block. Presence only — the custom @contract/@route/@model tags are declared
    // in tsdoc.json and intentionally not validated by this rule (TS-6 verify-contract-citations
    // checks @model/@contract content).
    files: ['src/types/**/*.ts', 'src/api/**/*.ts', 'src/hooks/**/*.ts'],
    plugins: { jsdoc },
    rules: {
      'jsdoc/require-jsdoc': [
        'error',
        {
          // Only the contexts below are enforced; disable the rule's default function checks so
          // internal/nested helpers (e.g. useAuthed, a hook's connect()) need not carry docs.
          require: {
            FunctionDeclaration: false,
            MethodDefinition: false,
            ClassDeclaration: false,
            ArrowFunctionExpression: false,
            FunctionExpression: false,
          },
          contexts: [
            'ExportNamedDeclaration > TSInterfaceDeclaration',
            'ExportNamedDeclaration > TSTypeAliasDeclaration',
            'ExportNamedDeclaration > FunctionDeclaration',
          ],
        },
      ],
    },
  },
  // FMT-3 (T-125.5): eslint-config-prettier LAST — turns off only the ESLint rules that
  // conflict with Prettier formatting. All TS-2..7 / LOG-2 / COMP-1 lint rules stay on;
  // Prettier itself runs via `npm run format:check`, not through ESLint (no eslint-plugin-prettier).
  eslintConfigPrettier,
])
