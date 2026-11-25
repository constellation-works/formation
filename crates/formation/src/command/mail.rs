//! Convert clap arguments to application commands.

use crate::output;
use clap::{Args, Subcommand};
use formation_core::Env;
use formation_core::app::mail;
use formation_core::app::mail::Command;
use std::io::Read;
use std::process::{Command as ProcessCommand, ExitCode, Stdio};
use std::time::{Duration, Instant};

#[derive(Args)]
pub(crate) struct Mail {
    /// Room name or directory path (default: $FORMATION_ROOM or session file).
    #[arg(long, global = true, value_name = "NAME|PATH")]
    room: Option<String>,
    /// Seat name for this command (default: seat recorded for this session).
    #[arg(long = "as", global = true, value_name = "SEAT")]
    seat: Option<String>,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Register this seat, or rebind it to this session.
    Join {
        /// Seat name to register or rebind.
        name: Option<String>,
    },
    /// List every joined seat, not only those seen lately: unread, held, seen.
    Who,
    /// Send to a joined seat, present or not. Reuse the ID when retrying.
    Send {
        /// Joined recipient seat name.
        to: String,
        /// Caller-generated message identifier.
        #[arg(long)]
        id: String,
        /// Message text (up to 8192 bytes).
        #[arg(long)]
        body: String,
    },
    /// Read unread messages or wait for up to 900 seconds.
    Inbox {
        /// Seconds to wait for the first unread message.
        #[arg(long, default_value_t=0, value_parser=clap::value_parser!(u64).range(0..=900))]
        wait: u64,
    },
    /// Mark a receipt in this seat's inbox as handled.
    Ack {
        /// Numeric receipt from the inbox.
        receipt: i64,
    },
}

pub(crate) fn run(env: Env, args: Mail) -> ExitCode {
    let explicit_room = args.room.is_some();
    let mut env = env.with_mail_options(args.room, args.seat);
    let cmd = match args.action {
        Action::Join { name } => {
            let name = match name {
                Some(name) => name,
                None => match env.scribe_join(explicit_room, run_scribe) {
                    Ok((resolved, name)) => {
                        env = resolved;
                        name
                    }
                    Err(error) => return output::error(&error),
                },
            };
            Command::Join(name)
        }
        Action::Who => Command::Who,
        Action::Send { to, id, body } => Command::Send { to, id, body },
        Action::Inbox { wait } => Command::Inbox(Duration::from_secs(wait)),
        Action::Ack { receipt } => Command::Ack(receipt),
    };
    mail::exec(&env, cmd, |result| match result {
        Ok(out) => output::mail(&output::payload::mail(out)),
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
