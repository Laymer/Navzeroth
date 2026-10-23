# Automated downloaded-draft Windows acceptance

Owner: root. Baseline: 9cc774e4f2656f26d544071d6bd174367ccca16a.

Implement the user-approved schema5 transport and native UI acceptance for the exact downloaded draft installer. Tooling owns core/CLI/schema/preparation contracts; desktop owns recovery/progress; world owns ADR/build-watch documentation. This task owns PowerShell consumers/producers, downloaded-release workflow and UI driver/tests. Preserve historical schema4 owner semantics. New evidence is external and must not rewrite signed assets after tagging. Maintain finite workload/suite limits, unsigned status checks and exact artifact/source bindings.

Windows11 local testing is supplemental, with verified 0.8.1/data backups before the same installer upgrade and user-provided elevation when the concrete launcher is ready. No local network-adapter controls. No private exports or secrets in Git. Actual tests and independent reviews will be recorded before integration.
