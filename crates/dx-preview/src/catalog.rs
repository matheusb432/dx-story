//! Catalog UI.

use std::num::NonZeroUsize;

use dioxus::prelude::*;

use crate::{Preview, RegistryError, Showcase, showcases};

mod markdown;
mod pagination;
mod showcase;

use markdown::summary;
use pagination::CatalogPage;
use showcase::{RenderPreview, ShowcaseView};

const CATALOG_STYLES: &str = include_str!("catalog/styles.css");
const DARK_READER_LOCK_META_NAME: &str = "darkreader-lock";

pub const DEFAULT_SHOWCASES_PER_PAGE: NonZeroUsize = match NonZeroUsize::new(12) {
    Some(value) => value,
    None => NonZeroUsize::MIN,
};

/// Catalog settings.
#[derive(Clone, Copy, Debug)]
pub struct CatalogConfig {
    title: &'static str,
    showcases_per_page: NonZeroUsize,
    canvas_class: &'static str,
    sidebar_footer: Option<fn() -> Element>,
}

impl CatalogConfig {
    #[must_use]
    pub const fn new(title: &'static str) -> Self {
        Self {
            title,
            showcases_per_page: DEFAULT_SHOWCASES_PER_PAGE,
            canvas_class: "",
            sidebar_footer: None,
        }
    }

    #[must_use]
    pub const fn with_showcases_per_page(mut self, showcases_per_page: NonZeroUsize) -> Self {
        self.showcases_per_page = showcases_per_page;
        self
    }

    /// Adds classes to every preview canvas.
    #[must_use]
    pub const fn with_canvas_class(mut self, canvas_class: &'static str) -> Self {
        self.canvas_class = canvas_class;
        self
    }

    /// Adds a sidebar footer. The renderer may use hooks and context.
    #[must_use]
    pub const fn with_sidebar_footer(mut self, render: fn() -> Element) -> Self {
        self.sidebar_footer = Some(render);
        self
    }

    const fn title(self) -> &'static str {
        self.title
    }

    const fn showcases_per_page(self) -> NonZeroUsize {
        self.showcases_per_page
    }

    pub(super) const fn canvas_class(self) -> &'static str {
        self.canvas_class
    }

    const fn sidebar_footer(self) -> Option<fn() -> Element> {
        self.sidebar_footer
    }
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self::new("Component catalog")
    }
}

impl PartialEq for CatalogConfig {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.showcases_per_page == other.showcases_per_page
            && self.canvas_class == other.canvas_class
            && optional_renderers_equal(self.sidebar_footer, other.sidebar_footer)
    }
}

impl Eq for CatalogConfig {}

fn optional_renderers_equal(left: Option<fn() -> Element>, right: Option<fn() -> Element>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => std::ptr::fn_addr_eq(left, right),
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

#[derive(Clone, Copy)]
pub(super) struct CatalogRegistry(&'static [&'static Showcase]);

impl CatalogRegistry {
    fn all(self) -> &'static [&'static Showcase] {
        self.0
    }

    pub(super) fn find(
        self,
        showcase_id: &str,
        preview_id: &str,
    ) -> Option<(&'static Showcase, &'static Preview)> {
        let showcase = self
            .0
            .iter()
            .copied()
            .find(|showcase| showcase.id() == showcase_id)?;
        let preview = showcase
            .previews()
            .iter()
            .copied()
            .find(|preview| preview.id() == preview_id)?;
        Some((showcase, preview))
    }
}

#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub(super) enum CatalogRoute {
    #[layout(CatalogLayout)]
        #[route("/")]
        Home {},
        #[route("/showcase/:showcase_id/:preview_id")]
        ShowcaseView { showcase_id: String, preview_id: String },
    #[end_layout]
    #[route("/render/:showcase_id/:preview_id")]
    RenderPreview { showcase_id: String, preview_id: String },
}

/// Renders the catalog.
#[component]
pub fn Catalog(config: CatalogConfig) -> Element {
    use_context_provider(|| config);
    let registered_showcases = match showcases() {
        Ok(showcases) => showcases,
        Err(error) => {
            return rsx! {
                document::Meta { name: DARK_READER_LOCK_META_NAME }
                document::Style { {CATALOG_STYLES} }
                document::Title { "{config.title()}" }
                RegistryFailure { error }
            };
        }
    };
    use_context_provider(|| CatalogRegistry(registered_showcases));

    rsx! {
        document::Meta { name: DARK_READER_LOCK_META_NAME }
        document::Style { {CATALOG_STYLES} }
        document::Title { "{config.title()}" }
        Router::<CatalogRoute> {}
    }
}

/// Validates the registry, then launches the catalog.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn launch(config: CatalogConfig) -> Result<(), RegistryError> {
    crate::init_registry()?;
    dioxus::LaunchBuilder::new()
        .with_context(config)
        .launch(launched_catalog);
    Ok(())
}

fn launched_catalog() -> Element {
    let config = use_context::<CatalogConfig>();
    rsx! { Catalog { config } }
}

#[component]
fn CatalogLayout() -> Element {
    let config = use_context::<CatalogConfig>();
    let mut sidebar_query = use_signal(String::new);
    let query = sidebar_query();

    rsx! {
        div { class: "dx-preview",
            aside { class: "dxb-sidebar",
                header { class: "dxb-sidebar-header",
                    Link { class: "dxb-brand-link", to: CatalogRoute::Home {},
                        span { "{config.title()}" }
                    }
                }
                details { class: "dxb-sidebar-explorer", open: true,
                    summary { class: "dxb-mobile-menu-summary",
                        span { "Components" }
                        span { class: "dxb-mobile-menu-chevron", aria_hidden: "true", ">" }
                    }
                    div { class: "dxb-sidebar-panel",
                        SidebarSearch {
                            query: query.clone(),
                            onquerychange: move |next_query| sidebar_query.set(next_query),
                        }
                        SidebarNavigation { query }
                        SidebarFooter { class: "dxb-sidebar-footer" }
                    }
                }
            }
            main { class: "dxb-main", Outlet::<CatalogRoute> {} }
        }
    }
}

#[component]
fn SidebarFooter(class: &'static str) -> Element {
    let config = use_context::<CatalogConfig>();
    let Some(render) = config.sidebar_footer() else {
        return rsx! {};
    };
    rsx! { div { class, {render()} } }
}

#[component]
fn SidebarSearch(query: String, onquerychange: EventHandler<String>) -> Element {
    rsx! {
        label { class: "dxb-sidebar-search",
            span { class: "dxb-visually-hidden", "Filter components and previews" }
            input {
                r#type: "search",
                value: query,
                placeholder: "Find components",
                autocomplete: "off",
                spellcheck: "false",
                oninput: move |event| onquerychange.call(event.value()),
            }
        }
    }
}

#[component]
fn SidebarNavigation(query: String) -> Element {
    let registry = use_context::<CatalogRegistry>();
    let route = use_route::<CatalogRoute>();
    let query = normalize_sidebar_query(&query);
    let has_matches = registry
        .all()
        .iter()
        .any(|showcase| sidebar_showcase_match(showcase, &query) != SidebarShowcaseMatch::Hidden);

    rsx! {
        nav { class: "dxb-showcase-nav", aria_label: "Component previews",
            if has_matches {
                ul { class: "dxb-showcase-tree",
                    for showcase in registry.all() {
                        ShowcaseNavigation {
                            key: "{showcase.id()}",
                            showcase: *showcase,
                            query: query.clone(),
                            route: route.clone(),
                        }
                    }
                }
            } else {
                p { class: "dxb-sidebar-empty", role: "status", "No matching components." }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SidebarShowcaseMatch {
    AllPreviews,
    MatchingPreviews,
    Hidden,
}

impl SidebarShowcaseMatch {
    fn includes(self, preview: &Preview, query: &str) -> bool {
        match self {
            Self::AllPreviews => true,
            Self::MatchingPreviews => preview_matches_query(preview, query),
            Self::Hidden => false,
        }
    }
}

fn normalize_sidebar_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn sidebar_showcase_match(showcase: &Showcase, query: &str) -> SidebarShowcaseMatch {
    if query.is_empty()
        || value_matches_query(showcase.id(), query)
        || value_matches_query(showcase.name(), query)
    {
        SidebarShowcaseMatch::AllPreviews
    } else if showcase
        .previews()
        .iter()
        .any(|preview| preview_matches_query(preview, query))
    {
        SidebarShowcaseMatch::MatchingPreviews
    } else {
        SidebarShowcaseMatch::Hidden
    }
}

fn preview_matches_query(preview: &Preview, query: &str) -> bool {
    value_matches_query(preview.id(), query) || value_matches_query(preview.name(), query)
}

fn value_matches_query(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

fn sidebar_showcase_starts_open(active_showcase: bool, query: &str) -> bool {
    active_showcase || !query.is_empty()
}

#[component]
fn ShowcaseNavigation(showcase: &'static Showcase, query: String, route: CatalogRoute) -> Element {
    let matched = sidebar_showcase_match(showcase, &query);
    if matched == SidebarShowcaseMatch::Hidden {
        return rsx! {};
    }
    let active_showcase = matches!(
        &route,
        CatalogRoute::ShowcaseView { showcase_id, .. } if showcase_id == showcase.id()
    );
    let group_class = if active_showcase {
        "dxb-showcase-group dxb-showcase-group-active"
    } else {
        "dxb-showcase-group"
    };

    rsx! {
        li { class: group_class,
            details {
                class: "dxb-showcase-disclosure",
                "data-showcase": showcase.id(),
                open: sidebar_showcase_starts_open(active_showcase, &query),
                summary { class: "dxb-showcase-group-label",
                    span { class: "dxb-showcase-group-chevron", aria_hidden: "true" }
                    span { class: "dxb-showcase-group-name", "{showcase.name()}" }
                    span { class: "dxb-showcase-group-count", "{showcase.previews().len()}" }
                }
                ul { class: "dxb-preview-list", aria_label: "{showcase.name()} previews",
                    for preview in showcase
                        .previews()
                        .iter()
                        .copied()
                        .filter(|preview| matched.includes(preview, &query))
                    {
                        li {
                            Link {
                                class: if route_selects_preview(&route, showcase, preview) {
                                    "dxb-preview-link dxb-preview-link-active"
                                } else {
                                    "dxb-preview-link"
                                },
                                to: CatalogRoute::ShowcaseView {
                                    showcase_id: showcase.id().to_owned(),
                                    preview_id: preview.id().to_owned(),
                                },
                                aria_current: if route_selects_preview(&route, showcase, preview) {
                                    "page"
                                } else {
                                    "false"
                                },
                                span { class: "dxb-preview-mark", aria_hidden: "true" }
                                span { "{preview.name()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn route_selects_preview(route: &CatalogRoute, showcase: &Showcase, preview: &Preview) -> bool {
    matches!(
        route,
        CatalogRoute::ShowcaseView {
            showcase_id,
            preview_id,
        } if showcase_id == showcase.id() && preview_id == preview.id()
    )
}

#[component]
fn Home() -> Element {
    let config = use_context::<CatalogConfig>();
    let registry = use_context::<CatalogRegistry>();
    let showcases = registry.all();
    let mut page = use_signal(|| CatalogPage::first(showcases.len(), config.showcases_per_page()));
    let current_page = page();
    let visible_showcases = &showcases[current_page.item_range()];
    let previous_page = current_page.previous();
    let next_page = current_page.next();

    rsx! {
        section { class: "dxb-home",
            div { class: "dxb-home-inner",
                header { class: "dxb-home-header",
                    div {
                        p { class: "dxb-eyebrow", "Registered components" }
                        h1 { class: "dxb-home-title", "{config.title()}" }
                    }
                    CatalogPagination {
                        page_number: current_page.page_number(),
                        page_count: current_page.page_count(),
                        previous_disabled: previous_page.is_none(),
                        next_disabled: next_page.is_none(),
                        onprevious: move |()| {
                            if let Some(previous_page) = previous_page {
                                page.set(previous_page);
                            }
                        },
                        onnext: move |()| {
                            if let Some(next_page) = next_page {
                                page.set(next_page);
                            }
                        },
                    }
                }
                if showcases.is_empty() {
                    div { class: "dxb-empty", role: "status",
                        strong { "No component showcases are registered." }
                        span { "Compile at least one #[showcase] declaration into this target." }
                    }
                } else {
                    div { class: "dxb-card-grid",
                        for showcase in visible_showcases {
                            ShowcaseCard { key: "{showcase.id()}", showcase: *showcase }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CatalogPagination(
    page_number: NonZeroUsize,
    page_count: NonZeroUsize,
    previous_disabled: bool,
    next_disabled: bool,
    onprevious: EventHandler<()>,
    onnext: EventHandler<()>,
) -> Element {
    rsx! {
        nav { class: "dxb-pagination", aria_label: "Component catalog pages",
            button {
                class: "dxb-icon-button",
                r#type: "button",
                aria_label: "Previous component page",
                disabled: previous_disabled,
                onclick: move |_| onprevious.call(()),
                span { aria_hidden: "true", "<" }
            }
            output { class: "dxb-page-count", aria_live: "polite",
                "Page {page_number} of {page_count}"
            }
            button {
                class: "dxb-icon-button",
                r#type: "button",
                aria_label: "Next component page",
                disabled: next_disabled,
                onclick: move |_| onnext.call(()),
                span { aria_hidden: "true", ">" }
            }
        }
    }
}

#[component]
fn ShowcaseCard(showcase: &'static Showcase) -> Element {
    let config = use_context::<CatalogConfig>();
    let canvas_class = format!("dxb-frame-content {}", config.canvas_class());
    let Some(first_preview) = showcase.first_preview() else {
        return rsx! {};
    };
    let Some(thumbnail) = showcase.thumbnail() else {
        return rsx! {};
    };
    let count = showcase.previews().len();
    let count_label = if count == 1 {
        "1 preview".to_owned()
    } else {
        format!("{count} previews")
    };
    let description = showcase.description().map_or_else(
        || "Component showcase.".to_owned(),
        |source| summary(source, 150),
    );
    let render = thumbnail.render();

    rsx! {
        article { class: "dxb-card",
            Link {
                class: "dxb-card-link",
                to: CatalogRoute::ShowcaseView {
                    showcase_id: showcase.id().to_owned(),
                    preview_id: first_preview.id().to_owned(),
                },
                span { class: "dxb-visually-hidden", "Open {showcase.name()} showcase" }
            }
            header { class: "dxb-card-header",
                div { class: "dxb-card-copy",
                    h2 { class: "dxb-card-title", "{showcase.name()}" }
                    p { class: "dxb-card-description", "{description}" }
                }
                span { class: "dxb-preview-count", aria_label: count_label,
                    span { aria_hidden: "true", "#" }
                    span { aria_hidden: "true", "{count}" }
                }
            }
            div {
                class: "dxb-frame dxb-card-frame",
                "data-dx-preview-thumbnail": showcase.id(),
                inert: true,
                aria_hidden: "true",
                div { class: canvas_class, {render()} }
            }
        }
    }
}

#[component]
fn RegistryFailure(error: RegistryError) -> Element {
    rsx! {
        main { class: "dx-preview dxb-message", role: "alert",
            p { class: "dxb-message-kicker dxb-message-kicker-danger", "Invalid showcase registry" }
            h1 { "Component preview unavailable" }
            p { "{error}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;

    use super::{
        CatalogConfig, DEFAULT_SHOWCASES_PER_PAGE, SidebarShowcaseMatch, normalize_sidebar_query,
        sidebar_showcase_match, sidebar_showcase_starts_open,
    };
    use crate::{Preview, Showcase};

    fn render_preview() -> Element {
        rsx! {}
    }

    static DEFAULT_PREVIEW: Preview = crate::__private::preview(
        "default",
        "Default",
        None,
        render_preview,
        "fn default() {}",
    );
    static OUTLINED_PREVIEW: Preview = crate::__private::preview(
        "outlined",
        "Outlined button",
        None,
        render_preview,
        "fn outlined() {}",
    );
    static BUTTON_PREVIEWS: [&Preview; 2] = [&DEFAULT_PREVIEW, &OUTLINED_PREVIEW];
    static BUTTON_SHOWCASE: Showcase =
        crate::__private::showcase("button", "Button", None, None, &BUTTON_PREVIEWS);

    #[test]
    fn catalog_configuration_has_bounded_defaults() {
        let config = CatalogConfig::new("UI catalog");

        assert_eq!(config.title(), "UI catalog");
        assert_eq!(config.showcases_per_page(), DEFAULT_SHOWCASES_PER_PAGE);
        assert!(config.canvas_class().is_empty());
        assert!(config.sidebar_footer().is_none());
    }

    #[test]
    fn catalog_configuration_preserves_consumer_canvas_classes() {
        let config = CatalogConfig::new("UI catalog").with_canvas_class("font-sans app-theme");

        assert_eq!(config.canvas_class(), "font-sans app-theme");
    }

    #[test]
    fn sidebar_search_matches_showcases_and_previews() {
        let cases = [
            ("", SidebarShowcaseMatch::AllPreviews),
            (" button ", SidebarShowcaseMatch::AllPreviews),
            ("BUTTON", SidebarShowcaseMatch::AllPreviews),
            ("outlined", SidebarShowcaseMatch::MatchingPreviews),
            ("missing", SidebarShowcaseMatch::Hidden),
        ];

        for (query, expected) in cases {
            let query = normalize_sidebar_query(query);
            assert_eq!(sidebar_showcase_match(&BUTTON_SHOWCASE, &query), expected);
        }
    }

    #[test]
    fn preview_only_matches_hide_siblings() {
        let query = normalize_sidebar_query("OUTLINED");
        let matched = sidebar_showcase_match(&BUTTON_SHOWCASE, &query);

        assert!(!matched.includes(&DEFAULT_PREVIEW, &query));
        assert!(matched.includes(&OUTLINED_PREVIEW, &query));
    }

    #[test]
    fn sidebar_components_open_only_for_navigation_context() {
        assert!(!sidebar_showcase_starts_open(false, ""));
        assert!(sidebar_showcase_starts_open(true, ""));
        assert!(sidebar_showcase_starts_open(false, "button"));
    }
}
