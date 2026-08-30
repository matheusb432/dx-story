use dioxus::prelude::*;
use facade::{preview, showcase};

#[preview]
fn default() -> Element {
    rsx! { "Default" }
}

#[showcase(id = "duplicate", name = "Duplicate")]
const DUPLICATE: () = &[default, default];

fn main() {}
