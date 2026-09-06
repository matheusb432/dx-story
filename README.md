# dx-story

Dioxus component stories, a browser catalog, and local development commands.
Stories render real components with hooks and context. The catalog provides
navigation, thumbnails, source display, and state reset.

The library and CLI are private and consumed through relative paths while the
API matures. No installation or publication is required.

## Try the catalog

From this checkout, with the matching Dioxus CLI and Wasm target available:

```sh
cargo run --quiet -p dx-story-cli -- doctor
cargo run --quiet -p dx-story-cli -- serve --open
```

The included counter example uses the catalog's embedded CSS. Deno and Tailwind
are only needed by consumers that configure a Tailwind build.

## Add stories to a consumer

From the consumer workspace, substitute the relative path to this checkout:

```sh
cargo run --quiet --manifest-path ../dx-story/Cargo.toml -p dx-story-cli -- init --package my-ui --dry-run
cargo run --quiet --manifest-path ../dx-story/Cargo.toml -p dx-story-cli -- init --package my-ui
cargo run --quiet --manifest-path ../dx-story/Cargo.toml -p dx-story-cli -- doctor
cargo run --quiet --manifest-path ../dx-story/Cargo.toml -p dx-story-cli -- serve --open
```

`init` adds a local optional dependency, a Cargo feature and example, one story,
and configuration. It preserves manifest comments and rejects existing output
files before writing. `--dry-run` prints the proposed contents. Initialization
resolves dependencies into the consumer's lockfile; subsequent development
commands use the lockfile by default.

The default setup puts stories in the example, where they can import public
components. Use `init --embedded` for crate-private components: it places stories
inside the library behind a feature and generates an editable catalog root.
Provide application context and assets in that root. Keep production component
modules in their existing locations.

Add each story module to the Rust module tree explicitly. `#[story]` annotates a
zero-argument function returning `Element`; `#[stories]` defines its set and
routes. The library's [counter example](crates/dx-story/examples/component-catalog.rs)
shows the complete minimal API.

See [CLI configuration and lifecycle](docs/cli.md) for overrides, styling,
watching, and test automation.
