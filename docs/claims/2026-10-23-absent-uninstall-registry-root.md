# Absent uninstall registry root

Owner: desktop_review. Baseline dee9998a; sole writer in isolated
fix/absent-uninstall-registry-root. Read STATUS, schema5 integration handoff and
implementation plan Windows environment guidance. Probe39916898751 failed before
workload because the fresh HKCU uninstall root was absent. Scope only
Get-InstalledDisplayVersion and focused inert helper regressions, plus handoff.
Skip only a genuinely absent known root; preserve access/enumeration/property
failures and existing registration refusal. No local registry writes, installer,
app/network controls, dispatch or canonical edits. Site branch stays unchanged.
