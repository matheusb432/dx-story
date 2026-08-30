mod previews {
    use dioxus::prelude::*;
    use facade::preview;

    #[preview]
    pub fn visible() -> Element {
        rsx! { "Visible" }
    }
}

fn main() {
    let _ = previews::PREVIEW_VISIBLE;
}
