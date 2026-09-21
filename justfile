set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

_default:
    @just --list --unsorted

[group('setup')]
doctor *args:
    @mise ls --local --missing --locked --no-header {{ args }}

[group('setup')]
bootstrap *args:
    mise bootstrap --yes {{ args }}

[group('setup')]
setup *args:
    cargo fetch {{ args }}

[group('setup')]
update:
    cargo install --path crates/dx-story-cli --locked --force

[group('quality')]
fmt:
    cargo fmt --all
    RUST_LOG=warn taplo fmt

[group('quality')]
fmt-check:
    cargo fmt --all --check
    RUST_LOG=warn taplo fmt --check

[group('quality')]
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

[group('quality')]
check: fmt-check lint

[group('quality')]
fix *args:
    cargo clippy --fix --workspace --all-targets --all-features --allow-dirty {{ args }}
    just fmt

[group('quality')]
test *args:
    @cargo run --quiet -p xtask -- test {{ args }}
