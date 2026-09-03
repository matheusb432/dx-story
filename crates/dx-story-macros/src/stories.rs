use std::collections::BTreeMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    Expr, ExprArray, ItemConst, LitStr, Meta, MetaNameValue, Path, Token, Type, ext::IdentExt,
    parse::Parser, punctuated::Punctuated, spanned::Spanned,
};

use crate::{attributes, diagnostics, facade, metadata::MetadataArguments, naming};

struct StorySetArguments {
    id: LitStr,
    name: LitStr,
    extra_docs: Option<Path>,
    thumbnail: Option<Ident>,
}

#[derive(Default)]
struct StorySetArgumentsBuilder {
    metadata: MetadataArguments,
    extra_docs: Option<Path>,
    thumbnail: Option<Ident>,
    errors: Option<syn::Error>,
}

pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let item = syn::parse2::<ItemConst>(item)?;
    let mut errors = None;
    let arguments = diagnostics::capture(&mut errors, StorySetArguments::parse(arguments));
    let story_idents = diagnostics::capture(&mut errors, parse_story_idents(&item));
    if let Err(error) = validate_unit_type(&item.ty) {
        diagnostics::combine(&mut errors, error);
    }
    if let Err(error) = attributes::validate(&item.attrs) {
        diagnostics::combine(&mut errors, error);
    }
    let facade = diagnostics::capture(&mut errors, facade::path(item.ident.span()));
    diagnostics::finish(errors)?;

    let arguments = diagnostics::validated(arguments, "stories arguments", item.ident.span())?;
    let story_idents =
        diagnostics::validated(story_idents, "registered stories", item.ident.span())?;
    let facade = diagnostics::validated(facade, "dx-story facade", item.ident.span())?;
    let story_set_const_ident = naming::story_set_const_ident(&item.ident);
    let documentation_const_ident = Ident::new(
        &format!("{story_set_const_ident}_DOCUMENTATION"),
        item.ident.span(),
    );
    let story_const_idents = story_idents
        .iter()
        .map(naming::story_const_ident)
        .collect::<Vec<_>>();
    let thumbnail = arguments.thumbnail.as_ref().map(naming::story_const_ident);
    let thumbnail = thumbnail.map_or_else(|| quote!(None), |thumbnail| quote!(Some(#thumbnail)));
    let description = description_expression(
        &facade,
        attributes::documentation(&item.attrs),
        arguments.extra_docs.as_ref(),
        item.ident.span(),
    );
    let story_set_id = arguments.id;
    let story_set_name = arguments.name;
    let generated_attributes = attributes::generated(&item.attrs);
    let configuration_attributes = attributes::configuration(&item.attrs);

    Ok(quote! {
        #(#generated_attributes)*
        const #documentation_const_ident: Option<&'static str> = #description;

        #(#generated_attributes)*
        const #story_set_const_ident: &'static #facade::StorySet =
            &#facade::__private::story_set(
                #story_set_id,
                #story_set_name,
                #documentation_const_ident,
                #thumbnail,
                &[#(#story_const_idents),*],
            );

        #(#configuration_attributes)*
        #facade::__private::submit! {
            #story_set_const_ident
        }
    })
}

impl StorySetArguments {
    fn parse(arguments: TokenStream) -> syn::Result<Self> {
        let parsed = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(arguments)?;
        let mut builder = StorySetArgumentsBuilder::default();
        for argument in parsed {
            builder.push(argument);
        }
        builder.finish()
    }
}

impl StorySetArgumentsBuilder {
    fn push(&mut self, argument: Meta) {
        let Meta::NameValue(argument) = argument else {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(
                    argument,
                    "stories arguments must use `key = value` syntax",
                ),
            );
            return;
        };

        if self.metadata.push(&argument, &mut self.errors) {
            return;
        }
        if argument.path.is_ident("extra_docs") {
            self.parse_extra_docs(argument);
        } else if argument.path.is_ident("thumbnail") {
            self.parse_thumbnail(argument);
        } else {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(argument.path, "unknown stories argument"),
            );
        }
    }

    fn parse_extra_docs(&mut self, argument: MetaNameValue) {
        if self.extra_docs.is_some() {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(argument, "duplicate `extra_docs` argument"),
            );
            return;
        }

        if let Expr::Path(path) = argument.value {
            self.extra_docs = Some(path.path);
        } else {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(
                    argument.value,
                    "`extra_docs` must be a path to a `&'static str` constant",
                ),
            );
        }
    }

    fn parse_thumbnail(&mut self, argument: MetaNameValue) {
        if self.thumbnail.is_some() {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(argument, "duplicate `thumbnail` argument"),
            );
            return;
        }
        let Expr::Path(path) = argument.value else {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(
                    argument.value,
                    "`thumbnail` must be an unqualified story function name",
                ),
            );
            return;
        };
        let Some(ident) = path.path.get_ident() else {
            diagnostics::combine(
                &mut self.errors,
                syn::Error::new_spanned(
                    path,
                    "`thumbnail` must be an unqualified story function name",
                ),
            );
            return;
        };
        self.thumbnail = Some(ident.clone());
    }

    fn finish(mut self) -> syn::Result<StorySetArguments> {
        self.metadata.id = required_argument(self.metadata.id, "id", &mut self.errors);
        self.metadata.name = required_argument(self.metadata.name, "name", &mut self.errors);
        self.metadata.validate("stories", &mut self.errors);

        diagnostics::finish(self.errors)?;
        Ok(StorySetArguments {
            id: diagnostics::validated(self.metadata.id, "stories ID", Span::call_site())?,
            name: diagnostics::validated(self.metadata.name, "stories name", Span::call_site())?,
            extra_docs: self.extra_docs,
            thumbnail: self.thumbnail,
        })
    }
}

fn required_argument(
    value: Option<LitStr>,
    name: &str,
    errors: &mut Option<syn::Error>,
) -> Option<LitStr> {
    if value.is_none() {
        diagnostics::combine(
            errors,
            syn::Error::new(
                Span::call_site(),
                format!("stories requires the `{name}` argument"),
            ),
        );
    }
    value
}

fn parse_story_idents(item: &ItemConst) -> syn::Result<Vec<Ident>> {
    let array = match &*item.expr {
        Expr::Array(array) => array,
        Expr::Reference(reference) => match &*reference.expr {
            Expr::Array(array) => array,
            expression => {
                return Err(syn::Error::new_spanned(
                    expression,
                    "`#[stories]` const must reference an array of story function names",
                ));
            }
        },
        expression => {
            return Err(syn::Error::new_spanned(
                expression,
                "`#[stories]` const must contain an array of story function names",
            ));
        }
    };
    extract_story_idents(array)
}

fn extract_story_idents(array: &ExprArray) -> syn::Result<Vec<Ident>> {
    let mut stories = Vec::with_capacity(array.elems.len());
    let mut first_occurrences = BTreeMap::new();
    let mut errors = None;

    if array.elems.is_empty() {
        diagnostics::combine(
            &mut errors,
            syn::Error::new(array.span(), "stories must contain at least one story"),
        );
    }

    for expression in &array.elems {
        let Expr::Path(path) = expression else {
            diagnostics::combine(
                &mut errors,
                syn::Error::new_spanned(expression, "expected a story function name"),
            );
            continue;
        };
        let Some(ident) = path.path.get_ident() else {
            diagnostics::combine(
                &mut errors,
                syn::Error::new_spanned(
                    path,
                    "story function names must be unqualified identifiers",
                ),
            );
            continue;
        };
        let canonical = ident.unraw().to_string();
        if first_occurrences
            .insert(canonical.clone(), ident.span())
            .is_some()
        {
            diagnostics::combine(
                &mut errors,
                syn::Error::new(
                    ident.span(),
                    format!("duplicate story `{canonical}` in `#[stories]` declaration"),
                ),
            );
        }
        stories.push(ident.clone());
    }

    diagnostics::finish(errors)?;
    Ok(stories)
}

fn validate_unit_type(ty: &Type) -> syn::Result<()> {
    if let Type::Tuple(tuple) = ty
        && tuple.elems.is_empty()
    {
        return Ok(());
    }

    Err(syn::Error::new_spanned(
        ty,
        "`#[stories]` const type must be `()`",
    ))
}

fn description_expression(
    facade: &TokenStream,
    documentation: Option<String>,
    extra_docs: Option<&Path>,
    span: Span,
) -> TokenStream {
    match (documentation, extra_docs) {
        (Some(documentation), Some(extra_docs)) => {
            let documentation = LitStr::new(&documentation, span);
            quote!(Some(#facade::__private::concatcp!(#documentation, "\n", #extra_docs)))
        }
        (Some(documentation), None) => {
            let documentation = LitStr::new(&documentation, span);
            quote!(Some(#documentation))
        }
        (None, Some(extra_docs)) => quote!(Some(#extra_docs)),
        (None, None) => quote!(None),
    }
}
