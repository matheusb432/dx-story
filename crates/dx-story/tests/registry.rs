use dioxus::prelude::*;
use dx_story::{Story, find, stories, story, story_sets};

const EXTRA_DOCUMENTATION: &str = "Additional usage guidance.";

/// Default component state.
#[story]
fn default_state() -> Element {
    rsx! { button { "Default" } }
}

/// Compact component state.
///
///     let compact = true;
#[story]
fn compact() -> Element {
    rsx! { button { "Compact" } }
}

#[story(id = "catalog_thumbnail", name = "Catalog thumbnail")]
fn thumbnail() -> Element {
    rsx! { button { "Story" } }
}

#[story(name = "Direct hook")]
fn direct_hook() -> Element {
    let count = use_signal(|| 0_u32);
    rsx! { output { "{count}" } }
}

/// Registry test component.
#[stories(
    id = "registry-test",
    name = "Registry test",
    extra_docs = EXTRA_DOCUMENTATION,
    thumbnail = thumbnail
)]
const REGISTRY_TEST_STORY_SET: () = &[default_state, compact, direct_hook];

#[test]
fn registered_story_sets_are_sorted_and_searchable() {
    let registered_story_sets = story_sets().unwrap();
    let story_set_names = registered_story_sets
        .windows(2)
        .all(|pair| pair[0].name() <= pair[1].name());
    assert!(story_set_names);

    let registered_story_set = find("registry-test", "default-state").unwrap();
    assert!(registered_story_set.is_some());
    if let Some((story_set, story)) = registered_story_set {
        assert_eq!(story_set.name(), "Registry test");
        assert_eq!(
            story_set.description(),
            Some("Registry test component.\nAdditional usage guidance.")
        );
        assert_eq!(story.name(), "Default State");
        assert_eq!(story.description(), Some("Default component state."));
        assert!(story.source().contains("fn default_state"));
        assert!(story.source().ends_with("\n}\n"));
        assert_eq!(
            story_set.thumbnail().map(Story::id),
            Some("catalog_thumbnail")
        );
    }

    let (_, compact_story) = find("registry-test", "compact").unwrap().unwrap();
    assert_eq!(
        compact_story.description(),
        Some("Compact component state.\n\n    let compact = true;")
    );
}

#[test]
fn unknown_story_set_paths_do_not_resolve() {
    assert!(find("registry-test", "missing").unwrap().is_none());
    assert!(find("missing", "default-state").unwrap().is_none());
}
