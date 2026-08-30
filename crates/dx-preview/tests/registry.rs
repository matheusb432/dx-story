use dioxus::prelude::*;
use dx_preview::{Preview, find, preview, showcase, showcases};

const EXTRA_DOCUMENTATION: &str = "Additional usage guidance.";

/// Default component state.
#[preview]
fn default_state() -> Element {
    rsx! { button { "Default" } }
}

/// Compact component state.
///
///     let compact = true;
#[preview]
fn compact() -> Element {
    rsx! { button { "Compact" } }
}

#[preview(id = "catalog_thumbnail", name = "Catalog thumbnail")]
fn thumbnail() -> Element {
    rsx! { button { "Preview" } }
}

#[preview(name = "Direct hook")]
fn direct_hook() -> Element {
    let count = use_signal(|| 0_u32);
    rsx! { output { "{count}" } }
}

/// Registry test component.
#[showcase(
    id = "registry-test",
    name = "Registry test",
    extra_docs = EXTRA_DOCUMENTATION,
    thumbnail = thumbnail
)]
const REGISTRY_TEST_SHOWCASE: () = &[default_state, compact, direct_hook];

#[test]
fn registered_showcases_are_sorted_and_searchable() {
    let registered_showcases = showcases().unwrap();
    let showcase_names = registered_showcases
        .windows(2)
        .all(|pair| pair[0].name() <= pair[1].name());
    assert!(showcase_names);

    let registered_showcase = find("registry-test", "default-state").unwrap();
    assert!(registered_showcase.is_some());
    if let Some((showcase, preview)) = registered_showcase {
        assert_eq!(showcase.name(), "Registry test");
        assert_eq!(
            showcase.description(),
            Some("Registry test component.\nAdditional usage guidance.")
        );
        assert_eq!(preview.name(), "Default State");
        assert_eq!(preview.description(), Some("Default component state."));
        assert!(preview.source().contains("fn default_state"));
        assert!(preview.source().ends_with("\n}\n"));
        assert_eq!(
            showcase.thumbnail().map(Preview::id),
            Some("catalog_thumbnail")
        );
    }

    let (_, compact_preview) = find("registry-test", "compact").unwrap().unwrap();
    assert_eq!(
        compact_preview.description(),
        Some("Compact component state.\n\n    let compact = true;")
    );
}

#[test]
fn unknown_showcase_paths_do_not_resolve() {
    assert!(find("registry-test", "missing").unwrap().is_none());
    assert!(find("missing", "default-state").unwrap().is_none());
}
