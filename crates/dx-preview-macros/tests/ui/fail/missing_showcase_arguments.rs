use dioxus::prelude::*;
use facade::{preview, showcase};

#[preview]
fn default() -> Element {
    rsx! { div { "Default" } }
}

#[showcase(name = "Missing ID")]
const MISSING_ID: () = &[default];

#[showcase(id = "missing-name")]
const MISSING_NAME: () = &[default];

fn main() {}
