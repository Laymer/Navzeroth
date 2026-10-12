import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "src-tauri"] },
  js.configs.recommended,
  ...tseslint.configs.strict,
  { languageOptions: { globals: { ...globals.browser } } },
  {
    rules: {
      // Command wrappers must not leak untyped values across the boundary.
      "@typescript-eslint/no-explicit-any": "error",
    },
  },
);
