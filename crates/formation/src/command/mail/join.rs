//! `formation mail join` argument mapping and Scribe seat discovery.

use crate::output;
use clap::Args;
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::io::Read;
use std::process::{Command as ProcessCommand, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// Register this seat, or rebind it to this session.
#[derive(Args)]
pub(super) struct Join {
    /// Seat name to register or rebind.
    name: Option<String>,
}

pub(super) fn run(mut env: Env, explicit_room: bool, args: Join) -> ExitCode {
    let name = match args.name {
        Some(name) => name,
        None => match env.scribe_join(explicit_room, run_scribe) {
            Ok((resolved, name)) => {
                env = resolved;
                name
            }
            Err(error) => return output::error(&error),
        },
    };
    mail::exec(&env, Command::Join(name), |result| match result {
        Ok(out) => output::mail_write(&output::mail_payload(out)),
        Err(error) => output::error(&error),
    })
}

fn run_scribe() -> Result<Vec<u8>, String> {
    let mut child = ProcessCommand::new("scribe")
        .args(["group", "list", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().ok_or("stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("stderr unavailable")?;
    let (sender, receiver) = std::sync::mpsc::channel();
    for (index, mut pipe) in [
        (0, Box::new(stdout) as Box<dyn Read + Send>),
        (1, Box::new(stderr) as Box<dyn Read + Send>),
    ] {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
            let _ = sender.send((index, result));
        });
    }
    drop(sender);
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timed out after 10 seconds".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    };
    let mut streams = [Vec::new(), Vec::new()];
    for _ in 0..2 {
        let (index, result) = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| "timed out reading scribe output".to_owned())?;
        streams[index] = result.map_err(|error| error.to_string())?;
    }
    if !status.success() {
        let message = String::from_utf8_lossy(&streams[1]);
        return Err(if message.trim().is_empty() {
            format!("scribe exited {status}")
        } else {
            message.into_owned()
        });
    }
    Ok(std::mem::take(&mut streams[0]))
}
