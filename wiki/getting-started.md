# Getting started

From a Cargo workspace containing your components:

```sh
cargo install dx-story-cli --locked
dx-story init --package my-ui --dry-run
dx-story init --package my-ui
# checks repo state and reports any actionable fix
dx-story doctor
dx-story serve --open
```

`init` creates a starter story in `dev/stories.rs`. A story set groups the states of one
component:

```rust
use dioxus::prelude::*;
use dx_story::{stories, story};

#[story]
fn interactive() -> Element {
    let mut count = use_signal(|| 0_u32);
    rsx! { button { onclick: move |_| count += 1, "Clicked {count} times" } }
}

#[stories(id = "counter", name = "Counter")]
const COUNTER: () = &[interactive];
```

Add more story modules to the Rust module tree so the compiler includes them. Use
`init --embedded` when stories need crate-private components.
