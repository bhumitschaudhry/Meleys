You are implementing Task 11 of a multi-engine architecture for Meleys (a Rust browser runtime).

**Project context:** Meleys is at E:\Meleys. Task 10 deleted `src/actions/capture.rs`. Now we need to remove screenshot/PDF references from HTTP transport.

**Task 11: Update HTTP Transport**

Modify `src/transport/http.rs`:

1. Remove these route registrations from `build_router`:
```rust
.route(
    "/v1/sessions/{session_id}/tabs/{tab_id}/screenshot",
    post(screenshot_handler),
)
.route(
    "/v1/sessions/{session_id}/tabs/{tab_id}/export_pdf",
    post(export_pdf_handler),
)
```

2. Remove these handler functions:
- `screenshot_handler`
- `export_pdf_handler`

3. Remove these request structs:
- `ScreenshotRequest`
- `ExportPdfRequest`

4. Run `cargo test --lib transport::http` to verify
5. Commit: `feat(http): remove screenshot/PDF routes`

Return: status, commits, test results, concerns.
