# Section12.6 main heading reflow repair

Owner and sole writer: Codex /root. Baseline protected main
fbbda8fb8cf2fc25a3e5218d34a8e29cac6fb412; published baseline035f28e/v0.7.0.
Branch fix/heading-reflow, worktree /tmp/navzeroth-wave8-heading-reflow.
Read STATUS, wave8 coordination handoff and plan12.6/20 before implementation.
Reference browser diagnostics found main-page H1/H2/H3 text overflowing their
bounds at320px/200% font size. Reproduce and fix bounded heading reflow in a
separate granular PR; no reference IPC/native/HUD/route ownership changes.
Browser1420 remains leased to reference until explicitly released. Record actual
checks and limitations in handoff; root owns protected-main integration/release.
