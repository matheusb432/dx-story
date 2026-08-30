use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_the_verb_surface() {
    Command::cargo_bin("xtask")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("test"));
}

#[test]
fn test_exposes_its_flags() {
    Command::cargo_bin("xtask")
        .unwrap()
        .args(["test", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--verbose"))
        .stdout(predicates::str::contains("--json"))
        .stdout(predicates::str::contains("--all"));
}

#[test]
fn unknown_verb_is_rejected() {
    Command::cargo_bin("xtask")
        .unwrap()
        .arg("definitely-not-a-verb")
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("unrecognized subcommand")
                .or(predicates::str::contains("unexpected argument")),
        );
}
