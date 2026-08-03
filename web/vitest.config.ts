import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    // Two environments: api.ts is plain fetch logic and needs none, but App.tsx renders.
    // jsdom for everything is simpler than annotating each file, and fast enough here.
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
