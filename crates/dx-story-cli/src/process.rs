use std::{
    io::Read,
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use command_group::{CommandGroup, GroupChild};
#[cfg(unix)]
use command_group::{Signal, UnixChildExt};

#[cfg(unix)]
mod terminal;

static STOP: AtomicBool = AtomicBool::new(false);
pub(crate) const POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) fn install_shutdown_handler() -> Result<()> {
    ctrlc::set_handler(|| STOP.store(true, Ordering::Relaxed)).context("install shutdown handler")
}

pub(crate) fn stopping() -> bool {
    STOP.load(Ordering::Relaxed)
}

pub(crate) struct ManagedProcess {
    label: &'static str,
    child: GroupChild,
    stopped: bool,
    #[cfg(unix)]
    terminal: Option<terminal::Foreground>,
}

impl ManagedProcess {
    pub(crate) fn spawn(command: &mut Command, label: &'static str) -> Result<Self> {
        ensure!(!stopping(), "interrupted before starting {label}");
        let child = command
            .group_spawn()
            .with_context(|| format!("start {label}"))?;
        Ok(Self {
            label,
            child,
            stopped: false,
            #[cfg(unix)]
            terminal: None,
        })
    }

    pub(crate) fn take_terminal(&mut self) -> Result<()> {
        #[cfg(unix)]
        {
            self.terminal = Some(terminal::Foreground::take(self.child.id())?);
            self.child
                .signal(Signal::SIGCONT)
                .context("resume foreground Dioxus process")?;
        }
        Ok(())
    }

    pub(crate) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child
            .try_wait()
            .with_context(|| format!("poll {}", self.label))
    }

    pub(crate) fn require_running(&mut self) -> Result<()> {
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("{} exited while serving (exit {status})", self.label);
        }
        Ok(())
    }

    fn wait(&mut self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while self.try_wait()?.is_none() {
            ensure!(!stopping(), "interrupted while running {}", self.label);
            ensure!(
                Instant::now() < deadline,
                "{} timed out after {} seconds",
                self.label,
                timeout.as_secs()
            );
            thread::sleep(POLL_INTERVAL);
        }
        let status = self.try_wait()?.context("process exit status is missing")?;
        ensure!(status.success(), "{} failed (exit {status})", self.label);
        Ok(())
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        if self.stopped {
            return Ok(());
        }
        #[cfg(unix)]
        if self.try_wait()?.is_none() {
            let _ = self.child.signal(Signal::SIGINT);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && self.try_wait()?.is_none() {
            thread::sleep(POLL_INTERVAL);
        }
        // Kill the group even if its leader exited, so grandchildren cannot survive it.
        let _ = self.child.kill();
        self.child
            .wait()
            .with_context(|| format!("reap {}", self.label))?;
        #[cfg(unix)]
        drop(self.terminal.take());
        self.stopped = true;
        Ok(())
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("{}: {error:#}", self.label);
        }
    }
}

pub(crate) fn run(command: &mut Command, label: &'static str, timeout: Duration) -> Result<()> {
    ManagedProcess::spawn(command, label)?.wait(timeout)
}

pub(crate) fn capture(command: &mut Command, label: &'static str) -> Result<String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut process = ManagedProcess::spawn(command, label)?;
    let stdout = process
        .child
        .inner()
        .stdout
        .take()
        .context("capture command output")?;
    let reader = thread::spawn(move || {
        let mut output = String::new();
        stdout
            .take(64 * 1024)
            .read_to_string(&mut output)
            .map(|_| output)
    });
    let result = process.wait(Duration::from_secs(30));
    process.stop()?;
    let output = reader
        .join()
        .map_err(|_| anyhow::anyhow!("{label} output reader failed"))??;
    result?;
    Ok(output)
}
