use dioxus::prelude::*;
use facade::{stories, story};

#[inline]
#[story]
fn default() -> Element {
    rsx! { "Default" }
}

#[deprecated]
#[stories(id = "unsupported", name = "Unsupported")]
const UNSUPPORTED: () = &[default];

fn main() {}
