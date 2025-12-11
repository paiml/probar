# Probar Makefile - Tiered Build System
# Following PMAT/Extreme TDD workflow

.PHONY: tier1 tier2 tier3 build test lint fmt coverage clean

# ============================================================================
# TIER 1: ON-SAVE - Sub-second feedback
# ============================================================================

tier1: lint-fast
	@echo "✓ Tier 1 passed"

lint-fast:
	cargo check --workspace

# ============================================================================
# TIER 2: ON-COMMIT - Full validation (1-5 min)
# ============================================================================

tier2: fmt lint test
	@echo "✓ Tier 2 passed"

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

test-all-features:
	cargo test --workspace --all-features

# ============================================================================
# TIER 3: ON-MERGE - Mutation testing + benchmarks
# ============================================================================

tier3: tier2 coverage
	@echo "✓ Tier 3 passed"

coverage:
	cargo llvm-cov --workspace --html
	@echo "Coverage report generated in target/llvm-cov/html/"

# ============================================================================
# BUILD
# ============================================================================

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

# ============================================================================
# CLEAN
# ============================================================================

clean:
	cargo clean

# ============================================================================
# DOCUMENTATION
# ============================================================================

doc:
	cargo doc --workspace --no-deps --open

# ============================================================================
# HELP
# ============================================================================

help:
	@echo "Probar Build System"
	@echo ""
	@echo "Tiered Workflow:"
	@echo "  make tier1      - ON-SAVE: Sub-second feedback"
	@echo "  make tier2      - ON-COMMIT: Full validation"
	@echo "  make tier3      - ON-MERGE: Coverage + benchmarks"
	@echo ""
	@echo "Individual Commands:"
	@echo "  make build      - Build all crates"
	@echo "  make test       - Run all tests"
	@echo "  make lint       - Run clippy"
	@echo "  make fmt        - Format code"
	@echo "  make coverage   - Generate coverage report"
	@echo "  make doc        - Generate documentation"
	@echo "  make clean      - Clean build artifacts"
