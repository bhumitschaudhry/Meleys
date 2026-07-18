---
id: 2026-07-18-multi-engine-architecture
title: Multi-Engine Architecture Implementation Plan
sidebar_position: 1
---

# Multi-Engine Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform Meleys from Chromium-only to multi-engine architecture with Lightpanda as primary and Chromium as fallback, removing screenshot/PDF actions.

**Architecture:** BrowserEngine trait with ChromiumEngine/LightpandaEngine. Session-level fallback. Shared Lightpanda process per runtime. Remove capture actions.

**Tech Stack:** Rust 2021, tokio, chromiumoxide, async-trait, figment, axum, tracing

## Global Constraints

- Rust edition 2021, MSRV 1.80.0
- All existing tests must continue passing
- No new dependencies unless necessary (Lightpanda speaks CDP)
- LIGHTPANDA_DISABLE_TELEMETRY=true when spawning Lightpanda
- Backward-compatible config migration

---

## File Structure

| File | Action |
|------|--------|
| `src/engine/mod.rs` | NEW - BrowserEngine trait, EngineKind, EngineCapabilities, EnginePreference, EngineTransition |
| `src/engine/chromium.rs` | NEW - ChromiumEngine wrapping cdp::launcher |
| `src/engine/lightpanda.rs` | NEW - LightpandaEngine with CDP client |
| `src/engine/fallback.rs` | NEW - fallback orchestration |
| `src/error.rs` | MODIFY - add EngineStartupFailed, EngineCapabilityUnsupported, EngineFallbackExhausted |
| `src/config.rs` | MODIFY - EngineConfig, nest BrowserConfig |
| `src/observation.rs` | MODIFY - remove Screenshot, add engine fields |
| `src/session/browser_session.rs` | MODIFY - hold `Arc<dyn BrowserEngine>`, engine_history |
| `src/session/mod.rs` | MODIFY - shared Lightpanda pool, EnginePreference |
| `src/actions/capture.rs` | DELETE |
| `src/actions/mod.rs` | MODIFY - remove capture |
| `src/transport/http.rs` | MODIFY - remove screenshot/PDF routes |
| `src/transport/mcp.rs` | MODIFY - remove screenshot/PDF tools |
| `src/cdp/launcher.rs` | MODIFY - Lightpanda binary discovery + spawn |
| `src/lib.rs` | MODIFY - add pub mod engine |
| `.github/workflows/ci.yml` | MODIFY - Lightpanda setup, engine tests |

---

### Task 1: Engine Abstraction Types

**Files:** Create `src/engine/mod.rs`

- [ ] Create `src/engine/` directory
- [ ] Write `src/engine/mod.rs` with BrowserEngine trait, EngineKind, EngineCapabilities, EnginePreference, EngineTransition, EnginePage
- [ ] Verify `cargo check` compiles
- [ ] Commit: `feat(engine): add BrowserEngine trait and core types`

### Task 2: Engine Error Variants

**Files:** Modify `src/error.rs`

- [ ] Add EngineStartupFailed, EngineCapabilityUnsupported, EngineFallbackExhausted variants
- [ ] Add error codes and retryability
- [ ] Add test cases for new variants
- [ ] Run `cargo test --lib error`
- [ ] Commit: `feat(error): add engine error variants`

### Task 3: Engine Configuration

**Files:** Modify `src/config.rs`

- [ ] Add EngineConfig, LightpandaConfig structs
- [ ] Nest BrowserConfig under engine.chromium
- [ ] Update Config struct to use EngineConfig
- [ ] Add backward-compat migration in Config::load()
- [ ] Update existing tests
- [ ] Run `cargo test --lib config`
- [ ] Commit: `feat(config): add EngineConfig with Lightpanda settings`

### Task 4: Observation Engine Fields

**Files:** Modify `src/observation.rs`

- [ ] Add engine_used and engine_fallback fields to Observation
- [ ] Remove ActionResult::Screenshot variant
- [ ] Update success/failure constructors
- [ ] Remove screenshot test
- [ ] Run `cargo test --lib observation`
- [ ] Commit: `feat(observation): add engine tracking, remove Screenshot`

### Task 5: ChromiumEngine Implementation

**Files:** Create `src/engine/chromium.rs`

- [ ] Implement ChromiumEngine struct and BrowserEngine trait
- [ ] Wire to existing cdp::launcher logic
- [ ] Verify compilation
- [ ] Commit: `feat(engine): implement ChromiumEngine`

### Task 6: LightpandaEngine Implementation

**Files:** Create `src/engine/lightpanda.rs`, modify `src/cdp/launcher.rs`

- [ ] Add find_lightpanda_binary and spawn_lightpanda to launcher.rs
- [ ] Implement LightpandaEngine with BrowserEngine trait
- [ ] Set capabilities: no screenshot, no PDF, yes JS, no CORS
- [ ] Verify compilation
- [ ] Commit: `feat(engine): implement LightpandaEngine with binary discovery`

### Task 7: Refactor BrowserSession

**Files:** Modify `src/session/browser_session.rs`

- [ ] Replace `Arc<Mutex<Browser>>` with `Arc<dyn BrowserEngine>`
- [ ] Add engine_history field
- [ ] Update new_tab, get_page, close methods
- [ ] Verify compilation (expect errors in callers - fixed later)
- [ ] Commit: `feat(session): refactor BrowserSession for engine trait`

### Task 8: Fallback Orchestration

**Files:** Create `src/engine/fallback.rs`

- [ ] Implement try_create_session with LightpandaWithFallback strategy
- [ ] Record EngineTransition history
- [ ] Verify compilation
- [ ] Commit: `feat(engine): add session-level fallback orchestration`

### Task 9: Refactor SessionManager

**Files:** Modify `src/session/mod.rs`

- [ ] Update create_session to accept EnginePreference
- [ ] Delegate to try_create_session
- [ ] Update callers
- [ ] Run `cargo test --lib session`
- [ ] Commit: `feat(session): SessionManager uses multi-engine fallback`

### Task 10: Remove Capture Actions

**Files:** Delete `src/actions/capture.rs`, modify `src/actions/mod.rs`

- [ ] Delete capture.rs
- [ ] Remove pub mod capture from mod.rs
- [ ] Verify compilation (expect errors in transports)
- [ ] Commit: `feat(actions): remove capture module (screenshot/PDF)`

### Task 11: Update HTTP Transport

**Files:** Modify `src/transport/http.rs`

- [ ] Remove screenshot/PDF routes from build_router
- [ ] Remove handler functions and request structs
- [ ] Run `cargo test --lib transport::http`
- [ ] Commit: `feat(http): remove screenshot/PDF routes`

### Task 12: Update MCP Transport

**Files:** Modify `src/transport/mcp.rs`

- [ ] Remove screenshot/export_pdf from tools_list
- [ ] Remove dispatch cases
- [ ] Run `cargo test --lib transport::mcp`
- [ ] Commit: `feat(mcp): remove screenshot/PDF tool definitions`

### Task 13: Register Engine Module

**Files:** Modify `src/lib.rs`

- [ ] Add `pub mod engine;`
- [ ] Verify full `cargo check`
- [ ] Commit: `feat(lib): register engine module`

### Task 14: CI/CD Pipeline Updates

**Files:** Modify `.github/workflows/ci.yml`

- [ ] Add Lightpanda binary download step (Linux only)
- [ ] Add engine integration test step
- [ ] Update release build verification
- [ ] Commit: `ci: add Lightpanda setup and engine integration tests`

### Task 15: Final Verification

- [ ] Run `cargo fmt --all -- --check`
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo test --locked`
- [ ] Verify release build: `cargo build --release`
- [ ] Commit any fixes
