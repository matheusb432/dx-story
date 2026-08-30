use clap::{Parser, Subcommand};

use crate::test::TestArguments;

#[derive(Parser)]
#[command(version, about = "Repository automation")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Run tests.
    Test(TestArguments),
}
