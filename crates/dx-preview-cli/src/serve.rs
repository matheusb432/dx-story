use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use cargo_metadata::MetadataCommand;
use clap::Args;
#[cfg(unix)]
use command_group::{Signal, UnixChildExt};
use walkdir::WalkDir;

use crate::config::{Configuration, OpenBrowser};

const SOURCE_DIRECTORY_COUNT_MAX: usize = 64;
const FEATURE_COUNT_MAX: usize = 64;
const SOURCE_ENTRY_COUNT_MAX: usize = 10_000;
const SOURCE_DEPTH_MAX: usize = 64;
const DEVELOPMENT_POLL_INTERVAL: Duration = Duration::from_millis(250);
const DEVELOPMENT_STOP_GRACE_PERIOD: Duration = Duration::from_secs(2);
const WEB_ASSET_LOCK_FILE: &str = "web-assets.lock";

#[derive(Args, Debug)]
pub(crate) struct ServeArguments {
    /// Port to serve on.
    #[arg(short, long)]
    port: Option<u16>,
    /// Open the browser when the server starts.
    #[arg(short, long, num_args = 0..=1, default_missing_value = "yes")]
    open: Option<OpenBrowser>,
    /// Arguments passed to `dx serve`.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "DX_ARGUMENTS"
    )]
    arguments: Vec<OsString>,
}

pub(crate) fn run(configuration: Configuration, arguments: &ServeArguments) -> Result<()> {
    let project = PreviewProject::try_from(configuration)?;
    let mut known_sources = rust_sources(&project.source_directories)?;

    build_project_styles(&project)?;
    let _tailwind_watcher = DevelopmentWatcher::spawn(&project.tailwind_command(true))?;
    let server = project.dioxus_serve_command(arguments);

    while let Some(current_sources) = run_server_until_source_change(
        &server,
        &project.preview_path,
        &project.source_directories,
        &known_sources,
    )? {
        known_sources = current_sources;
    }
    Ok(())
}

pub(crate) fn build_styles(configuration: Configuration) -> Result<()> {
    build_project_styles(&PreviewProject::try_from(configuration)?)
}

fn build_project_styles(project: &PreviewProject) -> Result<()> {
    let _lock = WebAssetLock::acquire(&project.preview_path)?;
    run_process(&project.tailwind_command(false))
}

struct PreviewProject {
    preview_path: PathBuf,
    package: String,
    example: String,
    features: Vec<String>,
    default_features: bool,
    locked: bool,
    source_directories: Vec<PathBuf>,
    tailwind_input: PathBuf,
    tailwind_output: PathBuf,
    serve_port: u16,
    serve_open: OpenBrowser,
}

impl TryFrom<Configuration> for PreviewProject {
    type Error = anyhow::Error;

    fn try_from(configuration: Configuration) -> Result<Self> {
        let preview_path = configuration.root.join(configuration.preview.path);
        ensure!(
            preview_path.is_dir(),
            "configured preview path is not a directory: {}",
            preview_path.display()
        );
        ensure!(
            preview_path.join("Cargo.toml").is_file(),
            "configured preview path has no Cargo.toml: {}",
            preview_path.display()
        );
        ensure!(
            !configuration.preview.package.trim().is_empty(),
            "configured preview package is empty"
        );
        ensure!(
            !configuration.preview.example.trim().is_empty(),
            "configured preview example is empty"
        );
        ensure!(
            configuration.preview.features.len() <= FEATURE_COUNT_MAX
                && configuration
                    .preview
                    .features
                    .iter()
                    .all(|feature| !feature.trim().is_empty()),
            "configured preview features are invalid"
        );
        ensure!(
            !configuration.preview.source_directories.is_empty()
                && configuration.preview.source_directories.len() <= SOURCE_DIRECTORY_COUNT_MAX,
            "configured source directory count must be between 1 and {SOURCE_DIRECTORY_COUNT_MAX}"
        );
        ensure!(
            !configuration.tailwind.input.as_os_str().is_empty(),
            "configured Tailwind input is empty"
        );
        ensure!(
            !configuration.tailwind.output.as_os_str().is_empty(),
            "configured Tailwind output is empty"
        );

        let source_directories = configuration
            .preview
            .source_directories
            .into_iter()
            .map(|directory| preview_path.join(directory))
            .collect();

        Ok(Self {
            preview_path,
            package: configuration.preview.package,
            example: configuration.preview.example,
            features: configuration.preview.features,
            default_features: configuration.preview.default_features,
            locked: configuration.preview.locked,
            source_directories,
            tailwind_input: configuration.tailwind.input,
            tailwind_output: configuration.tailwind.output,
            serve_port: configuration.serve.port,
            serve_open: configuration.serve.open,
        })
    }
}

impl PreviewProject {
    fn tailwind_command(&self, watch: bool) -> ProcessSpec {
        let mut arguments = vec![
            "run".into(),
            "--frozen".into(),
            "--allow-all".into(),
            "@tailwindcss/cli".into(),
            "--input".into(),
            self.tailwind_input.as_os_str().to_owned(),
            "--output".into(),
            self.tailwind_output.as_os_str().to_owned(),
            "--minify".into(),
        ];
        if watch {
            arguments.extend(["--watch=always".into(), "--poll=100".into()]);
        }

        ProcessSpec::new("dx-preview-tailwind", "deno", arguments, &self.preview_path)
    }

    fn dioxus_serve_command(&self, overrides: &ServeArguments) -> ProcessSpec {
        let mut arguments = vec![
            "serve".into(),
            "--web".into(),
            "--package".into(),
            self.package.clone().into(),
            "--example".into(),
            self.example.clone().into(),
        ];
        if !self.default_features {
            arguments.push("--no-default-features".into());
        }
        if !self.features.is_empty() {
            arguments.extend(["--features".into(), self.features.join(",").into()]);
        }
        if self.locked {
            arguments.push("--locked".into());
        }
        arguments.extend([
            "--hot-reload".into(),
            "true".into(),
            "--watch".into(),
            "true".into(),
            "--port".into(),
            overrides.port.unwrap_or(self.serve_port).to_string().into(),
            "--open".into(),
            overrides
                .open
                .unwrap_or(self.serve_open)
                .as_bool()
                .to_string()
                .into(),
        ]);
        arguments.extend(overrides.arguments.iter().cloned());

        ProcessSpec::new("dx-preview-serve", "dx", arguments, &self.preview_path)
            .with_environment("CARGO_INCREMENTAL", "1")
            .with_environment("RUSTC_WRAPPER", "")
    }
}

struct ProcessSpec {
    label: &'static str,
    program: &'static str,
    arguments: Vec<OsString>,
    environment: Vec<(&'static str, &'static str)>,
    current_directory: PathBuf,
}

impl ProcessSpec {
    fn new(
        label: &'static str,
        program: &'static str,
        arguments: Vec<OsString>,
        current_directory: &Path,
    ) -> Self {
        Self {
            label,
            program,
            arguments,
            environment: Vec::new(),
            current_directory: current_directory.to_owned(),
        }
    }

    fn with_environment(mut self, key: &'static str, value: &'static str) -> Self {
        self.environment.push((key, value));
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.program);
        command
            .args(&self.arguments)
            .envs(self.environment.iter().copied())
            .current_dir(&self.current_directory);
        command
    }
}

fn run_process(specification: &ProcessSpec) -> Result<()> {
    let status = specification
        .command()
        .status()
        .with_context(|| format!("start {}", specification.label))?;
    ensure!(
        status.success(),
        "{} failed (exit {})",
        specification.label,
        status.code().unwrap_or(-1)
    );
    Ok(())
}

fn run_server_until_source_change(
    specification: &ProcessSpec,
    preview_path: &Path,
    source_directories: &[PathBuf],
    known_sources: &BTreeSet<PathBuf>,
) -> Result<Option<BTreeSet<PathBuf>>> {
    let mut server = DevelopmentServer::spawn(specification)?;
    loop {
        if let Some(status) = server.try_wait()? {
            ensure!(
                status.success(),
                "{} failed (exit {})",
                specification.label,
                status.code().unwrap_or(-1)
            );
            return Ok(None);
        }

        let current_sources = rust_sources(source_directories)?;
        let new_sources = new_rust_sources(known_sources, &current_sources);
        if new_sources.is_empty() {
            thread::sleep(DEVELOPMENT_POLL_INTERVAL);
            continue;
        }

        let paths = new_sources
            .iter()
            .map(|path| {
                path.strip_prefix(preview_path)
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("dx-preview: restarting Dioxus to register new Rust source: {paths}");
        server.stop()?;
        return Ok(Some(current_sources));
    }
}

fn rust_sources(source_directories: &[PathBuf]) -> Result<BTreeSet<PathBuf>> {
    let mut entry_count = 0;
    let mut sources = BTreeSet::new();
    for directory in source_directories {
        collect_rust_sources(directory, &mut entry_count, &mut sources)?;
    }
    Ok(sources)
}

fn collect_rust_sources(
    directory: &Path,
    entry_count: &mut usize,
    sources: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in WalkDir::new(directory)
        .follow_links(false)
        .max_depth(SOURCE_DEPTH_MAX)
    {
        let entry = entry.with_context(|| format!("inventory {}", directory.display()))?;
        *entry_count += 1;
        ensure!(
            *entry_count <= SOURCE_ENTRY_COUNT_MAX,
            "Dioxus source inventory exceeds {SOURCE_ENTRY_COUNT_MAX} entries"
        );
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        {
            sources.insert(entry.into_path());
        }
    }
    Ok(())
}

fn new_rust_sources(known: &BTreeSet<PathBuf>, current: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    current.difference(known).cloned().collect()
}

struct DevelopmentServer {
    label: &'static str,
    child: Child,
}

impl DevelopmentServer {
    fn spawn(specification: &ProcessSpec) -> Result<Self> {
        let child = specification
            .command()
            .spawn()
            .with_context(|| format!("start {}", specification.label))?;
        Ok(Self {
            label: specification.label,
            child,
        })
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child
            .try_wait()
            .with_context(|| format!("poll {} process", self.label))
    }

    fn stop(&mut self) -> Result<()> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }

        self.request_stop()?;
        let deadline = Instant::now() + DEVELOPMENT_STOP_GRACE_PERIOD;
        while Instant::now() < deadline && self.try_wait()?.is_none() {
            thread::sleep(DEVELOPMENT_POLL_INTERVAL);
        }
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        if let Err(error) = self.child.kill()
            && self.try_wait()?.is_none()
        {
            return Err(error).with_context(|| format!("kill {} process", self.label));
        }
        self.child
            .wait()
            .with_context(|| format!("reap {} process", self.label))?;
        Ok(())
    }

    #[cfg(unix)]
    fn request_stop(&mut self) -> Result<()> {
        if let Err(error) = self.child.signal(Signal::SIGINT)
            && self.try_wait()?.is_none()
        {
            return Err(error).with_context(|| format!("stop {} process", self.label));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn request_stop(&mut self) -> Result<()> {
        if let Err(error) = self.child.kill()
            && self.try_wait()?.is_none()
        {
            return Err(error).with_context(|| format!("stop {} process", self.label));
        }
        Ok(())
    }
}

impl Drop for DevelopmentServer {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!(
                "{}: failed to stop development server: {error:#}",
                self.label
            );
        }
    }
}

struct DevelopmentWatcher {
    label: &'static str,
    child: Child,
}

impl DevelopmentWatcher {
    fn spawn(specification: &ProcessSpec) -> Result<Self> {
        let child = specification
            .command()
            .spawn()
            .with_context(|| format!("start {}", specification.label))?;
        Ok(Self {
            label: specification.label,
            child,
        })
    }
}

impl Drop for DevelopmentWatcher {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none()
            && let Err(error) = self.child.kill()
        {
            eprintln!("{}: failed to stop watcher: {error}", self.label);
        }
        if let Err(error) = self.child.wait() {
            eprintln!("{}: failed to reap watcher: {error}", self.label);
        }
    }
}

struct WebAssetLock(File);

impl WebAssetLock {
    fn acquire(preview_path: &Path) -> Result<Self> {
        let target_directory = MetadataCommand::new()
            .manifest_path(preview_path.join("Cargo.toml"))
            .current_dir(preview_path)
            .other_options(vec!["--locked".to_owned()])
            .exec()
            .context("resolve the Cargo target directory")?
            .target_directory
            .into_std_path_buf();
        let path = target_directory.join(WEB_ASSET_LOCK_FILE);
        fs::create_dir_all(path.parent().context("catalog asset lock has no parent")?)
            .with_context(|| format!("create lock parent for {}", path.display()))?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("open web asset lock {}", path.display()))?;
        file.lock()
            .with_context(|| format!("acquire web asset lock {}", path.display()))?;
        Ok(Self(file))
    }
}

impl Drop for WebAssetLock {
    fn drop(&mut self) {
        if let Err(error) = self.0.unlock() {
            eprintln!("failed to release web asset lock: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PreviewConfiguration, ServeConfiguration, TailwindConfiguration};

    fn configuration(project: &Path) -> Configuration {
        let preview_path = project.join("preview");
        fs::create_dir_all(preview_path.join("src")).unwrap();
        fs::create_dir_all(preview_path.join("dev")).unwrap();
        fs::write(
            preview_path.join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .unwrap();

        Configuration {
            root: project.to_owned(),
            preview: PreviewConfiguration {
                path: PathBuf::from("preview"),
                package: "fixture-ui".to_owned(),
                example: "component-preview".to_owned(),
                features: vec!["component-preview".to_owned()],
                default_features: false,
                locked: true,
                source_directories: vec!["src".into(), "dev".into()],
            },
            serve: ServeConfiguration {
                port: 8080,
                open: OpenBrowser::No,
            },
            tailwind: TailwindConfiguration {
                input: "dev/tailwind.css".into(),
                output: "assets/component-preview.css".into(),
            },
        }
    }

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn serve_command_combines_preview_configuration_and_cli_overrides() {
        let project = tempfile::tempdir().unwrap();
        let preview = PreviewProject::try_from(configuration(project.path())).unwrap();
        let overrides = ServeArguments {
            port: Some(3000),
            open: Some(OpenBrowser::Yes),
            arguments: vec!["--addr".into(), "127.0.0.1".into()],
        };

        let command = preview.dioxus_serve_command(&overrides);
        let actual = command
            .arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            arguments(&[
                "serve",
                "--web",
                "--package",
                "fixture-ui",
                "--example",
                "component-preview",
                "--no-default-features",
                "--features",
                "component-preview",
                "--locked",
                "--hot-reload",
                "true",
                "--watch",
                "true",
                "--port",
                "3000",
                "--open",
                "true",
                "--addr",
                "127.0.0.1",
            ])
        );
        assert_eq!(command.current_directory, project.path().join("preview"));
        assert_eq!(
            command.environment,
            [("CARGO_INCREMENTAL", "1"), ("RUSTC_WRAPPER", "")]
        );
    }

    #[test]
    fn tailwind_commands_share_inputs_and_add_watch_arguments_only_for_serve() {
        let project = tempfile::tempdir().unwrap();
        let preview = PreviewProject::try_from(configuration(project.path())).unwrap();

        let build = preview.tailwind_command(false);
        let watch = preview.tailwind_command(true);

        assert!(build.arguments.ends_with(&["--minify".into()]));
        assert!(
            watch
                .arguments
                .ends_with(&["--watch=always".into(), "--poll=100".into()])
        );
        assert_eq!(build.arguments, watch.arguments[..build.arguments.len()]);
    }

    #[test]
    fn source_inventory_reports_only_new_rust_files() {
        let project = tempfile::tempdir().unwrap();
        let configuration = configuration(project.path());
        let preview = PreviewProject::try_from(configuration).unwrap();
        let before = rust_sources(&preview.source_directories).unwrap();
        let rust_source = preview.preview_path.join("dev/new_showcase.rs");
        fs::write(&rust_source, "fn showcase() {}").unwrap();
        fs::write(preview.preview_path.join("dev/notes.txt"), "not Rust").unwrap();

        let after = rust_sources(&preview.source_directories).unwrap();

        assert_eq!(new_rust_sources(&before, &after), [rust_source]);
    }
}
