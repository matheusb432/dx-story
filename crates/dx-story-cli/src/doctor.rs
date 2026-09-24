use std::{process::Command, thread};

use anyhow::{Context, Result, anyhow, ensure};
use cargo_metadata::semver::Version;

use crate::{process, project::CatalogProject, styles};

pub(crate) fn check(project: &CatalogProject) -> Result<()> {
    let (dioxus, wasm, tailwind) = thread::scope(|scope| {
        let dioxus = scope.spawn(|| check_dioxus_cli(project));
        let wasm = scope.spawn(|| check_wasm_target(project));
        let tailwind = project
            .tailwind()
            .map(|_| scope.spawn(|| check_tailwind(project)));
        (
            dioxus.join(),
            wasm.join(),
            tailwind.map(thread::ScopedJoinHandle::join),
        )
    });
    dioxus.map_err(|_| anyhow!("Dioxus CLI probe panicked"))??;
    wasm.map_err(|_| anyhow!("Rust target inventory probe panicked"))??;
    if let Some(tailwind) = tailwind {
        tailwind.map_err(|_| anyhow!("Deno/Tailwind probe panicked"))??;
    }
    Ok(())
}

fn check_dioxus_cli(project: &CatalogProject) -> Result<()> {
    let version = process::capture(
        Command::new("dx")
            .arg("--version")
            .current_dir(project.path()),
        "Dioxus CLI",
    )
    .with_context(|| {
        format!(
            "install Dioxus CLI with: cargo install dioxus-cli --version {} --locked",
            project.dioxus_version()
        )
    })?;
    let actual = version
        .split_whitespace()
        .find(|part| part.as_bytes().first().is_some_and(u8::is_ascii_digit))
        .and_then(|part| Version::parse(part).ok());
    ensure!(
        actual.as_ref() == Some(project.dioxus_version()),
        "Dioxus CLI version does not match the catalog: got {}, need {}; install with: cargo install dioxus-cli --version {} --locked",
        version.trim(),
        project.dioxus_version(),
        project.dioxus_version()
    );
    Ok(())
}

fn check_wasm_target(project: &CatalogProject) -> Result<()> {
    let targets = process::capture(
        Command::new("rustup")
            .args(["target", "list", "--installed"])
            .current_dir(project.path()),
        "Rust target inventory",
    )?;
    ensure!(
        targets
            .lines()
            .any(|target| target == "wasm32-unknown-unknown"),
        "missing wasm32-unknown-unknown; run: rustup target add wasm32-unknown-unknown"
    );
    Ok(())
}

fn check_tailwind(project: &CatalogProject) -> Result<()> {
    process::capture(styles::tailwind_cli(project).arg("--help"), "Deno/Tailwind")
        .context("check deno.json imports and deno.lock")?;
    Ok(())
}

pub(crate) fn run(project: &CatalogProject) -> Result<()> {
    check(project)?;
    println!("Catalog: {} / {}", project.package(), project.example());
    println!("Dioxus: {}", project.dioxus_version());
    println!(
        "Styles: {}",
        if project.tailwind().is_some() {
            "Deno / Tailwind"
        } else {
            "consumer-provided CSS"
        }
    );
    println!("Watched directories:");
    for directory in project.source_directories() {
        println!("  {}", directory.display());
    }
    Ok(())
}
