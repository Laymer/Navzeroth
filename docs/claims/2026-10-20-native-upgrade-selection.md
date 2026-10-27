# Native upgrade selected-profile interaction

Owner desktop_review. Baseline728a9198cb2d72212d39cf06771f807a412967f3.
Sole writer /tmp/navzeroth-native-upgrade-selection, branchfix/native-upgrade-selection.
Scope native smoke driver/helper/tests, actual React ContentScreen regression tests,
focused claim/handoff; workflow invocation only if required. No runtime component,
content/source, canonical receipt, release branch or key changes.

Actual V12package39454144270 failed native upgrade verification timeout after
profile/active IPC succeeded; eight hosted cases skipped. Unconditional selection
of already-selected profile emits change and clears ContentScreen active state;
unchanged profile dependency does not refetch. Implement conditional selection,
retain strict issued IPC identity and bounded matching UI verification. Prove old
interaction failure using actual component before claiming repaired success.
