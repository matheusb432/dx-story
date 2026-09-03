//! Dioxus component stories.
//!
//! ```
//! use dioxus::prelude::*;
//! use dx_story::{stories, story};
//!
//! #[story]
//! fn default() -> Element {
//!     rsx! { button { "Save" } }
//! }
//!
//! #[stories(id = "button", name = "Button")]
//! const BUTTON_STORIES: () = &[default];
//! ```

extern crate self as dx_story;

use std::{collections::HashSet, error::Error, fmt, sync::OnceLock};

use dioxus::prelude::Element;
pub use dx_story_macros::{stories, story};

#[cfg(feature = "catalog")]
pub mod catalog;

#[doc(hidden)]
pub mod __private {
    pub use const_format::concatcp;
    pub use dioxus;
    pub use inventory::submit;

    #[doc(hidden)]
    #[must_use]
    pub const fn story(
        id: &'static str,
        name: &'static str,
        description: Option<&'static str>,
        render: fn() -> dioxus::prelude::Element,
        source: &'static str,
    ) -> super::Story {
        super::Story {
            id,
            name,
            description,
            render,
            source,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn story_set(
        id: &'static str,
        name: &'static str,
        description: Option<&'static str>,
        thumbnail: Option<&'static super::Story>,
        stories: &'static [&'static super::Story],
    ) -> super::StorySet {
        super::StorySet {
            id,
            name,
            description,
            thumbnail,
            stories,
        }
    }

    #[cfg(feature = "benchmark-support")]
    #[doc(hidden)]
    pub fn build_registry_for_benchmark(
        registrations: impl IntoIterator<Item = &'static super::StorySet>,
    ) -> Result<Box<[&'static super::StorySet]>, super::RegistryError> {
        super::build_registry(registrations)
    }
}

/// A renderable component state.
#[derive(Clone, Copy, Debug)]
pub struct Story {
    id: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    render: fn() -> Element,
    source: &'static str,
}

impl Story {
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub const fn description(&self) -> Option<&'static str> {
        self.description
    }

    #[must_use]
    pub const fn render(&self) -> fn() -> Element {
        self.render
    }

    #[must_use]
    pub const fn source(&self) -> &'static str {
        self.source
    }
}

impl PartialEq for Story {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.description == other.description
            && std::ptr::fn_addr_eq(self.render, other.render)
            && self.source == other.source
    }
}

impl Eq for Story {}

/// A component and its stories.
#[derive(Clone, Copy, Debug)]
pub struct StorySet {
    id: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    thumbnail: Option<&'static Story>,
    stories: &'static [&'static Story],
}

impl StorySet {
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub const fn description(&self) -> Option<&'static str> {
        self.description
    }

    #[must_use]
    pub const fn stories(&self) -> &'static [&'static Story] {
        self.stories
    }

    #[must_use]
    pub fn first_story(&self) -> Option<&'static Story> {
        self.stories.first().copied()
    }

    /// Falls back to the first story.
    #[must_use]
    pub fn thumbnail(&self) -> Option<&'static Story> {
        self.thumbnail.or_else(|| self.first_story())
    }
}

impl PartialEq for StorySet {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.description == other.description
            && self.thumbnail == other.thumbnail
            && self.stories == other.stories
    }
}

impl Eq for StorySet {}

/// A registry validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryError {
    DuplicateStorySetId {
        id: &'static str,
    },
    DuplicateStoryId {
        story_set_id: &'static str,
        story_id: &'static str,
    },
    StorySetWithoutStories {
        story_set_id: &'static str,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateStorySetId { id } => {
                write!(formatter, "duplicate story set ID `{id}`")
            }
            Self::DuplicateStoryId {
                story_set_id,
                story_id,
            } => write!(
                formatter,
                "duplicate story ID `{story_id}` in story set `{story_set_id}`"
            ),
            Self::StorySetWithoutStories { story_set_id } => {
                write!(
                    formatter,
                    "story set `{story_set_id}` has no routed stories"
                )
            }
        }
    }
}

impl Error for RegistryError {}

inventory::collect!(&'static StorySet);

static STORY_SET_REGISTRY: OnceLock<Result<Box<[&'static StorySet]>, RegistryError>> =
    OnceLock::new();
const STORY_SET_COUNT_LINEAR_VALIDATION_MAX: usize = 16;

#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

/// Validates registered story sets.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn init_registry() -> Result<(), RegistryError> {
    registry().map(|_| ())
}

/// Registered story sets in display-name order.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn story_sets() -> Result<&'static [&'static StorySet], RegistryError> {
    registry()
}

/// Finds a story by story set and story ID.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn find(
    story_set_id: &str,
    story_id: &str,
) -> Result<Option<(&'static StorySet, &'static Story)>, RegistryError> {
    let story_set = story_sets()?
        .iter()
        .copied()
        .find(|story_set| story_set.id() == story_set_id);
    let Some(story_set) = story_set else {
        return Ok(None);
    };
    let story = story_set
        .stories()
        .iter()
        .copied()
        .find(|story| story.id() == story_id);
    Ok(story.map(|story| (story_set, story)))
}

/// Validates the registry, then launches `root`.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
#[cfg(feature = "csr")]
pub fn launch(root: fn() -> Element) -> Result<(), RegistryError> {
    init_registry()?;
    dioxus::launch(root);
    Ok(())
}

fn registry() -> Result<&'static [&'static StorySet], RegistryError> {
    STORY_SET_REGISTRY
        .get_or_init(|| {
            initialize_distributed_registrations();
            build_registry(inventory::iter::<&'static StorySet>.into_iter().copied())
        })
        .as_ref()
        .map(Box::as_ref)
        .map_err(|error| *error)
}

fn initialize_distributed_registrations() {
    #[cfg(target_family = "wasm")]
    // SAFETY: Wasm synthesizes this function for static constructors. OnceLock
    // runs this initializer exactly once before inventory is read.
    unsafe {
        __wasm_call_ctors();
    }
}

fn build_registry(
    registrations: impl IntoIterator<Item = &'static StorySet>,
) -> Result<Box<[&'static StorySet]>, RegistryError> {
    let mut story_sets = registrations.into_iter().collect::<Vec<_>>();
    story_sets.sort_unstable_by_key(|story_set| (story_set.name(), story_set.id()));
    validate_unique_ids(&story_sets)?;
    Ok(story_sets.into_boxed_slice())
}

fn validate_unique_ids(story_sets: &[&'static StorySet]) -> Result<(), RegistryError> {
    if story_sets.len() > STORY_SET_COUNT_LINEAR_VALIDATION_MAX {
        return validate_unique_ids_with_hash_set(story_sets);
    }

    validate_unique_ids_with_linear_scan(story_sets)
}

fn validate_unique_ids_with_linear_scan(
    story_sets: &[&'static StorySet],
) -> Result<(), RegistryError> {
    for (index, story_set) in story_sets.iter().enumerate() {
        if story_sets[..index]
            .iter()
            .any(|registered| registered.id() == story_set.id())
        {
            return Err(RegistryError::DuplicateStorySetId { id: story_set.id() });
        }
        if story_set.stories().is_empty() {
            return Err(RegistryError::StorySetWithoutStories {
                story_set_id: story_set.id(),
            });
        }
        if let Some(story_id) = duplicate_story_id(story_set) {
            return Err(RegistryError::DuplicateStoryId {
                story_set_id: story_set.id(),
                story_id,
            });
        }
    }
    Ok(())
}

fn validate_unique_ids_with_hash_set(
    story_sets: &[&'static StorySet],
) -> Result<(), RegistryError> {
    let mut story_set_ids = HashSet::with_capacity(story_sets.len());
    for story_set in story_sets {
        if !story_set_ids.insert(story_set.id()) {
            return Err(RegistryError::DuplicateStorySetId { id: story_set.id() });
        }
        if story_set.stories().is_empty() {
            return Err(RegistryError::StorySetWithoutStories {
                story_set_id: story_set.id(),
            });
        }
        if let Some(story_id) = duplicate_story_id(story_set) {
            return Err(RegistryError::DuplicateStoryId {
                story_set_id: story_set.id(),
                story_id,
            });
        }
    }
    Ok(())
}

fn duplicate_story_id(story_set: &StorySet) -> Option<&'static str> {
    story_set
        .stories()
        .iter()
        .enumerate()
        .find_map(|(index, story)| {
            story_set.stories()[..index]
                .iter()
                .any(|registered| registered.id() == story.id())
                .then_some(story.id())
        })
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::VNode;

    use super::{
        RegistryError, STORY_SET_COUNT_LINEAR_VALIDATION_MAX, Story, StorySet, build_registry,
    };

    fn render_empty() -> dioxus::prelude::Element {
        VNode::empty()
    }

    static FIRST_STORY: Story = Story {
        id: "default",
        name: "Default",
        description: None,
        render: render_empty,
        source: "",
    };
    static DUPLICATE_STORY: Story = Story {
        id: "default",
        name: "Duplicate",
        description: None,
        render: render_empty,
        source: "",
    };
    static DUPLICATE_STORY_SET: StorySet = StorySet {
        id: "duplicate-story",
        name: "Duplicate story",
        description: None,
        thumbnail: None,
        stories: &[&FIRST_STORY, &DUPLICATE_STORY],
    };
    static FIRST_STORY_SET: StorySet = StorySet {
        id: "duplicate-story-set",
        name: "First",
        description: None,
        thumbnail: None,
        stories: &[&FIRST_STORY],
    };
    static SECOND_STORY_SET: StorySet = StorySet {
        id: "duplicate-story-set",
        name: "Second",
        description: None,
        thumbnail: None,
        stories: &[&FIRST_STORY],
    };
    static EMPTY_STORY_SET: StorySet = StorySet {
        id: "empty-story-set",
        name: "Empty story set",
        description: None,
        thumbnail: None,
        stories: &[],
    };

    #[test]
    fn registry_rejects_duplicate_story_set_ids() {
        assert_eq!(
            build_registry([&FIRST_STORY_SET, &SECOND_STORY_SET]),
            Err(RegistryError::DuplicateStorySetId {
                id: "duplicate-story-set"
            })
        );
    }

    #[test]
    fn registry_rejects_duplicate_story_ids() {
        assert_eq!(
            build_registry([&DUPLICATE_STORY_SET]),
            Err(RegistryError::DuplicateStoryId {
                story_set_id: "duplicate-story",
                story_id: "default",
            })
        );
    }

    #[test]
    fn large_registry_preserves_first_error_in_display_order() {
        let registrations = vec![&DUPLICATE_STORY_SET; STORY_SET_COUNT_LINEAR_VALIDATION_MAX + 1];
        assert_eq!(
            build_registry(registrations),
            Err(RegistryError::DuplicateStoryId {
                story_set_id: "duplicate-story",
                story_id: "default",
            })
        );
    }

    #[test]
    fn registry_rejects_story_sets_without_routed_stories() {
        assert_eq!(
            build_registry([&EMPTY_STORY_SET]),
            Err(RegistryError::StorySetWithoutStories {
                story_set_id: "empty-story-set",
            })
        );
    }

    #[test]
    fn thumbnail_falls_back_to_the_first_routed_story() {
        assert_eq!(FIRST_STORY_SET.first_story(), Some(&FIRST_STORY));
        assert_eq!(FIRST_STORY_SET.thumbnail(), Some(&FIRST_STORY));
    }
}
