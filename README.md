<p align="center">
  <img src="https://raw.githubusercontent.com/paiml/probar/main/docs/assets/probar-hero.svg?v=2" alt="Probar - Playwright-Compatible Testing for WASM + TUI" width="800"/>
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

## Installation

Add Probar to your `Cargo.toml`:

```toml
[dependencies]
probar = "0.1"

# Optional: Enable browser automation
[dependencies.probar]
version = "0.1"
features = ["browser"]

# Optional: Enable WASM runtime testing
[dependencies.probar]
version = "0.1"
features = ["runtime"]
```

Or install the CLI:

```bash
cargo install probar-cli
```

## Usage

### Running Tests

```bash
# Run all tests
probar test

# Run tests with filter
probar test --filter "game::*"

# Run with coverage
probar test --coverage

# Watch mode for development
probar test --watch
```

### Recording Test Sessions

```bash
# Record as GIF
probar record my_test --gif

# Record as MP4
probar record my_test --mp4 --fps 30
```

### Generating Reports

```bash
# HTML coverage report
probar report --html -o coverage.html

# LCOV format
probar report --lcov -o coverage.lcov
```

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `browser` | CDP browser automation | chromiumoxide, tokio |
| `runtime` | WASM runtime testing | wasmtime |
| `derive` | Type-safe derive macros | probar-derive |
| `tui` | TUI testing support (default) | ratatui, crossterm |

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

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork the repository** and create your branch from `main`
2. **Run tests** before submitting: `cargo test`
3. **Ensure formatting**: `cargo fmt`
4. **Check lints**: `cargo clippy --all-targets --all-features`
5. **Update documentation** if you change public APIs
6. **Add tests** for new functionality

### Development Setup

```bash
git clone https://github.com/paiml/probar.git
cd probar
cargo build
cargo test
```

### Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## License

MIT OR Apache-2.0
