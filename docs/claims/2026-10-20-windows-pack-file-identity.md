# Windows pack descriptor identity repair

Owner / sole writer: `/root/world_review`; branch
`fix/windows-bounded-file-identity`, baseline `aeb7fd8f`, isolated
`/tmp/navzeroth-windows-bounded-file-identity`.

Repair actual Windows coverage failure: same-sized/same-content path replacement
can collide under creation-time/mtime stamps. Use descriptor-bound volume/file
identity, retain bounds/reparse refusals and Unix semantics. No sleeps, retries,
key/signing/release changes or ancestor-race sandbox claims. Runtime change requires
new release production review/bindings and fresh functional validation.
