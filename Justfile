NIGHTLY_TOOLCHAIN := "nightly-2026-01-23"
SOLANA_VERSION := "3.1.10"

# Build Lotic CLI
build-lotic-cli:
    @cargo build --release --manifest-path ./cli/Cargo.toml

# Build Solana test programs
build-test-programs:
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-compute-budget/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-config/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-stake/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-system/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-token/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-tokenkeg/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-program-tokenz/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-signer/Cargo.toml
    @./target/release/lotic-cli build -- --manifest-path tests/constraint-writable/Cargo.toml

# Run clippy checks
clippy:
	@cargo +{{NIGHTLY_TOOLCHAIN}} clippy --all-targets --all-features -- -D warnings

# Run cargo check for errors
check:
	@cargo check --verbose

# Auto-fix clippy issues
clippy-fix:
	@cargo +{{NIGHTLY_TOOLCHAIN}} clippy --all --all-features --all-targets --fix --allow-dirty --allow-staged -- -D warnings
	
# Check formatting
format:
	@cargo +{{NIGHTLY_TOOLCHAIN}} fmt --all -- --check

# Fix formatting
format-fix:
	@cargo +{{NIGHTLY_TOOLCHAIN}} fmt --all

# Test all feature combos
hack:
	@cargo hack check --feature-powerset --all-targets

# Echo nightly version
nightly-version:
	@echo {{NIGHTLY_TOOLCHAIN}}

# Echo Solana version
solana-version:
	@echo {{SOLANA_VERSION}}

# Build then run tests
test:
    @just build-test-programs
    @cargo test --manifest-path tests/constraint-program-compute-budget/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-program-config/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-program-stake/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-program-system/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-program-token/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-program-tokenkeg/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-signer/Cargo.toml --all-features
    @cargo test --manifest-path tests/constraint-writable/Cargo.toml --all-features


