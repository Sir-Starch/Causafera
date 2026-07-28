import js from "@eslint/js";
import tseslint from "@typescript-eslint/eslint-plugin";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";

export default [
  {
    ignores: ["dist/**", "node_modules/**", "src-tauri/target/**"],
  },
  {
    ...js.configs.recommended,
    files: ["**/*.{ts,tsx}"],
  },
  ...tseslint.configs["flat/recommended"],
  reactHooks.configs.flat.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      globals: globals.browser,
    },
    plugins: {
      "react-refresh": reactRefresh,
    },
    rules: {
      "react-refresh/only-export-components": "off",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "react-hooks/set-state-in-effect": "off",
    },
  },
];
