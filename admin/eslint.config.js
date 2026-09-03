import js from "@eslint/js"
import globals from "globals"
import reactHooks from "eslint-plugin-react-hooks"
import reactRefresh from "eslint-plugin-react-refresh"
import tseslint from "typescript-eslint"
import { defineConfig, globalIgnores } from "eslint/config"

const inventedToken = {
  selector: "Literal[value=/(?:^|\\s)(?:bg|text|border)-(?:violet|fuchsia|pink|gray|zinc|slate|red)-\\d{2,3}\\b/]",
  message: "Invented token: use semantic classes (bg-primary, text-destructive, bg-muted), not palette ramps.",
}

const hardHex = {
  selector: "Literal[value=/#[0-9A-Fa-f]{3,8}\\b/]",
  message: "Invented token: no hard-coded hex. Use index.css semantic tokens.",
}

export default defineConfig([
  globalIgnores(["dist"]),
  {
    files: ["**/*.{ts,tsx}"],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
  },
  {
    files: ["src/pages/**/*.{ts,tsx}", "src/components/**/*.{ts,tsx}", "src/common/**/*.{ts,tsx}"],
    ignores: ["src/components/ui/**"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector: "JSXOpeningElement[name.name='select']",
          message: "Use shared Select, not native <select>.",
        },
        {
          selector: "JSXOpeningElement[name.name='dialog']",
          message: "Use shared Dialog, not native <dialog>.",
        },
        {
          selector: "CallExpression[callee.object.name='window'][callee.property.name=/^(alert|confirm|prompt)$/]",
          message: "Native confirm: use AlertDialog / toast.",
        },
        hardHex,
        inventedToken,
      ],
    },
  },
])
