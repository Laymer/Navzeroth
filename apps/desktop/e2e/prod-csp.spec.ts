import { expect, test } from "@playwright/test";
import type { ReferenceViewData } from "../src/commands/reference";
import report from "../../../fixtures/desktop/reference/imported.json" with { type: "json" };
import { contentSecurityPolicy } from "../csp";
import { installMock } from "./support/mockInvoke";
import { SCREENS } from "./support/screens";

// Production-build pre-check: the built bundle, served with the native CSP string.
// Tauri rewrites the CSP at runtime, so the native Windows smoke stays the authority.

interface Problems {
  violations: string[];
  consoleErrors: string[];
  pageErrors: string[];
  failed: string[];
}

function watch(page: import("@playwright/test").Page): Problems {
  const problems: Problems = { violations: [], consoleErrors: [], pageErrors: [], failed: [] };
  page.on("console", (message) => {
    if (message.type() === "error") problems.consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => problems.pageErrors.push(error.message));
  page.on("requestfailed", (request) => problems.failed.push(`${request.url()} ${request.failure()?.errorText ?? ""}`));
  page.on("response", (response) => {
    if (response.status() >= 400) problems.failed.push(`${response.url()} ${response.status()}`);
  });
  return problems;
}

async function collectViolations(page: import("@playwright/test").Page, problems: Problems) {
  const found = await page.evaluate(() => (window as unknown as { __cspViolations: string[] }).__cspViolations);
  problems.violations.push(...found);
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const violations: string[] = [];
    Object.defineProperty(window, "__cspViolations", { value: violations });
    document.addEventListener("securitypolicyviolation", (event) => {
      violations.push(`${event.violatedDirective} blocked ${event.blockedURI || "inline"}`);
    });
  });
});

test("the served document carries exactly the native CSP", async ({ page }) => {
  const response = await page.goto("/");
  expect(response?.headers()["content-security-policy"]).toBe(contentSecurityPolicy);
});

test("every screen of the production bundle runs under the native CSP", async ({ page }) => {
  const problems = watch(page);
  await page.goto("/");
  const nav = page.getByRole("navigation", { name: "Primary" });
  await expect(nav.getByRole("button")).toHaveCount(15);
  await expect(page.getByRole("heading", { name: "Azeroth Navigator", exact: true })).toBeVisible();
  for (const label of SCREENS) {
    await nav.getByRole("button", { name: label, exact: true }).click();
    await expect(nav.getByRole("button", { name: label, exact: true })).toHaveAttribute("aria-current", "page");
    // Fonts and images that exist must have loaded (zero-size means a blocked request).
    const broken = await page.evaluate(() =>
      Array.from(document.images).filter((image) => image.complete && image.naturalWidth === 0).map((image) => image.src),
    );
    expect(broken, `${label} has broken images`).toEqual([]);
    await collectViolations(page, problems);
  }
  expect(problems).toEqual({ violations: [], consoleErrors: [], pageErrors: [], failed: [] });
});

test("the fixed reference page runs under the native CSP", async ({ page }) => {
  const problems = watch(page);
  await installMock(page, { get_reference_view: () => report as unknown as ReferenceViewData });
  await page.goto("/#reference");
  await expect(page.getByRole("heading", { name: "Imported reference", exact: true })).toBeVisible();
  await collectViolations(page, problems);
  expect(problems).toEqual({ violations: [], consoleErrors: [], pageErrors: [], failed: [] });
});

test("the self-hosted Inter and Cinzel faces load under the native CSP", async ({ page }) => {
  const problems = watch(page);
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Azeroth Navigator", exact: true })).toBeVisible();
  const faces = await page.evaluate(async () => {
    await document.fonts.ready;
    return Array.from(document.fonts).map((face) => `${face.family}:${face.status}`);
  });
  // Both families are used on the first screen (Inter for body text, Cinzel for headings).
  expect(faces.sort()).toEqual(["Cinzel:loaded", "Inter:loaded"]);
  await collectViolations(page, problems);
  expect(problems).toEqual({ violations: [], consoleErrors: [], pageErrors: [], failed: [] });
});
