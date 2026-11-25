//! Local mail CLI and private credential files.
use clap::{Args, Subcommand};
use formation_core::mail;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Args)]
pub(super) struct Mail {
    /// Existing private directory shared by this shift (mode 0700).
    #[arg(long, env = "FORMATION_ROOM")]
    room: PathBuf,
    /// Private seat credential file. Join creates it if absent.
    #[arg(long, env = "FORMATION_IDENTITY")]
    identity: PathBuf,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Register this seat or refresh its presence.
    Join {
        /// Unique seat name for this identity.
        name: String,
    },
    /// List seats seen in the last 90 seconds.
    Who,
    /// Send to an online seat. Reuse the ID when retrying.
    Send {
        /// Online recipient seat name.
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

fn credential(path: &Path, create: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    if create && !path.try_exists()? {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut key = [0u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut key)?;
        let mut f = tempfile::NamedTempFile::new_in(parent)?;
        f.write_all(&key)?;
        f.as_file().sync_all()?;
        match f.persist_noclobber(path) {
            Ok(_) => File::open(parent)?.sync_all()?,
            Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }
    }
    let meta = path.symlink_metadata()?;
    if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 || meta.len() != 32 {
        return Err("identity must be a private 32-byte regular file".into());
    }
    let mut key = [0; 32];
    File::open(path)?.read_exact(&mut key)?;
    Ok(Sha256::digest(key).to_vec())
}

impl Mail {
    pub(super) fn run(self) -> Result<Value, Box<dyn Error>> {
        let meta = fs::symlink_metadata(&self.room)?;
        if !meta.is_dir() || meta.permissions().mode() & 0o077 != 0 {
            return Err("room must be a private directory (mode 0700)".into());
        }
        if let Action::Join { name } = &self.action {
            mail::check_name(name)?;
        }
        let key = credential(&self.identity, matches!(self.action, Action::Join { .. }))?;
        let mut room = mail::Room::open(&self.room.join("inbox.sqlite3"))?;
        let result = match self.action {
            Action::Join { name } => {
                room.join(&name, &key)?;
                json!({ "seat": name })
            }
            Action::Who => {
                room.heartbeat(&key)?;
                json!({ "online": room.who()? })
            }
            Action::Send { to, id, body } => {
                let sender = room.heartbeat(&key)?;
                let receipt = mail::send(&mut room, &sender, &to, &id, &body)?;
                json!({ "receipt": receipt, "id": id })
            }
            Action::Inbox { wait } => {
                let messages = mail::wait(&room, &key, Duration::from_secs(wait))?;
                json!({ "messages": messages })
            }
            Action::Ack { receipt } => {
                let recipient = room.heartbeat(&key)?;
                room.ack(&recipient, receipt)?;
                json!({ "acknowledged": receipt })
            }
        };
        Ok(result)
    }
}
