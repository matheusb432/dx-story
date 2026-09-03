mod stories {
    use dioxus::prelude::*;
    use facade::story;

    #[story]
    pub fn visible() -> Element {
        rsx! { "Visible" }
    }
}

fn main() {
    let _ = stories::STORY_VISIBLE;
}
