# Wave8 focused reference shortcut and explicit interaction

Owner: Codex `/root/release_review`, sole task writer.
Published baseline035f28ece68b0b81680ec42bbe2a792b8a95140d (v0.7.0).
Branch feat/reference-interaction; /tmp/navzeroth-wave8-reference.
Read STATUS, v0.7 release, reference89 and wave8 coordination handoffs and plan12.5/M10.
Own fixed focused-main shortcut, native reference controller/two narrow commands,
feature SDK/controls/pure policy and adapter tests. Root reconciles additive exports.
Default absent/hidden/topmostfalse; no global shortcut/game input listener, cursor
movement or set_focus. Actual main caller/focus required for shortcut; reference
invoke gate remains get_reference_view only. Interaction requests remain distinct
from observed visibility/topmost and partial failures. No native/live claims from mocks.
