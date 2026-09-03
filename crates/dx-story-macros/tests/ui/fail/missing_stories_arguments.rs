use dioxus::prelude::*;
use facade::{stories, story};

#[story]
fn default() -> Element {
    rsx! { div { "Default" } }
}

#[stories(name = "Missing ID")]
const MISSING_ID: () = &[default];

#[stories(id = "missing-name")]
const MISSING_NAME: () = &[default];

fn main() {}
