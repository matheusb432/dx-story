use std::{ffi::OsStr, process::Command};

use anyhow::{Context, Result, bail};

pub(crate) fn run<I, S>(label: &str, program: &str, arguments: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(program)
        .args(arguments)
        .status()
        .with_context(|| format!("spawning {label}"))?;
    if !status.success() {
        bail!("{label} failed (exit {})", status.code().unwrap_or(-1));
    }
    Ok(())
}
