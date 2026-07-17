# Agent Manager project memory

## Stable repository facts

- Agent Manager 2.0 is the `agent-manager` Rust crate (`edition = "2024"`) with a library at `src/lib.rs` and binary at `src/main.rs`; the desktop UI uses `eframe`/`egui`.
- The managed data shape is exactly `agents/<category>/<slug>/SKILL.md`. Storage operations validate that shape and containment before write or recursive delete; deletion backs up the complete Agent directory.
- The Rust 2.0 migration preserves the existing `agents/` corpus. The verified migration snapshot contained 37 categories and 306 `SKILL.md` files, and the migration worktree had no changes under `agents/`.
- Standard gates are `cargo fmt --check`, `cargo check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, and `git diff --check`. On this Windows machine, use `C:\Users\steven\.cargo\bin\cargo.exe` if `cargo` is absent from `PATH`.
- `cargo run -- --check` is headless and read-only. Its policy is exit 1 for any load diagnostic or any CRITICAL/HIGH validation issue; therefore a nonzero exit is expected for the current corpus until its existing high-severity data findings are remediated. Evaluate its summary and policy, not exit code alone.
