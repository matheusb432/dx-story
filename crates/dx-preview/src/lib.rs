//! Dioxus component showcases.
//!
//! ```
//! use dioxus::prelude::*;
//! use dx_preview::{preview, showcase};
//!
//! #[preview]
//! fn default() -> Element {
//!     rsx! { button { "Save" } }
//! }
//!
//! #[showcase(id = "button", name = "Button")]
//! const BUTTON_SHOWCASE: () = &[default];
//! ```

extern crate self as dx_preview;

use std::{collections::HashSet, error::Error, fmt, sync::OnceLock};

use dioxus::prelude::Element;
pub use dx_preview_macros::{preview, showcase};

#[cfg(feature = "catalog")]
pub mod catalog;

#[doc(hidden)]
pub mod __private {
    pub use const_format::concatcp;
    pub use dioxus;
    pub use inventory::submit;

    #[doc(hidden)]
    #[must_use]
    pub const fn preview(
        id: &'static str,
        name: &'static str,
        description: Option<&'static str>,
        render: fn() -> dioxus::prelude::Element,
        source: &'static str,
    ) -> super::Preview {
        super::Preview {
            id,
            name,
            description,
            render,
            source,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn showcase(
        id: &'static str,
        name: &'static str,
        description: Option<&'static str>,
        thumbnail: Option<&'static super::Preview>,
        previews: &'static [&'static super::Preview],
    ) -> super::Showcase {
        super::Showcase {
            id,
            name,
            description,
            thumbnail,
            previews,
        }
    }

    #[cfg(feature = "benchmark-support")]
    #[doc(hidden)]
    pub fn build_registry_for_benchmark(
        registrations: impl IntoIterator<Item = &'static super::Showcase>,
    ) -> Result<Box<[&'static super::Showcase]>, super::RegistryError> {
        super::build_registry(registrations)
    }
}

/// A renderable component state.
#[derive(Clone, Copy, Debug)]
pub struct Preview {
    id: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    render: fn() -> Element,
    source: &'static str,
}

impl Preview {
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

impl PartialEq for Preview {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.description == other.description
            && std::ptr::fn_addr_eq(self.render, other.render)
            && self.source == other.source
    }
}

impl Eq for Preview {}

/// A component and its previews.
#[derive(Clone, Copy, Debug)]
pub struct Showcase {
    id: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    thumbnail: Option<&'static Preview>,
    previews: &'static [&'static Preview],
}

impl Showcase {
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
    pub const fn previews(&self) -> &'static [&'static Preview] {
        self.previews
    }

    #[must_use]
    pub fn first_preview(&self) -> Option<&'static Preview> {
        self.previews.first().copied()
    }

    /// Falls back to the first preview.
    #[must_use]
    pub fn thumbnail(&self) -> Option<&'static Preview> {
        self.thumbnail.or_else(|| self.first_preview())
    }
}

impl PartialEq for Showcase {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.description == other.description
            && self.thumbnail == other.thumbnail
            && self.previews == other.previews
    }
}

impl Eq for Showcase {}

/// A registry validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryError {
    DuplicateShowcaseId {
        id: &'static str,
    },
    DuplicatePreviewId {
        showcase_id: &'static str,
        preview_id: &'static str,
    },
    ShowcaseWithoutPreviews {
        showcase_id: &'static str,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateShowcaseId { id } => {
                write!(formatter, "duplicate showcase ID `{id}`")
            }
            Self::DuplicatePreviewId {
                showcase_id,
                preview_id,
            } => write!(
                formatter,
                "duplicate preview ID `{preview_id}` in showcase `{showcase_id}`"
            ),
            Self::ShowcaseWithoutPreviews { showcase_id } => {
                write!(formatter, "showcase `{showcase_id}` has no routed previews")
            }
        }
    }
}

impl Error for RegistryError {}

inventory::collect!(&'static Showcase);

static SHOWCASE_REGISTRY: OnceLock<Result<Box<[&'static Showcase]>, RegistryError>> =
    OnceLock::new();
const SHOWCASE_COUNT_LINEAR_VALIDATION_MAX: usize = 16;

#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

/// Validates registered showcases.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn init_registry() -> Result<(), RegistryError> {
    registry().map(|_| ())
}

/// Registered showcases in display-name order.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn showcases() -> Result<&'static [&'static Showcase], RegistryError> {
    registry()
}

/// Finds a preview by showcase and preview ID.
///
/// # Errors
///
/// Returns the first [`RegistryError`].
pub fn find(
    showcase_id: &str,
    preview_id: &str,
) -> Result<Option<(&'static Showcase, &'static Preview)>, RegistryError> {
    let showcase = showcases()?
        .iter()
        .copied()
        .find(|showcase| showcase.id() == showcase_id);
    let Some(showcase) = showcase else {
        return Ok(None);
    };
    let preview = showcase
        .previews()
        .iter()
        .copied()
        .find(|preview| preview.id() == preview_id);
    Ok(preview.map(|preview| (showcase, preview)))
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

fn registry() -> Result<&'static [&'static Showcase], RegistryError> {
    SHOWCASE_REGISTRY
        .get_or_init(|| {
            initialize_distributed_registrations();
            build_registry(inventory::iter::<&'static Showcase>.into_iter().copied())
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
    registrations: impl IntoIterator<Item = &'static Showcase>,
) -> Result<Box<[&'static Showcase]>, RegistryError> {
    let mut showcases = registrations.into_iter().collect::<Vec<_>>();
    showcases.sort_unstable_by_key(|showcase| (showcase.name(), showcase.id()));
    validate_unique_ids(&showcases)?;
    Ok(showcases.into_boxed_slice())
}

fn validate_unique_ids(showcases: &[&'static Showcase]) -> Result<(), RegistryError> {
    if showcases.len() > SHOWCASE_COUNT_LINEAR_VALIDATION_MAX {
        return validate_unique_ids_with_hash_set(showcases);
    }

    validate_unique_ids_with_linear_scan(showcases)
}

fn validate_unique_ids_with_linear_scan(
    showcases: &[&'static Showcase],
) -> Result<(), RegistryError> {
    for (index, showcase) in showcases.iter().enumerate() {
        if showcases[..index]
            .iter()
            .any(|registered| registered.id() == showcase.id())
        {
            return Err(RegistryError::DuplicateShowcaseId { id: showcase.id() });
        }
        if showcase.previews().is_empty() {
            return Err(RegistryError::ShowcaseWithoutPreviews {
                showcase_id: showcase.id(),
            });
        }
        if let Some(preview_id) = duplicate_preview_id(showcase) {
            return Err(RegistryError::DuplicatePreviewId {
                showcase_id: showcase.id(),
                preview_id,
            });
        }
    }
    Ok(())
}

fn validate_unique_ids_with_hash_set(showcases: &[&'static Showcase]) -> Result<(), RegistryError> {
    let mut showcase_ids = HashSet::with_capacity(showcases.len());
    for showcase in showcases {
        if !showcase_ids.insert(showcase.id()) {
            return Err(RegistryError::DuplicateShowcaseId { id: showcase.id() });
        }
        if showcase.previews().is_empty() {
            return Err(RegistryError::ShowcaseWithoutPreviews {
                showcase_id: showcase.id(),
            });
        }
        if let Some(preview_id) = duplicate_preview_id(showcase) {
            return Err(RegistryError::DuplicatePreviewId {
                showcase_id: showcase.id(),
                preview_id,
            });
        }
    }
    Ok(())
}

fn duplicate_preview_id(showcase: &Showcase) -> Option<&'static str> {
    showcase
        .previews()
        .iter()
        .enumerate()
        .find_map(|(index, preview)| {
            showcase.previews()[..index]
                .iter()
                .any(|registered| registered.id() == preview.id())
                .then_some(preview.id())
        })
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::VNode;

    use super::{
        Preview, RegistryError, SHOWCASE_COUNT_LINEAR_VALIDATION_MAX, Showcase, build_registry,
    };

    fn render_empty() -> dioxus::prelude::Element {
        VNode::empty()
    }

    static FIRST_PREVIEW: Preview = Preview {
        id: "default",
        name: "Default",
        description: None,
        render: render_empty,
        source: "",
    };
    static DUPLICATE_PREVIEW: Preview = Preview {
        id: "default",
        name: "Duplicate",
        description: None,
        render: render_empty,
        source: "",
    };
    static DUPLICATE_PREVIEW_SHOWCASE: Showcase = Showcase {
        id: "duplicate-preview",
        name: "Duplicate preview",
        description: None,
        thumbnail: None,
        previews: &[&FIRST_PREVIEW, &DUPLICATE_PREVIEW],
    };
    static FIRST_SHOWCASE: Showcase = Showcase {
        id: "duplicate-showcase",
        name: "First",
        description: None,
        thumbnail: None,
        previews: &[&FIRST_PREVIEW],
    };
    static SECOND_SHOWCASE: Showcase = Showcase {
        id: "duplicate-showcase",
        name: "Second",
        description: None,
        thumbnail: None,
        previews: &[&FIRST_PREVIEW],
    };
    static EMPTY_SHOWCASE: Showcase = Showcase {
        id: "empty-showcase",
        name: "Empty showcase",
        description: None,
        thumbnail: None,
        previews: &[],
    };

    #[test]
    fn registry_rejects_duplicate_showcase_ids() {
        assert_eq!(
            build_registry([&FIRST_SHOWCASE, &SECOND_SHOWCASE]),
            Err(RegistryError::DuplicateShowcaseId {
                id: "duplicate-showcase"
            })
        );
    }

    #[test]
    fn registry_rejects_duplicate_preview_ids() {
        assert_eq!(
            build_registry([&DUPLICATE_PREVIEW_SHOWCASE]),
            Err(RegistryError::DuplicatePreviewId {
                showcase_id: "duplicate-preview",
                preview_id: "default",
            })
        );
    }

    #[test]
    fn large_registry_preserves_first_error_in_display_order() {
        let registrations =
            vec![&DUPLICATE_PREVIEW_SHOWCASE; SHOWCASE_COUNT_LINEAR_VALIDATION_MAX + 1];
        assert_eq!(
            build_registry(registrations),
            Err(RegistryError::DuplicatePreviewId {
                showcase_id: "duplicate-preview",
                preview_id: "default",
            })
        );
    }

    #[test]
    fn registry_rejects_showcases_without_routed_previews() {
        assert_eq!(
            build_registry([&EMPTY_SHOWCASE]),
            Err(RegistryError::ShowcaseWithoutPreviews {
                showcase_id: "empty-showcase",
            })
        );
    }

    #[test]
    fn thumbnail_falls_back_to_the_first_routed_preview() {
        assert_eq!(FIRST_SHOWCASE.first_preview(), Some(&FIRST_PREVIEW));
        assert_eq!(FIRST_SHOWCASE.thumbnail(), Some(&FIRST_PREVIEW));
    }
}
