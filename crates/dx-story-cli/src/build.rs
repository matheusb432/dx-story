use std::{
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::Result;
use clap::Args;

use crate::{
    doctor, process,
    project::CatalogProject,
    styles::{self, WebAssetLock},
};

#[derive(Args)]
#[command(
    after_help = "Examples:\n  dx-story build\n  dx-story build --release --timeout 900\n\nDioxus reports the output directory. Static hosts must fall back to index.html for catalog routes."
)]
pub(crate) struct BuildArguments {
    /// Optimize the catalog for distribution.
    #[arg(short, long)]
    release: bool,
    /// Maximum seconds to wait for Dioxus to finish building.
    #[arg(long, default_value = "600", value_parser = clap::value_parser!(u64).range(1..=3600))]
    timeout: u64,
}

pub(crate) fn run(project: &CatalogProject, arguments: &BuildArguments) -> Result<()> {
    doctor::check(project)?;
    let _lock = project
        .tailwind()
        .map(|_| WebAssetLock::acquire(project.target_directory()))
        .transpose()?;
    styles::build(project)?;
    let mut command = Command::new("dx");
    command
        .arg("build")
        .args(project.target_arguments())
        .current_dir(project.path())
        .stdin(Stdio::null())
        .stdout(std::io::stderr())
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTC_WRAPPER", "");
    if arguments.release {
        command.args(["--release", "--debug-symbols", "false"]);
    }
    process::run(
        &mut command,
        "Dioxus build",
        Duration::from_secs(arguments.timeout),
    )
}
