# M10 ordinary reference window claim

Owner: Codex `/root/release_review`, sole writer. Baseline current protected main
`c0d8580`; branch `feat/reference-window`. Root approved exact scope before code.
Read STATUS, wave6 coordination/guard handoffs, plan M10/12.1. Own new bounded
reference service/native module/frontend view+controls/DTO/tests/docs, plus
feature-specific bootstrap/App/module/command registration. Root reconciles
shared additive conflicts. Fixed bundled label/URL only, explicit show/hide,
default absent/topmost false, imported facts/capture and field freshness only.
No game bridge, input interception, inferred display support or native verification.

Root explicitly approved the native invoke-handler wrapper: actual reference-webview
metadata allows only get_reference_view; main handler remains unchanged. Management
commands require actual main-window caller. This gates application IPC only, not
game input; capability JSON alone is not claimed to restrict custom commands.
Fixed fragment semantics checked against pinned Tauri2.12.3 manager/webview.rs:
WebviewUrl::App joins the supplied path with url::Url::join; index.html#reference
therefore selects /index.html with fragment reference, rather than an escaped file.
A native URL regression covers dev/http/app-scheme bases; local URL2.5.8 parser
verification will be recorded separately from unverified native window behavior.
