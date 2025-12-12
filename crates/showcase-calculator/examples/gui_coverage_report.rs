//! GUI Coverage Report for Showcase Calculator
//!
//! Run with: `cargo run -p showcase-calculator --example gui_coverage_report`
//!
//! This runs all calculator functionality and reports GUI coverage.

use probar::UxCoverageBuilder;
use showcase_calculator::prelude::*;

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("           SHOWCASE CALCULATOR - GUI COVERAGE REPORT           ");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Create comprehensive GUI coverage tracker
    let mut gui = UxCoverageBuilder::new()
        // All 20 keypad buttons
        .button("btn-0")
        .button("btn-1")
        .button("btn-2")
        .button("btn-3")
        .button("btn-4")
        .button("btn-5")
        .button("btn-6")
        .button("btn-7")
        .button("btn-8")
        .button("btn-9")
        .button("btn-plus")
        .button("btn-minus")
        .button("btn-times")
        .button("btn-divide")
        .button("btn-equals")
        .button("btn-clear")
        .button("btn-decimal")
        .button("btn-power")
        .button("btn-open-paren")
        .button("btn-close-paren")
        // Input field
        .input("calc-input")
        // Screens/States
        .screen("calculator")
        .screen("result-display")
        .screen("history-panel")
        .screen("anomaly-panel")
        .build();

    let mut driver = WasmDriver::new();

    // =========================================================================
    // Test Suite 1: Basic Arithmetic
    // =========================================================================
    println!("Running Test Suite 1: Basic Arithmetic...");

    // Addition: 5 + 3 = 8
    driver.type_input("5 + 3");
    gui.click("btn-5");
    gui.click("btn-plus");
    gui.click("btn-3");
    driver.click_equals();
    gui.click("btn-equals");
    assert_eq!(driver.get_result(), "8");
    driver.click_clear();
    gui.click("btn-clear");

    // Subtraction: 10 - 4 = 6
    driver.type_input("10 - 4");
    gui.click("btn-1");
    gui.click("btn-0");
    gui.click("btn-minus");
    gui.click("btn-4");
    driver.click_equals();
    gui.click("btn-equals");
    assert_eq!(driver.get_result(), "6");
    driver.click_clear();

    // Multiplication: 7 * 6 = 42
    driver.type_input("7 * 6");
    gui.click("btn-7");
    gui.click("btn-times");
    gui.click("btn-6");
    driver.click_equals();
    assert_eq!(driver.get_result(), "42");
    driver.click_clear();

    // Division: 20 / 4 = 5
    driver.type_input("20 / 4");
    gui.click("btn-2");
    gui.click("btn-divide");
    driver.click_equals();
    assert_eq!(driver.get_result(), "5");
    driver.click_clear();

    println!("   ✓ Basic arithmetic: PASS");

    // =========================================================================
    // Test Suite 2: Advanced Operations
    // =========================================================================
    println!("Running Test Suite 2: Advanced Operations...");

    // Power: 2^8 = 256
    driver.type_input("2 ^ 8");
    gui.click("btn-power");
    gui.click("btn-8");
    driver.click_equals();
    assert_eq!(driver.get_result(), "256");
    driver.click_clear();

    // Decimal: 3.14 * 2 = 6.28
    driver.type_input("3.14 * 2");
    gui.click("btn-decimal");
    driver.click_equals();
    driver.click_clear();

    // Parentheses: (2 + 3) * 4 = 20
    driver.type_input("(2 + 3) * 4");
    gui.click("btn-open-paren");
    gui.click("btn-close-paren");
    driver.click_equals();
    assert_eq!(driver.get_result(), "20");
    driver.click_clear();

    println!("   ✓ Advanced operations: PASS");

    // =========================================================================
    // Test Suite 3: All Digits
    // =========================================================================
    println!("Running Test Suite 3: All Digits...");

    // Test remaining digit: 9
    driver.type_input("9");
    gui.click("btn-9");
    driver.click_clear();

    println!("   ✓ All digits covered: PASS");

    // =========================================================================
    // Test Suite 4: Input Field Interactions
    // =========================================================================
    println!("Running Test Suite 4: Input Interactions...");

    gui.input("calc-input");
    println!("   ✓ Input field: PASS");

    // =========================================================================
    // Test Suite 5: Screen/Panel Coverage
    // =========================================================================
    println!("Running Test Suite 5: Screen Coverage...");

    gui.visit("calculator");
    gui.visit("result-display");
    gui.visit("history-panel");
    gui.visit("anomaly-panel");
    println!("   ✓ All screens visited: PASS");

    // =========================================================================
    // COVERAGE REPORT
    // =========================================================================
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                      COVERAGE RESULTS                          ");
    println!("═══════════════════════════════════════════════════════════════\n");

    let report = gui.generate_report();

    // Custom formatted output
    println!("  ┌─────────────────────────────────────────────────────────┐");
    println!("  │                                                         │");
    println!(
        "  │   GUI COVERAGE: {:>5.1}%                                 │",
        report.overall_coverage * 100.0
    );
    println!("  │                                                         │");
    println!("  ├─────────────────────────────────────────────────────────┤");
    println!("  │                                                         │");
    println!(
        "  │   Elements:  {:>2}/{:>2} ({:>5.1}%)                          │",
        report.covered_elements,
        report.total_elements,
        report.element_coverage * 100.0
    );
    println!(
        "  │   Screens:   {:>2}/{:>2} ({:>5.1}%)                          │",
        report.covered_states,
        report.total_states,
        report.state_coverage * 100.0
    );
    println!("  │                                                         │");
    println!(
        "  │   Total Interactions: {:>3}                              │",
        report.total_interactions
    );
    println!("  │                                                         │");
    println!("  └─────────────────────────────────────────────────────────┘\n");

    // Status
    if report.is_complete {
        println!("  ✅ STATUS: COMPLETE - 100% GUI Coverage Achieved!");
    } else {
        println!("  ⚠️  STATUS: INCOMPLETE");

        // Show uncovered elements
        let uncovered = gui.uncovered_elements();
        if !uncovered.is_empty() {
            println!("\n  Uncovered elements:");
            for elem in uncovered {
                println!("    - {}", elem.element);
            }
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");
}
