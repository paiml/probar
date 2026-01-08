# Brick Architecture

Brick Architecture enables widget-level testing with built-in assertions, performance budgets, and verification. Each "brick" is a testable widget component that declares its testing requirements upfront.

## Overview

The Brick trait allows widgets to:
- Declare **assertions** they must satisfy (e.g., text visible, contrast ratio)
- Set **performance budgets** (e.g., 16ms for 60fps rendering)
- Define **verification** logic that runs before rendering

## Quick Start

```rust
use jugar_probar::brick::{Brick, BrickAssertion, BrickBudget, BrickVerification};

struct MyButton {
    label: String,
}

impl Brick for MyButton {
    fn brick_name(&self) -> &'static str {
        "MyButton"
    }

    fn assertions(&self) -> &[BrickAssertion] {
        &[
            BrickAssertion::TextVisible,
            BrickAssertion::ContrastRatio(4.5),
        ]
    }

    fn budget(&self) -> BrickBudget {
        BrickBudget::uniform(16) // 60fps target
    }

    fn verify(&self) -> BrickVerification {
        BrickVerification::pass()
    }
}
```

## Assertions

Built-in assertions include:

| Assertion | Description |
|-----------|-------------|
| `TextVisible` | Text content is visible |
| `ContrastRatio(f64)` | WCAG contrast ratio minimum |
| `MinSize { w, h }` | Minimum dimensions |
| `Accessible` | Meets accessibility requirements |
| `Custom { name, validator_id }` | Custom validation logic |

## Performance Budgets

```rust
// Uniform budget: all operations share 16ms
BrickBudget::uniform(16)

// Tiered budget: layout gets more time than paint
BrickBudget {
    layout_ms: 8,
    paint_ms: 4,
    total_ms: 16,
}
```

## Verification

```rust
fn verify(&self) -> BrickVerification {
    let mut passed = Vec::new();
    let mut failed = Vec::new();

    // Check contrast ratio
    if self.contrast_ratio() >= 4.5 {
        passed.push(BrickAssertion::ContrastRatio(4.5));
    } else {
        failed.push((
            BrickAssertion::ContrastRatio(4.5),
            format!("Contrast {} < 4.5", self.contrast_ratio()),
        ));
    }

    BrickVerification {
        passed,
        failed,
        verification_time: Duration::from_micros(100),
    }
}
```

## TUI Integration

Brick Architecture integrates seamlessly with ratatui widgets:

```rust
use jugar_probar::brick::tui::BrickWidget;

let button = BrickWidget::new("Submit")
    .with_assertion(BrickAssertion::TextVisible)
    .with_budget(BrickBudget::uniform(8));

// Verify before render
assert!(button.can_render());

// Render to terminal
frame.render_widget(button, area);
```

## Best Practices

1. **Declare assertions upfront** - Define what each widget must satisfy
2. **Set realistic budgets** - 16ms for 60fps, 8ms for 120fps
3. **Verify before render** - Call `can_render()` to ensure constraints are met
4. **Use presets** - Standard button, input, and container presets available

## BrickHouse: Budgeted Composition

Compose multiple bricks with a total performance budget. The BrickHouse enforces Jidoka (stop-the-line) principles - if any brick exceeds its budget, rendering halts.

```rust
use jugar_probar::brick_house::{BrickHouse, BrickHouseBuilder};
use std::sync::Arc;

// Build a house with bricks and budgets
let house = BrickHouseBuilder::new("whisper-app")
    .budget_ms(1000)  // 1 second total
    .brick(Arc::new(status_brick), 50)      // 50ms for status
    .brick(Arc::new(waveform_brick), 100)   // 100ms for waveform
    .brick(Arc::new(transcription_brick), 600) // 600ms for transcription
    .build()?;

// Verify all bricks can render
assert!(house.can_render());

// Render with budget tracking
let html = house.render()?;

// Check budget report
if let Some(report) = house.last_report() {
    println!("Utilization: {:.1}%", report.utilization());
    assert!(report.within_budget());
}
```

## web_sys_gen: Zero Hand-Written web_sys

The `web_sys_gen` module provides generated abstractions that replace hand-written `web_sys` calls. This ensures:

1. **Traceability** - All code is derived from brick specifications
2. **Consistency** - Error handling is uniform
3. **No hand-written web_sys** - Application code stays clean

### Performance Timing

```rust
use jugar_probar::brick::web_sys_gen::PerformanceTiming;

// Get high-resolution timestamp
let start = PerformanceTiming::now();

// Measure an operation with automatic timing
let (result, duration_ms) = PerformanceTiming::measure(|| {
    expensive_computation()
});

println!("Operation took {:.2}ms", duration_ms);
```

### Custom Events

```rust
use jugar_probar::brick::web_sys_gen::{CustomEventDispatcher, EventDetail};

// Create a dispatcher for your event type
let dispatcher = CustomEventDispatcher::new("transcription-complete");

// Dispatch with various detail types
dispatcher.dispatch()?;  // No detail
dispatcher.dispatch_with_detail(EventDetail::string("Done"))?;
dispatcher.dispatch_with_detail(EventDetail::number(42.0))?;
dispatcher.dispatch_with_detail(EventDetail::json(&my_data))?;
```

### Blob URLs

```rust
use jugar_probar::brick::web_sys_gen::BlobUrl;

// Create a blob URL from JavaScript code
let worker_code = "self.onmessage = (e) => self.postMessage(e.data * 2);";
let url = BlobUrl::from_js_code(worker_code)?;

// Use the URL to create a Worker
// ...

// Clean up when done
BlobUrl::revoke(&url)?;
```

### Fetch Client

```rust
use jugar_probar::brick::web_sys_gen::FetchClient;

let client = FetchClient::new();

// Fetch bytes from a URL (async)
let bytes = client.fetch_bytes("https://example.com/data.bin").await?;
```

## Examples

Run the brick examples:

```bash
# Basic brick architecture demo
cargo run --example brick_demo -p jugar-probar

# Visual TUI demo - shows bricks "lighting up" as tests pass
cargo run --example brick_tui_demo -p jugar-probar

# web_sys_gen utilities demo
cargo run --example web_sys_gen_demo -p jugar-probar
```

### Visual Brick Demo Output

The `brick_tui_demo` shows a live visualization of bricks being verified:

```
  ========================================
    BRICK ARCHITECTURE - VERIFICATION COMPLETE
  ========================================

    +-------+-------+-------+-------+-------+-------+
    |  S S  |  W W  |  A A  |  T T  |  E E  |  M M  |
    | Statu | Wave  | Audio | Trans | Error | Model |
    +-------+-------+-------+-------+-------+-------+
         [GREEN]  [GREEN]  [GREEN]  [GREEN]  [RED]   [GREEN]

  Final Verification Results
  -------------------------
    Status       [PASS] budget:  50ms actual:  12ms assertions: 3/3
    Wave         [PASS] budget: 100ms actual:  45ms assertions: 3/3
    Audio        [PASS] budget: 150ms actual:  67ms assertions: 3/3
    Trans        [PASS] budget: 600ms actual: 234ms assertions: 3/3
    Error        [FAIL] budget:  50ms actual:  15ms assertions: 1/2
    Model        [PASS] budget: 200ms actual:  89ms assertions: 3/3

  Summary
  -------
    Bricks: 5 passed, 1 failed
    Budget Utilization: 40.2% (healthy)
```

## See Also

- [PROBAR-SPEC-009: Brick Architecture](../specs/brick-architecture.md)
- [TUI Testing](./tui-testing.md)
- [Accessibility Testing](./accessibility.md)
