# Main heading reflow repair

Owner/sole writer Codex /root, claim66a0d3c, baselinefbbda8f (published035f28e/v0.7.0).
Source61f6769 on fix/heading-reflow. Plan12.6 requires scalable readable text.
Reference controls browser diagnostics exposed an independent main heading gap:
at320px/200% text, H1/H2/H3 words overflowed their240px content widths.

A single main heading rule enables wrapping long words for h1–h6 without clipping,
hiding content, reducing font size or changing text/semantics. No IPC/native,
reference controls, HUD, route or data behavior changes. Two mocked Chromium cases
measure actual visible H1/H2/H3 scroll widths and viewport bounds at100% and200%.

Actual baseline regression: one100% case passed, one200% case failed on heading
scroll overflow (/tmp/navzeroth-heading-before.log). After correction both passed
(/tmp/navzeroth-heading-after.log). Desktop lint/build (including TypeScript) and
git diff --check passed (/tmp/navzeroth-heading-{lint,build}.log). Browser server
stopped; port1420 lease returned. Synthetic layout checks do not establish native
DPI, screen-reader or actual Windows acceptance. No extra unit test mirrors CSS.

Next: independent source review, all nine latest exact-head hosted gates, protected
main merge and next preview release; actual environment acceptance remains unverified.
