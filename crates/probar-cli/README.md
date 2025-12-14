# Probador

CLI for [Probar](https://github.com/paiml/probar) - Rust-native testing framework for WASM games.

## Installation

```bash
cargo install probador
```

## Usage

```bash
# Validate a playbook state machine
probador playbook login.yaml --validate

# Run with mutation testing (M1-M5 falsification)
probador playbook login.yaml --mutate

# Export state diagram
probador playbook login.yaml --export svg -o diagram.svg

# Run tests
probador test

# Run tests with coverage
probador coverage --html

# Watch mode for development
probador watch tests/

# Start dev server for WASM
probador serve --port 8080
```

## Commands

| Command | Description |
|---------|-------------|
| `test` | Run tests with optional filtering |
| `playbook` | YAML-driven state machine testing |
| `coverage` | Generate coverage reports |
| `record` | Record test execution as GIF/MP4 |
| `report` | Generate HTML/JSON/JUnit reports |
| `serve` | Development server for WASM |
| `watch` | Watch mode with hot reload |
| `init` | Initialize new test project |

## Library Usage

For programmatic usage, add the library crate:

```bash
cargo add jugar-probar
```

```rust
use jugar_probar::prelude::*;
```

## Documentation

- [Book](https://paiml.github.io/probar/)
- [API Docs](https://docs.rs/jugar-probar)

## License

MIT OR Apache-2.0
