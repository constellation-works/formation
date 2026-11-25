//! Polling loop, transactional handover deduplication, and mail delivery for `formation watch`.

use super::{
    Command, Ctx, Error, Failure, Item, Outcome, ProbeLimits, ProbeOutput, ProbeRunner, SeatRef,
    Store, refused, rule,
};
use std::time::{Duration, Instant};

const LISTED_SEATS: usize = 32;

const SLEEP_SLICE: Duration = Duration::from_millis(50);

enum ProbeVerdict {
    Ok(Vec<rule::ParsedLine>),
    Failed(String),
}

enum MailEntry {
    Item(Item),
    Failure(Failure),
}

impl MailEntry {
    fn line(&self) -> String {
        match self {
            Self::Item(item) => rule::mail_line(&item.probe, &item.text),
            Self::Failure(failure) => rule::failed_mail_line(&failure.probe, &failure.error),
        }
    }
}

struct MailedTotals {
    envelopes: usize,
    items: usize,
    failed: usize,
}

pub(super) fn run(
    ctx: &Ctx<'_>,
    store: &Store,
    caller: SeatRef<'_>,
    cmd: Command<'_>,
) -> Result<Outcome, Error> {
    let probes = rule::parse_probes(&cmd.probes)?;
    let caller_seat = resolve_seats(store, caller, cmd.to.as_deref())?;
    let target_seat = cmd.to.as_deref().unwrap_or(&caller_seat);
    let effective_timeout = match cmd.to {
        Some(_) => cmd.timeout,
        None => Some(
            cmd.timeout
                .unwrap_or_else(|| Duration::from_secs(rule::DEFAULT_WAIT_TIMEOUT_SECONDS)),
        ),
    };

    let start = Instant::now();
    let deadline = effective_timeout
        .filter(|limit| !limit.is_zero())
        .and_then(|limit| start.checked_add(limit));
    let mut mailed = MailedTotals {
        envelopes: 0,
        items: 0,
        failed: 0,
    };

    loop {
        if cmd.runner.stopped() {
            return Ok(finish_outcome(cmd.to, mailed, false));
        }

        let cycle_start = Instant::now();
        let limits = ProbeLimits {
            probe_timeout: Duration::from_secs(rule::PROBE_TIMEOUT_SECONDS),
            command_remaining: deadline
                .map(|limit| limit.saturating_duration_since(Instant::now())),
        };
        let Some(verdicts) = run_probes(&probes, cmd.runner, limits) else {
            let expired = !cmd.runner.stopped() && is_expired(start, effective_timeout);
            return Ok(finish_outcome(cmd.to, mailed, expired));
        };

        let now = (ctx.clock)().unix_timestamp();
        if let Some(recipient) = cmd.to.as_deref() {
            let batch = commit_mailed(store, &caller_seat, recipient, &verdicts, now)?;
            mailed.envelopes += batch.envelopes;
            mailed.items += batch.items;
            mailed.failed += batch.failed;
        } else {
            let (items, failed, remaining) =
                commit_report(store, &caller_seat, target_seat, &verdicts, now)?;
            if !items.is_empty() || !failed.is_empty() {
                return Ok(Outcome::Report {
                    items,
                    failed,
                    remaining,
                    expired: false,
                });
            }
        }

        if cmd.runner.stopped() {
            return Ok(finish_outcome(cmd.to, mailed, false));
        }
        if is_expired(start, effective_timeout) {
            return Ok(finish_outcome(cmd.to, mailed, true));
        }

        let next_wait = compute_wait(cycle_start, cmd.every, start, effective_timeout);
        sleep_interruptible(next_wait, cmd.runner);

        if cmd.runner.stopped() {
            return Ok(finish_outcome(cmd.to, mailed, false));
        }
        if is_expired(start, effective_timeout) {
            return Ok(finish_outcome(cmd.to, mailed, true));
        }
    }
}

fn finish_outcome(to: Option<String>, mailed: MailedTotals, expired: bool) -> Outcome {
    match to {
        Some(recipient) => Outcome::Mailed {
            to: recipient,
            envelopes: mailed.envelopes,
            items: mailed.items,
            failed: mailed.failed,
            expired,
        },
        None => Outcome::Report {
            items: Vec::new(),
            failed: Vec::new(),
            remaining: 0,
            expired,
        },
    }
}

fn is_expired(start: Instant, timeout: Option<Duration>) -> bool {
    timeout.is_some_and(|limit| start.elapsed() >= limit)
}

fn compute_wait(
    cycle_start: Instant,
    every: Duration,
    start: Instant,
    timeout: Option<Duration>,
) -> Duration {
    let until_next = every.saturating_sub(cycle_start.elapsed());
    match timeout {
        Some(limit) => until_next.min(limit.saturating_sub(start.elapsed())),
        None => until_next,
    }
}

fn sleep_interruptible(duration: Duration, runner: &dyn ProbeRunner) {
    let until = Instant::now();
    while !runner.stopped() {
        let elapsed = until.elapsed();
        if elapsed >= duration {
            break;
        }
        let step = duration.saturating_sub(elapsed).min(SLEEP_SLICE);
        std::thread::sleep(step);
    }
}

fn resolve_seats(store: &Store, caller: SeatRef<'_>, to: Option<&str>) -> Result<String, Error> {
    let snapshot = store.read()?;
    let seats = snapshot.seats();

    let caller_seat = match caller {
        SeatRef::Name(name) => {
            if seats.exists(name)? {
                name.to_owned()
            } else {
                return Err(refused(
                    "unknown_seat",
                    "seat has not joined this room; run `formation mail join` first",
                ));
            }
        }
        SeatRef::Session(session) => seats.by_session(session)?.ok_or_else(|| {
            refused(
                "seat_required",
                "seat required: pass --as <seat> or join this room from your session",
            )
        })?,
    };

    if let Some(recipient) = to
        && !seats.exists(recipient)?
    {
        return Err(unknown_recipient(seats.names()?, recipient));
    }

    Ok(caller_seat)
}

fn run_probes<'a>(
    probes: &'a [rule::ProbeSpec],
    runner: &(dyn ProbeRunner + Sync),
    limits: ProbeLimits,
) -> Option<Vec<(&'a str, ProbeVerdict)>> {
    let outputs: Vec<ProbeOutput> = if probes.len() == 1 {
        vec![runner.run(&probes[0].command, limits)]
    } else {
        std::thread::scope(|scope| {
            let handles: Vec<_> = probes
                .iter()
                .map(|probe| scope.spawn(move || runner.run(&probe.command, limits)))
                .collect();
            handles
                .into_iter()
                .map(|handle| match handle.join() {
                    Ok(output) => output,
                    Err(_) => ProbeOutput::Failed("probe worker panicked".to_owned()),
                })
                .collect()
        })
    };

    let mut verdicts = Vec::with_capacity(probes.len());
    for (probe, output) in probes.iter().zip(outputs) {
        let verdict = match output {
            ProbeOutput::Cancelled => return None,
            ProbeOutput::Failed(message) => ProbeVerdict::Failed(rule::first_line(&message)),
            ProbeOutput::Success { stdout } => match rule::parse_output(&stdout) {
                Ok(lines) => ProbeVerdict::Ok(lines),
                Err(message) => ProbeVerdict::Failed(rule::first_line(&message)),
            },
        };
        verdicts.push((probe.name.as_str(), verdict));
    }

    Some(verdicts)
}

fn commit_report(
    store: &Store,
    caller_seat: &str,
    target_seat: &str,
    verdicts: &[(&str, ProbeVerdict)],
    now: i64,
) -> Result<(Vec<Item>, Vec<Failure>, usize), Error> {
    store.write(|tx| {
        if tx.seats().seen(caller_seat)?.is_none() {
            return Err(refused(
                "unknown_seat",
                "seat has not joined this room; run `formation mail join` first",
            ));
        }

        let (mut new_items, new_failures) = collect_changes(tx, target_seat, verdicts, now)?;
        let total_new = new_items.len();
        let remaining = total_new.saturating_sub(rule::MAX_BATCH_ITEMS);
        new_items.truncate(rule::MAX_BATCH_ITEMS);

        for item in &new_items {
            tx.watch()
                .mark_handed(target_seat, &item.probe, &item.fingerprint, now)?;
        }

        Ok((new_items, new_failures, remaining))
    })
}

fn commit_mailed(
    store: &Store,
    caller_seat: &str,
    recipient: &str,
    verdicts: &[(&str, ProbeVerdict)],
    now: i64,
) -> Result<MailedTotals, Error> {
    store.write(|tx| {
        if tx.seats().seen(caller_seat)?.is_none() {
            return Err(refused(
                "unknown_seat",
                "seat has not joined this room; run `formation mail join` first",
            ));
        }
        if tx.seats().seen(recipient)?.is_none() {
            return Err(unknown_recipient(
                tx.seats().names(LISTED_SEATS + 1)?,
                recipient,
            ));
        }

        let (new_items, new_failures) = collect_changes(tx, recipient, verdicts, now)?;
        let item_count = new_items.len();
        let failed_count = new_failures.len();
        let mut entries = Vec::with_capacity(item_count + failed_count);
        entries.extend(new_items.into_iter().map(MailEntry::Item));
        entries.extend(new_failures.into_iter().map(MailEntry::Failure));

        if entries.is_empty() {
            return Ok(MailedTotals {
                envelopes: 0,
                items: 0,
                failed: 0,
            });
        }

        let mut next_seq = tx.messages().next_receipt()?;
        let mut envelopes = 0;

        for chunk in entries.chunks(rule::MAX_BATCH_ITEMS) {
            let body = chunk
                .iter()
                .map(MailEntry::line)
                .collect::<Vec<_>>()
                .join("\n");

            let mut message_id = format!("watch-{now}-{next_seq}");
            while tx
                .messages()
                .prior_delivery(caller_seat, &message_id)?
                .is_some()
            {
                next_seq += 1;
                message_id = format!("watch-{now}-{next_seq}");
            }
            next_seq += 1;

            tx.messages()
                .insert(caller_seat, recipient, &message_id, &body, now)?;
            for entry in chunk {
                if let MailEntry::Item(item) = entry {
                    tx.watch()
                        .mark_handed(recipient, &item.probe, &item.fingerprint, now)?;
                }
            }
            envelopes += 1;
        }

        Ok(MailedTotals {
            envelopes,
            items: item_count,
            failed: failed_count,
        })
    })
}

fn collect_changes(
    tx: &formation_store::Tx<'_>,
    target_seat: &str,
    verdicts: &[(&str, ProbeVerdict)],
    now: i64,
) -> Result<(Vec<Item>, Vec<Failure>), Error> {
    let mut new_items = Vec::new();
    let mut new_failures = Vec::new();

    for (probe_name, verdict) in verdicts {
        match verdict {
            ProbeVerdict::Ok(lines) => {
                tx.watch().clear_failure(target_seat, probe_name)?;
                for line in lines {
                    if !tx
                        .watch()
                        .is_handed(target_seat, probe_name, &line.fingerprint)?
                    {
                        new_items.push(Item {
                            probe: (*probe_name).to_owned(),
                            fingerprint: line.fingerprint.clone(),
                            text: line.text.clone(),
                        });
                    }
                }
            }
            ProbeVerdict::Failed(error) => {
                if tx
                    .watch()
                    .record_failure(target_seat, probe_name, error, now)?
                {
                    new_failures.push(Failure {
                        probe: (*probe_name).to_owned(),
                        error: error.clone(),
                    });
                }
            }
        }
    }

    Ok((new_items, new_failures))
}

fn unknown_recipient(mut joined: Vec<String>, to: &str) -> Error {
    let more = joined.len() > LISTED_SEATS;
    joined.truncate(LISTED_SEATS);
    let more = if more { " and more" } else { "" };
    refused(
        "unknown_recipient",
        format!(
            "{to} has not joined this room; joined seats: {}{more}",
            joined.join(", ")
        ),
    )
}
