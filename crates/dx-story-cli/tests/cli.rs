use assert_cmd::Command;
use predicates::prelude::*;

fn dx_story() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dx-story"))
}

#[test]
fn help_lists_the_story_commands() {
    dx_story()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("build"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("styles"));
}

#[test]
fn init_help_explains_registry_and_local_setup() {
    dx_story()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("instead of crates.io"))
        .stdout(predicate::str::contains("--embedded"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn build_help_and_invalid_timeout_use_the_parser_without_loading_a_project() {
    dx_story()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--release"))
        .stdout(predicate::str::contains("--timeout"));
    dx_story()
        .args(["build", "--timeout", "0"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn dev_alias_uses_the_serve_help() {
    dx_story()
        .args(["dev", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--ready-json"));
}

#[test]
fn serve_help_lists_runtime_overrides() {
    dx_story()
        .args(["serve", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--port"))
        .stdout(predicate::str::contains("--open"));
}

#[test]
fn serve_rejects_an_invalid_project_configuration() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("dx-story.toml"), "invalid = [[[").unwrap();

    dx_story()
        .current_dir(project.path())
        .arg("serve")
        .assert()
        .failure()
        .stderr(predicate::str::contains("dx-story.toml"));
}

#[test]
fn serve_accepts_additional_dioxus_arguments() {
    let project = tempfile::tempdir().unwrap();

    dx_story()
        .current_dir(project.path())
        .args(["serve", "--addr", "127.0.0.1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("load dx-story.toml"))
        .stderr(predicate::str::contains("unexpected argument").not());
}
