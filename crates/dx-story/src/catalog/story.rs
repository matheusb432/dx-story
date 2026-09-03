use dioxus::prelude::*;

use super::{CatalogConfig, CatalogRegistry, CatalogRoute, markdown::Markdown};
use crate::{Story, StorySet};

#[component]
pub(super) fn StoryView(story_set_id: String, story_id: String) -> Element {
    let registry = use_context::<CatalogRegistry>();
    let Some((story_set, story)) = registry.find(&story_set_id, &story_id) else {
        return rsx! { MissingStory { story_set_id, story_id } };
    };
    let mut generation = use_signal(|| 0_u64);
    let render_path = format!("/render/{}/{}", story_set.id(), story.id());

    rsx! {
        section { class: "dxs-detail",
            header { class: "dxs-detail-header",
                StoryPicker { story_set, selected_story: story }
                div { class: "dxs-detail-heading-row",
                    div { class: "dxs-detail-copy",
                        p { class: "dxs-eyebrow", "{story_set.name()}" }
                        h1 { "{story.name()}" }
                        p { class: "dxs-detail-description",
                            {story.description().unwrap_or("Component story.")}
                        }
                    }
                    div { class: "dxs-detail-actions",
                        a {
                            class: "dxs-button dxs-button-secondary",
                            href: render_path,
                            target: "_blank",
                            rel: "noreferrer",
                            "Open canvas"
                        }
                        button {
                            class: "dxs-button",
                            r#type: "button",
                            onclick: move |_| *generation.write() += 1,
                            "Reset state"
                        }
                    }
                }
            }
            StoryCanvas { story_set, story, generation: generation() }
            StoryDocumentation { story_set, story }
        }
    }
}

#[component]
fn StoryPicker(story_set: &'static StorySet, selected_story: &'static Story) -> Element {
    let navigator = use_navigator();

    rsx! {
        label { class: "dxs-story-picker",
            span { "Story" }
            select {
                aria_label: "{story_set.name()} story",
                value: selected_story.id(),
                onchange: move |event| {
                    let selected_id = event.value();
                    let Some(next_story) = story_set
                        .stories()
                        .iter()
                        .copied()
                        .find(|story| story.id() == selected_id)
                    else {
                        return;
                    };
                    if next_story.id() != selected_story.id() {
                        navigator.push(CatalogRoute::StoryView {
                            story_set_id: story_set.id().to_owned(),
                            story_id: next_story.id().to_owned(),
                        });
                    }
                },
                for story in story_set.stories() {
                    option { key: "{story.id()}", value: story.id(), "{story.name()}" }
                }
            }
        }
    }
}

#[component]
fn StoryCanvas(story_set: &'static StorySet, story: &'static Story, generation: u64) -> Element {
    let config = use_context::<CatalogConfig>();
    let canvas_class = format!(
        "dxs-frame-content dxs-story-frame-content {}",
        config.canvas_class()
    );

    rsx! {
        div {
            class: "dxs-frame dxs-story-frame",
            "data-dx-story-ready": "true",
            "data-story-set": story_set.id(),
            "data-story": story.id(),
            div { class: canvas_class,
                for generation in [generation] {
                    StoryRender {
                        key: "{story_set.id()}-{story.id()}-{generation}",
                        story,
                    }
                }
            }
        }
    }
}

#[component]
fn StoryRender(story: &'static Story) -> Element {
    (story.render())()
}

#[component]
fn StoryDocumentation(story_set: &'static StorySet, story: &'static Story) -> Element {
    rsx! {
        section { class: "dxs-documentation",
            div { class: "dxs-documentation-inner",
                if let Some(description) = story_set.description() {
                    article { class: "dxs-markdown",
                        h2 { "Documentation" }
                        Markdown { source: description }
                    }
                }
                details { class: "dxs-source",
                    summary { "Rust source" }
                    pre { code { "{story.source()}" } }
                }
            }
        }
    }
}

#[component]
pub(super) fn RenderStory(story_set_id: String, story_id: String) -> Element {
    let config = use_context::<CatalogConfig>();
    let canvas_class = format!(
        "dxs-frame-content dxs-story-frame-content {}",
        config.canvas_class()
    );
    let registry = use_context::<CatalogRegistry>();
    let Some((story_set, story)) = registry.find(&story_set_id, &story_id) else {
        return rsx! { MissingStory { story_set_id, story_id } };
    };
    let render = story.render();

    rsx! {
        main {
            class: "dx-story dxs-frame dxs-render-frame",
            "data-dx-story-ready": "true",
            "data-story-set": story_set.id(),
            "data-story": story.id(),
            div { class: canvas_class, {render()} }
        }
    }
}

#[component]
fn MissingStory(story_set_id: String, story_id: String) -> Element {
    rsx! {
        section { class: "dxs-message",
            p { class: "dxs-message-kicker dxs-message-kicker-danger", "Unknown story" }
            h1 { "{story_set_id}/{story_id}" }
            p { "Choose a component from the catalog navigation." }
            Link { class: "dxs-button dxs-message-action", to: CatalogRoute::Home {},
                "Back to catalog"
            }
        }
    }
}
