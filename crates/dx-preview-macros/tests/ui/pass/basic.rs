use dioxus::prelude::*;
use facade::{preview, showcase};

const EXTRA_DOCUMENTATION: &str = "Extra documentation.";

#[preview]
fn default_state() -> Element {
    rsx! { button { "Default" } }
}

/// A downstream showcase using a renamed facade dependency.
#[showcase(
    id = "renamed-dependency",
    name = "Renamed dependency",
    extra_docs = EXTRA_DOCUMENTATION
)]
const RENAMED_DEPENDENCY_SHOWCASE: () = &[default_state];

fn main() {
    let _ = facade::find("renamed-dependency", "default-state");
}
