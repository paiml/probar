# Coverage Tooling

Probar includes advanced coverage instrumentation for WASM games.

## Overview

Traditional coverage tools (LLVM, gcov) don't work well with WASM. Probar implements a **renderfarm-inspired block coverage model** where:

- WASM code is decomposed into **coverage blocks** (like render buckets)
- Blocks are independently testable and falsifiable
- Coverage aggregation uses SIMD-accelerated operations via Trueno

## Basic Coverage

```rust
use jugar_probar::coverage::*;

// Enable coverage collection
let mut coverage = CoverageCollector::new();

// Run tests with coverage
coverage.start();
run_tests();
let report = coverage.finish();

// Print summary
println!("Line coverage: {:.1}%", report.line_coverage * 100.0);
println!("Branch coverage: {:.1}%", report.branch_coverage * 100.0);
println!("Function coverage: {:.1}%", report.function_coverage * 100.0);
```

## Block-Based Coverage

```rust
use jugar_probar::coverage::{BlockId, FunctionId, EdgeId};

// Type-safe identifiers (Poka-Yoke)
let block = BlockId::new(42);
let function = FunctionId::new(1);

// EdgeId encodes source and target
let edge = EdgeId::new(BlockId::new(10), BlockId::new(20));
assert_eq!(edge.source().as_u32(), 10);
assert_eq!(edge.target().as_u32(), 20);
```

## Thread-Local Buffering (Muda Elimination)

```rust
use jugar_probar::coverage::ThreadLocalCounters;

// Traditional: Atomic increment on every hit (contention)
// Probar: Thread-local buffering, batch flush

let counters = ThreadLocalCounters::new(1000);  // 1000 blocks

// Fast local increment
counters.hit(BlockId::new(42));

// Periodic flush to global
counters.flush();
```

## Running Coverage

### Via Makefile

```bash
# Full coverage report
make coverage

# E2E coverage
make test-e2e-coverage

# Quick summary
make coverage-summary

# Open HTML report
make coverage-open
```

### Via Cargo

```bash
# Generate report
cargo llvm-cov --html --output-dir target/coverage

# Summary only
cargo llvm-cov report --summary-only

# With nextest
cargo llvm-cov nextest --workspace
```

## Coverage Targets

| Metric | Minimum | Target |
|--------|---------|--------|
| Line Coverage | 85% | 95% |
| Branch Coverage | 75% | 90% |
| Function Coverage | 90% | 100% |
| Mutation Score | 80% | 90% |

## Coverage Report

```rust
pub struct CoverageReport {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub function_coverage: f64,
    pub covered_lines: u32,
    pub total_lines: u32,
    pub covered_branches: u32,
    pub total_branches: u32,
    pub covered_functions: u32,
    pub total_functions: u32,
    pub uncovered_lines: Vec<LineInfo>,
}
```

## Superblock Scheduling

For parallel coverage analysis:

```rust
use jugar_probar::coverage::{Superblock, Scheduler};

// Group blocks into superblocks
let superblocks = Scheduler::create_superblocks(&blocks, num_workers);

// Execute in parallel
let results = superblocks
    .par_iter()
    .map(|sb| execute_superblock(sb))
    .collect();

// Merge results
let final_coverage = CoverageMerger::merge(&results);
```

## Soft Jidoka (Error Classification)

```rust
use jugar_probar::coverage::{Error, Severity};

// Distinguish fatal vs recoverable errors
match error.severity {
    Severity::Fatal => {
        // Stop immediately (Andon cord)
        panic!("Fatal error in coverage: {}", error);
    }
    Severity::Recoverable => {
        // Log and continue
        log::warn!("Recoverable error: {}", error);
        continue;
    }
    Severity::Ignorable => {
        // Skip silently
    }
}
```

## Coverage Example

```bash
cargo run --example coverage_demo -p jugar-probar
```

Output:
```
=== Probar Coverage Demo ===

--- Type-Safe Identifiers (Poka-Yoke) ---
BlockId(42) - Type-safe block identifier
FunctionId(1) - Type-safe function identifier
EdgeId(10 -> 20) - Encodes source and target

--- Thread-Local Counters (Muda Elimination) ---
Created counters for 1000 blocks
Hit block 42: 1000 times
Hit block 99: 500 times
After flush: block 42 = 1000, block 99 = 500

--- Superblock Scheduling ---
4 workers, 16 superblocks
Superblock 0: blocks [0..62]
Superblock 1: blocks [63..125]
...

✅ Coverage demo complete!
```

## Integration with CI

```yaml
- name: Generate coverage
  run: |
    cargo llvm-cov --lcov --output-path lcov.info
    cargo llvm-cov report --summary-only

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    files: lcov.info
```
