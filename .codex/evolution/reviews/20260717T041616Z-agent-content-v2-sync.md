# Agent content V2 sync isolated self-review

- Review time (UTC): `2026-07-17T04:16:16Z`
- Scope: final learning pass only; no Agent content, production code, tests, build files, planning/final documents, ignore rules, user-authored skills, or Git state changed.
- Evidence reviewed: the prior migration review, `.codex/evolution/memory/MEMORY.md`, `.codex/evolution/LESSONS.md`, `.codex/evolution/state/CODEX_STATE.md`, `plan.md`, `test.md`, `final.md`, the source README/upgrade report/validator, and `.codex/evolution/ptc-agent-sync.json`.

## Task summary

The official Agent Content V2 package was applied as an in-place update to all 306 existing `agents/<category>/<slug>/SKILL.md` paths across 37 categories. No category+slug/path identity was added or deleted. The official apply workflow first created the rollback copy at `E:\agent-manager\.agent-backup\20260717-120128\agents`; the backup is local and ignored rather than committed. Destination-only `AGENTS_INDEX.md`, `KNOWLEDGE_GRAPH.md`, and `README.md` were preserved.

The source package directory, source zip, and `.agent-backup/` are root-ignored local artifacts. They remain available for validation and rollback without entering the repository history.

## Corrections and independent review finding

The independent review initially identified a `name` contract conflict because 185 frontmatter `name` values changed from legacy display-style values to slugs. That finding correctly surfaced a user-visible change, but its identity conclusion was too broad.

| Failure | Cause | Correction |
|---|---|---|
| The 185 `name` changes were treated as identity changes and therefore as a contract violation. | The reviewer over-assumed that frontmatter `name` was part of Agent identity without first applying the authoritative source contract. | Identity was resolved to `(category, slug)`, equivalently the relative `category/slug/SKILL.md` path. The V2 README and upgrade report explicitly preserve category+slug, while `validate_agents.py` separately requires `name == slug`. All 306 identities remained in place. |
| Passing source hashes could have hidden the semantic impact of the normalized field. | Byte equality, identity preservation, and field-level semantic normalization were being conflated into one acceptance claim. | The final evidence reports all three dimensions separately and transparently discloses the 185 `name` normalizations, including the resulting UI-visible change from some title-case names to slug display values. |

The accepted correction is therefore not to restore legacy `name` values. Restoring them would violate the source validator and break source/destination hash equality. The durable boundary is category+slug/path identity plus explicit disclosure of semantic display-name normalization.

## Verification evidence

- Official source validator: PASS, 306 Agents / 37 categories; its contract rejects `name != slug` and duplicate `(category, slug)` identities.
- Official source checksum manifest: PASS, 319 files checked / 0 failures.
- Official package workflow: dry-run completed before the backup-producing apply; 306 existing Skill paths were updated in place.
- PTC SHA reduction used the read-only normalized-path/hash contract in `.codex/evolution/ptc-agent-sync.json`. Post-sync output was source=306, destination=309, identical=306, add=0, update=0, destination-only=3. Here, `update=0` means no source/destination byte differences remained after the 306-file in-place apply.
- Post-sync equality: 306/306 matching relative Skill paths had identical SHA-256 values; no Agent path was added or deleted.
- Destination-only documentation: all three files remained present with their pre-apply hashes unchanged.
- Rust gates: `cargo fmt --check`, `cargo check`, strict Clippy, `cargo build --release`, the focused CLI test, and `git diff --check` all passed.
- Tests: `cargo test --all-targets` passed 23 / 23 with 0 failures.
- Release headless check: exit 0; 306 skills, 0 load errors, and CRITICAL/HIGH/MEDIUM/LOW all 0.
- Bounded private-key/token/private-path scan over the 306 updated Skills: 0 matching files.

This isolated pass audited the recorded evidence and source contracts. It did not rerun the already completed build/test suite because its write boundary is limited to `.codex/evolution/**` and no production input changed.

## Durable memory changes

Updated `.codex/evolution/memory/MEMORY.md` to replace the superseded pre-V2 `--check` expectation with the verified current corpus state: Agent Content V2, 306 Skills / 37 categories, identity as category+slug/path, 185 source-defined `name` normalizations, release `--check` exit 0 with all severities zero, and the ignored local backup/source-artifact policy.

## Reusable skill decision

No skill candidate was created. The cross-project lesson is an acceptance-model refinement, not a new procedure requiring its own narrow skill. It was appended to `.codex/evolution/LESSONS.md` under bulk synchronization.

## Remaining risks

- Validation was structural, hash-based, automated, and test-backed; the 306 regenerated Skill bodies did not receive a line-by-line human editorial review.
- The 185 normalized frontmatter names are validator-compliant but change some UI display names from title-style text to slugs. This is accepted and disclosed, yet remains a user-visible compatibility change.
- The source package and rollback backup are intentionally untracked local artifacts; their continued availability depends on local workspace retention rather than repository history.
