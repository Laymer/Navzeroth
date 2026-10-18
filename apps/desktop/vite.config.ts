import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";
import { contentSecurityPolicy } from "./csp.ts";

// Fixed dev port: src-tauri/tauri.conf.json points devUrl at it.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  // Never inline assets: a `data:` URI would be blocked by the native CSP.
  build: { target: "es2024", assetsInlineLimit: 0 },
  // Production-build pre-check: serve dist with the native CSP (see csp.ts).
  preview: {
    host: "127.0.0.1",
    port: 4173,
    strictPort: true,
    headers: { "Content-Security-Policy": contentSecurityPolicy },
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}", "csp.ts", "vite.config.ts"],
      exclude: ["src/**/*.test.{ts,tsx}", "src/test-setup.ts", "src/**/test-fixtures/**"],
      reporter: ["text", "json", "json-summary", "lcovonly"],
      thresholds: { perFile: true, lines: 90 },
    },
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    // Dense-map tests took 5.3 s on a loaded runner against the 5 s default.
    testTimeout: 15000,
  },
});
