//! A watch in the room's watch set.

/// Whether the reader runs a watch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchState {
    /// The reader runs the watch when it is due.
    On,
    /// The watch is kept but not run.
    Paused,
}

impl WatchState {
    /// Parse a stored token.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "on" => Some(Self::On),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }

    /// Stable token stored in the room and printed in listings.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Paused => "paused",
        }
    }
}

/// A watch definition as a seat adds it.
pub struct NewWatch<'a> {
    /// Joined seat that is mailed.
    pub recipient: &'a str,
    /// Name, unique per recipient.
    pub name: &'a str,
    /// Command run through `sh -c`.
    pub command: &'a str,
    /// Working directory the watch was added from.
    pub cwd: &'a str,
    /// Seconds between reads.
    pub every: i64,
    /// Seat that set the watch.
    pub added_by: &'a str,
    /// Unix seconds when it was set.
    pub added_at: i64,
}

/// A stored watch, with what the reader last recorded about it.
pub struct WatchEntry {
    /// Joined seat that is mailed.
    pub recipient: String,
    /// Name, unique per recipient.
    pub name: String,
    /// Command run through `sh -c`.
    pub command: String,
    /// Working directory the watch was added from.
    pub cwd: String,
    /// Seconds between reads.
    pub every: i64,
    /// Whether the reader runs it.
    pub state: WatchState,
    /// Seat that set the watch.
    pub added_by: String,
    /// Unix seconds when it was set.
    pub added_at: i64,
    /// Unix seconds of the reader's last read, if it has read the watch.
    pub last_read: Option<i64>,
    /// What the reader's last read found, if it has read the watch.
    pub last_result: Option<String>,
    /// Fingerprints remembered for the watch.
    pub remembered: i64,
    /// Version of the definition used to fence in-flight results.
    pub revision: i64,
}

/// One invocation of the room reader.
pub struct WatchParticipant {
    /// Room-assigned invocation identity, independent of PID reuse.
    pub id: i64,
    /// Seat that started the invocation.
    pub seat: String,
    /// Process ID for display.
    pub pid: u32,
    /// Unix seconds of the last beat.
    pub heartbeat: i64,
    /// Unix seconds when this invocation's registration expires.
    pub expires_at: i64,
}

/// The room's reader record and its live standbys.
#[derive(Default)]
pub struct WatchLeaseStatus {
    /// The lease holder, which may have expired.
    pub reader: Option<WatchParticipant>,
    /// Live invocations eligible for takeover.
    pub standbys: Vec<WatchParticipant>,
    /// Unix seconds of a deliberate stop, if stopped.
    pub stopped_at: Option<i64>,
    /// Unix seconds of a graceful reader exit, if no reader has taken over.
    pub released_at: Option<i64>,
}

/// Role held by an active reader invocation.
pub enum WatchRole {
    /// Holds the room's lease.
    Reader,
    /// Eligible to claim the lease when it is free.
    Standby,
}

/// One remembered watch fingerprint and how many consecutive complete reads omitted it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchHandover {
    /// Item fingerprint remembered for a `(seat, probe)` watch key.
    pub fingerprint: String,
    /// Consecutive successful complete reads that omitted this fingerprint.
    pub absent_reads: u32,
}
