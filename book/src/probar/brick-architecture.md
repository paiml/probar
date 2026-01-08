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

## See Also

- [PROBAR-SPEC-009: Brick Architecture](../specs/brick-architecture.md)
- [TUI Testing](./tui-testing.md)
- [Accessibility Testing](./accessibility.md)
