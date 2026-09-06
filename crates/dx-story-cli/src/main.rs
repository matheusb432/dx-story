use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{config::Configuration, serve::ServeArguments};

mod config;
mod doctor;
mod init;
mod process;
mod project;
mod serve;

#[derive(Parser)]
#[command(
    name = "dx-story",
    version,
    about = "Dioxus component catalog tools",
    styles = clap_cargo::style::CLAP_STYLING
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
    Serve(ServeArguments),
    /// Build the catalog stylesheet.
    Styles,
    /// Check Cargo targets, Dioxus, Wasm, and optional stylesheet tooling.
    Doctor,
    /// Create a component catalog using a local dx-story dependency.
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
        Command::Styles => serve::build_styles(&resolve()?),
        Command::Doctor => doctor::run(&resolve()?),
    }
}
