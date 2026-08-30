use dioxus::prelude::*;
use facade::{preview, showcase};

#[preview(id = "bad--id", name = " Padded ")]
fn bad__name() -> Element {
    rsx! { "Bad" }
}

#[showcase(id = "bad-_showcase", name = " Showcase ")]
const BAD: () = &[bad__name];

fn main() {}
