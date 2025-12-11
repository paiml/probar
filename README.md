<p align="center">
  <img src="https://raw.githubusercontent.com/paiml/probar/main/docs/assets/probar-hero.svg" alt="Probar Testing Framework" width="800"/>
</p>

<h1 align="center">Probar</h1>

<p align="center">
  <strong>Playwright-Compatible Testing for WASM + TUI Applications</strong><br>
  <em>Pure Rust • Zero JavaScript • Games • Simulations • Web Apps</em>
</p>

<p align="center">
  <a href="https://github.com/paiml/probar/actions/workflows/ci.yml">
    <img src="https://github.com/paiml/probar/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://crates.io/crates/probar">
    <img src="https://img.shields.io/crates/v/probar.svg" alt="Crates.io">
  </a>
  <a href="https://docs.rs/probar">
    <img src="https://docs.rs/probar/badge.svg" alt="Documentation">
  </a>
  <a href="https://paiml.github.io/probar/">
    <img src="https://img.shields.io/badge/book-mdbook-blue" alt="Book">
  </a>
</p>

---

**Probar** (Spanish: "to test/prove") is a **Playwright-compatible** testing framework written in **pure Rust**. It provides comprehensive testing for:

- **WASM Applications** - Games, simulations, web apps running in browsers
- **TUI Applications** - Terminal interfaces built with ratatui/crossterm
- **Headless Testing** - Fast CI/CD without browser overhead

## Features

### Playwright-Compatible API
```rust
// Familiar Playwright-style locators and assertions
let button = page.locator("button").with_text("Start Game");
button.click().await?;

expect(&score).to_have_text("100").await?;
```

### Test Targets

| Target | Description | Use Case |
|--------|-------------|----------|
| **WASM Browser** | Chrome DevTools Protocol (CDP) | Games, web apps, simulations |
| **WASM Headless** | wasmtime runtime | Fast CI, logic testing |
| **TUI** | ratatui/crossterm backends | Terminal applications |

### Core Capabilities

- **Browser Automation**: Chrome DevTools Protocol (CDP) via chromiumoxide
- **WASM Runtime Testing**: Logic-only testing via wasmtime (no browser overhead)
- **TUI Testing**: Frame capture and assertion for terminal UIs
- **Visual Regression**: Image comparison for UI stability
- **Accessibility Auditing**: WCAG compliance checking
- **Deterministic Replay**: Record and replay sessions with seed control
- **Monte Carlo Fuzzing**: Random input generation with invariant checking
- **100% UX Coverage**: Provable UI element and interaction coverage

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
