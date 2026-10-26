# Parent watchdog startup regression

Owner desktop_review; baseline97a81de9fd8d07270ed86f55ca9bf445b010e60a.
Sole writer /tmp/navzeroth-stable-watchdog-startup branchfix/stable-watchdog-startup.
Actual V14package39497088109 failed PS7 helper's2s synthetic parent timeout because
child progress file was absent. No installer/native/hosted acceptance cases ran.
Investigate using portable official Windows PS7/Node outside Git; prefer test-only
repair allowing bounded child startup, preserve actual forced kill/finally absence/
partial+watchdog checks. No production20/60 limits, assertions, runtime, content,
canonical, keys, install policy changes or workflow retries authorized here.
