// The one Content-Security-Policy string. The native shell reads it from
// src-tauri/tauri.conf.json; the Playwright "prod-csp" project serves the built
// bundle through `vite preview` with exactly this value. Tauri rewrites the CSP at
// runtime (nonces and hashes), so this is a cheap pre-check, not the authority;
// the native Windows smoke is.
import tauriConfig from "./src-tauri/tauri.conf.json" with { type: "json" };

export const contentSecurityPolicy: string = tauriConfig.app.security.csp;
