# V16 Windows offline failure after actual recovery pass

Owner/root, claim8db7e6c4; candidate764fcac347f0c75ee628df619e9bd224ebf00b6f.
Actual checkout277a6e6ab6160d7e16b00cfe711fbf100103841e/tree8d549949613a2106a379991cdf17234d57bd33f7 equals candidate.
Production23b4f54dbd7c567c727a5c2431c332a49f8e3b0b79f3423fd27c95d6dbac1535.
CI39586958591, coverage39586958564, source39586958458 and Pages39586958669 validation
passed. CI all9applicable; stability skipped and still required on exact resulting
main. Both source platforms actually rebuilt whole corpus twice and compared
committed bytes/parity. Coverage305inventory/276measured at exact90percent; no
missing/below/errors or incomplete profiles. Tooling independently audited and root
recomputed all276 integer thresholds. Pages deployment skipped, not performed.

Package39586958727 FAILED finalguard55 at12:28:05UTC21October. Actual producer51
OUTCOME was failure, despite GitHub step CONCLUSION success under continue-on-error.
Root initially reported producer success from that conclusion, then corrected it
when the guard exposed actual outcome. Recorder52 and uploads53/54 passed. Draft
job skipped. This failure is not a missing-evidence success or an acceptance pass.

Actual five hosted passes: install/resources/first-start/performance/persistence-
recovery. OFFLINE failed: Controlled adapters could not all be restored to their
original active states. RC1 upgrade/downgrade never ran; owner unverified. All six
case files match overlay bytes and source/production/installer/target bindings.
Native regression32/0 and genericupgrade7/0 passed, separately from stable cases.
Installer26058789B SHA256b028e4091e8b03d6533f5fdad755ecd792b83c7c675f591b88b4ce38ef0eb2fc;
installer/application NotSigned, version1.0.0. This is a candidate installer, not
an owner-verified final draft or published artifact.

Actual recovery passed1187756ms, about12244ms below unchanged1200s bound. Fresh
385512ms and restart388050ms passed. Full ready/receipt-cleanup, content/profile/
settings/inferredfalse/default-marker and integrity assertions passed; check_database
completed1162610ms. Final progress retains only last32events, so earlier inspect/
restore IPC timings cannot be recovered from this report. The narrow time margin
is disclosed; no Windows performance robustness or ordinary-PC claim follows.

Actual hostedperformance: install8258.0357/load239736/activation0/browse14388.8088/
planning1935.1183ms, peak6137241600B (nativePID2144,1862samples,target200ms,0errors).
Microsoft VM/AMD EPYC9V74/RAM17174360064B. Peak excludes WebView2 descendants and
installer; caches uncontrolled and after-last-sample allocations may be missed.
Installedresource inventory630records:628sourceverified plus two expected generated
application/uninstaller records; generated is not source-verified.

Offline had NO raw workload or progress file. Finally threw at Helpers.ps1:119 and
masked any earlier failure. Duplicate descriptions, hidden/non-controllable adapters
and enable/status errors are hypotheses, not proven root causes. Desktop owns a
separate GUID/CIM identity repair and bounded primary+cleanup diagnostics, preserving
all originally-Up hidden adapters, zero-Up offline proof and60s exact restoration.
No local host adapter mutation or deadline relaxation is allowed. Tooling owns a
separate tiny opt-in hostedWindows probe to establish actual adapter behavior before
a new full candidate. Root/world/tooling independently review; one writer per scope.

Exact signed/raw/overlay/coverage/CI/source records archivedde9fa566 plus05fa3988
before any repair integration. Root verified all52FileRefs and committed Git blobs;
13 raw logs were explicitly added after general log-ignore omitted them initially.
Canonical V16 bindings and all9unverified observation rows remain untouched.
Main807ef674/PR239draft/generated243deferred. No tag, publication or owner smoke.
Next reviewed offline repair + bounded actual hosted probe, then renewed source/
review/receipts and exact fresh candidate gates; failures continue to hold release.
