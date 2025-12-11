# Probar

**Probar** (Spanish: "to test/prove") is a Rust-native testing framework for WASM games, providing a pure Rust alternative to Playwright/Puppeteer.

## Features

- **Browser Automation**: Chrome DevTools Protocol (CDP) via chromiumoxide
- **WASM Runtime Testing**: Logic-only testing via wasmtime (no browser overhead)
- **Visual Regression**: Image comparison for UI stability
- **Accessibility Auditing**: WCAG compliance checking
- **Deterministic Replay**: Record and replay game sessions
- **Monte Carlo Fuzzing**: Random input generation with invariant checking
- **Type-Safe Selectors**: Compile-time checked entity/component queries

## Getting Started

Add Probar to your `Cargo.toml`:

```toml
[dev-dependencies]
probar = "0.1"
```

Or with specific features:

```toml
[dev-dependencies]
probar = { version = "0.1", features = ["browser", "runtime", "derive"] }
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `browser` | CDP browser automation (chromiumoxide, tokio) |
| `runtime` | WASM runtime testing (wasmtime) |
| `derive` | Type-safe derive macros (probar-derive) |

## Toyota Way Principles

Probar is built on Toyota Production System principles:

- **Poka-Yoke** (Mistake-Proofing): Type-safe selectors prevent runtime errors
- **Muda** (Waste Elimination): Zero-copy memory views for efficiency
- **Jidoka** (Autonomation): Fail-fast with configurable error handling
- **Genchi Genbutsu** (Go and See): Abstract drivers allow swapping implementations
- **Heijunka** (Level Loading): Superblock scheduling for consistent performance
