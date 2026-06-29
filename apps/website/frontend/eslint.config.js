import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import jsdoc from 'eslint-plugin-jsdoc'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  // Generated contract projections (make schema-codegen) are not hand-linted.
  globalIgnores(['dist', 'src/types/contract/**']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      globals: globals.browser,
    },
  },
  {
    // Contract layer (DOCUMENTATION_STANDARDS §5/§10): every exported symbol must carry a
    // TSDoc/JSDoc block. Presence only — the custom @contract/@route/@model tags are declared
    // in tsdoc.json and intentionally not validated by this rule.
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
])
