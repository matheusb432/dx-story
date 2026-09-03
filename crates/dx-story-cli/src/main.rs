use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{config::Configuration, serve::ServeArguments};

mod config;
mod serve;

#[derive(Parser)]
#[command(
    name = "dx-story",
    version,
    about = "Dioxus component catalog tools",
    styles = clap_cargo::style::CLAP_STYLING
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the catalog development server.
    Serve(ServeArguments),
    /// Build the catalog stylesheet.
    Styles,
}

fn main() {
    let command = Cli::parse().command;
    if let Err(error) = run(command) {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run(command: Command) -> Result<()> {
    let configuration = Configuration::load()?;
    match command {
        Command::Serve(arguments) => serve::run(configuration, &arguments),
        Command::Styles => serve::build_styles(configuration),
    }
}
