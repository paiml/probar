# Assertions

Probar provides a rich set of assertions for testing game state.

## Basic Assertions

```rust
use jugar_probar::Assertion;

// Equality
let eq = Assertion::equals(&actual, &expected);
assert!(eq.passed);
assert_eq!(eq.message, "Values are equal");

// Inequality
let ne = Assertion::not_equals(&a, &b);

// Boolean
let truthy = Assertion::is_true(condition);
let falsy = Assertion::is_false(condition);
```

## Numeric Assertions

```rust
// Range check
let range = Assertion::in_range(value, min, max);

// Approximate equality (for floats)
let approx = Assertion::approx_eq(3.14159, std::f64::consts::PI, 0.001);

// Greater/Less than
let gt = Assertion::greater_than(value, threshold);
let lt = Assertion::less_than(value, threshold);
let gte = Assertion::greater_than_or_equal(value, threshold);
let lte = Assertion::less_than_or_equal(value, threshold);
```

## Collection Assertions

```rust
// Contains
let contains = Assertion::contains(&collection, &item);

// Length
let len = Assertion::has_length(&vec, expected_len);

// Empty
let empty = Assertion::is_empty(&vec);
let not_empty = Assertion::is_not_empty(&vec);

// All match predicate
let all = Assertion::all_match(&vec, |x| x > 0);

// Any match predicate
let any = Assertion::any_match(&vec, |x| x == 42);
```

## String Assertions

```rust
// Contains substring
let contains = Assertion::string_contains(&text, "expected");

// Starts/ends with
let starts = Assertion::starts_with(&text, "prefix");
let ends = Assertion::ends_with(&text, "suffix");

// Regex match
let matches = Assertion::matches_regex(&text, r"\d{3}-\d{4}");

// Length
let len = Assertion::string_length(&text, expected_len);
```

## Option/Result Assertions

```rust
// Option
let some = Assertion::is_some(&option_value);
let none = Assertion::is_none(&option_value);

// Result
let ok = Assertion::is_ok(&result);
let err = Assertion::is_err(&result);
```

## Custom Assertions

```rust
// Create custom assertion
fn assert_valid_score(score: u32) -> Assertion {
    Assertion::custom(
        score <= 10,
        format!("Score {} should be <= 10", score),
    )
}

// Use it
let assertion = assert_valid_score(game.score);
assert!(assertion.passed);
```

## Assertion Result

All assertions return an `Assertion` struct:

```rust
pub struct Assertion {
    pub passed: bool,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}
```

## Combining Assertions

```rust
// All must pass
let all_pass = Assertion::all(&[
    Assertion::in_range(x, 0.0, 800.0),
    Assertion::in_range(y, 0.0, 600.0),
    Assertion::greater_than(health, 0),
]);

// Any must pass
let any_pass = Assertion::any(&[
    Assertion::equals(&state, &State::Running),
    Assertion::equals(&state, &State::Paused),
]);
```

## Game-Specific Assertions

```rust
// Entity exists
let exists = Assertion::entity_exists(&world, entity_id);

// Component value
let has_component = Assertion::has_component::<Position>(&world, entity);

// Position bounds
let in_bounds = Assertion::position_in_bounds(
    position,
    Bounds::new(0.0, 0.0, 800.0, 600.0),
);

// Collision occurred
let collided = Assertion::entities_colliding(&world, entity_a, entity_b);
```

## Example Test

```rust
#[test]
fn test_game_state_validity() {
    let mut platform = WebPlatform::new_for_test(WebConfig::default());

    // Advance game
    for _ in 0..100 {
        platform.advance_frame(1.0 / 60.0);
    }

    let state = platform.get_game_state();

    // Multiple assertions
    assert!(Assertion::in_range(state.ball.x, 0.0, 800.0).passed);
    assert!(Assertion::in_range(state.ball.y, 0.0, 600.0).passed);
    assert!(Assertion::in_range(state.paddle_left.y, 0.0, 600.0).passed);
    assert!(Assertion::in_range(state.paddle_right.y, 0.0, 600.0).passed);
    assert!(Assertion::lte(state.score_left, 10).passed);
    assert!(Assertion::lte(state.score_right, 10).passed);
}
```
