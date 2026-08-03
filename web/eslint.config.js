// Flat config. Deliberately small: `tsc --strict` does the heavy lifting, so ESLint only
// covers what the type system cannot see — chiefly promises nobody is waiting on.
import js from "@eslint/js";
import ts from "typescript-eslint";
import globals from "globals";

export default ts.config(
  { ignores: ["dist", "node_modules", "coverage"] },
  js.configs.recommended,

  // Type-aware rules need a tsconfig project, and only `src` is in one. Applying them
  // repository-wide fails on this very file, which is not TypeScript the app compiles.
  {
    files: ["src/**/*.{ts,tsx}"],
    extends: [...ts.configs.recommendedTypeChecked],
    languageOptions: {
      globals: globals.browser,
      parserOptions: { project: true, tsconfigRootDir: import.meta.dirname },
    },
    rules: {
      // A floating promise in a UI is a request whose failure nobody sees. This rule is
      // why `void submit()` appears in App.tsx rather than a bare call.
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "no-console": ["warn", { allow: ["warn", "error"] }],
    },
  },

  // Build and test configuration: linted, but without type information.
  {
    files: ["*.{js,ts}", "src/**/*.test.{ts,tsx}"],
    extends: [ts.configs.disableTypeChecked],
    languageOptions: { globals: { ...globals.node, ...globals.browser } },
  },
);
