# dx-story

Dioxus component stories and a browser catalog for Rust. Stories render real
components with hooks and context. The catalog provides navigation, search,
thumbnail cards, Markdown descriptions, source display, and state reset.

Requires Rust 1.98 or newer and Dioxus 0.7.10 or newer within the 0.7 series.

| Package | Purpose |
| --- | --- |
| `dx-story` | Library, story attributes, registry, and optional browser catalog |
| `dx-story-cli` | Optional `dx-story` command for setup, serving, builds, and diagnostics |
| `dx-story-macros` | Supporting procedural macros, installed automatically by the library |

## Quick start with the CLI

From an existing Cargo workspace, select the package containing your components:

```sh
cargo install dx-story-cli --locked
dx-story init --package my-ui --dry-run
dx-story init --package my-ui
cargo install dioxus-cli --version 0.7.10 --locked
rustup target add wasm32-unknown-unknown
dx-story doctor
dx-story serve --open
```

Match the Dioxus CLI to the exact Dioxus version resolved by your project;
`doctor` reports mismatches and the installation command. The generated catalog
uses embedded CSS. Deno and Tailwind are only needed if you configure a Tailwind
build.

`init` adds a crates.io dependency, a feature-gated example, a starter story, and
configuration. It preserves manifest comments and rejects existing output files
before writing. `--dry-run` prints the proposed files. Use `init --embedded` when
stories need access to crate-private components; its editable catalog root can
supply application context and product styles.

Use `dx-story build --release` to bundle a catalog for static hosting. See the
[CLI guide](https://crates.io/crates/dx-story-cli) for configuration, styling,
watching, and browser-test automation.

## Use the library without the CLI

The CLI is optional. Add these entries to the component package's `Cargo.toml`,
keeping its existing Dioxus configuration if present:

```toml
[dependencies]
dioxus = { version = "0.7.10", default-features = false }
dx-story = { version = "0.1.0", optional = true }

[features]
component-catalog = ["dep:dx-story", "dx-story/catalog"]

[[example]]
name = "component-catalog"
required-features = ["component-catalog"]
```

Create `examples/component-catalog.rs`:

```rust
use dioxus::prelude::*;
use dx_story::{stories, story};

#[story]
fn interactive() -> Element {
    let mut count = use_signal(|| 0_u32);
    rsx! {
        button { onclick: move |_| count += 1, "Clicked {count} times" }
    }
}

#[stories(id = "counter", name = "Counter")]
const COUNTER: () = &[interactive];

fn main() -> Result<(), dx_story::RegistryError> {
    dx_story::catalog::launch(dx_story::catalog::CatalogConfig::default())
}
```

With a matching Dioxus CLI and the Wasm target installed:

```sh
dx serve --web --example component-catalog --no-default-features --features component-catalog
```

The library does not discover source files. Include each story module in the
Rust module tree. A `#[story]` function takes no arguments and returns `Element`.
Its snake-case function name supplies the route slug unless `id = "..."`
overrides it; `name = "..."` supplies a display label. Group stories with
`#[stories(id = "...", name = "...")]`. Doc comments provide Markdown
descriptions. An optional `thumbnail = thumbnail_story` selects a separate,
compact story for the catalog card.

## Library features

- No default features: story macros and registry, suitable for custom renderers.
- `csr`: browser launch support through `dx_story::launch`.
- `catalog`: the catalog shell, router, styles, and Markdown; includes `csr`.
- `benchmark-support`: internal benchmark access, not needed by consumers.

For context or custom assets, pass your own root to `dx_story::launch` and render
`dx_story::catalog::Catalog` inside it. Initialize providers above the catalog so
both the canvas and thumbnails receive the required context. Registry validation
rejects duplicate identifiers and empty story sets before launch.

See the [API documentation](https://docs.rs/dx-story) for the complete API.

## Work on this repository

```sh
just check
just test
cargo run --quiet -p dx-story-cli -- doctor
cargo run --quiet -p dx-story-cli -- serve --open
```

The checkout includes a counter catalog and `dx-story.toml`. To initialize a
consumer against a local checkout, pass `--library-path` pointing to its
`crates/dx-story` directory. Publication instructions are in `docs/releasing.md`.
