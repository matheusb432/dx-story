set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# List repository commands.
_default:
    @just --list --unsorted

# Report missing project tools without changing the host.
[group('setup')]
doctor *args:
    @mise ls --local --missing --locked --no-header {{ args }}

# Install the pinned project tools.
[group('setup')]
bootstrap *args:
    mise bootstrap --yes {{ args }}

# Fetch Cargo dependencies.
[group('setup')]
setup *args:
    cargo fetch {{ args }}

# Reinstall the CLI from this checkout.
[group('setup')]
update:
    cargo install --path crates/dx-story-cli --locked --force

# Apply Rust, TOML, and Markdown formatters.
[group('quality')]
fmt:
    cargo fmt --all
    RUST_LOG=warn taplo fmt
    rumdl fmt .

# Check formatting without changing files.
[group('quality')]
fmt-check:
    cargo fmt --all --check
    RUST_LOG=warn taplo fmt --check
    rumdl fmt --check .

# Lint every workspace target and feature.
[group('quality')]
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run formatting and lint checks.
[group('quality')]
check: fmt-check lint

# Apply Clippy fixes and reformat.
[group('quality')]
fix *args:
    cargo clippy --fix --workspace --all-targets --all-features --allow-dirty {{ args }}
    just fmt

# Run workspace tests through xtask.
[group('quality')]
test *args:
    @cargo run --quiet -p xtask -- test {{ args }}
