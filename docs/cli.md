# CLI configuration and lifecycle

Invoke the CLI through its local Cargo manifest. For example, another project keeps:

```sh
cargo run --quiet --manifest-path ../../shared-libs/dx-story/Cargo.toml -p dx-story-cli -- serve
```

The executable name remains `dx-story`. Run a subcommand with `--help` for its
arguments. No globally installed dx-story executable is required.

## Configuration

The CLI searches the current directory and its ancestors for `dx-story.toml`.
`--config path/to/file.toml` selects an exact file. Configuration-relative paths
are resolved independently of the invocation directory.

A minimal configuration is:

```toml
[catalog]
package = "my-ui"
```

Cargo metadata supplies the package directory, the `component-catalog` example
(or the only example), its required features, and source directories of resolved
local dependencies. Multiple nonconventional examples require `catalog.example`.
Registry and Git checkout sources are excluded from automatic source discovery.

Additional options:

```toml
[catalog]
package = "my-ui"
example = "component-preview"
# path = "crates/my-ui"             # Relative to the configuration directory.
# features = ["component-preview"]  # Replaces inferred required features.
# default-features = false          # Default: false.
# locked = true                     # Default: true.
# extra-source-directories = ["fixtures"]
# source-directories = ["src", "dev"] # Replaces automatic discovery.

[serve]
port = 8080
open = false

[tailwind]
input = "dev/styles/tailwind.css"
output = "assets/component-preview.css"
```

Source and Tailwind paths are relative to the selected package directory.
Existing configurations with explicit paths, features, and watched directories
remain supported. Explicit feature lists must enable the example's required
features. Omit `features` to infer them; an empty list explicitly enables none.

Serve flags override `[serve]` settings, which override built-in defaults. There
are no dx-story-specific environment overrides. Child tools retain their normal
environment, except `CARGO_INCREMENTAL` is enabled for development and disabled
with `--no-watch`, and `RUSTC_WRAPPER` is cleared for Dioxus.

## Styles

Omit `[tailwind]` for plain CSS or an existing stylesheet workflow. The catalog
embeds its own styles; load product styles in your custom root using Dioxus
`document::Link` or `document::Style`.

When `[tailwind]` is present, provide Deno, the `@tailwindcss/cli` import in
`deno.json`, and a matching `deno.lock`. dx-story uses the consumer's Deno setup
with `--frozen`; it does not install Node or manage package versions.
`styles` performs one build. A catalog without `[tailwind]` reports that no
stylesheet build is needed. Serving builds styles before starting Dioxus and
supervises the stylesheet watcher. An exited watcher fails the serve command.

## Development and diagnostics

`doctor` resolves and validates the Cargo example, checks that `dx --version`
matches the resolved Dioxus version, checks the installed Wasm target, and checks
Deno/Tailwind when configured. It lists the resolved source directories. It does
not install tools or compile the catalog. `serve` runs these checks before
starting its children.

Dioxus handles edits to existing files. dx-story inventories local Rust paths
and restarts Dioxus when files are added or removed. Build directories and common
caches are excluded; scans are bounded. Module declarations still determine
which files Rust compiles. Restart the command after changing configuration,
Cargo dependencies, or enabled features.

The server binds loopback by default. It rejects an occupied address and reports
HTTP readiness with a bounded timeout. Readiness excludes Dioxus's initial build
placeholder, but does not prove successful browser rendering or Wasm execution.
Ctrl-C and termination requests shut down the supervised process groups. A shared
asset lock prevents concurrent stylesheet writers; acquiring it times out after
30 seconds.

## Test automation

```sh
cargo run --quiet --manifest-path ../dx-story/Cargo.toml -p dx-story-cli -- serve --no-watch --ready-json --ready-timeout 300 --port 8091
```

`--no-watch` disables both watchers and hot reload. `--ready-json` disables the
Dioxus terminal interface, routes child output to stderr, and flushes an event to
stdout when HTTP is ready:

```json
{"event":"ready","url":"http://127.0.0.1:8091"}
```

Keep the process alive while tests run, consume its event, and send SIGINT or
SIGTERM when finished. Without `--no-watch`, a restart emits another ready event.
The test runner still owns its sandbox, chosen port, logs, and browser assertions.
Startup failure or timeout exits nonzero. CLI usage errors exit 2; operational
failures exit 1. Normal shutdown after readiness exits 0.

Extra Dioxus arguments may follow `--`. With `--no-watch` or `--ready-json`, use
dx-story's own address, port, browser, and interaction flags so forwarded options
cannot invalidate its lifecycle contract.
