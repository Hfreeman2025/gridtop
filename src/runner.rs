use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    process::{Command, ExitStatus},
};

pub fn command(args: &[OsString]) -> Result<Command> {
    ensure!(!args.is_empty(), "supply a command after --");
    let mut command = Command::new(&args[0]);
    command.args(&args[1..]);
    Ok(command)
}
pub fn exit_code(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
    }
    #[cfg(not(unix))]
    {
        status.code().unwrap_or(1)
    }
}
pub fn run(args: &[OsString]) -> Result<i32> {
    let status = command(args)?
        .status()
        .context("could not start the held command")?;
    Ok(exit_code(status))
}

/// Give a child process group the terminal and forward signals sent to gridtop.
/// Keyboard signals reach a foreground child directly, avoiding duplicate Ctrl-C delivery.
pub fn supervised(
    args: &[OsString],
    signals: &crate::signals::Signals,
    on_spawn: impl FnOnce(u32),
) -> Result<i32> {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::{Pid, getpgrp, tcgetpgrp},
    };
    use std::{os::unix::process::CommandExt, thread, time::Duration};
    if let Some(signal) = signals.take() {
        return Ok(128 + signal as i32);
    }
    let stdin = std::io::stdin();
    let foreground = tcgetpgrp(&stdin).ok().filter(|p| *p == getpgrp());
    let terminal_mode = foreground.and_then(|_| nix::sys::termios::tcgetattr(&stdin).ok());
    let mut child = command(args)?
        .process_group(0)
        .spawn()
        .context("could not start the command")?;
    let pid = Pid::from_raw(i32::try_from(child.id())?);
    let _restore = Foreground {
        group: foreground,
        terminal_mode,
    };
    if foreground.is_some() {
        if let Err(e) = set_foreground(pid) {
            let _ = killpg(pid, Signal::SIGKILL);
            let _ = child.wait();
            return Err(e.context("could not hand terminal input to child"));
        }
        let _ = killpg(pid, Signal::SIGCONT);
    }
    on_spawn(child.id());
    loop {
        if let Some(signal) = signals.take() {
            match killpg(pid, signal) {
                Ok(()) | Err(nix::errno::Errno::ESRCH) => {}
                Err(e) => eprintln!("gridtop: could not forward {signal:?}: {e}"),
            }
        }
        if let Some(status) = child.try_wait().context("could not wait for child")? {
            return Ok(exit_code(status));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn set_foreground(pid: nix::unistd::Pid) -> Result<()> {
    use nix::sys::signal::{SigSet, SigmaskHow, Signal, pthread_sigmask};
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGTTOU);
    let mut old = SigSet::empty();
    pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&mask), Some(&mut old))?;
    let result = nix::unistd::tcsetpgrp(std::io::stdin(), pid);
    pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&old), None)?;
    result?;
    Ok(())
}
struct Foreground {
    group: Option<nix::unistd::Pid>,
    terminal_mode: Option<nix::sys::termios::Termios>,
}
impl Drop for Foreground {
    fn drop(&mut self) {
        if let Some(pid) = self.group {
            let _ = set_foreground(pid);
        }
        if let Some(mode) = &self.terminal_mode {
            let _ = nix::sys::termios::tcsetattr(
                std::io::stdin(),
                nix::sys::termios::SetArg::TCSANOW,
                mode,
            );
        }
    }
}
