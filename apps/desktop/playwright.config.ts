import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  use: { baseURL: "http://127.0.0.1:1420", browserName: "chromium", trace: "retain-on-failure" },
  webServer: { command: "pnpm dev --host 127.0.0.1", url: "http://127.0.0.1:1420", reuseExistingServer: false },
});
