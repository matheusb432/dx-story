use dioxus::prelude::*;
use facade::{preview, showcase};

#[preview(id = "type_state", name = "Type state")]
fn r#type() -> Element {
    rsx! { "Type" }
}

#[showcase(id = "raw_showcase", name = "Raw showcase")]
const r#match: () = &[r#type];

fn main() {}
