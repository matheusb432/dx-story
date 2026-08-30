use proc_macro::TokenStream;

mod attributes;
mod diagnostics;
mod facade;
mod metadata;
mod naming;
mod preview;
mod showcase;

/// Registers a showcase from a const array of [`preview`] functions.
///
/// `id` and `name` are required. `thumbnail` selects the card preview;
/// `extra_docs` appends Markdown. The const must have type `()` and contain
/// `&[preview_name, ...]`.
///
/// ```
/// use dioxus::prelude::*;
/// # use facade as dx_preview;
/// use dx_preview::{preview, showcase};
///
/// #[preview]
/// fn default() -> Element {
///     rsx! { "Default" }
/// }
///
/// #[showcase(id = "example_showcase", name = "Example showcase")]
/// const EXAMPLE: () = &[default];
/// ```
#[proc_macro_attribute]
pub fn showcase(arguments: TokenStream, item: TokenStream) -> TokenStream {
    showcase::expand(arguments.into(), item.into())
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
/// # use facade as dx_preview;
/// use dx_preview::preview;
///
/// #[preview(id = "with_count", name = "With count")]
/// fn stateful() -> Element {
///     let count = use_signal(|| 0_u32);
///     rsx! { output { "{count}" } }
/// }
/// ```
#[proc_macro_attribute]
pub fn preview(arguments: TokenStream, item: TokenStream) -> TokenStream {
    preview::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
