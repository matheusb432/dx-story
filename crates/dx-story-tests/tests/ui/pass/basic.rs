use dioxus::prelude::*;
use facade::{stories, story};

const EXTRA_DOCUMENTATION: &str = "Extra documentation.";

#[story]
fn default_state() -> Element {
    rsx! { button { "Default" } }
}

/// A downstream story set using a renamed facade dependency.
#[stories(
    id = "renamed-dependency",
    name = "Renamed dependency",
    extra_docs = EXTRA_DOCUMENTATION
)]
const RENAMED_DEPENDENCY_STORY_SET: () = &[default_state];

fn main() {
    let _ = facade::find("renamed-dependency", "default-state");
}
