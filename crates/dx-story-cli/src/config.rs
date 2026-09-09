use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::Deserialize;

pub(crate) const CONFIGURATION_FILE_NAME: &str = "dx-story.toml";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Configuration {
    pub(crate) catalog: CatalogConfiguration,
    #[serde(default)]
    pub(crate) serve: ServeConfiguration,
    pub(crate) tailwind: Option<TailwindConfiguration>,
    #[serde(skip)]
    pub(crate) root: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct CatalogConfiguration {
    #[serde(default)]
    pub(crate) path: Option<PathBuf>,
    pub(crate) package: String,
    #[serde(default)]
    pub(crate) example: Option<String>,
    #[serde(default)]
    pub(crate) features: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) default_features: bool,
    #[serde(default = "default_locked")]
    pub(crate) locked: bool,
    #[serde(default)]
    pub(crate) source_directories: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub(crate) extra_source_directories: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum OpenBrowser {
    #[default]
    No,
    Yes,
}

impl OpenBrowser {
    pub(crate) const fn as_bool(self) -> bool {
        matches!(self, Self::Yes)
    }
}

impl<'de> Deserialize<'de> for OpenBrowser {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(if bool::deserialize(deserializer)? {
            Self::Yes
        } else {
            Self::No
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ServeConfiguration {
    #[serde(default = "default_port")]
    pub(crate) port: u16,
    #[serde(default)]
    pub(crate) open: OpenBrowser,
}

impl Default for ServeConfiguration {
    fn default() -> Self {
        Self {
            port: default_port(),
            open: OpenBrowser::No,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TailwindConfiguration {
    pub(crate) input: PathBuf,
    pub(crate) output: PathBuf,
}

impl Configuration {
    pub(crate) fn load(explicit: Option<&Path>) -> Result<Self> {
        let current = std::env::current_dir().context("resolve the current directory")?;
        let path = if let Some(path) = explicit {
            current.join(path)
        } else {
            current.ancestors()
                .map(|root| root.join(CONFIGURATION_FILE_NAME))
                .find(|path| path.is_file())
                .with_context(|| format!("load {CONFIGURATION_FILE_NAME}: no configuration found in {} or its parents; run dx-story init", current.display()))?
        };
        Self::load_file(&path)
    }

    fn load_file(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .with_context(|| format!("load {}", path.display()))?;
        let source =
            fs::read_to_string(&path).with_context(|| format!("load {}", path.display()))?;
        let mut configuration =
            toml::from_str::<Self>(&source).with_context(|| format!("load {}", path.display()))?;
        path.parent()
            .context("configuration has no parent")?
            .clone_into(&mut configuration.root);
        Ok(configuration)
    }
}

const fn default_locked() -> bool {
    true
}

const fn default_port() -> u16 {
    8080
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_uses_serve_defaults() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join(CONFIGURATION_FILE_NAME),
            r#"
[catalog]
package = "example-ui"
source-directories = ["src", "dev"]

[tailwind]
input = "dev/tailwind.css"
output = "assets/component-catalog.css"
"#,
        )
        .unwrap();

        let configuration =
            Configuration::load_file(&project.path().join(CONFIGURATION_FILE_NAME)).unwrap();

        assert!(configuration.catalog.path.is_none());
        assert!(configuration.catalog.example.is_none());
        assert!(configuration.catalog.features.is_none());
        assert!(!configuration.catalog.default_features);
        assert!(configuration.catalog.locked);
        assert_eq!(configuration.serve.port, 8080);
        assert_eq!(configuration.serve.open, OpenBrowser::No);
    }

    #[test]
    fn configuration_reads_the_boolean_browser_setting() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join(CONFIGURATION_FILE_NAME),
            r#"
[catalog]
package = "example-ui"
source-directories = ["src", "dev"]

[serve]
open = true

[tailwind]
input = "dev/tailwind.css"
output = "assets/component-catalog.css"
"#,
        )
        .unwrap();

        let configuration =
            Configuration::load_file(&project.path().join(CONFIGURATION_FILE_NAME)).unwrap();

        assert_eq!(configuration.serve.open, OpenBrowser::Yes);
    }
}
