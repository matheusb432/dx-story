use std::process::Command;

use anyhow::{Result, ensure};

use crate::{process, project::CatalogProject};

pub(crate) fn check(project: &CatalogProject) -> Result<()> {
    let version = process::capture(
        Command::new("dx")
            .arg("--version")
            .current_dir(project.path()),
        "Dioxus CLI; provision the project's dioxus-cli version",
    )?;
    let actual = version
        .split_whitespace()
        .find(|part| part.as_bytes().first().is_some_and(u8::is_ascii_digit));
    ensure!(
        actual == Some(project.dioxus_version()),
        "Dioxus CLI version does not match the catalog: got {}, need {}",
        version.trim(),
        project.dioxus_version()
    );
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
        "missing wasm32-unknown-unknown; add it to the project's Rust toolchain"
    );
    if project.tailwind().is_some() {
        process::capture(
            Command::new("deno")
                .args([
                    "run",
                    "--frozen",
                    "--allow-all",
                    "@tailwindcss/cli",
                    "--help",
                ])
                .current_dir(project.path()),
            "Deno/Tailwind; check deno.json imports and deno.lock",
        )?;
    }
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
