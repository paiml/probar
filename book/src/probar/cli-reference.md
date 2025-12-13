# CLI Reference

Command-line interface reference for Probar.

## Installation

```bash
# Install from source
cargo install --path crates/probar-cli

# Or build locally
cargo build --release -p probar-cli
```

## Commands

### probar test

Run tests with optional coverage and filtering.

```bash
# Run all tests
probar test

# Filter tests by pattern
probar test --filter "game::*"

# Run with coverage
probar test --coverage

# Parallel execution
probar test -j 4

# Fail fast on first error
probar test --fail-fast

# Watch mode (re-run on changes)
probar test --watch

# Custom timeout (ms)
probar test --timeout 60000

# Custom output directory
probar test --output target/my-tests
```

### probar coverage

Generate pixel coverage heatmaps and reports.

```bash
# Generate PNG heatmap
probar coverage --png output.png

# Choose color palette (viridis, magma, heat)
probar coverage --png output.png --palette magma

# Add legend and gap highlighting
probar coverage --png output.png --legend --gaps

# Add title
probar coverage --png output.png --title "My Coverage Report"

# Custom dimensions
probar coverage --png output.png --width 1920 --height 1080

# Export JSON report
probar coverage --json report.json

# Full example
probar coverage --png heatmap.png \
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

### probar record

Record test execution to media files.

```bash
# Record as GIF (default)
probar record test_login

# Record as PNG screenshots
probar record test_login --format png

# Custom output path
probar record test_login --output recording.gif

# Set frame rate
probar record test_login --fps 30

# Set quality (1-100)
probar record test_login --quality 90
```

**Formats:** `gif`, `png`, `svg`, `mp4`

### probar report

Generate test reports in various formats.

```bash
# HTML report (default)
probar report

# Specific format
probar report --format lcov
probar report --format junit
probar report --format cobertura
probar report --format json

# Custom output directory
probar report --output target/reports

# Open in browser after generation
probar report --open
```

**Formats:** `html`, `junit`, `lcov`, `cobertura`, `json`

### probar init

Initialize a new Probar project.

```bash
# Initialize in current directory
probar init

# Initialize in specific path
probar init ./my-project

# Force overwrite existing files
probar init --force
```

### probar config

View and manage configuration.

```bash
# Show current configuration
probar config --show

# Set a configuration value
probar config --set "parallel=4"

# Reset to defaults
probar config --reset
```

### probar serve

Start a WASM development server with hot reload support.

```bash
# Serve current directory on port 8080
probar serve

# Serve a specific directory
probar serve ./www

# Custom port
probar serve --port 3000

# Enable CORS for cross-origin requests
probar serve --cors

# Open browser automatically
probar serve --open

# Full example
probar serve ./dist --port 8080 --cors --open
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

### probar build

Build a Rust project to WASM using wasm-pack.

```bash
# Build in development mode
probar build

# Build in release mode
probar build --release

# Specify build target
probar build --target web
probar build --target bundler
probar build --target nodejs

# Custom output directory
probar build --out-dir ./dist

# Enable profiling (adds names section)
probar build --profiling

# Full example
probar build ./my-game --target web --release --out-dir ./www/pkg
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `<path>` | Package directory | `.` |
| `-t, --target <target>` | WASM target (web/bundler/nodejs/no-modules) | web |
| `--release` | Build in release mode | false |
| `-o, --out-dir <path>` | Output directory | pkg |
| `--profiling` | Enable profiling | false |

### probar watch

Watch for file changes and rebuild automatically.

```bash
# Watch current directory
probar watch

# Watch with dev server
probar watch --serve

# Custom port when serving
probar watch --serve --port 3000

# Build in release mode
probar watch --release

# Custom debounce delay
probar watch --debounce 1000

# Full example
probar watch ./my-game --serve --port 8080 --target web
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

## Global Options

These options work with all commands:

```bash
# Verbose output (-v, -vv, -vvv for more detail)
probar -v test
probar -vvv test

# Quiet mode (suppress non-error output)
probar -q test

# Color output (auto, always, never)
probar --color never test
probar --color always report
```

## Examples

### Basic Test Run

```bash
probar test
```

### Coverage with Heatmap

```bash
# Run tests with coverage
probar test --coverage

# Generate heatmap
probar coverage --png coverage.png --legend --gaps --title "Test Coverage"
```

### CI/CD Pipeline

```bash
# Run tests, fail fast, generate reports
probar test --fail-fast --coverage
probar report --format lcov --output coverage/
probar report --format junit --output test-results/
probar coverage --json coverage/pixel-report.json
```

### Watch Mode Development

```bash
# Run tests on file changes
probar test --watch --filter "unit::*"
```

### WASM Development Workflow

```bash
# Build WASM package
probar build --target web --release

# Start dev server with hot reload
probar serve ./www --port 8080 --cors

# Or combine watch + serve for full development experience
probar watch --serve --port 8080
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
