# Simulation

Probar provides deterministic game simulation for testing.

## Basic Simulation

```rust
use jugar_probar::{run_simulation, SimulationConfig};

let config = SimulationConfig::new(seed, num_frames);
let result = run_simulation(config, |frame| {
    // Return inputs for this frame
    vec![]  // No inputs
});

assert!(result.completed);
println!("Final state hash: {}", result.state_hash);
```

## Simulation with Inputs

```rust
use jugar_probar::{run_simulation, SimulationConfig, InputEvent};

let config = SimulationConfig::new(42, 300);
let result = run_simulation(config, |frame| {
    // Move paddle up for first 100 frames
    if frame < 100 {
        vec![InputEvent::key_held("KeyW")]
    } else {
        vec![]
    }
});
```

## Input Events

```rust
// Keyboard
InputEvent::key_press("Space")      // Just pressed
InputEvent::key_held("KeyW")        // Held down
InputEvent::key_release("Escape")   // Just released

// Mouse
InputEvent::mouse_move(400.0, 300.0)
InputEvent::mouse_press(MouseButton::Left)
InputEvent::mouse_release(MouseButton::Left)

// Touch
InputEvent::touch_start(0, 100.0, 200.0)  // id, x, y
InputEvent::touch_move(0, 150.0, 250.0)
InputEvent::touch_end(0)
```

## Deterministic Replay

```rust
use jugar_probar::{run_simulation, run_replay, SimulationConfig};

// Record a simulation
let config = SimulationConfig::new(42, 500);
let recording = run_simulation(config, |frame| {
    vec![InputEvent::key_press("ArrowUp")]
});

// Replay it
let replay = run_replay(&recording);

// Verify determinism
assert!(replay.determinism_verified);
assert_eq!(recording.state_hash, replay.state_hash);
```

## Simulation Config

```rust
pub struct SimulationConfig {
    pub seed: u64,           // Random seed for reproducibility
    pub frames: u32,         // Number of frames to simulate
    pub fixed_dt: f32,       // Timestep (default: 1/60)
    pub max_time: f32,       // Max real time (for timeout)
}

let config = SimulationConfig {
    seed: 12345,
    frames: 1000,
    fixed_dt: 1.0 / 60.0,
    max_time: 60.0,
};
```

## Simulation Result

```rust
pub struct SimulationResult {
    pub completed: bool,
    pub frames_run: u32,
    pub state_hash: u64,
    pub final_state: GameState,
    pub recording: Recording,
    pub events: Vec<GameEvent>,
}
```

## Recording Format

```rust
pub struct Recording {
    pub seed: u64,
    pub frames: Vec<FrameInputs>,
    pub state_snapshots: Vec<StateSnapshot>,
}

pub struct FrameInputs {
    pub frame: u32,
    pub inputs: Vec<InputEvent>,
}
```

## Invariant Checking

```rust
use jugar_probar::{run_simulation_with_invariants, Invariant};

let invariants = vec![
    Invariant::new("ball_in_bounds", |state| {
        state.ball.x >= 0.0 && state.ball.x <= 800.0 &&
        state.ball.y >= 0.0 && state.ball.y <= 600.0
    }),
    Invariant::new("score_valid", |state| {
        state.score_left <= 10 && state.score_right <= 10
    }),
];

let result = run_simulation_with_invariants(config, invariants, |_| vec![]);

assert!(result.all_invariants_held);
for violation in &result.violations {
    println!("Violation at frame {}: {}", violation.frame, violation.invariant);
}
```

## Scenario Testing

```rust
#[test]
fn test_game_scenarios() {
    let scenarios = vec![
        ("player_wins", |f| if f < 500 { vec![key("KeyW")] } else { vec![] }),
        ("ai_wins", |_| vec![]),  // No player input
        ("timeout", |_| vec![key("KeyP")]),  // Pause
    ];

    for (name, input_fn) in scenarios {
        let config = SimulationConfig::new(42, 1000);
        let result = run_simulation(config, input_fn);

        println!("Scenario '{}': score = {} - {}",
            name, result.final_state.score_left, result.final_state.score_right);
    }
}
```

## Performance Benchmarking

```rust
use std::time::Instant;

#[test]
fn benchmark_simulation() {
    let config = SimulationConfig::new(42, 10000);

    let start = Instant::now();
    let result = run_simulation(config, |_| vec![]);
    let elapsed = start.elapsed();

    println!("10000 frames in {:?}", elapsed);
    println!("FPS: {}", 10000.0 / elapsed.as_secs_f64());

    // Should run faster than real-time
    assert!(elapsed.as_secs_f64() < 10000.0 / 60.0);
}
```
