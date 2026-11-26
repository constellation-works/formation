//! Run the room's watch set while holding its reader lease.

use super::poll::{ProbeVerdict, send_changes};
use super::{ProbeLimits, ProbeRunner, rule, set};
use crate::Env;
use crate::runtime::{self, App, Ctx, RoomAccess};
use cw_base::{Error, Kind};
use formation_store::Store;
use formation_store::contract::{WatchEntry, WatchRole, WatchState};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::Scope;
use std::time::{Duration, Instant};

const HEARTBEAT: Duration = Duration::from_secs(15);

const LEASE_SECONDS: i64 = 45;

const PASS_INTERVAL: Duration = Duration::from_secs(1);

/// Start the reader or wait as an eligible standby.
pub struct Command<'a> {
    /// The display PID of this invocation.
    pub pid: u32,
    /// Host probe runner and signal observer.
    pub runner: &'a (dyn ProbeRunner + Sync),
    /// Queue a fixed wake for an idle Codex session.
    pub queue: &'a dyn Fn(&str) -> bool,
}

/// A reader invocation exited.
pub struct Outcome {
    /// Room whose reader or standby exited.
    pub room: String,
}

struct ReaderApp<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> App for ReaderApp<'a> {
    type Command = Command<'a>;

    type Outcome = Outcome;

    fn room_access<'cmd>(_: &'cmd Command<'a>) -> RoomAccess<'cmd> {
        RoomAccess::Existing
    }

    fn audit(_: &Command<'a>) -> Option<&'static str> {
        None
    }

    fn run(ctx: &Ctx<'_>, cmd: Command<'a>) -> Result<Outcome, Error> {
        let mail = ctx.mail()?;
        let caller = super::caller_ref(&mail.caller)?;
        let clock = ctx.clock;
        let store = &mail.store;
        let (seat, id, role) = store.write(|tx| {
            let seat = set::caller_in(tx, caller)?;
            let now = clock().unix_timestamp();
            let (id, role) =
                tx.watch_reader()
                    .register(&seat, cmd.pid, now, now + LEASE_SECONDS)?;
            Ok((seat, id, role))
        })?;

        let result = std::thread::scope(|scope| {
            let mut reader = Reader {
                store,
                seat,
                id,
                ctx,
                queue: cmd.queue,
                runner: cmd.runner,
                jobs: HashMap::new(),
                role,
            };
            reader.read_set(scope)
        });
        // Release only after cancelling and reaping the probes from this invocation.
        let leave = store.write(|tx| tx.watch_reader().leave(id, clock().unix_timestamp()));
        result?;
        leave?;
        Ok(Outcome {
            room: mail.room.clone(),
        })
    }
}

/// Run the leased reader and hand its typed outcome to the CLI.
pub fn exec<R>(env: &Env, cmd: Command<'_>, finish: impl FnOnce(Result<Outcome, Error>) -> R) -> R {
    runtime::exec::<ReaderApp<'_>, R>(env, cmd, finish)
}

type WatchKey = (String, String);

type Completed = (WatchEntry, Option<ProbeVerdict>);

struct Job {
    revision: i64,
    cancelled: Arc<AtomicBool>,
    result: Receiver<Completed>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

struct Reader<'a> {
    store: &'a Store,
    seat: String,
    id: i64,
    ctx: &'a Ctx<'a>,
    queue: &'a dyn Fn(&str) -> bool,
    runner: &'a (dyn ProbeRunner + Sync),
    jobs: HashMap<WatchKey, Job>,
    role: WatchRole,
}

impl<'a> Reader<'a> {
    fn read_set<'scope>(&mut self, scope: &'scope Scope<'scope, 'a>) -> Result<(), Error> {
        let mut next_beat = Instant::now() + HEARTBEAT;
        while !self.runner.stopped() {
            let pass_start = Instant::now();
            if !self.pulse(&mut next_beat)? {
                break;
            }

            if matches!(self.role, WatchRole::Reader) {
                if !self.commit_completed(&mut next_beat)? {
                    break;
                }
                let snapshot = self.store.read()?;
                let now = (self.ctx.clock)().unix_timestamp();
                if !snapshot.watch().holds_lease(self.id, now)? {
                    break;
                }
                let watches = snapshot.watch().list()?;
                drop(snapshot);
                self.schedule(scope, watches, now)?;
            }

            let next_pass = (pass_start + PASS_INTERVAL).min(next_beat);
            std::thread::sleep(next_pass.saturating_duration_since(Instant::now()));
        }
        Ok(())
    }

    fn pulse(&mut self, next_beat: &mut Instant) -> Result<bool, Error> {
        let start = Instant::now();
        let beat_due = start >= *next_beat;
        if !beat_due && matches!(self.role, WatchRole::Reader) {
            return Ok(true);
        }
        let role = self.store.write(|tx| {
            let now = (self.ctx.clock)().unix_timestamp();
            let renewal = beat_due.then_some(now + LEASE_SECONDS);
            tx.watch_reader().pulse(self.id, now, renewal)
        })?;
        let Some(role) = role else {
            return Ok(false);
        };
        self.role = role;
        if beat_due {
            *next_beat = start + HEARTBEAT;
        }
        Ok(true)
    }

    fn commit_completed(&mut self, next_beat: &mut Instant) -> Result<bool, Error> {
        let mut results = Vec::new();
        self.jobs.retain(|_, job| match job.result.try_recv() {
            Ok(result) => {
                results.push(result);
                false
            }
            Err(TryRecvError::Disconnected) => false,
            Err(TryRecvError::Empty) => true,
        });
        for (watch, verdict) in results {
            if let Some(verdict) = verdict
                && !self.runner.stopped()
            {
                self.commit(&watch, verdict)?;
                // A batch can contain multiple bounded queue calls. Keep the lease
                // renewed between them.
                if !self.pulse(next_beat)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn schedule<'scope>(
        &mut self,
        scope: &'scope Scope<'scope, 'a>,
        watches: Vec<WatchEntry>,
        now: i64,
    ) -> Result<(), Error> {
        let mut current = HashMap::new();
        for watch in watches
            .into_iter()
            .filter(|watch| watch.state == WatchState::On)
        {
            let key = (watch.recipient.clone(), watch.name.clone());
            current.insert(key.clone(), watch.revision);
            if self.jobs.contains_key(&key)
                || watch
                    .last_read
                    .is_some_and(|last| now < last.saturating_add(watch.every))
            {
                continue;
            }

            let cancelled = Arc::new(AtomicBool::new(false));
            let worker_cancelled = Arc::clone(&cancelled);
            let runner = self.runner;
            let revision = watch.revision;
            // A worker sends one result. The single slot lets it finish before the next pass.
            let (sender, result) = mpsc::sync_channel(1);
            std::thread::Builder::new()
                .spawn_scoped(scope, move || {
                    let limits = ProbeLimits {
                        probe_timeout: Duration::from_secs(rule::PROBE_TIMEOUT_SECONDS),
                        command_remaining: None,
                    };
                    let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        runner.run(&watch.command, Some(Path::new(&watch.cwd)), limits, &|| {
                            worker_cancelled.load(Ordering::Relaxed)
                        })
                    }))
                    .unwrap_or_else(|_| super::ProbeOutput::Failed("probe worker panicked".into()));
                    let _ = sender.send((watch, ProbeVerdict::from_output(output)));
                })
                .map_err(|error| Error::new(Kind::Io, "probe_worker", error))?;
            self.jobs.insert(
                key,
                Job {
                    revision,
                    cancelled,
                    result,
                },
            );
        }
        for (key, job) in &self.jobs {
            if current.get(key) != Some(&job.revision) {
                job.cancelled.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    fn commit(&self, watch: &WatchEntry, verdict: ProbeVerdict) -> Result<(), Error> {
        let inserted = self.store.write(|tx| {
            let now = (self.ctx.clock)().unix_timestamp();
            if tx.watch_reader().holds_lease(self.id, now)?
                && tx.watch().record_read(watch, now, &verdict.summary())?
            {
                return send_changes(
                    tx,
                    &self.seat,
                    &watch.recipient,
                    &[(&watch.name, verdict)],
                    now,
                );
            }
            Ok(false)
        })?;
        if inserted {
            crate::app::wake::after_commit(self.ctx, &watch.recipient, self.queue);
        }
        Ok(())
    }
}
