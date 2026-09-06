use anyhow::{Context, Result};
use nix::{
    sys::{
        signal::{SigSet, SigmaskHow, Signal, pthread_sigmask},
        termios::{SetArg, Termios, tcgetattr, tcsetattr},
    },
    unistd::{Pid, tcgetpgrp, tcsetpgrp},
};

pub(super) struct Foreground {
    group: Pid,
    settings: Termios,
}

impl Foreground {
    pub(super) fn take(group: u32) -> Result<Self> {
        let previous = tcgetpgrp(std::io::stdin()).context("read terminal foreground group")?;
        let group = Pid::from_raw(i32::try_from(group).context("process group exceeds pid range")?);
        let settings = tcgetattr(std::io::stdin()).context("read terminal settings")?;
        set_foreground(group)?;
        Ok(Self {
            group: previous,
            settings,
        })
    }
}

impl Drop for Foreground {
    fn drop(&mut self) {
        if let Err(error) = set_foreground(self.group) {
            eprintln!("dx-story: restore terminal: {error:#}");
        }
        if let Err(error) = tcsetattr(std::io::stdin(), SetArg::TCSANOW, &self.settings) {
            eprintln!("dx-story: restore terminal settings: {error}");
        }
    }
}

fn set_foreground(group: Pid) -> Result<()> {
    // Changing terminal ownership from a background group otherwise raises SIGTTOU.
    let mut blocked = SigSet::empty();
    blocked.add(Signal::SIGTTOU);
    let mut previous = SigSet::empty();
    pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&blocked), Some(&mut previous))
        .context("block terminal ownership signal")?;
    let changed = tcsetpgrp(std::io::stdin(), group);
    let restored = pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&previous), None);
    changed.context("set terminal foreground group")?;
    restored.context("restore terminal signal mask")
}
