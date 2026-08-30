use anyhow::Result;
use clap::Parser;

mod cli;
mod process;
mod test;

fn main() {
    if let Err(error) = run(cli::Cli::parse().command) {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run(command: cli::Command) -> Result<()> {
    match command {
        cli::Command::Test(arguments) => test::run(&arguments),
    }
}
