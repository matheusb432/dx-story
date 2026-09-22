use proc_macro::TokenStream;

mod attributes;
mod diagnostics;
mod facade;
mod metadata;
mod naming;
mod stories;
mod story;

/// Implements `dx_story::stories`; use the re-export from the dx-story crate.
#[proc_macro_attribute]
pub fn stories(arguments: TokenStream, item: TokenStream) -> TokenStream {
    stories::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `dx_story::story`; use the re-export from the dx-story crate.
#[proc_macro_attribute]
pub fn story(arguments: TokenStream, item: TokenStream) -> TokenStream {
    story::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
