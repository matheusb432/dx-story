use dioxus::prelude::*;
use facade::{stories, story};

#[story]
fn default() -> Element {
    rsx! { "Default" }
}

#[stories(id = "duplicate", name = "Duplicate")]
const DUPLICATE: () = &[default, default];

fn main() {}
