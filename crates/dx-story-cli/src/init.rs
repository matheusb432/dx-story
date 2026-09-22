use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use cargo_metadata::{CargoOpt, MetadataCommand};
use clap::Args;
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};

use crate::{
    config::CONFIGURATION_FILE_NAME,
    project::{package_root, select_package},
};

#[derive(Args)]
#[command(
    after_help = "Examples:\n  dx-story init --package my-ui\n  dx-story init --package my-ui --embedded --dry-run\n  dx-story init --package my-ui --library-path ../dx-story/crates/dx-story"
)]
pub(crate) struct InitArguments {
    /// Workspace package that owns the components.
    #[arg(short, long)]
    package: String,
    /// Put stories inside the library to access crate-private components.
    #[arg(long)]
    embedded: bool,
    /// Use a local dx-story library crate instead of crates.io.
    #[arg(long)]
    library_path: Option<PathBuf>,
    /// Print all proposed file contents without writing files.
    #[arg(long)]
    dry_run: bool,
}

struct FileChange {
    path: PathBuf,
    previous: Option<String>,
    contents: String,
}

enum LibrarySource {
    Registry,
    Local {
        path: PathBuf,
        dioxus_requirement: String,
    },
}

impl LibrarySource {
    fn resolve(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::Registry);
        };
        let path = path.canonicalize().context("resolve --library-path")?;
        let metadata = MetadataCommand::new()
            .manifest_path(path.join("Cargo.toml"))
            .no_deps()
            .exec()
            .context("read local dx-story manifest")?;
        let library = select_package(&metadata, "dx-story")?;
        let dioxus = library
            .dependencies
            .iter()
            .find(|dependency| dependency.name == "dioxus")
            .context("dx-story has no dioxus dependency")?;
        Ok(Self::Local {
            path: package_root(library)?.to_owned(),
            dioxus_requirement: dioxus.req.to_string(),
        })
    }

    fn dioxus_requirement(&self) -> &str {
        match self {
            Self::Registry => "0.7.10",
            Self::Local {
                dioxus_requirement, ..
            } => dioxus_requirement,
        }
    }

    fn dependency(&self, root: &Path) -> Result<InlineTable> {
        let mut dependency = InlineTable::new();
        match self {
            Self::Registry => {
                dependency.insert("version", Value::from(env!("CARGO_PKG_VERSION")));
            }
            Self::Local { path, .. } => {
                dependency.insert(
                    "path",
                    Value::from(relative_path(root, path)?.to_string_lossy().as_ref()),
                );
            }
        }
        dependency.insert("optional", Value::from(true));
        Ok(dependency)
    }
}

pub(crate) fn run(arguments: &InitArguments, configuration: Option<&Path>) -> Result<()> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("read workspace for initialization")?;
    let package = select_package(&metadata, &arguments.package)?;
    let root = package_root(package)?;
    let configuration = std::path::absolute(configuration.map_or_else(
        || {
            metadata
                .workspace_root
                .join(CONFIGURATION_FILE_NAME)
                .into_std_path_buf()
        },
        Path::to_owned,
    ))
    .context("resolve the configuration path")?;
    let library = LibrarySource::resolve(arguments.library_path.as_deref())?;
    let mut settings = DocumentMut::new();
    settings["catalog"]["package"] = value(arguments.package.as_str());
    if configuration.parent() != Some(metadata.workspace_root.as_std_path()) {
        let directory = configuration
            .parent()
            .context("configuration has no parent")?;
        settings["catalog"]["path"] =
            value(relative_path(directory, root)?.to_string_lossy().as_ref());
    }
    let mut changes = vec![
        manifest_change(package, root, &library)?,
        FileChange {
            path: configuration,
            previous: None,
            contents: settings.to_string(),
        },
        FileChange {
            path: root.join("dev/stories.rs"),
            previous: None,
            contents: include_str!("init/stories.rs.txt").to_owned(),
        },
    ];
    let main = if arguments.embedded {
        embedded_main(package, root, &mut changes)?
    } else {
        include_str!("init/main.rs.txt").to_owned()
    };
    changes.push(FileChange {
        path: root.join("dev/main.rs"),
        previous: None,
        contents: main,
    });
    apply(&changes, arguments.dry_run)?;
    if !arguments.dry_run {
        resolve_initialized_dependencies(package)?;
        eprintln!(
            "dx-story: initialized {}; run dx-story doctor or dx-story serve --open",
            arguments.package
        );
    }
    Ok(())
}

fn resolve_initialized_dependencies(package: &cargo_metadata::Package) -> Result<()> {
    MetadataCommand::new().manifest_path(&package.manifest_path)
        .features(CargoOpt::NoDefaultFeatures)
        .features(CargoOpt::SomeFeatures(vec!["component-catalog".to_owned()]))
        .exec().context("catalog files were created, but dependency resolution failed; resolve Cargo.lock before serving")?;
    Ok(())
}

fn manifest_change(
    package: &cargo_metadata::Package,
    root: &Path,
    library: &LibrarySource,
) -> Result<FileChange> {
    let manifest_source =
        fs::read_to_string(&package.manifest_path).context("read consumer manifest")?;
    let mut manifest = manifest_source
        .parse::<DocumentMut>()
        .context("parse consumer manifest")?;
    ensure!(
        !package.features.contains_key("component-catalog"),
        "package already has a component-catalog feature"
    );
    ensure!(
        !package
            .targets
            .iter()
            .any(|target| target.name == "component-catalog"),
        "package already has a component-catalog target"
    );
    ensure!(
        !manifest.contains_key("target")
            || !package
                .dependencies
                .iter()
                .any(|dependency| dependency.name == "dx-story" && dependency.target.is_some()),
        "move the target-specific dx-story dependency to [dependencies] before initialization"
    );
    ensure!(
        !package.dependencies.iter().any(|dependency| {
            matches!(dependency.name.as_str(), "dx-story" | "dioxus")
                && dependency
                    .rename
                    .as_deref()
                    .is_some_and(|rename| rename != dependency.name)
        }),
        "initialization requires the dependency names dx-story and dioxus; configure renamed dependencies manually"
    );
    let dependencies = manifest
        .entry("dependencies")
        .or_insert(Item::Table(Table::new()))
        .as_table_like_mut()
        .context("dependencies must be a table")?;
    if !dependencies.contains_key("dx-story") {
        dependencies.insert(
            "dx-story",
            Item::Value(Value::InlineTable(library.dependency(root)?)),
        );
    }
    let optional = dependencies
        .get("dx-story")
        .and_then(Item::as_table_like)
        .and_then(|table| table.get("optional"))
        .and_then(Item::as_bool)
        .unwrap_or(false);
    if !dependencies.contains_key("dioxus") {
        let mut dependency = InlineTable::new();
        dependency.insert("version", Value::from(library.dioxus_requirement()));
        dependency.insert("default-features", Value::from(false));
        dependencies.insert("dioxus", Item::Value(Value::InlineTable(dependency)));
    }
    let mut features = Array::new();
    if optional {
        features.push("dep:dx-story");
    }
    if package
        .dependencies
        .iter()
        .any(|dependency| dependency.name == "dioxus" && dependency.optional)
    {
        features.push("dep:dioxus");
    }
    features.push("dx-story/catalog");
    manifest["features"]["component-catalog"] = value(features);
    let mut example = Table::new();
    example["name"] = value("component-catalog");
    example["path"] = value("dev/main.rs");
    let mut required = Array::new();
    required.push("component-catalog");
    example["required-features"] = value(required);
    manifest
        .entry("example")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .context("example must be an array of tables")?
        .push(example);
    Ok(FileChange {
        path: package.manifest_path.clone().into_std_path_buf(),
        previous: Some(manifest_source),
        contents: manifest.to_string(),
    })
}

fn embedded_main(
    package: &cargo_metadata::Package,
    root: &Path,
    changes: &mut Vec<FileChange>,
) -> Result<String> {
    let library = package
        .targets
        .iter()
        .find(|target| target.is_lib())
        .context("--embedded requires a library target")?;
    let library_path = library.src_path.as_std_path();
    let source = fs::read_to_string(library_path).context("read consumer library root")?;
    ensure!(
        !source.contains("launch_component_catalog") && !source.contains("mod component_catalog"),
        "library already has component catalog wiring"
    );
    let module_path = relative_path(
        library_path
            .parent()
            .context("library root has no parent")?,
        &root.join("dev/catalog.rs"),
    )?;
    let contents = format!(
        "{source}\n{}",
        include_str!("init/embedded_library.rs.txt").replace(
            "{module_path}",
            &format!("{:?}", module_path.to_string_lossy())
        )
    );
    changes.push(FileChange {
        path: library_path.to_owned(),
        previous: Some(source),
        contents,
    });
    changes.push(FileChange {
        path: root.join("dev/catalog.rs"),
        previous: None,
        contents: include_str!("init/catalog.rs.txt").to_owned(),
    });
    Ok(include_str!("init/embedded_main.rs.txt")
        .replace("{crate_name}", &library.name.replace('-', "_")))
}

fn apply(changes: &[FileChange], dry_run: bool) -> Result<()> {
    for change in changes {
        if let Some(previous) = &change.previous {
            ensure!(
                fs::read_to_string(&change.path)? == *previous,
                "file changed during initialization: {}",
                change.path.display()
            );
        } else {
            ensure!(
                !change.path.exists(),
                "refusing to overwrite {}; no files were changed",
                change.path.display()
            );
        }
    }
    for change in changes {
        if dry_run {
            println!("--- {}\n{}", change.path.display(), change.contents);
        } else {
            write_change(change)?;
        }
    }
    Ok(())
}

fn write_change(change: &FileChange) -> Result<()> {
    fs::create_dir_all(change.path.parent().context("output has no parent")?)?;
    if change.previous.is_none() {
        use std::io::Write;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&change.path)?
            .write_all(change.contents.as_bytes())?;
    } else {
        fs::write(&change.path, &change.contents)?;
    }
    eprintln!("dx-story: wrote {}", change.path.display());
    Ok(())
}

fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let from = std::path::absolute(from)?;
    let to = std::path::absolute(to)?;
    let common = from
        .components()
        .zip(to.components())
        .take_while(|(left, right)| left == right)
        .count();
    ensure!(common > 0, "paths must share a filesystem root");
    Ok(std::iter::repeat_n(
        std::path::Component::ParentDir,
        from.components().count() - common,
    )
    .chain(to.components().skip(common))
    .collect())
}
