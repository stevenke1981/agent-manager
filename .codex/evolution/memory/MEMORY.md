# Agent Manager project memory

## Stable repository facts

- Agent Manager 2.0 is the `agent-manager` Rust crate (`edition = "2024"`) with a library at `src/lib.rs` and binary at `src/main.rs`; the desktop UI uses `eframe`/`egui`.
- The managed data shape is exactly `agents/<category>/<slug>/SKILL.md`. Storage operations validate that shape and containment before write or recursive delete; deletion backs up the complete Agent directory.
- The current corpus is official Agent Content V2: 37 categories and 306 `SKILL.md` paths. Agent identity is `(category, slug)`, equivalently `category/slug/SKILL.md`; the source validator separately requires frontmatter `name == slug`.
- V2 updated all 306 existing Agent paths in place without adding or deleting an identity. It source-normalized 185 frontmatter `name` values to slugs; those entries therefore use slug-style UI display names.
- Standard gates are `cargo fmt --check`, `cargo check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, and `git diff --check`. On this Windows machine, use `C:\Users\steven\.cargo\bin\cargo.exe` if `cargo` is absent from `PATH`.
- `cargo run -- --check` is headless and read-only. After the V2 sync, the verified release check loaded all 306 Skills with 0 load errors, CRITICAL/HIGH/MEDIUM/LOW all 0, and exit 0.
- The official apply workflow stores local rollback copies under `.agent-backup/<timestamp>/agents`. The V2 source directory, source zip, and `.agent-backup/` are ignored local artifacts and are not committed.
