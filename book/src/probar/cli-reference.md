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
