use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use cargo_metadata::{CargoOpt, Metadata, MetadataCommand, Package, Target, semver::Version};

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
    dioxus_version: Version,
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
        validate_features(package, &features)?;
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
        validate_required_features(&resolved, package, example)?;
        let reachable = reachable_packages(&resolved, &package.id);
        let dioxus = resolved
            .packages
            .iter()
            .find(|candidate| candidate.name == "dioxus" && reachable.contains(&candidate.id))
            .context("catalog package must depend on dioxus")?;
        let path = package_root(package)?.to_owned();
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
            source_directories(&resolved, &reachable)?
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
            dioxus_version: dioxus.version.clone(),
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
    pub(crate) fn dioxus_version(&self) -> &Version {
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

pub(crate) fn package_root(package: &Package) -> Result<&Path> {
    Ok(package
        .manifest_path
        .parent()
        .context("package manifest has no parent")?
        .as_std_path())
}

fn validate_tailwind(path: &Path, tailwind: Option<&TailwindConfiguration>) -> Result<()> {
    let Some(tailwind) = tailwind else {
        return Ok(());
    };
    let input = path.join(&tailwind.input);
    ensure!(
        input.is_file(),
        "Tailwind input does not exist: {}",
        input.display()
    );
    ensure!(
        !tailwind.output.as_os_str().is_empty(),
        "Tailwind output must not be empty"
    );
    ensure!(
        input != path.join(&tailwind.output),
        "Tailwind input and output must differ"
    );
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

fn validate_features(package: &Package, features: &[String]) -> Result<()> {
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
    Ok(())
}

/// Checks the example's `required-features` against the feature set cargo resolved,
/// which already accounts for `dep:` and `package/feature` syntax.
fn validate_required_features(
    resolved: &Metadata,
    package: &Package,
    example: &Target,
) -> Result<()> {
    let enabled = resolved
        .resolve
        .as_ref()
        .and_then(|resolve| resolve.nodes.iter().find(|node| node.id == package.id))
        .context("cargo did not resolve the catalog package")?;
    for required in &example.required_features {
        ensure!(
            enabled.features.iter().any(|feature| feature == required),
            "example {} requires feature {required}",
            example.name
        );
    }
    Ok(())
}

fn reachable_packages(
    metadata: &Metadata,
    root: &cargo_metadata::PackageId,
) -> BTreeSet<cargo_metadata::PackageId> {
    let nodes: HashMap<_, _> = metadata
        .resolve
        .iter()
        .flat_map(|resolve| &resolve.nodes)
        .map(|node| (&node.id, node))
        .collect();
    let mut visited = BTreeSet::new();
    let mut pending = vec![root.clone()];
    while let Some(id) = pending.pop() {
        if visited.insert(id.clone())
            && let Some(node) = nodes.get(&id)
        {
            pending.extend(node.dependencies.iter().cloned());
        }
    }
    visited
}

fn source_directories(
    metadata: &Metadata,
    reachable: &BTreeSet<cargo_metadata::PackageId>,
) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for package in metadata
        .packages
        .iter()
        .filter(|package| package.source.is_none() && reachable.contains(&package.id))
    {
        let root = package_root(package)?;
        directories.extend(
            ["src", "dev", "examples"]
                .into_iter()
                .map(|name| root.join(name))
                .filter(|path| path.is_dir()),
        );
        directories.extend(
            package
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
                }),
        );
    }
    Ok(directories)
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
    let mut minimal: Vec<PathBuf> = Vec::new();
    for path in directories {
        if minimal
            .last()
            .is_none_or(|parent| !path.starts_with(parent))
        {
            minimal.push(path);
        }
    }
    ensure!(
        !minimal.is_empty() && minimal.len() <= 64,
        "catalog must have between 1 and 64 watched source directories"
    );
    Ok(minimal)
}
