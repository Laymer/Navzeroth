# Actual PowerShell preview packaging failure and bounded-reader repair

Owner /root/desktop_review, sole writer; branch fix/stable-preview-packaging-gate,
isolated /tmp/navzeroth-stable-preview-packaging-gate. Baseline
7f55cd19270e09ad0d6d622c44953275973dde23. Code repair34971fc25e915e936d78a87efaced0a29b192254.

Actual delivery PR238 package run39395175310/job118536022883 failed the native
PowerShell5.1 Test-ReleaseGate RC preview assertion after six stable-refusal
assertions passed. Preliminary stable package39395927777 failed the same early
assertion; neither run built or exercised the real installer or measured hosted
performance. Those failures remain failures, not acceptance evidence.

The existing test captured the child Build-Windows error and hid it on failure.
Adding a failure-only diagnostic exposed the actual exception in a native PS5.1
reproduction: "Cannot process argument transformation on parameter 'BytesRead'.
Reference type is expected in argument." Internal Read-ReleaseTextWithCap had an
optional typed [ref] parameter defaulting to null. The generic16MiB wrapper omitted
that argument, so PS5.1 failed binding before reading SOURCE_COMMIT/checksums.
The schema4 contract wrapper already passed an explicit reference and worked.
Earlier throw-only generic negative tests mistakenly accepted this wrong failure.

Both callers now supply explicit local references; the internal reference parameter
is mandatory. No byte cap, strict UTF-8 decoder, path/reparse checks, schema policy,
source permissions, release phase or signature gate was loosened. Added successful
generic small/exact16MiB/BOM checks, explicit overflow-message verification and
DecoderFallbackException verification. The release-gate fixture now prints a
failed child result before its success assertion, so future CI failures are visible.

Actual native Windows PowerShell5.1.26100.9711:
- Old production reader plus new positive regression: failed with the same
  BytesRead binding exception, without Git or any packaging subprocess involved.
- Full Test-ReleaseGate before repair reproduced six passes then the same RC
  failure; after repair all29checks passed, including RC/0.x preview packaging,
  stable/malformed/candidate refusal and unchanged prior artifacts on refusal.
- Expanded Test-ReleaseContractBounds with actual repository inputs:29passed.
  Inventory27957511bytes SHA f7030627acbc61fbbec32deeacc9bfc4dc1f8adc8fb6a183771559c656ce9eac;
  evidence27969970bytes SHA61c3f4ce220b93042dc039d83553bc0ad5073a7807b4ee33d2dca7ab4c2c91a8.
  Historical16MiB/schema4 64MiB byte limits and strict refusal reasons passed.
- Test-ReleaseMaterials:19passed, Test-StablePreparation:42passed,
  Test-StableWorkflowGate:12passed. Whitespace/diff checks passed.

Local native Windows Git is absent. For the two synthetic packaging/material
suites only, a task-external C# executable forwarded actual Git operations to WSL
Git with Windows path conversion; no repository/product code or assertions were
mocked. Native PowerShell ZIP/file/reader and subprocess paths ran normally.
First repaired gate invocation used a malformed local PATH expression and could
not locate powershell.exe; corrected invocation passed. WSL PATH warnings on the
before run did not cause the independently reproduced pure-reader binding error.
The bridge and temporary fixtures stay outside Git. PowerShell7 is unavailable;
fresh hosted checks under both shells remain required after independent review.

Logs retained outside Git: /tmp/navzeroth-preview-gate-before.log,
/tmp/navzeroth-preview-bounds-before.log, /tmp/navzeroth-preview-gate-after.log,
/tmp/navzeroth-preview-contract-bounds-after.log and corresponding material/prep/
workflow logs. No real installer, imported personal data, source/packs/policy pin,
key, signature, hosted/manual acceptance status or other-owner coverage-reader
file changed. No claim of ordinary-PC or Windows installer performance.

Next: independent review, separate repair PR against release/stable-content-delivery,
normal merge and fresh protected-main/source/package checks. Integrate the separate
reviewed coverage bounded-reader/LF repair normally; recompute final production and
receipt bindings only after all production changes and actual tests are settled.
