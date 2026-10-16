import { describe, expect, test } from "vitest";
import tauriConfig from "../src-tauri/tauri.conf.json";
import viteConfig from "../vite.config";
import { contentSecurityPolicy } from "../csp";

const directives = (policy: string) =>
  Object.fromEntries(
    policy.split(";").map((part) => part.trim()).filter(Boolean).map((part) => {
      const [name = "", ...sources] = part.split(/\s+/);
      return [name, sources];
    }),
  );

const LOCAL = new Set(["http://atlas.localhost", "http://ipc.localhost"]);

describe("content security policy", () => {
  test("the preview server serves the exact native policy", () => {
    expect(contentSecurityPolicy).toBe(tauriConfig.app.security.csp);
    expect(viteConfig.preview?.headers).toEqual({ "Content-Security-Policy": tauriConfig.app.security.csp });
  });

  test("the policy stays local-only and data-free", () => {
    const policy = directives(contentSecurityPolicy);
    expect(policy["default-src"]).toEqual(["'self'"]);
    for (const [name, sources] of Object.entries(policy)) {
      for (const source of sources.filter((value) => !LOCAL.has(value))) {
        expect(source, `${name} must not allow ${source}`).not.toMatch(/^(data:|blob:|https?:|\*|'unsafe-eval')/);
      }
    }
    // The only non-self hosts are the local protocol and IPC endpoints.
    expect(policy["img-src"]).toEqual(["'self'", "atlas:", "http://atlas.localhost"]);
    expect(policy["connect-src"]).toEqual(["'self'", "ipc:", "http://ipc.localhost"]);
    expect(policy["script-src"]).toBeUndefined();
  });

  test("assets are never inlined as data URIs", () => {
    expect(viteConfig.build?.assetsInlineLimit).toBe(0);
  });
});
