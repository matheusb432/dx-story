use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use cargo_metadata::{CargoOpt, Metadata, MetadataCommand, Package, Target};

use crate::config::{Configuration, ServeConfiguration, TailwindConfiguration};

pub(crate) struct CatalogProject {
    path: PathBuf,
    package: String,
    example: String,
    features: Vec<String>,
    default_features: bool,
    locked: bool,
    source_directories: Vec<PathBuf>,
    tailwind: Option<TailwindConfiguration>,
    serve: ServeConfiguration,
    dioxus_version: String,
    target_directory: PathBuf,
}

impl CatalogProject {
    pub(crate) fn resolve(configuration: Configuration) -> Result<Self> {
        let manifest = configuration
            .root
            .join(
                configuration
                    .catalog
                    .path
                    .as_deref()
                    .unwrap_or(Path::new(".")),
            )
            .join("Cargo.toml");
        let metadata = MetadataCommand::new()
            .manifest_path(&manifest)
            .no_deps()
            .exec()
            .with_context(|| format!("read Cargo targets from {}", manifest.display()))?;
        let package = select_package(&metadata, &configuration.catalog.package)?;
        let example = select_example(package, configuration.catalog.example.as_deref())?;
        let features = configuration
            .catalog
            .features
            .unwrap_or_else(|| example.required_features.clone());
        validate_features(
            package,
            example,
            &features,
            configuration.catalog.default_features,
        )?;
        let mut command = MetadataCommand::new();
        command.manifest_path(&package.manifest_path);
        if configuration.catalog.locked {
            command.other_options(vec!["--locked".to_owned()]);
        }
        if !configuration.catalog.default_features {
            command.features(CargoOpt::NoDefaultFeatures);
        }
        command.features(CargoOpt::SomeFeatures(features.clone()));
        let resolved = command.exec().context("resolve catalog dependencies")?;
        let reachable = reachable_packages(&resolved, &package.id);
        let dioxus = resolved
            .packages
            .iter()
            .find(|candidate| candidate.name == "dioxus" && reachable.contains(&candidate.id))
            .context("catalog package must depend on dioxus")?;
        let path = package
            .manifest_path
            .parent()
            .context("package manifest has no parent")?
            .as_std_path()
            .to_owned();
        let mut directories = if let Some(explicit) = configuration.catalog.source_directories {
            ensure!(
                !explicit.is_empty(),
                "source-directories must not be empty; omit it for discovery"
            );
            explicit
                .into_iter()
                .map(|directory| path.join(directory))
                .collect()
        } else {
            source_directories(&resolved, &reachable)
        };
        directories.extend(
            configuration
                .catalog
                .extra_source_directories
                .iter()
                .map(|directory| path.join(directory)),
        );
        let source_directories = normalize_directories(directories)?;
        validate_tailwind(&path, configuration.tailwind.as_ref())?;
        ensure!(
            configuration.serve.port != 0,
            "serve.port must be between 1 and 65535"
        );
        Ok(Self {
            path,
            package: package.name.to_string(),
            example: example.name.clone(),
            features,
            default_features: configuration.catalog.default_features,
            locked: configuration.catalog.locked,
            source_directories,
            tailwind: configuration.tailwind,
            serve: configuration.serve,
            dioxus_version: dioxus.version.to_string(),
            target_directory: resolved.target_directory.into_std_path_buf(),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn package(&self) -> &str {
        &self.package
    }
    pub(crate) fn example(&self) -> &str {
        &self.example
    }
    pub(crate) fn source_directories(&self) -> &[PathBuf] {
        &self.source_directories
    }
    pub(crate) fn tailwind(&self) -> Option<&TailwindConfiguration> {
        self.tailwind.as_ref()
    }
    pub(crate) fn serve(&self) -> &ServeConfiguration {
        &self.serve
    }
    pub(crate) fn dioxus_version(&self) -> &str {
        &self.dioxus_version
    }
    pub(crate) fn target_directory(&self) -> &Path {
        &self.target_directory
    }

    pub(crate) fn target_arguments(&self) -> Vec<String> {
        let mut arguments = vec![
            "--web".to_owned(),
            "--package".to_owned(),
            self.package.clone(),
            "--example".to_owned(),
            self.example.clone(),
        ];
        if !self.default_features {
            arguments.push("--no-default-features".to_owned());
        }
        if !self.features.is_empty() {
            arguments.extend(["--features".to_owned(), self.features.join(",")]);
        }
        if self.locked {
            arguments.push("--locked".to_owned());
        }
        arguments
    }
}

pub(crate) fn select_package<'a>(metadata: &'a Metadata, name: &str) -> Result<&'a Package> {
    metadata
        .workspace_packages()
        .into_iter()
        .find(|package| package.name == name)
        .with_context(|| format!("workspace has no package {name}"))
}

fn validate_tailwind(path: &Path, tailwind: Option<&TailwindConfiguration>) -> Result<()> {
    if let Some(tailwind) = tailwind {
        ensure!(
            path.join(&tailwind.input).is_file(),
            "Tailwind input does not exist: {}",
            path.join(&tailwind.input).display()
        );
        ensure!(
            !tailwind.output.as_os_str().is_empty(),
            "Tailwind output must not be empty"
        );
        ensure!(
            path.join(&tailwind.input) != path.join(&tailwind.output),
            "Tailwind input and output must differ"
        );
    }
    Ok(())
}

fn select_example<'a>(package: &'a Package, name: Option<&str>) -> Result<&'a Target> {
    let examples: Vec<_> = package
        .targets
        .iter()
        .filter(|target| target.is_example())
        .collect();
    if let Some(name) = name {
        return examples
            .into_iter()
            .find(|target| target.name == name)
            .with_context(|| format!("package {} has no example {name}", package.name));
    }
    let conventional = examples
        .iter()
        .find(|target| target.name == "component-catalog");
    if let Some(target) = conventional {
        return Ok(target);
    }
    ensure!(
        examples.len() == 1,
        "package {} must have one example or set catalog.example explicitly",
        package.name
    );
    Ok(examples[0])
}

fn validate_features(
    package: &Package,
    example: &Target,
    features: &[String],
    defaults: bool,
) -> Result<()> {
    ensure!(
        features.len() <= 64,
        "at most 64 catalog features are supported"
    );
    for feature in features {
        ensure!(
            package.features.contains_key(feature),
            "package {} has no feature {feature}",
            package.name
        );
    }
    for required in &example.required_features {
        ensure!(
            feature_enabled(package, features, defaults, required),
            "example {} requires feature {required}",
            example.name
        );
    }
    Ok(())
}

fn feature_enabled(package: &Package, features: &[String], defaults: bool, required: &str) -> bool {
    let mut pending = features.to_vec();
    if defaults {
        pending.push("default".to_owned());
    }
    let mut visited = BTreeSet::new();
    while let Some(feature) = pending.pop() {
        if feature == required {
            return true;
        }
        if visited.insert(feature.clone()) {
            pending.extend(
                package
                    .features
                    .get(&feature)
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
    }
    false
}

fn reachable_packages(
    metadata: &Metadata,
    root: &cargo_metadata::PackageId,
) -> BTreeSet<cargo_metadata::PackageId> {
    let mut visited = BTreeSet::new();
    let mut pending = vec![root.clone()];
    while let Some(id) = pending.pop() {
        if visited.insert(id.clone()) {
            pending.extend(
                metadata
                    .resolve
                    .iter()
                    .flat_map(|resolve| &resolve.nodes)
                    .filter(|node| node.id == id)
                    .flat_map(|node| node.dependencies.clone()),
            );
        }
    }
    visited
}

fn source_directories(
    metadata: &Metadata,
    reachable: &BTreeSet<cargo_metadata::PackageId>,
) -> Vec<PathBuf> {
    metadata
        .packages
        .iter()
        .filter(|package| package.source.is_none() && reachable.contains(&package.id))
        .flat_map(|package| {
            let root = package
                .manifest_path
                .parent()
                .map(cargo_metadata::camino::Utf8Path::as_std_path);
            let conventional = ["src", "dev", "examples"]
                .into_iter()
                .filter_map(move |name| root.map(|root| root.join(name)))
                .filter(|path| path.is_dir());
            let targets = package
                .targets
                .iter()
                .filter(|target| {
                    !target.is_custom_build() && !target.is_test() && !target.is_bench()
                })
                .filter_map(|target| {
                    target
                        .src_path
                        .parent()
                        .map(|path| path.as_std_path().to_owned())
                });
            conventional.chain(targets)
        })
        .collect()
}

fn normalize_directories(directories: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let directories: BTreeSet<_> = directories
        .into_iter()
        .map(|path| {
            ensure!(
                path.is_dir(),
                "watched directory does not exist: {}",
                path.display()
            );
            path.canonicalize()
                .with_context(|| format!("resolve watched directory {}", path.display()))
        })
        .collect::<Result<_>>()?;
    let minimal: Vec<_> = directories
        .iter()
        .filter(|path| {
            !directories
                .iter()
                .any(|parent| parent != *path && path.starts_with(parent))
        })
        .cloned()
        .collect();
    ensure!(
        !minimal.is_empty() && minimal.len() <= 64,
        "catalog must have between 1 and 64 watched source directories"
    );
    Ok(minimal)
}
