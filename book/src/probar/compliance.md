# Compliance Checking

Probar includes a comprehensive compliance system to ensure your WASM application meets quality standards before deployment.

## Overview

The `probador comply` command runs 10 automated checks covering:

- Code execution verification
- Console error detection
- Custom element registration
- Threading mode support
- Memory handling
- Header configuration
- Replay determinism
- Cache behavior
- Binary size limits
- Panic-free code paths

## Quick Start

```bash
# Run all compliance checks
probador comply .

# Run specific checks
probador comply . --checks C001,C003,C010

# Strict mode (production requirements)
probador comply . --strict

# Generate detailed report
probador comply report . --format html --output report.html
```

## The 10-Point Checklist

### C001: Code Execution Verified

Ensures WASM code actually executes, not just that DOM elements exist.

```bash
probador comply . --checks C001
# [✓] C001: Code execution verified
```

### C002: Console Errors Fail Tests

Captures and fails on any `console.error` calls.

### C003: Custom Elements Tested

Verifies custom elements via `customElements.get()`, not just DOM presence.

### C004: Threading Modes Tested

Validates both single-threaded and multi-threaded code paths.

### C005: Low Memory Tested

Tests graceful degradation under memory pressure.

### C006: COOP/COEP Headers

Verifies Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy headers.

### C007: Replay Hash Matches

Ensures deterministic replay produces identical state hashes.

### C008: Cache Handling

Tests Service Worker and browser caching behavior.

### C009: WASM Size Limit

Enforces binary size constraints (default: 5MB).

```bash
# Custom size limit
probador comply . --max-wasm-size 2097152  # 2MB
```

### C010: No Panic Paths

Scans for `.unwrap()` and `.expect()` patterns in production code.

## Strict Modes

### Production Mode

Maximum strictness for production deployments:

```rust
use jugar_probar::strict::WasmStrictMode;

let mode = WasmStrictMode::production();
// - Console errors fail: true
// - Network errors fail: true
// - Max WASM size: 5MB
// - Max load time: 5s
```

### Development Mode

Relaxed settings for development:

```rust
let mode = WasmStrictMode::development();
// - Console errors fail: false
// - Network errors fail: false
// - Max WASM size: 20MB
// - Max load time: 30s
```

### Custom Mode

Build your own configuration:

```rust
let mode = WasmStrictMode::builder()
    .console_errors_fail(true)
    .network_errors_fail(false)
    .require_wasm_execution(true)
    .max_wasm_size(2 * 1024 * 1024)  // 2MB
    .max_load_time(Duration::from_secs(3))
    .build();
```

## E2E Test Checklist

Ensure comprehensive test coverage:

```rust
use jugar_probar::strict::E2ETestChecklist;

let mut checklist = E2ETestChecklist::new()
    .with_strict_mode(WasmStrictMode::production());

// During test execution
checklist.mark_wasm_executed();
checklist.mark_components_registered();
checklist.mark_console_checked();
checklist.mark_network_verified();
checklist.mark_error_paths_tested();

// Validate all items completed
match checklist.validate() {
    Ok(_) => println!("All checks passed!"),
    Err(missing) => println!("Missing: {:?}", missing),
}
```

## Subcommands

### Check

Run compliance checks:

```bash
probador comply check . --detailed
```

### Report

Generate compliance report:

```bash
probador comply report . --format json --output compliance.json
probador comply report . --format html --output report.html
```

### Migrate

Update configuration for new versions:

```bash
probador comply migrate .
```

### Diff

Compare compliance between versions:

```bash
probador comply diff v1.0 v1.1
```

### Enforce

Install git pre-commit hook:

```bash
probador comply enforce .
# Installs hook that runs compliance checks before each commit
```

## CI Integration

### GitHub Actions

```yaml
- name: Compliance Check
  run: |
    cargo install jugar-probar
    probador comply . --strict --format junit --output compliance.xml
```

### GitLab CI

```yaml
compliance:
  script:
    - probador comply . --strict
  artifacts:
    reports:
      junit: compliance.xml
```

## Example

Run the compliance demo:

```bash
cargo run --example comply_demo -p jugar-probar
```
