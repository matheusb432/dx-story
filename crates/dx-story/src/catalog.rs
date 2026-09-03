//! Catalog UI.

use std::num::NonZeroUsize;

use dioxus::prelude::*;

use crate::{RegistryError, Story, StorySet, story_sets};

mod markdown;
mod pagination;
mod story;

use markdown::summary;
use pagination::CatalogPage;
use story::{RenderStory, StoryView};

const CATALOG_STYLES: &str = include_str!("catalog/styles.css");
const DARK_READER_LOCK_META_NAME: &str = "darkreader-lock";

pub const DEFAULT_STORY_SETS_PER_PAGE: NonZeroUsize = match NonZeroUsize::new(12) {
    Some(value) => value,
    None => NonZeroUsize::MIN,
};

/// Catalog settings.
#[derive(Clone, Copy, Debug)]
pub struct CatalogConfig {
    title: &'static str,
    story_sets_per_page: NonZeroUsize,
    canvas_class: &'static str,
    sidebar_footer: Option<fn() -> Element>,
}

impl CatalogConfig {
    #[must_use]
    pub const fn new(title: &'static str) -> Self {
        Self {
            title,
            story_sets_per_page: DEFAULT_STORY_SETS_PER_PAGE,
            canvas_class: "",
            sidebar_footer: None,
        }
    }

    #[must_use]
    pub const fn with_story_sets_per_page(mut self, story_sets_per_page: NonZeroUsize) -> Self {
        self.story_sets_per_page = story_sets_per_page;
        self
    }

    /// Adds classes to every story canvas.
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

    const fn story_sets_per_page(self) -> NonZeroUsize {
        self.story_sets_per_page
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
            && self.story_sets_per_page == other.story_sets_per_page
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
pub(super) struct CatalogRegistry(&'static [&'static StorySet]);

impl CatalogRegistry {
    fn all(self) -> &'static [&'static StorySet] {
        self.0
    }

    pub(super) fn find(
        self,
        story_set_id: &str,
        story_id: &str,
    ) -> Option<(&'static StorySet, &'static Story)> {
        let story_set = self
            .0
            .iter()
            .copied()
            .find(|story_set| story_set.id() == story_set_id)?;
        let story = story_set
            .stories()
            .iter()
            .copied()
            .find(|story| story.id() == story_id)?;
        Some((story_set, story))
    }
}

#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub(super) enum CatalogRoute {
    #[layout(CatalogLayout)]
        #[route("/")]
        Home {},
        #[route("/stories/:story_set_id/:story_id")]
        StoryView { story_set_id: String, story_id: String },
    #[end_layout]
    #[route("/render/:story_set_id/:story_id")]
    RenderStory { story_set_id: String, story_id: String },
}

/// Renders the catalog.
#[component]
pub fn Catalog(config: CatalogConfig) -> Element {
    use_context_provider(|| config);
    let registered_story_sets = match story_sets() {
        Ok(story_sets) => story_sets,
        Err(error) => {
            return rsx! {
                document::Meta { name: DARK_READER_LOCK_META_NAME }
                document::Style { {CATALOG_STYLES} }
                document::Title { "{config.title()}" }
                RegistryFailure { error }
            };
        }
    };
    use_context_provider(|| CatalogRegistry(registered_story_sets));

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
        div { class: "dx-story",
            aside { class: "dxs-sidebar",
                header { class: "dxs-sidebar-header",
                    Link { class: "dxs-brand-link", to: CatalogRoute::Home {},
                        span { "{config.title()}" }
                    }
                }
                details { class: "dxs-sidebar-explorer", open: true,
                    summary { class: "dxs-mobile-menu-summary",
                        span { "Components" }
                        span { class: "dxs-mobile-menu-chevron", aria_hidden: "true", ">" }
                    }
                    div { class: "dxs-sidebar-panel",
                        SidebarSearch {
                            query: query.clone(),
                            onquerychange: move |next_query| sidebar_query.set(next_query),
                        }
                        SidebarNavigation { query }
                        SidebarFooter { class: "dxs-sidebar-footer" }
                    }
                }
            }
            main { class: "dxs-main", Outlet::<CatalogRoute> {} }
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
        label { class: "dxs-sidebar-search",
            span { class: "dxs-visually-hidden", "Filter components and stories" }
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
    let has_matches = registry.all().iter().any(|story_set| {
        sidebar_story_set_match(story_set, &query) != SidebarStorySetMatch::Hidden
    });

    rsx! {
        nav { class: "dxs-story-set-nav", aria_label: "Component stories",
            if has_matches {
                ul { class: "dxs-story-set-tree",
                    for story_set in registry.all() {
                        StorySetNavigation {
                            key: "{story_set.id()}",
                            story_set: *story_set,
                            query: query.clone(),
                            route: route.clone(),
                        }
                    }
                }
            } else {
                p { class: "dxs-sidebar-empty", role: "status", "No matching components." }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SidebarStorySetMatch {
    AllStories,
    MatchingStories,
    Hidden,
}

impl SidebarStorySetMatch {
    fn includes(self, story: &Story, query: &str) -> bool {
        match self {
            Self::AllStories => true,
            Self::MatchingStories => story_matches_query(story, query),
            Self::Hidden => false,
        }
    }
}

fn normalize_sidebar_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn sidebar_story_set_match(story_set: &StorySet, query: &str) -> SidebarStorySetMatch {
    if query.is_empty()
        || value_matches_query(story_set.id(), query)
        || value_matches_query(story_set.name(), query)
    {
        SidebarStorySetMatch::AllStories
    } else if story_set
        .stories()
        .iter()
        .any(|story| story_matches_query(story, query))
    {
        SidebarStorySetMatch::MatchingStories
    } else {
        SidebarStorySetMatch::Hidden
    }
}

fn story_matches_query(story: &Story, query: &str) -> bool {
    value_matches_query(story.id(), query) || value_matches_query(story.name(), query)
}

fn value_matches_query(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

fn sidebar_story_set_starts_open(active_story_set: bool, query: &str) -> bool {
    active_story_set || !query.is_empty()
}

#[component]
fn StorySetNavigation(story_set: &'static StorySet, query: String, route: CatalogRoute) -> Element {
    let matched = sidebar_story_set_match(story_set, &query);
    if matched == SidebarStorySetMatch::Hidden {
        return rsx! {};
    }
    let active_story_set = matches!(
        &route,
        CatalogRoute::StoryView { story_set_id, .. } if story_set_id == story_set.id()
    );
    let group_class = if active_story_set {
        "dxs-story-set-group dxs-story-set-group-active"
    } else {
        "dxs-story-set-group"
    };

    rsx! {
        li { class: group_class,
            details {
                class: "dxs-story-set-disclosure",
                "data-story-set": story_set.id(),
                open: sidebar_story_set_starts_open(active_story_set, &query),
                summary { class: "dxs-story-set-group-label",
                    span { class: "dxs-story-set-group-chevron", aria_hidden: "true" }
                    span { class: "dxs-story-set-group-name", "{story_set.name()}" }
                    span { class: "dxs-story-set-group-count", "{story_set.stories().len()}" }
                }
                ul { class: "dxs-story-list", aria_label: "{story_set.name()} stories",
                    for story in story_set
                        .stories()
                        .iter()
                        .copied()
                        .filter(|story| matched.includes(story, &query))
                    {
                        li {
                            Link {
                                class: if route_selects_story(&route, story_set, story) {
                                    "dxs-story-link dxs-story-link-active"
                                } else {
                                    "dxs-story-link"
                                },
                                to: CatalogRoute::StoryView {
                                    story_set_id: story_set.id().to_owned(),
                                    story_id: story.id().to_owned(),
                                },
                                aria_current: if route_selects_story(&route, story_set, story) {
                                    "page"
                                } else {
                                    "false"
                                },
                                span { class: "dxs-story-mark", aria_hidden: "true" }
                                span { "{story.name()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn route_selects_story(route: &CatalogRoute, story_set: &StorySet, story: &Story) -> bool {
    matches!(
        route,
        CatalogRoute::StoryView {
            story_set_id,
            story_id,
        } if story_set_id == story_set.id() && story_id == story.id()
    )
}

#[component]
fn Home() -> Element {
    let config = use_context::<CatalogConfig>();
    let registry = use_context::<CatalogRegistry>();
    let story_sets = registry.all();
    let mut page =
        use_signal(|| CatalogPage::first(story_sets.len(), config.story_sets_per_page()));
    let current_page = page();
    let visible_story_sets = &story_sets[current_page.item_range()];
    let previous_page = current_page.previous();
    let next_page = current_page.next();

    rsx! {
        section { class: "dxs-home",
            div { class: "dxs-home-inner",
                header { class: "dxs-home-header",
                    div {
                        p { class: "dxs-eyebrow", "Registered components" }
                        h1 { class: "dxs-home-title", "{config.title()}" }
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
                if story_sets.is_empty() {
                    div { class: "dxs-empty", role: "status",
                        strong { "No component stories are registered." }
                        span { "Compile at least one #[stories] declaration into this target." }
                    }
                } else {
                    div { class: "dxs-card-grid",
                        for story_set in visible_story_sets {
                            StorySetCard { key: "{story_set.id()}", story_set: *story_set }
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
        nav { class: "dxs-pagination", aria_label: "Component catalog pages",
            button {
                class: "dxs-icon-button",
                r#type: "button",
                aria_label: "Previous component page",
                disabled: previous_disabled,
                onclick: move |_| onprevious.call(()),
                span { aria_hidden: "true", "<" }
            }
            output { class: "dxs-page-count", aria_live: "polite",
                "Page {page_number} of {page_count}"
            }
            button {
                class: "dxs-icon-button",
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
fn StorySetCard(story_set: &'static StorySet) -> Element {
    let config = use_context::<CatalogConfig>();
    let canvas_class = format!("dxs-frame-content {}", config.canvas_class());
    let Some(first_story) = story_set.first_story() else {
        return rsx! {};
    };
    let Some(thumbnail) = story_set.thumbnail() else {
        return rsx! {};
    };
    let count = story_set.stories().len();
    let count_label = if count == 1 {
        "1 story".to_owned()
    } else {
        format!("{count} stories")
    };
    let description = story_set.description().map_or_else(
        || "Component stories.".to_owned(),
        |source| summary(source, 150),
    );
    let render = thumbnail.render();

    rsx! {
        article { class: "dxs-card",
            Link {
                class: "dxs-card-link",
                to: CatalogRoute::StoryView {
                    story_set_id: story_set.id().to_owned(),
                    story_id: first_story.id().to_owned(),
                },
                span { class: "dxs-visually-hidden", "Open {story_set.name()} stories" }
            }
            header { class: "dxs-card-header",
                div { class: "dxs-card-copy",
                    h2 { class: "dxs-card-title", "{story_set.name()}" }
                    p { class: "dxs-card-description", "{description}" }
                }
                span { class: "dxs-story-count", aria_label: count_label,
                    span { aria_hidden: "true", "#" }
                    span { aria_hidden: "true", "{count}" }
                }
            }
            div {
                class: "dxs-frame dxs-card-frame",
                "data-dx-story-thumbnail": story_set.id(),
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
        main { class: "dx-story dxs-message", role: "alert",
            p { class: "dxs-message-kicker dxs-message-kicker-danger", "Invalid story registry" }
            h1 { "Component story unavailable" }
            p { "{error}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;

    use super::{
        CatalogConfig, DEFAULT_STORY_SETS_PER_PAGE, SidebarStorySetMatch, normalize_sidebar_query,
        sidebar_story_set_match, sidebar_story_set_starts_open,
    };
    use crate::{Story, StorySet};

    fn render_story() -> Element {
        rsx! {}
    }

    static DEFAULT_STORY: Story =
        crate::__private::story("default", "Default", None, render_story, "fn default() {}");
    static OUTLINED_STORY: Story = crate::__private::story(
        "outlined",
        "Outlined button",
        None,
        render_story,
        "fn outlined() {}",
    );
    static BUTTON_STORIES: [&Story; 2] = [&DEFAULT_STORY, &OUTLINED_STORY];
    static BUTTON_STORY_SET: StorySet =
        crate::__private::story_set("button", "Button", None, None, &BUTTON_STORIES);

    #[test]
    fn catalog_configuration_has_bounded_defaults() {
        let config = CatalogConfig::new("UI catalog");

        assert_eq!(config.title(), "UI catalog");
        assert_eq!(config.story_sets_per_page(), DEFAULT_STORY_SETS_PER_PAGE);
        assert!(config.canvas_class().is_empty());
        assert!(config.sidebar_footer().is_none());
    }

    #[test]
    fn catalog_configuration_preserves_consumer_canvas_classes() {
        let config = CatalogConfig::new("UI catalog").with_canvas_class("font-sans app-theme");

        assert_eq!(config.canvas_class(), "font-sans app-theme");
    }

    #[test]
    fn sidebar_search_matches_story_sets_and_stories() {
        let cases = [
            ("", SidebarStorySetMatch::AllStories),
            (" button ", SidebarStorySetMatch::AllStories),
            ("BUTTON", SidebarStorySetMatch::AllStories),
            ("outlined", SidebarStorySetMatch::MatchingStories),
            ("missing", SidebarStorySetMatch::Hidden),
        ];

        for (query, expected) in cases {
            let query = normalize_sidebar_query(query);
            assert_eq!(sidebar_story_set_match(&BUTTON_STORY_SET, &query), expected);
        }
    }

    #[test]
    fn story_only_matches_hide_siblings() {
        let query = normalize_sidebar_query("OUTLINED");
        let matched = sidebar_story_set_match(&BUTTON_STORY_SET, &query);

        assert!(!matched.includes(&DEFAULT_STORY, &query));
        assert!(matched.includes(&OUTLINED_STORY, &query));
    }

    #[test]
    fn sidebar_components_open_only_for_navigation_context() {
        assert!(!sidebar_story_set_starts_open(false, ""));
        assert!(sidebar_story_set_starts_open(true, ""));
        assert!(sidebar_story_set_starts_open(false, "button"));
    }
}
