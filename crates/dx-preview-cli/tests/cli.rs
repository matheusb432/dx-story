use assert_cmd::Command;
use predicates::prelude::*;

fn dx_preview() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dx-preview"))
}

#[test]
fn help_lists_the_preview_commands() {
    dx_preview()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("styles"));
}

#[test]
fn serve_help_lists_runtime_overrides() {
    dx_preview()
        .args(["serve", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--port"))
        .stdout(predicate::str::contains("--open"));
}

#[test]
fn serve_rejects_an_invalid_project_configuration() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("dx-preview.toml"), "invalid = [[[").unwrap();

    dx_preview()
        .current_dir(project.path())
        .arg("serve")
        .assert()
        .failure()
        .stderr(predicate::str::contains("load dx-preview.toml"));
}

#[test]
fn serve_accepts_additional_dioxus_arguments() {
    let project = tempfile::tempdir().unwrap();

    dx_preview()
        .current_dir(project.path())
        .args(["serve", "--addr", "127.0.0.1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("load dx-preview.toml"))
        .stderr(predicate::str::contains("unexpected argument").not());
}
