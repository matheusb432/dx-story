use anyhow::Result;
use clap::{Args, Subcommand};
use xtk_test::{OutputPath, Run, Test, TestCountDiscovery, surface};

use crate::process;

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct TestArguments {
    #[command(subcommand)]
    command: Option<TestCommand>,
    /// Stream full test output live; the log still captures it.
    #[arg(long)]
    verbose: bool,
    /// Emit one JSON report to stdout.
    #[arg(long)]
    json: bool,
    /// Include ignored tests in the complete workspace suite.
    #[arg(long)]
    all: bool,
}

#[derive(Subcommand)]
enum TestCommand {
    /// Collect workspace test coverage with cargo-llvm-cov.
    Coverage(TestCoverageArguments),
}

#[derive(Args)]
#[command(disable_help_flag = true)]
struct TestCoverageArguments {
    /// Extra cargo-llvm-cov arguments; output defaults to --quiet when unspecified.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    arguments_extra: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scope {
    Default,
    All,
}

impl Scope {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "test",
            Self::All => "all",
        }
    }
}

pub(crate) fn run(arguments: &TestArguments) -> Result<()> {
    match &arguments.command {
        Some(TestCommand::Coverage(coverage)) => test_coverage(&coverage.arguments_extra),
        None => run_scope(arguments.scope(), arguments.verbose, arguments.json),
    }
}

impl TestArguments {
    const fn scope(&self) -> Scope {
        if self.all { Scope::All } else { Scope::Default }
    }
}

fn run_scope(scope: Scope, verbose: bool, json: bool) -> Result<()> {
    Run::try_new(scope.as_str(), tests(scope)?)?
        .verbose(verbose)
        .json(json)
        .output_path(OutputPath::default())
        .execute()?;
    Ok(())
}

fn tests(scope: Scope) -> Result<Vec<Test>> {
    let test = Test::try_new("workspace", surface::CARGO, "cargo")?
        .args(test_arguments(scope))
        .test_count_discovery(TestCountDiscovery::CARGO_TEST_HARNESS);
    let test = if scope == Scope::All {
        test.verbose_arguments(["--nocapture"])
    } else {
        test.verbose_arguments(["--", "--nocapture"])
    };
    Ok(vec![test])
}

fn test_arguments(scope: Scope) -> Vec<&'static str> {
    match scope {
        Scope::Default => vec!["test", "--workspace", "--all-features"],
        Scope::All => vec![
            "test",
            "--workspace",
            "--all-features",
            "--",
            "--include-ignored",
        ],
    }
}

fn test_coverage(arguments_extra: &[String]) -> Result<()> {
    let arguments = test_coverage_arguments(arguments_extra);
    process::run("test coverage", "cargo", &arguments)?;
    if coverage_cleanup_is_required(arguments_extra) {
        process::run(
            "clean coverage artifacts",
            "cargo",
            ["clean", "--target-dir", "target/llvm-cov-target"],
        )?;
    }
    Ok(())
}

fn test_coverage_arguments(arguments_extra: &[String]) -> Vec<String> {
    let mut arguments = vec!["llvm-cov".to_string()];
    if !cargo_package_scope_is_explicit(arguments_extra) {
        arguments.push("--workspace".to_string());
    }
    if !coverage_output_is_explicit(arguments_extra) {
        arguments.push("--quiet".to_string());
    }
    arguments.extend(arguments_extra.iter().cloned());
    arguments
}

fn cargo_package_scope_is_explicit(arguments: &[String]) -> bool {
    arguments
        .iter()
        .take_while(|argument| argument.as_str() != "--")
        .any(|argument| {
            matches!(
                argument.as_str(),
                "-p" | "--package" | "--workspace" | "--all" | "--manifest-path"
            ) || argument.starts_with("-p=")
                || argument.starts_with("--package=")
                || argument.starts_with("--manifest-path=")
        })
}

fn coverage_cleanup_is_required(arguments: &[String]) -> bool {
    !arguments
        .iter()
        .take_while(|argument| argument.as_str() != "--")
        .any(|argument| matches!(argument.as_str(), "-h" | "--help" | "--no-report"))
}

fn coverage_output_is_explicit(arguments: &[String]) -> bool {
    arguments
        .iter()
        .take_while(|argument| argument.as_str() != "--")
        .any(|argument| {
            matches!(argument.as_str(), "--quiet" | "--verbose")
                || argument.strip_prefix('-').is_some_and(|flags| {
                    !flags.is_empty() && flags.bytes().all(|flag| matches!(flag, b'q' | b'v'))
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scope_runs_the_complete_workspace_suite() {
        assert_eq!(
            test_arguments(Scope::Default),
            ["test", "--workspace", "--all-features"]
        );
    }

    #[test]
    fn all_scope_includes_ignored_tests() {
        assert_eq!(
            test_arguments(Scope::All),
            [
                "test",
                "--workspace",
                "--all-features",
                "--",
                "--include-ignored",
            ]
        );
    }

    #[test]
    fn test_coverage_defaults_to_quiet_and_forwards_arguments() {
        assert_eq!(
            test_coverage_arguments(&["--show-missing-lines".to_string()]),
            ["llvm-cov", "--workspace", "--quiet", "--show-missing-lines"]
        );
    }

    #[test]
    fn test_coverage_preserves_an_explicit_package_scope() {
        assert_eq!(
            test_coverage_arguments(&["--package=xtask".to_string()]),
            ["llvm-cov", "--quiet", "--package=xtask"]
        );
    }

    #[test]
    fn test_coverage_preserves_explicit_output_options_before_test_arguments() {
        for option in ["-q", "-v", "-vv", "--quiet", "--verbose"] {
            assert!(coverage_output_is_explicit(&[option.to_string()]));
        }
        assert!(!coverage_output_is_explicit(&[
            "--".to_string(),
            "--verbose".to_string(),
        ]));
    }

    #[test]
    fn test_coverage_cleanup_requires_a_generated_report() {
        assert!(!coverage_cleanup_is_required(&["--help".to_string()]));
        assert!(!coverage_cleanup_is_required(&["--no-report".to_string()]));
        assert!(coverage_cleanup_is_required(&[
            "--".to_string(),
            "--help".to_string(),
        ]));
    }
}
