//! Coverage Demo - WASM Coverage Tooling
//!
//! Demonstrates Probar's novel WASM coverage instrumentation framework
//! using Toyota Way principles (Poka-Yoke, Muda, Jidoka, Heijunka).
//!
//! # Running
//!
//! ```bash
//! cargo run --example coverage_demo -p probar
//! ```
//!
//! # Features
//!
//! - Type-safe block identifiers (Poka-Yoke)
//! - Thread-local buffering (Muda elimination)
//! - Soft Jidoka (Stop vs `LogAndContinue`)
//! - Superblock tiling (Heijunka)
//! - Popperian falsification methodology

#![allow(clippy::uninlined_format_args)]

use jugar_probar::coverage::{
    BlockId, CoverageCollector, CoverageConfig, CoverageExecutor, CoverageHypothesis,
    CoverageViolation, EdgeId, FunctionId, Granularity, NullificationConfig, Superblock,
    SuperblockBuilder, SuperblockId, SuperblockResult, TaintedBlocks, ThreadLocalCounters,
};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║         PROBAR WASM COVERAGE TOOLING DEMO                     ║");
    println!("║     Toyota Way Principles for Test Coverage                   ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Demo 1: Poka-Yoke (Error Prevention)
    demo_poka_yoke();

    // Demo 2: Thread-Local Buffering (Muda Elimination)
    demo_thread_local_buffering();

    // Demo 3: Soft Jidoka
    demo_soft_jidoka();

    // Demo 4: Superblock Tiling (Heijunka)
    demo_superblock_tiling();

    // Demo 5: Coverage Collection
    demo_coverage_collection();

    // Demo 6: Coverage Executor
    demo_coverage_executor();

    // Demo 7: Popperian Falsification
    demo_popperian_falsification();

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║              Coverage Demo Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}

/// Demo 1: Poka-Yoke - Type-safe block identifiers prevent errors at compile time
fn demo_poka_yoke() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 1: POKA-YOKE (Error Prevention)                            │");
    println!("│ Type-safe IDs prevent mixing BlockId, FunctionId, EdgeId        │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    // Create type-safe identifiers
    let block_id = BlockId::new(42);
    let function_id = FunctionId::new(1);

    println!("  BlockId(42):    {:?}", block_id);
    println!("  FunctionId(1):  {:?}", function_id);

    // EdgeId encodes both source and target
    let from = BlockId::new(10);
    let to = BlockId::new(20);
    let edge = EdgeId::new(from, to);

    println!("\n  Edge from block 10 to block 20:");
    println!("    EdgeId:       {:?}", edge);
    println!("    Source:       {:?}", edge.source());
    println!("    Target:       {:?}", edge.target());
    println!("    Raw u64:      {}", edge.as_u64());

    // Demonstrate that types are NOT interchangeable (compile-time safety)
    println!("\n  ✓ BlockId and FunctionId are distinct types");
    println!("  ✓ Mixing them causes compile-time errors");
    println!("  ✓ EdgeId encodes source/target in single u64\n");
}

/// Demo 2: Thread-Local Buffering - Muda (waste) elimination
fn demo_thread_local_buffering() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 2: MUDA ELIMINATION (Thread-Local Buffering)               │");
    println!("│ Reduces atomic contention from O(N) to O(B) batched flushes     │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    // Create thread-local counters for 10 blocks
    let mut counters = ThreadLocalCounters::new(10);

    println!("  Simulating block executions (no atomics yet):");

    // Simulate increments (no atomic operations)
    for i in 0..5 {
        counters.increment(BlockId::new(i));
        println!(
            "    Block {} hit: count = {}",
            i,
            counters.get(BlockId::new(i))
        );
    }

    // Hit block 0 multiple times
    counters.increment(BlockId::new(0));
    counters.increment(BlockId::new(0));
    println!(
        "    Block 0 hit 2 more times: count = {}",
        counters.get(BlockId::new(0))
    );

    // Flush to global (single atomic operation)
    println!("\n  Flushing to global counters (single batch operation):");
    let flushed = counters.flush();
    println!("    Flushed counts: {:?}", &flushed[..5]);
    println!("    Flush count: {}", counters.flush_count());

    // After flush, local counters reset
    println!("\n  After flush, local counters reset:");
    println!("    Block 0 count: {}", counters.get(BlockId::new(0)));

    println!("\n  ✓ Thread-local buffering eliminates atomic contention");
    println!("  ✓ Batched flushes reduce synchronization overhead\n");
}

/// Demo 3: Soft Jidoka - Stop vs `LogAndContinue`
fn demo_soft_jidoka() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 3: SOFT JIDOKA (Stop vs LogAndContinue)                    │");
    println!("│ Distinguishes instrumentation failures from test failures       │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    // Critical violations -> Stop
    let uninstrumented = CoverageViolation::UninstrumentedExecution {
        block_id: BlockId::new(42),
    };
    let impossible_edge = CoverageViolation::ImpossibleEdge {
        from: BlockId::new(1),
        to: BlockId::new(99),
    };

    // Non-critical violations -> LogAndContinue
    let overflow = CoverageViolation::CounterOverflow {
        block_id: BlockId::new(5),
    };
    let regression = CoverageViolation::CoverageRegression {
        expected: 95.0,
        actual: 90.0,
    };

    println!("  Violation Classification:");
    println!();
    println!(
        "  {:50} -> {:?}",
        uninstrumented.description(),
        uninstrumented.action()
    );
    println!(
        "  {:50} -> {:?}",
        impossible_edge.description(),
        impossible_edge.action()
    );
    println!("  {:50} -> {:?}", overflow.description(), overflow.action());
    println!(
        "  {:50} -> {:?}",
        regression.description(),
        regression.action()
    );

    // Tainted blocks tracker
    println!("\n  TaintedBlocks Tracker:");
    let mut tainted = TaintedBlocks::new();
    tainted.taint(BlockId::new(5), overflow);
    tainted.record_violation(regression);

    println!("    Tainted blocks: {}", tainted.tainted_count());
    println!("    Total violations: {}", tainted.violation_count());
    println!(
        "    Block 5 tainted: {}",
        tainted.is_tainted(BlockId::new(5))
    );

    println!("\n  ✓ Instrumentation failures trigger hard stop (can't trust data)");
    println!("  ✓ Test failures log and continue (taint the block, collect others)\n");
}

/// Demo 4: Superblock Tiling - Heijunka (work leveling)
fn demo_superblock_tiling() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 4: HEIJUNKA (Superblock Tiling)                            │");
    println!("│ Groups blocks to amortize work-stealing scheduler overhead      │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    // Create blocks for a function
    let blocks: Vec<BlockId> = (0..20).map(BlockId::new).collect();
    let function = FunctionId::new(0);

    // Build superblocks with target size 6
    let builder = SuperblockBuilder::new()
        .with_target_size(6)
        .with_max_size(10);

    let superblocks = builder.build_from_blocks(&blocks, function);

    println!("  Input: 20 blocks, target size 6, max size 10");
    println!("  Output: {} superblocks\n", superblocks.len());

    for sb in &superblocks {
        println!(
            "    Superblock {}: {} blocks, cost estimate {}",
            sb.id().as_u32(),
            sb.block_count(),
            sb.cost_estimate()
        );
    }

    // Check containment
    let test_block = BlockId::new(5);
    println!(
        "\n  Block {} is in superblock: {}",
        test_block.as_u32(),
        superblocks
            .iter()
            .find(|sb| sb.contains(test_block))
            .map_or(0, |sb| sb.id().as_u32())
    );

    println!("\n  ✓ Superblocks group related blocks for efficient scheduling");
    println!("  ✓ Work-stealing scheduler operates on superblocks, not blocks\n");
}

/// Demo 5: Coverage Collection
fn demo_coverage_collection() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 5: COVERAGE COLLECTION                                     │");
    println!("│ Session-based coverage tracking with Jidoka support             │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    // Configure coverage collection
    let config = CoverageConfig::builder()
        .granularity(Granularity::BasicBlock)
        .parallel(false)
        .jidoka_enabled(true)
        .build();

    println!("  Configuration:");
    println!("    Granularity: {:?}", config.granularity);
    println!("    Parallel: {}", config.parallel);
    println!("    Jidoka: {}", config.jidoka_enabled);

    // Create collector and run session
    let mut collector = CoverageCollector::new(config);

    collector.begin_session("pong_tests");
    println!("\n  Session 'pong_tests' started");

    // Test 1: Ball movement
    collector.begin_test("test_ball_movement");
    collector.record_hit(BlockId::new(0)); // update_ball
    collector.record_hit(BlockId::new(1)); // check_bounds
    collector.record_hit(BlockId::new(0)); // update_ball again
    collector.end_test();
    println!("    Completed: test_ball_movement");

    // Test 2: Paddle input
    collector.begin_test("test_paddle_input");
    collector.record_hit(BlockId::new(2)); // handle_input
    collector.record_hit(BlockId::new(3)); // update_paddle
    collector.end_test();
    println!("    Completed: test_paddle_input");

    // Record a violation (soft - continues)
    collector.record_violation(CoverageViolation::CoverageRegression {
        expected: 95.0,
        actual: 92.0,
    });
    println!("    Recorded: coverage regression warning");

    // End session and get report
    let report = collector.end_session();
    let summary = report.summary();

    println!("\n  Coverage Report:");
    println!("    Total blocks: {}", summary.total_blocks);
    println!("    Covered blocks: {}", summary.covered_blocks);
    println!("    Coverage: {:.1}%", summary.coverage_percent);
    println!("    Violations: {}", report.violation_count());

    println!("\n  Hit counts:");
    for i in 0..4 {
        let block = BlockId::new(i);
        println!("    Block {}: {} hits", i, report.get_hit_count(block));
    }

    println!("\n  ✓ Session-based tracking for test isolation");
    println!("  ✓ Violations recorded without stopping collection\n");
}

/// Demo 6: Coverage Executor with work-stealing
fn demo_coverage_executor() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 6: COVERAGE EXECUTOR                                       │");
    println!("│ Work-stealing scheduler for parallel coverage execution         │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    // Create superblocks
    let blocks1 = vec![BlockId::new(0), BlockId::new(1)];
    let blocks2 = vec![BlockId::new(2), BlockId::new(3)];
    let blocks3 = vec![BlockId::new(4), BlockId::new(5)];

    let sb1 = Superblock::new(SuperblockId::new(0), blocks1, FunctionId::new(0));
    let sb2 = Superblock::new(SuperblockId::new(1), blocks2, FunctionId::new(0));
    let sb3 = Superblock::new(SuperblockId::new(2), blocks3, FunctionId::new(0));

    // Create executor
    let executor = CoverageExecutor::new(vec![sb1, sb2, sb3])
        .with_workers(4)
        .with_work_stealing(true);

    println!("  Executor Configuration:");
    println!("    Superblocks: {}", executor.superblock_count());
    println!("    Total blocks: {}", executor.total_block_count());
    println!("    Workers: {}", executor.worker_count());

    // Execute with a test function (simulate some failures)
    let report = executor.execute(|sb| {
        // Simulate: superblock 1 fails
        if sb.id().as_u32() == 1 {
            SuperblockResult {
                id: sb.id(),
                success: false,
                error: Some("Test assertion failed".to_string()),
            }
        } else {
            SuperblockResult {
                id: sb.id(),
                success: true,
                error: None,
            }
        }
    });

    let summary = report.summary();
    println!("\n  Execution Results:");
    println!(
        "    Covered blocks: {}/{}",
        summary.covered_blocks, summary.total_blocks
    );
    println!("    Coverage: {:.1}%", summary.coverage_percent);

    println!("\n  Per-block coverage:");
    for i in 0..6 {
        let covered = report.is_covered(BlockId::new(i));
        println!("    Block {}: {}", i, if covered { "✓" } else { "✗" });
    }

    println!("\n  ✓ Work-stealing distributes load across workers");
    println!("  ✓ Failed superblocks don't block successful ones\n");
}

/// Demo 7: Popperian Falsification - Hypothesis testing
fn demo_popperian_falsification() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Demo 7: POPPERIAN FALSIFICATION                                 │");
    println!("│ Every coverage claim must be falsifiable                        │");
    println!("└─────────────────────────────────────────────────────────────────┘\n");

    println!("  Princeton Methodology Configuration:");
    let config = NullificationConfig::princeton();
    println!("    Independent runs: {}", config.runs);
    println!("    Significance level (α): {}", config.alpha);

    // Hypothesis 1: Determinism
    println!("\n  H₀-COV-01: Coverage is deterministic across runs");
    let h1 = CoverageHypothesis::determinism();

    // Consistent results (not rejected)
    let consistent = vec![95.0, 95.0, 95.0, 95.0, 95.0];
    let result = h1.evaluate(&consistent);
    println!("    Observations: {:?}", consistent);
    println!("    {}", result.report());

    // Inconsistent results (rejected)
    let inconsistent = vec![95.0, 90.0, 92.0, 88.0, 95.0];
    let result = h1.evaluate(&inconsistent);
    println!("    Observations: {:?}", inconsistent);
    println!("    {}", result.report());

    // Hypothesis 2: Completeness
    println!("\n  H₀-COV-02: Coverage meets 90% threshold");
    let h2 = CoverageHypothesis::completeness(90.0);

    let above_threshold = vec![92.0, 93.0, 91.5, 94.0, 92.5];
    let result = h2.evaluate(&above_threshold);
    println!("    Observations: {:?}", above_threshold);
    println!("    {}", result.report());

    let below_threshold = vec![85.0, 86.0, 84.0, 87.0, 85.5];
    let result = h2.evaluate(&below_threshold);
    println!("    Observations: {:?}", below_threshold);
    println!("    {}", result.report());

    // Hypothesis 3: No Regression
    println!("\n  H₀-COV-03: No regression from 88% baseline");
    let h3 = CoverageHypothesis::no_regression(88.0);

    let no_regression = vec![90.0, 89.0, 91.0, 88.5, 90.5];
    let result = h3.evaluate(&no_regression);
    println!("    Observations: {:?}", no_regression);
    println!("    {}", result.report());

    println!("\n  ✓ Hypotheses are falsifiable through statistical tests");
    println!("  ✓ Results include p-value, effect size, confidence intervals\n");
}
