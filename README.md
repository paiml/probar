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

## Key Features

### GUI Coverage Tracking

Probar introduces **GUI Coverage** - a new paradigm for measuring UI test completeness:

```rust
use probar::gui_coverage;

// Define what needs testing (one line!)
let mut gui = gui_coverage! {
    buttons: ["start", "pause", "quit"],
    screens: ["title", "playing", "game_over"]
};

// Record interactions during tests
gui.click("start");
gui.visit("title");

// Get coverage - one line!
println!("{}", gui.summary());  // "GUI: 33% (1/3 elements, 1/3 screens)"
assert!(gui.meets(80.0));       // Fail if below 80%
```

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

| Feature | Description |
|---------|-------------|
| **Browser Automation** | Chrome DevTools Protocol (CDP) via chromiumoxide |
| **WASM Runtime Testing** | Logic-only testing via wasmtime (no browser overhead) |
| **TUI Testing** | Frame capture and assertion for terminal UIs |
| **GUI Coverage** | Provable UI element and interaction coverage |
| **Visual Regression** | Image comparison for UI stability |
| **Accessibility Auditing** | WCAG compliance checking |
| **Deterministic Replay** | Record and replay sessions with seed control |
| **Monte Carlo Fuzzing** | Random input generation with invariant checking |

## Quick Start

```rust
use probar::prelude::*;

#[test]
fn test_calculator_gui() {
    // Create driver (works for TUI or WASM)
    let mut driver = WasmDriver::new();

    // Track GUI coverage
    let mut gui = gui_coverage! {
        buttons: ["btn-7", "btn-times", "btn-6", "btn-equals"],
        screens: ["calculator"]
    };

    // Test: 7 × 6 = 42
    driver.type_input("7 * 6");
    gui.click("btn-7");
    gui.click("btn-times");
    gui.click("btn-6");

    driver.click_equals();
    gui.click("btn-equals");
    gui.visit("calculator");

    assert_eq!(driver.get_result(), "42");
    assert!(gui.is_complete());  // 100% GUI coverage!
}
```

## Installation

Add Probar to your `Cargo.toml`:

```toml
[dev-dependencies]
probar = "0.1"

# With TUI testing (default)
probar = { version = "0.1", features = ["tui"] }

# With browser automation
probar = { version = "0.1", features = ["browser"] }
```

Or install the CLI:

```bash
cargo install probar-cli
```

## Usage

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage
cargo llvm-cov --html

# Watch mode
cargo watch -x test
```

### GUI Coverage Example

```bash
# Run the GUI coverage example
cargo run --example gui_coverage
```

Output:
```
=== GUI Coverage Example ===

1. Using gui_coverage! macro (simplest)...
   GUI: 50% (1/3 elements, 2/3 screens)

2. Calculator preset (20 buttons + 2 screens)...
   GUI: 60% (14/20 elements, 1/2 screens)

3. Achieving 100% coverage...
   GUI: 100% (3/3 elements, 1/1 screens)
   Complete? true
```

### Showcase Calculator

The repository includes a full showcase calculator demonstrating 100% test coverage:

```bash
# Run GUI coverage report
cargo run -p showcase-calculator --example gui_coverage_report

# Run TUI version
cargo run -p showcase-calculator --example calculator_tui

# View WASM version
cd crates/showcase-calculator/www && python3 -m http.server 8080
# Open http://localhost:8080
```

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `tui` | TUI testing support (default) | ratatui, crossterm |
| `browser` | CDP browser automation | chromiumoxide, tokio |
| `runtime` | WASM runtime testing | wasmtime |
| `derive` | Type-safe derive macros | probar-derive |

## Probar Principles

Probar is built on pragmatic testing principles:

| Principle | Description |
|-----------|-------------|
| **Error Prevention** | Type-safe selectors prevent runtime errors |
| **Efficiency** | Zero-copy memory views minimize overhead |
| **Fail-Fast** | Immediate feedback on test failures |
| **Balanced Testing** | Even coverage across all UI elements |
| **Continuous Improvement** | Mutation testing for test quality |

## Documentation

- **[Book](https://paiml.github.io/probar/)** - Comprehensive guide
- **[API Docs](https://docs.rs/probar)** - Rust documentation
- **[GUI Coverage Guide](book/src/probar/ux-coverage.md)** - GUI coverage tutorial
- **[Examples](crates/probar/examples/)** - 20+ runnable examples

## Examples

| Example | Description |
|---------|-------------|
| `gui_coverage` | GUI coverage tracking |
| `soft_assertions` | Collect multiple failures |
| `retry_assertions` | Retry with backoff |
| `pong_simulation` | Game simulation testing |
| `accessibility_demo` | WCAG compliance |
| `watch_mode` | Hot-reload testing |

Run any example:
```bash
cargo run --example <name>
```

## Project Structure

```
probar/
├── crates/
│   ├── probar/              # Core library
│   ├── probar-cli/          # Command-line interface
│   ├── probar-derive/       # Derive macros
│   └── showcase-calculator/ # 100% coverage demo
├── book/                    # mdBook documentation
└── docs/                    # Specifications
```

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

### Quality Gates

```bash
make lint      # Clippy checks
make test      # All tests
make coverage  # Coverage report
```

## License

MIT OR Apache-2.0

---

<p align="center">
  <strong>Probar</strong> - by <a href="https://paiml.com">Pragmatic AI Labs</a>
</p>
