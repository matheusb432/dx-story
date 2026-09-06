use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use cargo_metadata::{CargoOpt, MetadataCommand};
use clap::Args;
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};

use crate::project::select_package;

#[derive(Args)]
pub(crate) struct InitArguments {
    /// Workspace package that owns the components.
    #[arg(short, long)]
    package: String,
    /// Put stories inside the library to access crate-private components.
    #[arg(long)]
    embedded: bool,
    /// Local dx-story library crate (defaults to this CLI's sibling crate).
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

pub(crate) fn run(arguments: &InitArguments, configuration: Option<&Path>) -> Result<()> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("read workspace for initialization")?;
    let package = select_package(&metadata, &arguments.package)?;
    let root = package
        .manifest_path
        .parent()
        .context("package manifest has no parent")?
        .as_std_path();
    let configuration = configuration.map_or_else(
        || {
            metadata
                .workspace_root
                .join("dx-story.toml")
                .into_std_path_buf()
        },
        Path::to_owned,
    );
    let library_path = arguments
        .library_path
        .clone()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../dx-story"));
    let library_path = library_path
        .canonicalize()
        .context("resolve local dx-story library; use --library-path")?;
    let library_metadata = MetadataCommand::new()
        .manifest_path(library_path.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("read local dx-story manifest")?;
    let library = select_package(&library_metadata, "dx-story")?;
    let dioxus = library
        .dependencies
        .iter()
        .find(|dependency| dependency.name == "dioxus")
        .context("dx-story has no dioxus dependency")?;
    let mut settings = DocumentMut::new();
    settings["catalog"]["package"] = value(arguments.package.as_str());
    if configuration.parent() != Some(metadata.workspace_root.as_std_path()) {
        let absolute = std::path::absolute(&configuration)?;
        let directory = absolute.parent().context("configuration has no parent")?;
        settings["catalog"]["path"] =
            value(relative_path(directory, root)?.to_string_lossy().as_ref());
    }
    let mut changes = vec![
        manifest_change(package, root, &library_path, &dioxus.req.to_string())?,
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
    library_path: &Path,
    dioxus_requirement: &str,
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
    let dependencies = manifest
        .entry("dependencies")
        .or_insert(Item::Table(Table::new()))
        .as_table_like_mut()
        .context("dependencies must be a table")?;
    if !dependencies.contains_key("dx-story") {
        let mut dependency = InlineTable::new();
        dependency.insert(
            "path",
            Value::from(
                relative_path(root, library_path)?
                    .to_string_lossy()
                    .as_ref(),
            ),
        );
        dependency.insert("optional", Value::from(true));
        dependencies.insert("dx-story", Item::Value(Value::InlineTable(dependency)));
    }
    let optional = dependencies
        .get("dx-story")
        .and_then(Item::as_table_like)
        .and_then(|table| table.get("optional"))
        .and_then(Item::as_bool)
        .unwrap_or(false);
    if !dependencies.contains_key("dioxus") {
        let mut dependency = InlineTable::new();
        dependency.insert("version", Value::from(dioxus_requirement));
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
        "{source}\n#[cfg(feature = \"component-catalog\")]\n#[path = {:?}]\nmod component_catalog;\n\n#[cfg(feature = \"component-catalog\")]\n/// Validates the story registry and launches the catalog.\n///\n/// # Errors\n///\n/// Returns a registry error for duplicate or empty story sets.\npub fn launch_component_catalog() -> Result<(), dx_story::RegistryError> {{\n    dx_story::launch(component_catalog::App)\n}}\n",
        module_path.to_string_lossy()
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
    Ok(format!(
        "fn main() -> Result<(), dx_story::RegistryError> {{\n    {}::launch_component_catalog()\n}}\n",
        library.name.replace('-', "_")
    ))
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
