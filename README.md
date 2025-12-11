# Probar

[![Crates.io](https://img.shields.io/crates/v/probar.svg)](https://crates.io/crates/probar)
[![Documentation](https://docs.rs/probar/badge.svg)](https://docs.rs/probar)
[![License](https://img.shields.io/crates/l/probar.svg)](LICENSE)

**Probar** (Spanish: "to test/prove") is a Rust-native testing framework for WASM games, providing a pure Rust alternative to Playwright/Puppeteer.

## Features

- **Browser Automation**: Chrome DevTools Protocol (CDP) via chromiumoxide
- **WASM Runtime Testing**: Logic-only testing via wasmtime (no browser overhead)
- **Visual Regression**: Image comparison for UI stability
- **Accessibility Auditing**: WCAG compliance checking (contrast, flash, keyboard nav)
- **Deterministic Replay**: Record and replay game sessions
- **Monte Carlo Fuzzing**: Random input generation with invariant checking
- **Type-Safe Selectors**: Compile-time checked entity/component queries

## Quick Start

```rust
use probar::prelude::*;

// Browser-based testing (requires `browser` feature)
#[cfg(feature = "browser")]
async fn test_game_start() -> ProbarResult<()> {
    let config = BrowserConfig::default().with_viewport(800, 600);
    let browser = Browser::launch(config).await?;
    let mut page = browser.new_page().await?;

    page.goto("http://localhost:8080/game").await?;
    page.wait_for_wasm_ready().await?;

    // Playwright-style locators
    let start_button = Locator::new("button").with_text("Start Game");
    start_button.click()?;

    // Smart assertions
    let score = Locator::new("[data-testid='score']");
    expect(score).to_have_text("0").validate("0")?;

    Ok(())
}

// Logic-only testing (requires `runtime` feature)
#[cfg(feature = "runtime")]
fn test_physics() -> ProbarResult<()> {
    let config = RuntimeConfig::default()
        .with_wasm_path("target/wasm32-unknown-unknown/release/game.wasm");

    let mut runtime = WasmRuntime::new(config)?;

    // Advance simulation
    for _ in 0..60 {
        runtime.step()?;
    }

    // Query game state
    let entities = runtime.entities()?;
    assert!(!entities.is_empty());

    Ok(())
}
```

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `browser` | CDP browser automation | chromiumoxide, tokio |
| `runtime` | WASM runtime testing | wasmtime |
| `derive` | Type-safe derive macros | probar-derive |

## Toyota Way Principles

Probar is built on Toyota Production System principles:

- **Poka-Yoke** (Mistake-Proofing): Type-safe selectors prevent runtime errors
- **Muda** (Waste Elimination): Zero-copy memory views for efficiency
- **Jidoka** (Autonomation): Fail-fast with configurable error handling
- **Genchi Genbutsu** (Go and See): Abstract drivers allow swapping implementations
- **Heijunka** (Level Loading): Superblock scheduling for consistent performance

## Documentation

- [Book Chapter](book/src/probar/why-probar.md)
- [WASM Testing Spec](docs/specifications/probar-wasm-testing-spec.md)
- [Coverage Tooling Spec](docs/specifications/probar-wasm-coverage-tooling.md)

## License

MIT OR Apache-2.0
