use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use command_group::{Signal, UnixChildExt};
use predicates::prelude::*;
use tempfile::TempDir;

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Result<Self> {
        let fixture = Self(tempfile::tempdir()?);
        fixture.write(
            "Cargo.toml",
            "[workspace]\nmembers=['ui','dioxus','helper']\nresolver='3'\n",
        )?;
        fixture.write("ui/Cargo.toml", "[package]\nname='fixture-ui'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ndioxus={path='../dioxus'}\nhelper={path='../helper',optional=true}\n[features]\npreview=['dep:helper']\n[[example]]\nname='preview'\npath='dev/main.rs'\nrequired-features=['preview']\n")?;
        fixture.write("ui/src/lib.rs", "")?;
        fixture.write("ui/dev/main.rs", "fn main() {}\n")?;
        fixture.write(
            "dioxus/Cargo.toml",
            "[package]\nname='dioxus'\nversion='0.7.10'\n",
        )?;
        fixture.write("dioxus/src/lib.rs", "")?;
        fixture.write(
            "helper/Cargo.toml",
            "[package]\nname='helper'\nversion='0.1.0'\n",
        )?;
        fixture.write("helper/src/lib.rs", "")?;
        fixture.write(
            "dx-story.toml",
            "[catalog]\npackage='fixture-ui'\nlocked=false\n",
        )?;
        fixture.script("dx", "if [ \"$1\" = --version ]; then echo 'dioxus 0.7.10'; exit 0; fi\nprintf '%s\\n' \"$@\" > \"$DX_STORY_TEST_ROOT/arguments\"\nexec \"$DX_STORY_TEST_EXECUTABLE\" --exact tests::tool_process --nocapture\n")?;
        fixture.script("rustup", "echo wasm32-unknown-unknown\n")?;
        Ok(fixture)
    }

    fn write(&self, path: &str, contents: &str) -> Result<()> {
        let path = self.0.path().join(path);
        fs::create_dir_all(path.parent().context("fixture path has no parent")?)?;
        fs::write(path, contents)?;
        Ok(())
    }

    fn script(&self, name: &str, contents: &str) -> Result<()> {
        self.write(
            &format!("bin/{name}"),
            &format!("#!/bin/sh\nset -eu\n{contents}"),
        )?;
        fs::set_permissions(
            self.0.path().join("bin").join(name),
            fs::Permissions::from_mode(0o755),
        )?;
        Ok(())
    }

    fn command(&self) -> Result<Command> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dx-story"));
        let mut paths = vec![self.0.path().join("bin")];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").context("PATH is missing")?,
        ));
        command
            .current_dir(self.0.path())
            .env("PATH", std::env::join_paths(paths)?)
            .env("DX_STORY_TEST_ROOT", self.0.path())
            .env("DX_STORY_TEST_EXECUTABLE", std::env::current_exe()?);
        Ok(command)
    }

    fn assert_command(&self) -> Result<assert_cmd::Command> {
        Ok(assert_cmd::Command::from_std(self.command()?))
    }
}

#[test]
fn doctor_discovers_ancestors_target_features_and_local_dependencies_without_tailwind() {
    let fixture = Fixture::new().unwrap();
    fixture
        .assert_command()
        .unwrap()
        .current_dir(fixture.0.path().join("ui/src"))
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture-ui / preview"))
        .stdout(predicate::str::contains("helper/src"))
        .stdout(predicate::str::contains("consumer-provided CSS"));
}

#[test]
fn explicit_config_and_source_overrides_take_precedence() {
    let fixture = Fixture::new().unwrap();
    fixture.write("custom.toml", "[catalog]\npackage='fixture-ui'\nlocked=false\nsource-directories=['dev']\nextra-source-directories=['../helper/src']\n").unwrap();
    fixture
        .write("dx-story.toml", "broken configuration")
        .unwrap();
    fixture
        .assert_command()
        .unwrap()
        .args(["doctor", "--config", "custom.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("helper/src"))
        .stdout(predicate::str::contains("dioxus/src").not());
}

#[test]
fn invalid_features_fail_before_starting_tools() {
    let fixture = Fixture::new().unwrap();
    fixture
        .write(
            "dx-story.toml",
            "[catalog]\npackage='fixture-ui'\nfeatures=['missing']\nlocked=false\n",
        )
        .unwrap();
    fixture
        .assert_command()
        .unwrap()
        .arg("serve")
        .assert()
        .failure()
        .stderr(predicate::str::contains("has no feature missing"));
    assert!(!fixture.0.path().join("arguments").exists());
}

#[test]
fn ambiguous_examples_require_a_selection() {
    let fixture = Fixture::new().unwrap();
    fixture
        .write("ui/examples/other.rs", "fn main() {}\n")
        .unwrap();
    fixture
        .assert_command()
        .unwrap()
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("set catalog.example explicitly"));
}

#[test]
fn doctor_reports_a_dioxus_version_mismatch() {
    let fixture = Fixture::new().unwrap();
    fixture.script("dx", "echo 'dioxus 0.6.0'\n").unwrap();
    fixture
        .assert_command()
        .unwrap()
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("need 0.7.10"));
}

#[test]
fn serve_timeout_and_watcher_exit_are_failures() {
    let fixture = Fixture::new().unwrap();
    fixture
        .assert_command()
        .unwrap()
        .env("DX_STORY_TEST_MODE", "no-http")
        .args([
            "serve",
            "--no-watch",
            "--ready-json",
            "--ready-timeout",
            "1",
            "--port",
            &available_port().unwrap().to_string(),
        ])
        .timeout(Duration::from_secs(15))
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("did not become HTTP-ready"));
    fixture.write("dx-story.toml", "[catalog]\npackage='fixture-ui'\nlocked=false\n[tailwind]\ninput='dev/style.css'\noutput='assets/style.css'\n").unwrap();
    fixture.write("ui/dev/style.css", "").unwrap();
    fixture
        .script(
            "deno",
            "case \"$*\" in *--watch=always*) exit 7;; *) exit 0;; esac\n",
        )
        .unwrap();
    fixture
        .assert_command()
        .unwrap()
        .args([
            "serve",
            "--ready-json",
            "--port",
            &available_port().unwrap().to_string(),
        ])
        .timeout(Duration::from_secs(15))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Tailwind watcher exited"));
}

#[test]
fn serve_emits_ready_json_and_cleans_up_on_interrupt() {
    let fixture = Fixture::new().unwrap();
    let mut child = RunningChild(
        fixture
            .command()
            .unwrap()
            .args([
                "serve",
                "--no-watch",
                "--ready-json",
                "--port",
                &available_port().unwrap().to_string(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        sender.send(BufReader::new(stdout).lines().next()).unwrap();
    });
    let line = receiver
        .recv_timeout(Duration::from_secs(15))
        .unwrap()
        .unwrap()
        .unwrap();
    let event: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(event["event"], "ready");
    let arguments = fs::read_to_string(fixture.0.path().join("arguments")).unwrap();
    assert!(arguments.contains("--features\npreview"));
    assert!(arguments.contains("--watch\nfalse"));
    assert!(arguments.contains("--interactive\nfalse"));
    child.0.signal(Signal::SIGINT).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.0.try_wait().unwrap().is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(child.0.try_wait().unwrap().unwrap().success());
    reader.join().unwrap();
    let pid = fs::read_to_string(fixture.0.path().join("descendant")).unwrap();
    let status = Command::new("kill")
        .args(["-0", pid.trim()])
        .stderr(Stdio::null())
        .status()
        .unwrap();
    // A terminated orphan may briefly remain as a zombie until init reaps it.
    if status.success() {
        let state = fs::read_to_string(format!("/proc/{}/stat", pid.trim())).unwrap_or_default();
        assert!(state.is_empty() || state.contains(") Z "));
    }
}

struct RunningChild(Child);
impl Drop for RunningChild {
    fn drop(&mut self) {
        let _ = self.0.signal(Signal::SIGTERM);
        let deadline = Instant::now() + Duration::from_secs(4);
        while self.0.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn available_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

#[test]
fn tool_process() {
    let Some(root) = std::env::var_os("DX_STORY_TEST_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let arguments = fs::read_to_string(root.join("arguments")).unwrap();
    let mut arguments = arguments.lines();
    let port = arguments
        .find(|argument| *argument == "--port")
        .and_then(|_| arguments.next())
        .unwrap();
    let mut descendant = Command::new("sleep").arg("30").spawn().unwrap();
    fs::write(root.join("descendant"), descendant.id().to_string()).unwrap();
    if std::env::var("DX_STORY_TEST_MODE").as_deref() == Ok("no-http") {
        thread::sleep(Duration::from_secs(10));
        let _ = descendant.kill();
        let _ = descendant.wait();
        return;
    }
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut requests = 0;
    while Instant::now() < deadline {
        if let Ok((mut connection, _)) = listener.accept() {
            connection
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let _ = connection.read(&mut [0; 1024]);
            requests += 1;
            let body = response_body(requests);
            write!(
                connection,
                "HTTP/1.0 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        thread::sleep(Duration::from_millis(20));
    }
    let _ = descendant.kill();
    let _ = descendant.wait();
}

fn response_body(requests: usize) -> &'static str {
    if requests < 3 {
        "<html>We're building your app now</html>"
    } else {
        "<html>catalog</html>"
    }
}

#[test]
fn source_additions_and_removals_restart_the_supervised_server() {
    let fixture = Fixture::new().unwrap();
    let mut child = RunningChild(
        fixture
            .command()
            .unwrap()
            .args([
                "serve",
                "--ready-json",
                "--port",
                &available_port().unwrap().to_string(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().take(3) {
            let _ = sender.send(line);
        }
    });
    let first = receiver
        .recv_timeout(Duration::from_secs(15))
        .unwrap()
        .unwrap();
    assert!(first.contains("\"event\":\"ready\""));
    fixture.write("ui/src/new_story.rs", "").unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .unwrap(),
        first
    );
    fs::remove_file(fixture.0.path().join("ui/src/new_story.rs")).unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .unwrap(),
        first
    );
    drop(child);
    reader.join().unwrap();
}
