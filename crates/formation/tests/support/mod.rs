//! Own and reap processes started by CLI test fixtures.
#![allow(clippy::unwrap_used, reason = "fixture failures should panic")]

use std::process::{Child, Command, Output, Stdio};
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
