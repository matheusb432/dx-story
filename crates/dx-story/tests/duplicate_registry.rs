use dioxus::prelude::VNode;
use dx_story::{RegistryError, Story, StorySet, story_sets};

fn render_empty() -> dioxus::prelude::Element {
    VNode::empty()
}

static STORY: Story = dx_story::__private::story("default", "Default", None, render_empty, "");
static FIRST: StorySet =
    dx_story::__private::story_set("distributed-duplicate", "First", None, None, &[&STORY]);
static SECOND: StorySet =
    dx_story::__private::story_set("distributed-duplicate", "Second", None, None, &[&STORY]);

dx_story::__private::submit! {
    &FIRST
}
dx_story::__private::submit! {
    &SECOND
}

#[test]
fn distributed_registry_reports_duplicate_story_set_ids() {
    assert_eq!(
        story_sets(),
        Err(RegistryError::DuplicateStorySetId {
            id: "distributed-duplicate"
        })
    );
}
