use std::{fs, path::Path, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn dry_run_and_conflicts_preserve_existing_files() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    let manifest =
        "# Keep this comment.\n[package]\nname='starter'\nversion='0.1.0'\nedition='2024'\n";
    fs::write(project.path().join("Cargo.toml"), manifest).unwrap();
    fs::write(project.path().join("src/lib.rs"), "").unwrap();
    Command::new(env!("CARGO_BIN_EXE_dx-story"))
        .current_dir(project.path())
        .args(["init", "--package", "starter", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Keep this comment."))
        .stdout(predicate::str::contains("dev/stories.rs"));
    assert_eq!(
        fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        manifest
    );
    assert!(!project.path().join("dev").exists());
    assert!(!project.path().join("dx-story.toml").exists());
    fs::create_dir_all(project.path().join("dev")).unwrap();
    fs::write(project.path().join("dev/stories.rs"), "user-owned").unwrap();
    Command::new(env!("CARGO_BIN_EXE_dx-story"))
        .current_dir(project.path())
        .args(["init", "--package", "starter"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to overwrite"));
    assert_eq!(
        fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read_to_string(project.path().join("dev/stories.rs")).unwrap(),
        "user-owned"
    );
    assert!(!project.path().join("dx-story.toml").exists());
}

#[test]
fn generated_public_and_embedded_catalogs_compile() {
    for embedded in [false, true] {
        let project = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join("src")).unwrap();
        fs::write(
            project.path().join("Cargo.toml"),
            "# Preserved\n[package]\nname='starter'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(project.path().join("src/lib.rs"), "").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_dx-story"));
        command
            .current_dir(project.path())
            .args(["init", "--package", "starter"]);
        if embedded {
            command.arg("--embedded");
        }
        command.assert().success();
        assert!(project.path().join("Cargo.lock").is_file());
        let manifest = fs::read_to_string(project.path().join("Cargo.toml")).unwrap();
        assert!(manifest.contains("# Preserved"));
        assert!(manifest.contains("optional = true"));
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/consumer-check");
        Command::new("cargo")
            .current_dir(project.path())
            .env("CARGO_TARGET_DIR", target)
            .args([
                "check",
                "--offline",
                "--no-default-features",
                "--features",
                "component-catalog",
                "--example",
                "component-catalog",
            ])
            .assert()
            .success();
    }
}
