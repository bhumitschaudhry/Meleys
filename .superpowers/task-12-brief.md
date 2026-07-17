You are implementing Task 12 of a multi-engine architecture for Meleys (a Rust browser runtime).

**Project context:** Meleys is at E:\Meleys. Task 10 deleted `src/actions/capture.rs`. Now we need to remove screenshot/PDF references from MCP transport.

**Task 12: Update MCP Transport**

Modify `src/transport/mcp.rs`:

1. Remove the "screenshot" tool schema from `tools_list()` function
2. Remove the "export_pdf" tool schema from `tools_list()` function
3. Remove the "screenshot" match arm from `dispatch_tool` function
4. Remove the "export_pdf" match arm from `dispatch_tool` function

5. Run `cargo test --lib transport::mcp` to verify
6. Commit: `feat(mcp): remove screenshot/PDF tool definitions`

Return: status, commits, test results, concerns.
