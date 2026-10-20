# Original PR212 dungeon consumer correction

Owner/sole integration writer Codex `/root`; baseline3fc850c3; claim9e910c48.
Cherry-picked independently reviewed6270131e ascd1a18f9. Its only conflict was
an EOF test addition: retained the108-line source-reference bounds regression;
excluded unrelated future dungeon-estimate test context. No future estimates,
roles, objectives, authored data or generated world outputs were copied.

Root inspected schema1=32/schema2=4096 reference-only limits, uniqueness/context
checks, bounded expansion before append and existing1MiB request/4MiB complete
report refusal. UI24-row pages retain all references and query only visible origins.
Independent root tests on original reviewed branch:10authored-content+8UI passed.
After original-scope integration:9authored-content+8dungeon-preparation+8UI passed;
all-target app Clippy/fmt/diff passed. Independent tooling review atcd1a18f9 found
no blocker and reran9authored-content tests. Logs:/tmp/navzeroth-212-consumer-
{tests,ui,clippy}.log and/tmp/navzeroth-root-dungeon-consumer-{tests,ui}.log.
Original specialist's affected-file90%+ coverage and277app/8UI checks are retained
in stable-dungeon-consumer-bounds; they do not replace final protected checks.

This necessary repair targets PR212's feature branch. Merge it normally, combine
with separately reviewed original mapper Unsupported-overflow correction, then
require all ten protected-main checks and actual cross-platform source parity on
the exact final212 head. No Windows install/manual/functional/stable publication
pass is inferred. Four source-backed overflow lists remain explicitly unsupported
until a separate versioned full-association contract restores every link for stable.
