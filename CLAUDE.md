# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## CRITICAL CONSTRAINTS

### ABSOLUTE ZERO JAVASCRIPT

```
❌ FORBIDDEN                         ✅ REQUIRED
─────────────────────────────────────────────────────
• JavaScript files (.js/.ts)         • Pure Rust only
• npm/node_modules/package.json      • wasm32-unknown-unknown
• Any JS bundler                     • web-sys (Rust bindings)
• JS interop beyond web-sys          • Single .wasm binary output
```

**Rationale**: JavaScript introduces non-determinism and GC pauses. Probar compiles to a single `.wasm` file with ZERO JS for testing WASM games.

### Crates.io Name

Published as `jugar-probar` on crates.io (the name `probar` was taken).

```toml
[dev-dependencies]
jugar-probar = "0.2"
```

```rust
use jugar_probar::prelude::*;
```

## Project Overview

Probar (Spanish: "to test/prove") is a Rust-native testing framework for WASM games. It provides:
- Browser automation via Chrome DevTools Protocol (CDP)
- WASM runtime testing via wasmtime
- Visual regression testing
- Accessibility auditing (WCAG)
- Deterministic simulation and replay
- Monte Carlo fuzzing

## Build Commands

```bash
# Build
cargo build                    # Host target (dev)
cargo build --all-features     # All features enabled

# Test
cargo test                     # Run all tests
cargo test --all-features      # Run all tests with all features

# Quality
cargo clippy -- -D warnings    # Lint with strict warnings
cargo fmt --check              # Check formatting
```

## Architecture

### Crate Structure

```
crates/
├── probar/           # Main testing framework
└── probar-derive/    # Proc-macro crate for type-safe selectors
```

### Feature Flags

| Feature | Description |
|---------|-------------|
| `browser` | Enable CDP browser control (chromiumoxide, tokio) |
| `runtime` | Enable WASM runtime (wasmtime) for logic testing |
| `derive` | Enable derive macros for type-safe ECS selectors |

### Key Components

- **Locator API**: Playwright-style element selection with auto-waiting
- **WasmRuntime**: Logic-only testing without browser overhead
- **StateBridge**: Game state snapshot and diffing
- **InputFuzzer**: Monte Carlo input generation
- **VisualRegressionTester**: Image comparison for UI testing

## Toyota Way Principles

- **Poka-Yoke**: Type-safe selectors prevent stringly-typed errors at compile time
- **Muda**: Zero-copy memory views eliminate serialization waste
- **Jidoka**: Fail-fast with soft Jidoka (LogAndContinue vs Stop)
- **Genchi Genbutsu**: Abstract ProbarDriver allows swapping browser implementations
- **Heijunka**: Superblock tiling for amortized scheduling

## Quality Standards

- **95% minimum test coverage**
- **Zero tolerance for panic paths** (clippy: deny unwrap_used, expect_used)
- **Comprehensive inline documentation**
