// Minimal and strict. The heavy lifting is done by tsc --strict; ESLint covers what the
// type system cannot see.
module.exports = {
  root: true,
  env: { browser: true, es2022: true },
  parser: "@typescript-eslint/parser",
  parserOptions: { ecmaVersion: "latest", sourceType: "module" },
  plugins: ["@typescript-eslint"],
  extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  rules: {
    "no-console": ["warn", { allow: ["warn", "error"] }],
    "@typescript-eslint/no-floating-promises": "off",
  },
  ignorePatterns: ["dist", "node_modules"],
};
