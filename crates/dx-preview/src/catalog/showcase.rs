use dioxus::prelude::*;

use super::{CatalogConfig, CatalogRegistry, CatalogRoute, markdown::Markdown};
use crate::{Preview, Showcase};

#[component]
pub(super) fn ShowcaseView(showcase_id: String, preview_id: String) -> Element {
    let registry = use_context::<CatalogRegistry>();
    let Some((showcase, preview)) = registry.find(&showcase_id, &preview_id) else {
        return rsx! { MissingShowcase { showcase_id, preview_id } };
    };
    let mut generation = use_signal(|| 0_u64);
    let render_path = format!("/render/{}/{}", showcase.id(), preview.id());

    rsx! {
        section { class: "dxb-detail",
            header { class: "dxb-detail-header",
                PreviewPicker { showcase, selected_preview: preview }
                div { class: "dxb-detail-heading-row",
                    div { class: "dxb-detail-copy",
                        p { class: "dxb-eyebrow", "{showcase.name()}" }
                        h1 { "{preview.name()}" }
                        p { class: "dxb-detail-description",
                            {preview.description().unwrap_or("Component preview.")}
                        }
                    }
                    div { class: "dxb-detail-actions",
                        a {
                            class: "dxb-button dxb-button-secondary",
                            href: render_path,
                            target: "_blank",
                            rel: "noreferrer",
                            "Open canvas"
                        }
                        button {
                            class: "dxb-button",
                            r#type: "button",
                            onclick: move |_| *generation.write() += 1,
                            "Reset state"
                        }
                    }
                }
            }
            ShowcaseCanvas { showcase, preview, generation: generation() }
            ShowcaseDocumentation { showcase, preview }
        }
    }
}

#[component]
fn PreviewPicker(showcase: &'static Showcase, selected_preview: &'static Preview) -> Element {
    let navigator = use_navigator();

    rsx! {
        label { class: "dxb-preview-picker",
            span { "Preview" }
            select {
                aria_label: "{showcase.name()} preview",
                value: selected_preview.id(),
                onchange: move |event| {
                    let selected_id = event.value();
                    let Some(next_preview) = showcase
                        .previews()
                        .iter()
                        .copied()
                        .find(|preview| preview.id() == selected_id)
                    else {
                        return;
                    };
                    if next_preview.id() != selected_preview.id() {
                        navigator.push(CatalogRoute::ShowcaseView {
                            showcase_id: showcase.id().to_owned(),
                            preview_id: next_preview.id().to_owned(),
                        });
                    }
                },
                for preview in showcase.previews() {
                    option { key: "{preview.id()}", value: preview.id(), "{preview.name()}" }
                }
            }
        }
    }
}

#[component]
fn ShowcaseCanvas(
    showcase: &'static Showcase,
    preview: &'static Preview,
    generation: u64,
) -> Element {
    let config = use_context::<CatalogConfig>();
    let canvas_class = format!(
        "dxb-frame-content dxb-showcase-frame-content {}",
        config.canvas_class()
    );

    rsx! {
        div {
            class: "dxb-frame dxb-showcase-frame",
            "data-dx-preview-ready": "true",
            "data-showcase": showcase.id(),
            "data-preview": preview.id(),
            div { class: canvas_class,
                for generation in [generation] {
                    PreviewRender {
                        key: "{showcase.id()}-{preview.id()}-{generation}",
                        preview,
                    }
                }
            }
        }
    }
}

#[component]
fn PreviewRender(preview: &'static Preview) -> Element {
    (preview.render())()
}

#[component]
fn ShowcaseDocumentation(showcase: &'static Showcase, preview: &'static Preview) -> Element {
    rsx! {
        section { class: "dxb-documentation",
            div { class: "dxb-documentation-inner",
                if let Some(description) = showcase.description() {
                    article { class: "dxb-markdown",
                        h2 { "Documentation" }
                        Markdown { source: description }
                    }
                }
                details { class: "dxb-source",
                    summary { "Rust source" }
                    pre { code { "{preview.source()}" } }
                }
            }
        }
    }
}

#[component]
pub(super) fn RenderPreview(showcase_id: String, preview_id: String) -> Element {
    let config = use_context::<CatalogConfig>();
    let canvas_class = format!(
        "dxb-frame-content dxb-showcase-frame-content {}",
        config.canvas_class()
    );
    let registry = use_context::<CatalogRegistry>();
    let Some((showcase, preview)) = registry.find(&showcase_id, &preview_id) else {
        return rsx! { MissingShowcase { showcase_id, preview_id } };
    };
    let render = preview.render();

    rsx! {
        main {
            class: "dx-preview dxb-frame dxb-render-frame",
            "data-dx-preview-ready": "true",
            "data-showcase": showcase.id(),
            "data-preview": preview.id(),
            div { class: canvas_class, {render()} }
        }
    }
}

#[component]
fn MissingShowcase(showcase_id: String, preview_id: String) -> Element {
    rsx! {
        section { class: "dxb-message",
            p { class: "dxb-message-kicker dxb-message-kicker-danger", "Unknown showcase" }
            h1 { "{showcase_id}/{preview_id}" }
            p { "Choose a component from the catalog navigation." }
            Link { class: "dxb-button dxb-message-action", to: CatalogRoute::Home {},
                "Back to catalog"
            }
        }
    }
}
