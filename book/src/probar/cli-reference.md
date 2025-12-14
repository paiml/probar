# CLI Reference

Command-line interface reference for **probador** - the CLI tool for Probar.

## Installation

```bash
cargo install probador
```

Or build from source:

```bash
cargo build --release -p probador
```

## Commands

### probador test

Run tests with optional coverage and filtering.

```bash
# Run all tests
probador test

# Filter tests by pattern
probador test --filter "game::*"

# Run with coverage
probador test --coverage

# Parallel execution
probador test -j 4

# Fail fast on first error
probador test --fail-fast

# Watch mode (re-run on changes)
probador test --watch

# Custom timeout (ms)
probador test --timeout 60000

# Custom output directory
probador test --output target/my-tests
```

### probador coverage

Generate pixel coverage heatmaps and reports.

```bash
# Generate PNG heatmap
probador coverage --png output.png

# Choose color palette (viridis, magma, heat)
probador coverage --png output.png --palette magma

# Add legend and gap highlighting
probador coverage --png output.png --legend --gaps

# Add title
probador coverage --png output.png --title "My Coverage Report"

# Custom dimensions
probador coverage --png output.png --width 1920 --height 1080

# Export JSON report
probador coverage --json report.json

# Full example
probador coverage --png heatmap.png \
  --palette viridis \
  --legend \
  --gaps \
  --title "Sprint 42 Coverage" \
  --width 800 \
  --height 600
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--png <path>` | Export PNG heatmap | - |
| `--json <path>` | Export JSON report | - |
| `--palette <name>` | Color palette (viridis/magma/heat) | viridis |
| `--legend` | Show color legend | false |
| `--gaps` | Highlight gaps in red | false |
| `--title <text>` | Title text | - |
| `--width <px>` | PNG width | 800 |
| `--height <px>` | PNG height | 600 |
| `-i, --input <path>` | Input coverage data (JSON) | - |

### probador record

Record test execution to media files.

```bash
# Record as GIF (default)
probador record test_login

# Record as PNG screenshots
probador record test_login --format png

# Custom output path
probador record test_login --output recording.gif

# Set frame rate
probador record test_login --fps 30

# Set quality (1-100)
probador record test_login --quality 90
```

**Formats:** `gif`, `png`, `svg`, `mp4`

### probador report

Generate test reports in various formats.

```bash
# HTML report (default)
probador report

# Specific format
probador report --format lcov
probador report --format junit
probador report --format cobertura
probador report --format json

# Custom output directory
probador report --output target/reports

# Open in browser after generation
probador report --open
```

**Formats:** `html`, `junit`, `lcov`, `cobertura`, `json`

### probador init

Initialize a new Probar project.

```bash
# Initialize in current directory
probador init

# Initialize in specific path
probador init ./my-project

# Force overwrite existing files
probador init --force
```

### probador config

View and manage configuration.

```bash
# Show current configuration
probador config --show

# Set a configuration value
probador config --set "parallel=4"

# Reset to defaults
probador config --reset
```

### probador serve

Start a WASM development server with hot reload support.

```bash
# Serve current directory on port 8080
probador serve

# Serve a specific directory
probador serve ./www

# Custom port
probador serve --port 3000

# Enable CORS for cross-origin requests
probador serve --cors

# Open browser automatically
probador serve --open

# Full example
probador serve ./dist --port 8080 --cors --open
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `<directory>` | Directory to serve | `.` |
| `-p, --port <port>` | HTTP port | 8080 |
| `--ws-port <port>` | WebSocket port for hot reload | 8081 |
| `--cors` | Enable CORS | false |
| `--open` | Open browser automatically | false |

**Features:**
- Serves WASM files with correct `application/wasm` MIME type
- WebSocket endpoint at `/ws` for hot reload notifications
- Automatic CORS headers when enabled
- No-cache headers for development

### probador build

Build a Rust project to WASM using wasm-pack.

```bash
# Build in development mode
probador build

# Build in release mode
probador build --release

# Specify build target
probador build --target web
probador build --target bundler
probador build --target nodejs

# Custom output directory
probador build --out-dir ./dist

# Enable profiling (adds names section)
probador build --profiling

# Full example
probador build ./my-game --target web --release --out-dir ./www/pkg
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `<path>` | Package directory | `.` |
| `-t, --target <target>` | WASM target (web/bundler/nodejs/no-modules) | web |
| `--release` | Build in release mode | false |
| `-o, --out-dir <path>` | Output directory | pkg |
| `--profiling` | Enable profiling | false |

### probador watch

Watch for file changes and rebuild automatically.

```bash
# Watch current directory
probador watch

# Watch with dev server
probador watch --serve

# Custom port when serving
probador watch --serve --port 3000

# Build in release mode
probador watch --release

# Custom debounce delay
probador watch --debounce 1000

# Full example
probador watch ./my-game --serve --port 8080 --target web
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `<path>` | Directory to watch | `.` |
| `--serve` | Also start dev server | false |
| `-p, --port <port>` | Server port (with --serve) | 8080 |
| `--ws-port <port>` | WebSocket port | 8081 |
| `-t, --target <target>` | WASM target | web |
| `--release` | Build in release mode | false |
| `--debounce <ms>` | Debounce delay | 500 |

**Watched files:** `.rs`, `.toml`

### probador playbook

Run YAML-driven state machine playbook tests with validation and mutation testing.

```bash
# Validate a playbook
probador playbook login.yaml --validate

# Run multiple playbooks
probador playbook login.yaml checkout.yaml profile.yaml

# Export state diagram as SVG
probador playbook login.yaml --export svg --export-output diagram.svg

# Export as DOT (Graphviz)
probador playbook login.yaml --export dot --export-output diagram.dot

# Run mutation testing (M1-M5)
probador playbook login.yaml --mutate

# Run specific mutation classes
probador playbook login.yaml --mutate --mutation-classes M1,M2,M3

# JSON output for CI integration
probador playbook login.yaml --format json

# JUnit XML for test reporting
probador playbook login.yaml --format junit

# Fail fast on first error
probador playbook login.yaml --fail-fast

# Full example
probador playbook tests/*.yaml \
  --validate \
  --mutate \
  --mutation-classes M1,M2,M5 \
  --format json \
  --output results/
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `<files>...` | Playbook YAML file(s) | (required) |
| `--validate` | Validate without executing | false |
| `--export <format>` | Export diagram (dot/svg) | - |
| `--export-output <path>` | Diagram output file | - |
| `--mutate` | Run mutation testing | false |
| `--mutation-classes <M>` | Mutation classes (M1-M5) | all |
| `--fail-fast` | Stop on first error | false |
| `--continue-on-error` | Continue on step failure | false |
| `-f, --format <format>` | Output format (text/json/junit) | text |
| `-o, --output <dir>` | Output directory | target/probar/playbooks |

**Mutation Classes:**

| Class | Description |
|-------|-------------|
| M1 | State removal |
| M2 | Transition removal |
| M3 | Event swap |
| M4 | Target swap |
| M5 | Guard negation |

## Global Options

These options work with all commands:

```bash
# Verbose output (-v, -vv, -vvv for more detail)
probador -v test
probador -vvv test

# Quiet mode (suppress non-error output)
probador -q test

# Color output (auto, always, never)
probador --color never test
probador --color always report
```

## Examples

### Basic Test Run

```bash
probador test
```

### Coverage with Heatmap

```bash
# Run tests with coverage
probador test --coverage

# Generate heatmap
probador coverage --png coverage.png --legend --gaps --title "Test Coverage"
```

### CI/CD Pipeline

```bash
# Run tests, fail fast, generate reports
probador test --fail-fast --coverage
probador report --format lcov --output coverage/
probador report --format junit --output test-results/
probador coverage --json coverage/pixel-report.json
```

### Watch Mode Development

```bash
# Run tests on file changes
probador test --watch --filter "unit::*"
```

### WASM Development Workflow

```bash
# Build WASM package
probador build --target web --release

# Start dev server with hot reload
probador serve ./www --port 8080 --cors

# Or combine watch + serve for full development experience
probador watch --serve --port 8080
```

### Playbook State Machine Testing

```bash
# Validate playbook
probador playbook login.yaml --validate

# Export diagram
probador playbook login.yaml --export svg -o login.svg

# Run mutation testing
probador playbook login.yaml --mutate
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Test failure(s) |
| 2 | Configuration error |
| 3 | I/O error |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `PROBAR_COLOR` | Color output (auto/always/never) |
| `PROBAR_PARALLEL` | Default parallel jobs |
| `PROBAR_TIMEOUT` | Default test timeout (ms) |

## Library Usage

For programmatic usage in Rust code, use the library crate:

```bash
cargo add jugar-probar --dev
```

```rust
use jugar_probar::prelude::*;
```

See [API Reference](./api-reference.md) for library documentation.
