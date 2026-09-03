use dioxus::prelude::*;
use facade::{stories, story};

#[story(id = "bad--id", name = " Padded ")]
fn bad__name() -> Element {
    rsx! { "Bad" }
}

#[stories(id = "bad-_stories", name = " Stories ")]
const BAD: () = &[bad__name];

fn main() {}
