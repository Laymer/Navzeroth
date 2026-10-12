# Handoff: project documentation setup

Owner: ChatGPT/Codex
Date: 2026-10-12
Repository: Laymer/Navzeroth
Base commit: 655efe296a1803e36ba176770741a457a2ba996e
Change reference: the commit that introduces this handoff and the implementation documents.
Working tree state: remote baseline inspected; no developer's local checkout was available for inspection.

## Goal and scope

Place the prepared implementation plan and collaboration files in the existing project at the user's request.

## Implemented

- Added the implementation plan without changing its contents.
- Instantiated AGENTS.md and CLAUDE.md from plan section 22.
- Initialized docs/STATUS.md with M0.1 as the next unclaimed task.
- Added this handoff and linked the entry points from the existing README.

The plan retains its original standalone-document notes about templates. The companion instruction and status files now exist in this repository. This setup does not complete M0 or M1.

## Decisions

No architecture or scope change. Keep the existing repository name Navzeroth and product name Azeroth Navigator. Preserve the existing license and ignore rules.

## Validation

- Inspected the remote repository metadata, root file listing, branch head, and initial README through the GitHub connector.
- Checked the plan against its prepared-source SHA-256: 0ebef13de515a6e075fe69323441cb44898869a6f20fe62709b470513168c67e.
- Compared AGENTS.md and CLAUDE.md with the section 22 templates.
- Checked Markdown fences and local links, including heading anchors.
- Core tests, desktop checks, and live WoW checks were not run: no application implementation exists yet.

## Limitations

No local developer environment, game build, live addon capability, official Forever API, or third-party game dataset has been verified. Toolchain and CI setup remain implementation tasks.

## Review focus

Keep docs/STATUS.md aligned with actual work. Claim the task and record the current Git baseline before beginning implementation. Read the relevant plan sections and preserve unrelated local changes.

## Next task

M0.1: inspect the actual implementation checkout, instructions, uncommitted changes, license, toolchain, and CI. Then follow the dependency order in the plan. If live-client access is unavailable, record the affected checks as unverified and continue independent M1 foundation work.
