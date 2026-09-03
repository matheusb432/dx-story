use dioxus::prelude::*;
use facade::{stories, story};

mod public_story {
    use dioxus::prelude::*;
    use facade::story;

    #[story]
    pub fn visible() -> Element {
        rsx! { "Visible" }
    }
}

#[allow(clippy::let_unit_value)]
#[story]
fn default() -> Element {
    let _value = ();
    rsx! { "Default" }
}

#[allow(non_upper_case_globals)]
#[stories(id = "attributes", name = "Attributes")]
const attributes: () = &[default];

fn main() {
    let _ = public_story::visible();
}
