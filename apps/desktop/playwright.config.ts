import { defineConfig } from "@playwright/test";

const DEV = "http://127.0.0.1:1420";
const PREVIEW = "http://127.0.0.1:4173";

// Two projects. "dev" runs every spec against the vite dev server. "prod-csp" runs
// only prod-csp.spec.ts against the built bundle served with the native CSP.
// Retries stay at zero everywhere: a failure is a defect to fix, never to rerun.
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  use: { browserName: "chromium", trace: "retain-on-failure" },
  projects: [
    { name: "dev", testIgnore: /prod-csp\.spec\.ts/, use: { baseURL: DEV } },
    { name: "prod-csp", testMatch: /prod-csp\.spec\.ts/, use: { baseURL: PREVIEW } },
  ],
  webServer: [
    { command: "pnpm dev --host 127.0.0.1", url: DEV, reuseExistingServer: false },
    {
      command: "pnpm build && pnpm exec vite preview",
      url: PREVIEW,
      reuseExistingServer: false,
      timeout: 180_000,
    },
  ],
});
