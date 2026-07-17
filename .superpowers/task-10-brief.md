You are implementing Task 10 of a multi-engine architecture for Meleys (a Rust browser runtime).

**Project context:** Meleys is at E:\Meleys. Tasks 1-9 created the engine abstraction, config, fallback logic, and wired SessionManager. Now we remove screenshot/PDF capture actions.

**Task 10: Remove Capture Actions**

1. Delete `src/actions/capture.rs`
2. Remove `pub mod capture;` from `src/actions/mod.rs`
3. Run `cargo check` — expect compile errors in HTTP/MCP transport referencing capture (fixed in Tasks 11-12)
4. Commit: `feat(actions): remove capture module (screenshot/PDF)`

Return: status, commits, test results, concerns.
