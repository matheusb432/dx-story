use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{config::Configuration, serve::ServeArguments};

mod build;
mod config;
mod doctor;
mod init;
mod process;
mod project;
mod serve;
mod styles;

#[derive(Parser)]
#[command(
    name = "dx-story",
    version,
    about = "Dioxus component catalog tools",
    styles = clap_cargo::style::CLAP_STYLING,
    after_help = "Quick start:\n  dx-story init --package my-ui\n  dx-story doctor\n  dx-story serve --open\n\nUse dx-story <COMMAND> --help for command options."
)]
struct Cli {
    /// Configuration file; otherwise search the current directory and its parents.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the catalog development server.
    #[command(visible_alias = "dev")]
    Serve(ServeArguments),
    /// Build the catalog and its assets for static hosting.
    Build(build::BuildArguments),
    /// Build the catalog stylesheet.
    Styles,
    /// Check Cargo targets, Dioxus, Wasm, and optional stylesheet tooling.
    Doctor,
    /// Add a component catalog to a Cargo package.
    Init(init::InitArguments),
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    process::install_shutdown_handler()?;
    let resolve = || project::CatalogProject::resolve(Configuration::load(cli.config.as_deref())?);
    match cli.command {
        Command::Init(arguments) => init::run(&arguments, cli.config.as_deref()),
        Command::Serve(arguments) => serve::run(&resolve()?, &arguments),
        Command::Build(arguments) => build::run(&resolve()?, &arguments),
        Command::Styles => styles::run(&resolve()?),
        Command::Doctor => doctor::run(&resolve()?),
    }
}
