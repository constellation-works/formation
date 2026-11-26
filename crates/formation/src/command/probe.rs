//! Supervised shell probe execution and termination-signal handling.

use formation_core::app::watch::{ProbeLimits, ProbeOutput, ProbeRunner};
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command as ProcessCommand, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_STDOUT_BYTES: usize = 4 * 1024 * 1024;

const MAX_STDERR_BYTES: usize = 64 * 1024;

const POLL_SLICE: Duration = Duration::from_millis(20);

const KILL_GRACE: Duration = Duration::from_millis(250);

const DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Guard that installs SIGTERM/SIGINT handlers and restores prior dispositions on drop.
pub(super) struct SignalGuard {
    #[cfg(unix)]
    previous_term: Option<libc::sigaction>,
    #[cfg(unix)]
    previous_int: Option<libc::sigaction>,
}

impl SignalGuard {
    pub(super) fn install() -> Self {
        STOP_REQUESTED.store(false, Ordering::SeqCst);
        #[cfg(unix)]
        {
            Self {
                previous_term: install_signal(libc::SIGTERM),
                previous_int: install_signal(libc::SIGINT),
            }
        }
        #[cfg(not(unix))]
        {
            Self {}
        }
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(previous) = self.previous_term.take() {
                restore_signal(libc::SIGTERM, &previous);
            }
            if let Some(previous) = self.previous_int.take() {
                restore_signal(libc::SIGINT, &previous);
            }
        }
    }
}

/// Runs caller-supplied probe commands through `sh -c` in isolated process groups.
pub(super) struct ShellProbeRunner;

impl ProbeRunner for ShellProbeRunner {
    fn run(
        &self,
        command: &str,
        cwd: Option<&Path>,
        limits: ProbeLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> ProbeOutput {
        run_probe(command, cwd, limits, &|| self.stopped() || cancelled())
    }

    fn stopped(&self) -> bool {
        stop_requested()
    }
}

struct ChildGuard {
    child: Option<Child>,
    #[cfg(unix)]
    pgid: Option<libc::pid_t>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        #[cfg(unix)]
        let pgid = libc::pid_t::try_from(child.id())
            .ok()
            .filter(|pid| *pid > 0);
        Self {
            child: Some(child),
            #[cfg(unix)]
            pgid,
        }
    }

    #[cfg(unix)]
    fn signal_group(&self, signal: libc::c_int) {
        if let Some(pgid) = self.pgid {
            // SAFETY: `pgid > 0` and the group leader has not been reaped yet.
            unsafe {
                let _ = libc::kill(-pgid, signal);
            }
        }
    }

    #[cfg(unix)]
    fn group_alive(&self) -> bool {
        let Some(pgid) = self.pgid else {
            return false;
        };
        // SAFETY: `pgid > 0` and the group leader has not been reaped yet.
        let rc = unsafe { libc::kill(-pgid, 0) };
        if rc == 0 {
            return true;
        }
        io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }

    #[cfg(unix)]
    fn try_exited_unreaped(&self) -> io::Result<bool> {
        let Some(child) = self.child.as_ref() else {
            return Ok(true);
        };
        let pid = child.id();
        // SAFETY: `waitid` with `WNOWAIT` inspects exit state without reaping the leader.
        let (rc, signo) = unsafe {
            let mut info: libc::siginfo_t = std::mem::zeroed();
            let rc = libc::waitid(
                libc::P_PID,
                pid,
                &raw mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            );
            (rc, info.si_signo)
        };
        if rc == 0 {
            return Ok(signo != 0);
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        Err(error)
    }

    #[cfg(unix)]
    fn sweep_and_reap(&mut self) -> io::Result<ExitStatus> {
        // Sweep surviving descendants while the leader is still unreaped, then reap the leader.
        self.signal_group(libc::SIGKILL);
        self.pgid = None;
        let Some(mut child) = self.child.take() else {
            return Err(io::Error::other("child already reaped"));
        };
        child.wait()
    }

    #[cfg(not(unix))]
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let Some(child) = self.child.as_mut() else {
            return Err(io::Error::other("child already reaped"));
        };
        match child.try_wait()? {
            Some(status) => {
                self.child = None;
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }

    fn terminate_and_reap(&mut self) {
        #[cfg(unix)]
        {
            if self.child.is_none() {
                return;
            }
            self.signal_group(libc::SIGTERM);
            let grace_end = Instant::now() + KILL_GRACE;
            while Instant::now() < grace_end && self.group_alive() {
                let remaining = grace_end.saturating_duration_since(Instant::now());
                std::thread::sleep(remaining.min(Duration::from_millis(10)));
            }
            if self.group_alive() {
                self.signal_group(libc::SIGKILL);
            }
            self.pgid = None;
            if let Some(mut child) = self.child.take() {
                let _ = child.wait();
            }
        }
        #[cfg(not(unix))]
        {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                self.signal_group(libc::SIGKILL);
                self.pgid = None;
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

#[cfg(unix)]
extern "C" fn handle_stop_signal(_signal: libc::c_int) {
    STOP_REQUESTED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
fn install_signal(signal: libc::c_int) -> Option<libc::sigaction> {
    // SAFETY: `sigaction` with a plain C handler writing only an atomic bool is async-signal-safe.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handle_stop_signal as *const () as usize;
        libc::sigemptyset(&raw mut action.sa_mask);
        action.sa_flags = 0;
        let mut previous: libc::sigaction = std::mem::zeroed();
        if libc::sigaction(signal, &raw const action, &raw mut previous) == 0 {
            Some(previous)
        } else {
            None
        }
    }
}

#[cfg(unix)]
fn restore_signal(signal: libc::c_int, previous: &libc::sigaction) {
    // SAFETY: restoring a previously retrieved `sigaction` value is valid.
    unsafe {
        let _ = libc::sigaction(signal, previous, std::ptr::null_mut());
    }
}

pub(super) fn stop_requested() -> bool {
    STOP_REQUESTED.load(Ordering::SeqCst)
}

fn run_probe(
    command: &str,
    cwd: Option<&Path>,
    limits: ProbeLimits,
    cancelled: &dyn Fn() -> bool,
) -> ProbeOutput {
    let mut process = ProcessCommand::new("sh");
    process.arg("-c").arg(command);
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    run_process(process, limits, cancelled)
}

pub(super) fn run_process(
    mut process: ProcessCommand,
    limits: ProbeLimits,
    cancelled: &dyn Fn() -> bool,
) -> ProbeOutput {
    if cancelled() {
        return ProbeOutput::Cancelled;
    }
    process
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        process.process_group(0);
    }

    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => return ProbeOutput::Failed(format!("failed to start probe: {error}")),
    };

    let Some(stdout) = child.stdout.take() else {
        let mut guard = ChildGuard::new(child);
        guard.terminate_and_reap();
        return ProbeOutput::Failed("probe stdout unavailable".to_owned());
    };
    let Some(stderr) = child.stderr.take() else {
        let mut guard = ChildGuard::new(child);
        guard.terminate_and_reap();
        return ProbeOutput::Failed("probe stderr unavailable".to_owned());
    };

    let mut guard = ChildGuard::new(child);
    let overflowed = Arc::new(AtomicBool::new(false));
    // Bounded channel (capacity 2, one message per pipe); sends complete once EOF or cap is hit.
    let (sender, receiver) = mpsc::sync_channel::<(usize, io::Result<Vec<u8>>)>(2);

    spawn_reader(
        stdout,
        0,
        MAX_STDOUT_BYTES,
        Some(Arc::clone(&overflowed)),
        sender.clone(),
    );
    spawn_reader(stderr, 1, MAX_STDERR_BYTES, None, sender);

    let start = Instant::now();
    let probe_deadline = start + limits.probe_timeout;
    let command_deadline = limits
        .command_remaining
        .and_then(|remaining| start.checked_add(remaining));

    let status = loop {
        if cancelled() {
            guard.terminate_and_reap();
            drain_readers(&receiver);
            return ProbeOutput::Cancelled;
        }
        if overflowed.load(Ordering::SeqCst) {
            guard.terminate_and_reap();
            drain_readers(&receiver);
            return ProbeOutput::Failed(format!("probe output exceeded {MAX_STDOUT_BYTES} bytes"));
        }

        match wait_step(&mut guard) {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                guard.terminate_and_reap();
                drain_readers(&receiver);
                return ProbeOutput::Failed(error.to_string());
            }
        }

        let now = Instant::now();
        if now >= probe_deadline {
            guard.terminate_and_reap();
            drain_readers(&receiver);
            return ProbeOutput::Failed(format!(
                "probe timed out after {} seconds",
                limits.probe_timeout.as_secs()
            ));
        }
        if command_deadline.is_some_and(|limit| now >= limit) {
            guard.terminate_and_reap();
            drain_readers(&receiver);
            return ProbeOutput::Cancelled;
        }

        let until_probe = probe_deadline.saturating_duration_since(now);
        let until_limit = match command_deadline {
            Some(limit) => until_probe.min(limit.saturating_duration_since(now)),
            None => until_probe,
        };
        std::thread::sleep(POLL_SLICE.min(until_limit));
    };

    let streams = match collect_readers(&receiver) {
        Ok(streams) => streams,
        Err(message) => return ProbeOutput::Failed(message),
    };

    if overflowed.load(Ordering::SeqCst) {
        return ProbeOutput::Failed(format!("probe output exceeded {MAX_STDOUT_BYTES} bytes"));
    }

    if status.success() {
        return ProbeOutput::Success {
            stdout: streams[0].clone(),
        };
    }

    ProbeOutput::Failed(failure_message(&streams[1], &streams[0], status))
}

fn wait_step(guard: &mut ChildGuard) -> io::Result<Option<ExitStatus>> {
    #[cfg(unix)]
    {
        if guard.try_exited_unreaped()? {
            return guard.sweep_and_reap().map(Some);
        }
        Ok(None)
    }
    #[cfg(not(unix))]
    {
        guard.try_wait()
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    mut pipe: R,
    index: usize,
    limit: usize,
    overflow_flag: Option<Arc<AtomicBool>>,
    sender: mpsc::SyncSender<(usize, io::Result<Vec<u8>>)>,
) {
    std::thread::spawn(move || {
        let result = read_capped(&mut pipe, limit, overflow_flag.as_deref());
        let _ = sender.send((index, result));
    });
}

fn read_capped(
    pipe: &mut dyn Read,
    limit: usize,
    overflow_flag: Option<&AtomicBool>,
) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut chunk = [0_u8; 8192];

    loop {
        let count = pipe.read(&mut chunk)?;
        if count == 0 {
            return Ok(output);
        }
        let remaining = limit.saturating_sub(output.len());
        if count <= remaining {
            output.extend_from_slice(&chunk[..count]);
        } else {
            output.extend_from_slice(&chunk[..remaining]);
            if let Some(flag) = overflow_flag {
                flag.store(true, Ordering::SeqCst);
                return Ok(output);
            }
        }
    }
}

fn collect_readers(
    receiver: &mpsc::Receiver<(usize, io::Result<Vec<u8>>)>,
) -> Result<[Vec<u8>; 2], String> {
    let deadline = Instant::now() + DRAIN_TIMEOUT;
    let mut streams = [Vec::new(), Vec::new()];

    for _ in 0..2 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let (index, result) = receiver
            .recv_timeout(remaining)
            .map_err(|_| "timed out draining probe output".to_owned())?;
        streams[index] = result.map_err(|error| error.to_string())?;
    }

    Ok(streams)
}

fn drain_readers(receiver: &mpsc::Receiver<(usize, io::Result<Vec<u8>>)>) {
    let deadline = Instant::now() + DRAIN_TIMEOUT;
    for _ in 0..2 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if receiver.recv_timeout(remaining).is_err() {
            break;
        }
    }
}

fn failure_message(stderr: &[u8], stdout: &[u8], status: ExitStatus) -> String {
    if let Some(line) = first_non_empty_line(stderr) {
        return line;
    }
    if let Some(line) = first_non_empty_line(stdout) {
        return line;
    }
    if let Some(code) = status.code() {
        return format!("probe exited with status {code}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!("probe terminated by signal {signal}");
        }
    }
    "probe failed".to_owned()
}

fn first_non_empty_line(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim_end_matches('\r').to_owned())
}
