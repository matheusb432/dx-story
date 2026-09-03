use dioxus::prelude::*;
use facade::{stories, story};

#[story(id = "type_state", name = "Type state")]
fn r#type() -> Element {
    rsx! { "Type" }
}

#[stories(id = "raw-stories", name = "Raw stories")]
const r#match: () = &[r#type];

fn main() {}
