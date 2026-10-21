# Hosted offline adapter identity and failure diagnostics

Owner: desktop_review. Baseline764fcac347f0c75ee628df619e9bd224ebf00b6f. Sole writer in /tmp/navzeroth-stable-offline-adapter-identity, branch fix/stable-offline-adapter-identity. Scope: Stable-Acceptance-Helpers.ps1, synthetic helper tests and focused handoff; producer call-site only if needed for bounded diagnostics. Read STATUS, V16 runtime handoff and implementation plan M9 offline acceptance.

Actual V16 failed offline cleanup; original underlying error and adapter-specific state were not retained. Do not assert hidden/duplicate adapters caused the actual failure. Preserve hosted-only execution, all-adapters-offline proof, exact restoration, finite bounds and all case assertions. Never disable local networking. No runtime/source/content/canonical/key edits. Native Windows shell testing uses mocks only. Coordinator/root and tooling/world independently review frozen code before integration.
