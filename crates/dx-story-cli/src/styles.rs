use std::{
    fs::{self, File, OpenOptions},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};

use crate::{
    process::{self, POLL_INTERVAL},
    project::CatalogProject,
};

pub(crate) fn run(project: &CatalogProject) -> Result<()> {
    if project.tailwind().is_none() {
        eprintln!("dx-story: no [tailwind] configuration; no stylesheet build is needed");
        return Ok(());
    }
    let _lock = WebAssetLock::acquire(project.target_directory())?;
    build(project)
}

pub(crate) fn build(project: &CatalogProject) -> Result<()> {
    if let Some(mut command) = tailwind_command(project, false) {
        process::run(&mut command, "Tailwind build", Duration::from_secs(120))?;
    }
    Ok(())
}

/// The Deno invocation of the Tailwind CLI that both the stylesheet build and the
/// doctor's availability probe start from.
pub(crate) fn tailwind_cli(project: &CatalogProject) -> Command {
    let mut command = Command::new("deno");
    command
        .args(["run", "--frozen", "--allow-all", "@tailwindcss/cli"])
        .current_dir(project.path());
    command
}

pub(crate) fn tailwind_command(project: &CatalogProject, watch: bool) -> Option<Command> {
    let tailwind = project.tailwind()?;
    let mut command = tailwind_cli(project);
    command
        .arg("--input")
        .arg(&tailwind.input)
        .arg("--output")
        .arg(&tailwind.output)
        .arg("--minify")
        .stdin(Stdio::null())
        .stdout(std::io::stderr())
        .stderr(Stdio::inherit());
    if watch {
        command.args(["--watch=always", "--poll=100"]);
    }
    Some(command)
}

pub(crate) struct WebAssetLock(File);

impl WebAssetLock {
    pub(crate) fn acquire(target: &Path) -> Result<Self> {
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
