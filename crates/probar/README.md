# jugar-probar

**Probar** (Spanish: "to test/prove") is a pure Rust testing framework for WASM games that provides full Playwright feature parity while adding WASM-native capabilities.

## Why Probar?

| Aspect | Playwright | Probar |
|--------|-----------|--------|
| **Language** | TypeScript | Pure Rust |
| **Browser** | Required (Chromium) | Not needed |
| **Game State** | Black box (DOM only) | Direct API access |
| **CI Setup** | Node.js + browser | Just `cargo test` |
| **Zero JS** | Violates constraint | Pure Rust |

## Features

### Playwright Parity
- CSS, text, testid, XPath, role-based locators
- All standard assertions (visibility, text, count)
- All actions (click, fill, type, hover, drag)
- Auto-waiting with configurable timeouts
- Network interception and mobile emulation

### WASM-Native Extensions
- **Zero-copy memory views** - Direct WASM memory inspection
- **Type-safe entity selectors** - Compile-time verified game object access
- **Deterministic replay** - Record inputs with seed, replay identically
- **Invariant fuzzing** - Concolic testing for game invariants
- **Frame-perfect timing** - Fixed timestep control
- **WCAG accessibility** - Color contrast and photosensitivity checking

## Quick Start

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
jugar-probar = { path = "../jugar-probar" }
```

Write tests using `WebPlatform` directly:

```rust
use jugar_probar::Assertion;
use jugar_web::{WebConfig, WebPlatform};

#[test]
fn test_game_starts() {
    let config = WebConfig::new(800, 600);
    let mut platform = WebPlatform::new_for_test(config);

    // Run a frame
    let output = platform.frame(0.0, "[]");

    // Verify output
    assert!(output.contains("commands"));

    // Use Probar assertions
    let assertion = Assertion::in_range(60.0, 0.0, 100.0);
    assert!(assertion.passed);
}
```

## Examples

```bash
# Deterministic simulation with replay verification
cargo run --example pong_simulation -p jugar-probar

# Playwright-style locator API demo
cargo run --example locator_demo -p jugar-probar

# WCAG accessibility checking
cargo run --example accessibility_demo -p jugar-probar
```

### Example Output

```
=== Probar Pong Simulation Demo ===

--- Demo 1: Pong Simulation ---
Initial state:
  Ball: (400.0, 300.0)
  Paddles: P1=300.0, P2=300.0
  Score: 0 - 0

Simulating 300 frames...

Final state after 300 frames:
  Ball: (234.5, 412.3)
  Paddles: P1=180.0, P2=398.2
  Score: 2 - 1
  State valid: true

--- Demo 2: Deterministic Replay ---
Recording simulation (seed=42, frames=500)...
  Completed: true
  Final hash: 6233835744931225727

Replaying simulation...
  Determinism verified: true
  Hashes match: true
```

## Architecture

### Dual-Runtime Strategy

```
┌─────────────────────────────────┐     ┌─────────────────────────────────┐
│  WasmRuntime (wasmtime)         │     │  BrowserController (Chrome)     │
│  ─────────────────────────      │     │  ─────────────────────────      │
│  Purpose: LOGIC-ONLY testing    │     │  Purpose: GOLDEN MASTER         │
│                                 │     │                                 │
│  ✓ Unit tests                   │     │  ✓ E2E tests                    │
│  ✓ Deterministic replay         │     │  ✓ Visual regression            │
│  ✓ Invariant fuzzing            │     │  ✓ Browser compatibility        │
│  ✓ Performance benchmarks       │     │  ✓ Production parity            │
│                                 │     │                                 │
│  ✗ NOT for rendering            │     │  This is the SOURCE OF TRUTH    │
│  ✗ NOT for browser APIs         │     │  for "does it work?"            │
└─────────────────────────────────┘     └─────────────────────────────────┘
```

### Toyota Way Principles

| Principle | How Probar Applies It |
|-----------|----------------------|
| **Poka-Yoke** | Type-safe selectors make typos impossible at compile time |
| **Muda** | Zero-copy memory views eliminate serialization waste |
| **Genchi Genbutsu** | ProbarDriver abstraction allows swapping browser backends |
| **Andon Cord** | Fail-fast mode stops on first critical failure |
| **Jidoka** | Quality built into the type system |

## API Overview

### Assertions

```rust
use jugar_probar::Assertion;

// Value equality
let eq = Assertion::equals(&actual, &expected);

// Numeric range
let range = Assertion::in_range(value, 0.0, 100.0);

// Boolean checks
let truthy = Assertion::is_true(condition);

// Approximate equality (floats)
let approx = Assertion::approx_eq(3.14159, std::f64::consts::PI, 0.001);
```

### Simulation

```rust
use jugar_probar::{run_simulation, run_replay, SimulationConfig, InputEvent};

// Record a simulation
let config = SimulationConfig::new(seed, frames);
let recording = run_simulation(config, |frame| {
    vec![InputEvent::key_press("ArrowUp")]
});

// Replay and verify determinism
let replay = run_replay(&recording);
assert!(replay.determinism_verified);
```

### Fuzzing

```rust
use jugar_probar::{RandomWalkAgent, Seed};

let seed = Seed::from_u64(12345);
let mut agent = RandomWalkAgent::new(seed);

// Generate random inputs
let inputs = agent.next_inputs();
```

## Running Tests

```bash
# Run all Probar E2E tests for Pong
cargo test -p jugar-web --test probar_pong

# Run with verbose output
cargo test -p jugar-web --test probar_pong -- --nocapture

# Via Makefile
make test-e2e
make test-e2e-verbose
```

## Test Coverage

The Probar test suite (`probar_pong.rs`) includes 39 tests covering:

| Suite | Tests | Coverage |
|-------|-------|----------|
| Pong WASM Game (Core) | 6 | WASM loading, rendering, input |
| Pong Demo Features | 22 | Game modes, HUD, AI widgets |
| Release Readiness | 11 | Stress tests, performance, edge cases |

## License

MIT OR Apache-2.0
