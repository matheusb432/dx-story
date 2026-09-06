use dioxus::prelude::*;
use dx_story::{stories, story};

#[story]
fn interactive() -> Element {
    let mut count = use_signal(|| 0_u32);
    rsx! {
        button { onclick: move |_| count += 1, "Clicked {count} times" }
    }
}

#[stories(id = "counter", name = "Counter")]
const COUNTER: () = &[interactive];

fn main() -> Result<(), dx_story::RegistryError> {
    dx_story::catalog::launch(dx_story::catalog::CatalogConfig::default())
}
