use proc_macro::TokenStream;

mod attributes;
mod diagnostics;
mod facade;
mod metadata;
mod naming;
mod stories;
mod story;

/// Registers a story set from a const array of [`story`] functions.
///
/// `id` and `name` are required. `thumbnail` selects the card story;
/// `extra_docs` appends Markdown. The const must have type `()` and contain
/// `&[story_name, ...]`.
///
/// ```
/// use dioxus::prelude::*;
/// # use facade as dx_story;
/// use dx_story::{stories, story};
///
/// #[story]
/// fn default() -> Element {
///     rsx! { "Default" }
/// }
///
/// #[stories(id = "example", name = "Example")]
/// const EXAMPLE: () = &[default];
/// ```
#[proc_macro_attribute]
pub fn stories(arguments: TokenStream, item: TokenStream) -> TokenStream {
    stories::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Registers a zero-argument Dioxus render function.
///
/// The function must be non-generic and return `Element`. `id` and `name`
/// default to values derived from the function name.
///
/// ```
/// use dioxus::prelude::*;
/// # use facade as dx_story;
/// use dx_story::story;
///
/// #[story(id = "with_count", name = "With count")]
/// fn stateful() -> Element {
///     let count = use_signal(|| 0_u32);
///     rsx! { output { "{count}" } }
/// }
/// ```
#[proc_macro_attribute]
pub fn story(arguments: TokenStream, item: TokenStream) -> TokenStream {
    story::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
