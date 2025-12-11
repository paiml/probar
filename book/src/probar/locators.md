# Locators

Probar provides Playwright-style locators for finding game elements.

## Basic Locators

```rust
use jugar_probar::locator::*;

// By ID
let player = Locator::id("player");

// By name
let score = Locator::name("score_display");

// By component type
let balls = Locator::component::<Ball>();

// By tag
let enemies = Locator::tag("enemy");
```

## Entity Queries

```rust
let platform = WebPlatform::new_for_test(config);

// Find single entity
let player = platform.locate(Locator::id("player"))?;
let pos = platform.get_position(player);

// Find all matching
let coins: Vec<Entity> = platform.locate_all(Locator::tag("coin"));
assert_eq!(coins.len(), 5);

// First matching
let first_enemy = platform.locate_first(Locator::tag("enemy"));
```

## Compound Locators

```rust
// AND - must match all
let armed_enemy = Locator::tag("enemy")
    .and(Locator::has_component::<Weapon>());

// OR - match any
let interactable = Locator::tag("door")
    .or(Locator::tag("chest"));

// NOT - exclude
let non_player = Locator::component::<Character>()
    .not(Locator::id("player"));
```

## Spatial Locators

```rust
// Within radius
let nearby = Locator::within_radius(player_pos, 100.0);

// In bounds
let visible = Locator::in_bounds(screen_bounds);

// Nearest to point
let closest_enemy = Locator::nearest_to(player_pos)
    .with_filter(Locator::tag("enemy"));
```

## Component-Based Locators

```rust
// Has specific component
let physics_entities = Locator::has_component::<RigidBody>();

// Component matches predicate
let low_health = Locator::component_matches::<Health>(|h| h.value < 20);

// Has all components
let complete_entities = Locator::has_all_components::<(
    Position,
    Velocity,
    Sprite,
)>();
```

## Type-Safe Locators (with derive)

Using `jugar-probar-derive` for compile-time checked selectors:

```rust
use jugar_probar_derive::Entity;

#[derive(Entity)]
#[entity(id = "player")]
struct Player;

#[derive(Entity)]
#[entity(tag = "enemy")]
struct Enemy;

// Compile-time verified
let player = platform.locate::<Player>()?;
let enemies = platform.locate_all::<Enemy>();
```

## Waiting for Elements

```rust
// Wait for entity to exist
let boss = platform.wait_for(
    Locator::id("boss"),
    Duration::from_secs(5),
)?;

// Wait for condition
platform.wait_until(
    || platform.locate(Locator::id("door")).is_some(),
    Duration::from_secs(2),
)?;
```

## Locator Chains

```rust
// Find children
let player_weapon = Locator::id("player")
    .child(Locator::tag("weapon"));

// Find parent
let weapon_owner = Locator::id("sword")
    .parent();

// Find siblings
let adjacent_tiles = Locator::id("current_tile")
    .siblings();
```

## Actions on Located Elements

```rust
let button = platform.locate(Locator::id("start_button"))?;

// Get info
let pos = platform.get_position(button);
let bounds = platform.get_bounds(button);
let visible = platform.is_visible(button);

// Interact
platform.click(button);
platform.hover(button);

// Check state
let enabled = platform.is_enabled(button);
let focused = platform.is_focused(button);
```

## Example Test

```rust
#[test]
fn test_coin_collection() {
    let mut platform = WebPlatform::new_for_test(config);

    // Count initial coins
    let initial_coins = platform.locate_all(Locator::tag("coin")).len();
    assert_eq!(initial_coins, 5);

    // Move player to first coin
    let first_coin = platform.locate_first(Locator::tag("coin")).unwrap();
    let coin_pos = platform.get_position(first_coin);

    // Simulate movement
    move_player_to(&mut platform, coin_pos);

    // Coin should be collected
    let remaining_coins = platform.locate_all(Locator::tag("coin")).len();
    assert_eq!(remaining_coins, 4);

    // Score should increase
    let score_display = platform.locate(Locator::id("score")).unwrap();
    let score_text = platform.get_text(score_display);
    assert!(score_text.contains("10"));
}
```
