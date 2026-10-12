import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Fixed dev port: src-tauri/tauri.conf.json points devUrl at it.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2024" },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
  },
});
