import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The API is proxied rather than called cross-origin, so the browser sees one origin in
// development and in production. That removes a class of CORS-only-in-dev bug.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: process.env.API_URL ?? "http://127.0.0.1:8080",
        changeOrigin: true,
      },
    },
  },
});
