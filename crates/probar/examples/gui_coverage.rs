//! Example: GUI Coverage (Feature 24)
//!
//! Demonstrates: Trivially simple GUI coverage tracking for WASM and TUI apps
//!
//! Run with: `cargo run --example gui_coverage`
//!
//! Probar: Complete UX verification with minimal boilerplate

use jugar_probar::prelude::*;

fn main() -> ProbarResult<()> {
    println!("=== GUI Coverage Example ===\n");

    // =========================================================================
    // 1. The Simplest Way: gui_coverage! macro
    // =========================================================================
    println!("1. Using gui_coverage! macro (simplest)...");

    let mut gui = jugar_probar::gui_coverage! {
        buttons: ["start", "pause", "quit"],
        screens: ["title", "playing", "game_over"]
    };

    // Simulate test interactions
    gui.click("start");
    gui.visit("title");
    gui.visit("playing");

    println!("   {}", gui.summary());
    println!("   Coverage: {:.1}%", gui.percent());

    // =========================================================================
    // 2. Pre-built Calculator Coverage
    // =========================================================================
    println!("\n2. Calculator preset (20 buttons + 2 screens)...");

    let mut calc = calculator_coverage();

    // Simulate clicking all digit buttons
    for i in 0..=9 {
        calc.click(&format!("btn-{}", i));
    }

    // Operators
    calc.click("btn-plus");
    calc.click("btn-minus");
    calc.click("btn-equals");
    calc.click("btn-clear");

    // Screens
    calc.visit("calculator");

    println!("   {}", calc.summary());

    // =========================================================================
    // 3. Game Coverage Helper
    // =========================================================================
    println!("\n3. Game coverage helper...");

    let mut game = game_coverage(
        &["play", "pause", "restart", "menu", "settings"],
        &["splash", "main_menu", "gameplay", "pause_menu", "game_over"],
    );

    // Simulate a game session
    game.visit("splash");
    game.click("play");
    game.visit("main_menu");
    game.visit("gameplay");
    game.click("pause");
    game.visit("pause_menu");
    game.click("restart");
    game.visit("game_over");
    game.click("menu");

    println!("   {}", game.summary());
    println!("   Meets 80%? {}", game.meets(80.0));

    // =========================================================================
    // 4. Builder Pattern for Custom Coverage
    // =========================================================================
    println!("\n4. Custom coverage with builder...");

    let mut custom = UxCoverageBuilder::new()
        .button("login")
        .button("logout")
        .input("username")
        .input("password")
        .screen("login_page")
        .screen("dashboard")
        .modal("confirm_logout")
        .build();

    // Simulate login flow
    custom.visit("login_page");
    custom.input("username");
    custom.input("password");
    custom.click("login");
    custom.visit("dashboard");

    println!("   {}", custom.summary());

    // =========================================================================
    // 5. Full Coverage Demonstration
    // =========================================================================
    println!("\n5. Achieving 100% coverage...");

    let mut full = jugar_probar::gui_coverage! {
        buttons: ["a", "b", "c"],
        screens: ["home"]
    };

    full.click("a");
    full.click("b");
    full.click("c");
    full.visit("home");

    println!("   {}", full.summary());
    println!("   Complete? {}", full.is_complete());

    // Assert complete coverage
    full.assert_complete()?;

    // =========================================================================
    // 6. Detailed Report
    // =========================================================================
    println!("\n6. Detailed coverage report...");

    let mut detailed = jugar_probar::gui_coverage! {
        buttons: ["save", "cancel", "delete"],
        inputs: ["name", "email"],
        screens: ["form", "confirmation"],
        modals: ["delete_confirm"]
    };

    detailed.click("save");
    detailed.input("name");
    detailed.visit("form");

    let report = detailed.generate_report();
    println!("{}", report);

    // =========================================================================
    // 7. Tracking User Journeys
    // =========================================================================
    println!("\n7. User journey tracking...");

    let mut journey = UxCoverageTracker::new();
    journey.register_screen("home");
    journey.register_screen("products");
    journey.register_screen("cart");
    journey.register_screen("checkout");

    // Journey 1: Browse and buy
    journey.visit("home");
    journey.visit("products");
    journey.visit("cart");
    journey.visit("checkout");
    journey.end_journey();

    // Journey 2: Just browse
    journey.visit("home");
    journey.visit("products");
    journey.end_journey();

    println!("   Journeys recorded: {}", journey.journeys().len());
    println!("   {}", journey.summary());

    println!("\n✅ GUI coverage example completed!");
    Ok(())
}
