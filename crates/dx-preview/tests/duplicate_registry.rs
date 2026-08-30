use dioxus::prelude::VNode;
use dx_preview::{Preview, RegistryError, Showcase, showcases};

fn render_empty() -> dioxus::prelude::Element {
    VNode::empty()
}

static PREVIEW: Preview =
    dx_preview::__private::preview("default", "Default", None, render_empty, "");
static FIRST: Showcase =
    dx_preview::__private::showcase("distributed-duplicate", "First", None, None, &[&PREVIEW]);
static SECOND: Showcase =
    dx_preview::__private::showcase("distributed-duplicate", "Second", None, None, &[&PREVIEW]);

dx_preview::__private::submit! {
    &FIRST
}
dx_preview::__private::submit! {
    &SECOND
}

#[test]
fn distributed_registry_reports_duplicate_showcase_ids() {
    assert_eq!(
        showcases(),
        Err(RegistryError::DuplicateShowcaseId {
            id: "distributed-duplicate"
        })
    );
}
