use dioxus::prelude::*;
use facade::{preview, showcase};

mod public_preview {
    use dioxus::prelude::*;
    use facade::preview;

    #[preview]
    pub fn visible() -> Element {
        rsx! { "Visible" }
    }
}

#[allow(clippy::let_unit_value)]
#[preview]
fn default() -> Element {
    let _value = ();
    rsx! { "Default" }
}

#[allow(non_upper_case_globals)]
#[showcase(id = "attributes", name = "Attributes")]
const attributes: () = &[default];

fn main() {
    let _ = public_preview::visible();
}
