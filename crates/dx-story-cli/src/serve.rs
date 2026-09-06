use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{IsTerminal, Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use clap::Args;
use walkdir::WalkDir;

use crate::{
    config::OpenBrowser,
    doctor,
    process::{self, ManagedProcess, POLL_INTERVAL},
    project::CatalogProject,
};

#[derive(Args, Debug)]
pub(crate) struct ServeArguments {
    /// Port to serve on (configuration default: 8080).
    #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..))]
    port: Option<u16>,
    /// Open the browser when the server starts.
    #[arg(short, long, num_args = 0..=1, default_missing_value = "yes")]
    open: Option<OpenBrowser>,
    /// Address to listen on.
    #[arg(long, default_value = "127.0.0.1")]
    addr: Ipv4Addr,
    /// Disable Rust and stylesheet watchers and hot reload.
    #[arg(long)]
    no_watch: bool,
    /// Disable the Dioxus terminal interface.
    #[arg(long)]
    non_interactive: bool,
    /// Emit one JSON ready event on stdout; send all child output to stderr.
    #[arg(long)]
    ready_json: bool,
    /// Maximum seconds to wait for an HTTP-ready catalog after starting Dioxus.
    #[arg(long, default_value = "300", value_parser = clap::value_parser!(u64).range(1..=3600))]
    ready_timeout: u64,
    /// Additional arguments passed to `dx serve` after `--`.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "DX_ARGUMENTS"
    )]
    arguments: Vec<OsString>,
}

pub(crate) fn run(project: &CatalogProject, arguments: &ServeArguments) -> Result<()> {
    validate_forwarded_arguments(arguments)?;
    doctor::check(project)?;
    let port = arguments.port.unwrap_or(project.serve().port);
    let address = SocketAddrV4::new(arguments.addr, port);
    drop(
        TcpListener::bind(address)
            .with_context(|| format!("catalog address {address} is unavailable"))?,
    );
    let _lock = project
        .tailwind()
        .map(|_| WebAssetLock::acquire(project.target_directory()))
        .transpose()?;
    build_styles_unlocked(project)?;
    let mut watcher = if arguments.no_watch {
        None
    } else {
        tailwind_command(project, true)
            .map(|mut command| ManagedProcess::spawn(&mut command, "Tailwind watcher"))
            .transpose()?
    };
    let mut known_sources = if arguments.no_watch {
        BTreeSet::new()
    } else {
        rust_sources(project.source_directories())?
    };
    let mut server = spawn_server(project, arguments)?;
    let mut deadline = Instant::now() + Duration::from_secs(arguments.ready_timeout);
    let mut ready = false;
    loop {
        if process::stopping() {
            return Ok(());
        }
        if let Some(watcher) = &mut watcher {
            watcher.require_running()?;
        }
        if let Some(status) = server.try_wait()? {
            ensure!(
                status.success() && ready,
                "Dioxus server exited (exit {status}; ready: {ready})"
            );
            return Ok(());
        }
        if !ready {
            ready = poll_ready(address, deadline, arguments)?;
        }
        if let Some(sources) = changed_sources(project, arguments, &known_sources)? {
            known_sources = sources;
            eprintln!("dx-story: restarting Dioxus after Rust source files were added or removed");
            server.stop()?;
            server = spawn_server(project, arguments)?;
            deadline = Instant::now() + Duration::from_secs(arguments.ready_timeout);
            ready = false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn spawn_server(project: &CatalogProject, arguments: &ServeArguments) -> Result<ManagedProcess> {
    let mut server =
        ManagedProcess::spawn(&mut dioxus_command(project, arguments), "Dioxus server")?;
    if !arguments.non_interactive && !arguments.ready_json && std::io::stdin().is_terminal() {
        server.take_terminal()?;
    }
    Ok(server)
}

fn changed_sources(
    project: &CatalogProject,
    arguments: &ServeArguments,
    known: &BTreeSet<PathBuf>,
) -> Result<Option<BTreeSet<PathBuf>>> {
    if arguments.no_watch {
        return Ok(None);
    }
    let sources = rust_sources(project.source_directories())?;
    Ok((sources != *known).then_some(sources))
}

fn poll_ready(
    address: SocketAddrV4,
    deadline: Instant,
    arguments: &ServeArguments,
) -> Result<bool> {
    if !catalog_responds(address) {
        ensure!(
            Instant::now() < deadline,
            "catalog did not become HTTP-ready within {} seconds",
            arguments.ready_timeout
        );
        return Ok(false);
    }
    let url = format!("http://{address}");
    if arguments.ready_json {
        println!("{}", serde_json::json!({"event": "ready", "url": url}));
        std::io::stdout().flush().context("flush readiness event")?;
    } else {
        eprintln!("dx-story: ready at {url}");
    }
    Ok(true)
}

fn validate_forwarded_arguments(arguments: &ServeArguments) -> Result<()> {
    if arguments.ready_json || arguments.no_watch {
        for argument in &arguments.arguments {
            let value = argument.to_string_lossy();
            let flag = value.split('=').next().unwrap_or_default();
            ensure!(
                !matches!(
                    flag,
                    "--port"
                        | "-p"
                        | "--addr"
                        | "--watch"
                        | "--hot-reload"
                        | "--interactive"
                        | "--open"
                ),
                "{flag} cannot be forwarded with --ready-json or --no-watch; use dx-story's serve options"
            );
        }
    }
    Ok(())
}

pub(crate) fn build_styles(project: &CatalogProject) -> Result<()> {
    ensure!(
        project.tailwind().is_some(),
        "this catalog has no [tailwind] configuration; no stylesheet build is needed"
    );
    let _lock = WebAssetLock::acquire(project.target_directory())?;
    build_styles_unlocked(project)
}

fn build_styles_unlocked(project: &CatalogProject) -> Result<()> {
    if let Some(mut command) = tailwind_command(project, false) {
        process::run(&mut command, "Tailwind build", Duration::from_secs(120))?;
    }
    Ok(())
}

fn tailwind_command(project: &CatalogProject, watch: bool) -> Option<Command> {
    let tailwind = project.tailwind()?;
    let mut command = Command::new("deno");
    command
        .args([
            "run",
            "--frozen",
            "--allow-all",
            "@tailwindcss/cli",
            "--input",
        ])
        .arg(&tailwind.input)
        .arg("--output")
        .arg(&tailwind.output)
        .arg("--minify")
        .current_dir(project.path())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if watch {
        command.args(["--watch=always", "--poll=100"]);
    }
    command.stdout(std::io::stderr());
    Some(command)
}

fn dioxus_command(project: &CatalogProject, overrides: &ServeArguments) -> Command {
    let mut command = Command::new("dx");
    command.arg("serve").args(project.target_arguments()).args([
        "--hot-reload",
        &(!overrides.no_watch).to_string(),
        "--watch",
        &(!overrides.no_watch).to_string(),
        "--port",
        &overrides.port.unwrap_or(project.serve().port).to_string(),
        "--addr",
        &overrides.addr.to_string(),
        "--open",
        &overrides
            .open
            .unwrap_or(project.serve().open)
            .as_bool()
            .to_string(),
    ]);
    if overrides.non_interactive || overrides.ready_json {
        command
            .args(["--interactive", "false"])
            .stdin(Stdio::null());
    }
    if overrides.ready_json {
        command.stdout(std::io::stderr());
    }
    command
        .args(&overrides.arguments)
        .current_dir(project.path())
        .env(
            "CARGO_INCREMENTAL",
            if overrides.no_watch { "0" } else { "1" },
        )
        .env("RUSTC_WRAPPER", "");
    command
}

fn rust_sources(directories: &[PathBuf]) -> Result<BTreeSet<PathBuf>> {
    let mut sources = BTreeSet::new();
    let mut count = 0;
    for directory in directories {
        collect_sources(directory, &mut count, &mut sources)?;
    }
    Ok(sources)
}

fn collect_sources(
    directory: &Path,
    count: &mut usize,
    sources: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in WalkDir::new(directory)
        .follow_links(false)
        .max_depth(64)
        .into_iter()
        .filter_entry(|entry| {
            !entry.file_type().is_dir()
                || !matches!(
                    entry.file_name().to_str(),
                    Some("target" | "node_modules" | ".git" | "dist" | ".cache")
                )
        })
    {
        let entry = entry.with_context(|| format!("inventory {}", directory.display()))?;
        *count += 1;
        ensure!(
            *count <= 100_000,
            "Dioxus source inventory exceeds 100000 entries"
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

fn catalog_responds(mut address: SocketAddrV4) -> bool {
    if address.ip().is_unspecified() {
        address.set_ip(Ipv4Addr::LOCALHOST);
    }
    let Ok(mut connection) = TcpStream::connect_timeout(&address.into(), POLL_INTERVAL) else {
        return false;
    };
    let timeout = Some(Duration::from_millis(250));
    if connection.set_read_timeout(timeout).is_err()
        || connection.set_write_timeout(timeout).is_err()
    {
        return false;
    }
    if connection
        .write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\nAccept: text/html\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = String::new();
    connection
        .take(1024 * 1024)
        .read_to_string(&mut response)
        .is_ok()
        && (response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"))
        && response.contains("</html>")
        && !response.contains("We're building your app now")
}

struct WebAssetLock(File);

impl WebAssetLock {
    fn acquire(target: &Path) -> Result<Self> {
        fs::create_dir_all(target).context("create Cargo target directory")?;
        let path = target.join("web-assets.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .context("open web asset lock")?;
        let deadline = Instant::now() + Duration::from_secs(30);
        while !try_asset_lock(&file)? {
            ensure!(
                !process::stopping() && Instant::now() < deadline,
                "web assets are busy; stop the other preview server or stylesheet build ({})",
                path.display()
            );
            thread::sleep(POLL_INTERVAL);
        }
        Ok(Self(file))
    }
}

fn try_asset_lock(file: &File) -> Result<bool> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(error) => Err(error).context("lock web assets"),
    }
}

impl Drop for WebAssetLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
