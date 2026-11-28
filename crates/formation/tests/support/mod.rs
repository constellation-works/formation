//! Own and reap processes started by CLI test fixtures.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

pub struct SpawnedChild(Option<Child>);

impl SpawnedChild {
    pub fn spawn(cmd: &mut Command) -> Self {
        let child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        Self(Some(child))
    }

    pub fn pid(&self) -> u32 {
        self.0.as_ref().unwrap().id()
    }

    #[cfg(unix)]
    pub fn terminate(&self) -> std::io::Result<()> {
        let pid = libc::pid_t::try_from(self.pid())
            .ok()
            .filter(|pid| *pid > 0)
            .ok_or_else(|| std::io::Error::other("invalid child PID"))?;
        // SAFETY: The fixture spawned the child and has not reaped it.
        if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn until<T>(&mut self, timeout: Duration, mut condition: impl FnMut() -> Option<T>) -> T {
        let deadline = Instant::now() + timeout;
        loop {
            let status = self.0.as_mut().unwrap().try_wait().unwrap();
            assert!(status.is_none(), "child exited while waiting: {status:?}");
            if let Some(value) = condition() {
                return value;
            }
            assert!(
                Instant::now() < deadline,
                "condition not met within {timeout:?}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn kill(&mut self) -> std::io::Result<()> {
        self.0.as_mut().unwrap().kill()
    }

    pub fn wait_within(mut self, timeout: Duration) -> Output {
        let start = Instant::now();
        loop {
            if self.0.as_mut().unwrap().try_wait().unwrap().is_some() {
                return self.0.take().unwrap().wait_with_output().unwrap();
            }
            assert!(
                start.elapsed() < timeout,
                "child did not exit within {timeout:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for SpawnedChild {
    fn drop(&mut self) {
        if self.0.is_some() {
            #[cfg(unix)]
            if matches!(
                self.0.as_mut().and_then(|child| child.try_wait().ok()),
                Some(None)
            ) {
                let _ = self.terminate();
                let child = self.0.as_mut().unwrap();
                let deadline = Instant::now() + Duration::from_secs(3);
                while Instant::now() < deadline {
                    if matches!(child.try_wait(), Ok(Some(_))) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            let _ = self.kill();
            if let Some(mut child) = self.0.take() {
                let _ = child.wait();
            }
        }
    }
}

/// A command's result when its stdout was a real terminal.
#[cfg(unix)]
#[allow(dead_code, reason = "test crates that never use a terminal skip this")]
pub struct OnTerminal {
    pub status: ExitStatus,
    /// What the command wrote to the terminal, with line endings as `\n`.
    pub stdout: String,
    pub stderr: String,
}

/// Run `cmd` with stdout on a pseudo-terminal, so `is_terminal` is true, and stderr piped.
#[cfg(unix)]
#[allow(dead_code, reason = "test crates that never use a terminal skip this")]
pub fn on_terminal(mut cmd: Command) -> OnTerminal {
    use std::io::Read;
    use std::os::fd::{FromRawFd, OwnedFd};
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: both out-pointers are valid and the optional arguments may be null.
    let opened = unsafe {
        libc::openpty(
            &raw mut master,
            &raw mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(opened, 0, "{}", std::io::Error::last_os_error());
    // SAFETY: `openpty` returned two open descriptors that nothing else owns.
    let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::from(slave))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The command still holds the child's end of the terminal. Dropping it lets the read below end.
    drop(cmd);
    let mut terminal = std::fs::File::from(master);
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match terminal.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
        }
    }
    let output = child.wait_with_output().unwrap();
    OnTerminal {
        status: output.status,
        stdout: String::from_utf8(bytes).unwrap().replace("\r\n", "\n"),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}
