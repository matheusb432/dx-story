use dioxus::prelude::*;
use facade::{preview, showcase};

#[inline]
#[preview]
fn default() -> Element {
    rsx! { "Default" }
}

#[deprecated]
#[showcase(id = "unsupported", name = "Unsupported")]
const UNSUPPORTED: () = &[default];

fn main() {}
