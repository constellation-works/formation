//! Polling loop, transactional handover deduplication, and mail delivery for `formation watch`.

use super::{
    Command, Ctx, Error, Failure, Item, Outcome, ProbeLimits, ProbeOutput, ProbeRunner, SeatRef,
    Store, refused, rule, set,
};
use formation_store::Tx;
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) const LISTED_SEATS: usize = 32;

const SLEEP_SLICE: Duration = Duration::from_millis(50);

pub(super) enum ProbeVerdict {
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

pub(super) fn run(
    ctx: &Ctx<'_>,
    store: &Store,
    caller: SeatRef<'_>,
    cmd: Command<'_>,
) -> Result<Outcome, Error> {
    let probes = rule::parse_probes(&cmd.probes)?;
    let seat = set::caller_in_snapshot(&store.read()?, caller)?;
    let timeout = cmd
        .timeout
        .unwrap_or_else(|| Duration::from_secs(rule::DEFAULT_WAIT_TIMEOUT_SECONDS));
    let start = Instant::now();
    let mut cycle_start = start;

    loop {
        if cmd.runner.stopped() {
            return Ok(empty_report(false));
        }
        let limits = ProbeLimits {
            probe_timeout: Duration::from_secs(rule::PROBE_TIMEOUT_SECONDS),
            command_remaining: (!timeout.is_zero())
                .then(|| timeout.saturating_sub(start.elapsed())),
        };
        let Some(verdicts) = run_probes(&probes, cmd.runner, ctx.cwd, limits) else {
            return Ok(empty_report(
                !cmd.runner.stopped() && start.elapsed() >= timeout,
            ));
        };

        let now = (ctx.clock)().unix_timestamp();
        let (items, failed, remaining) = commit_report(store, &seat, &verdicts, now)?;
        if !items.is_empty() || !failed.is_empty() {
            return Ok(Outcome {
                items,
                failed,
                remaining,
                expired: false,
            });
        }

        let wait = cmd
            .every
            .saturating_sub(cycle_start.elapsed())
            .min(timeout.saturating_sub(start.elapsed()));
        sleep_interruptible(wait, cmd.runner);
        if cmd.runner.stopped() || start.elapsed() >= timeout {
            return Ok(empty_report(!cmd.runner.stopped()));
        }
        cycle_start = Instant::now();
    }
}

fn empty_report(expired: bool) -> Outcome {
    Outcome {
        items: Vec::new(),
        failed: Vec::new(),
        remaining: 0,
        expired,
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

fn run_probes<'a>(
    probes: &'a [rule::ProbeSpec],
    runner: &(dyn ProbeRunner + Sync),
    cwd: Option<&Path>,
    limits: ProbeLimits,
) -> Option<Vec<(&'a str, ProbeVerdict)>> {
    let outputs: Vec<ProbeOutput> = if probes.len() == 1 {
        vec![runner.run(&probes[0].command, cwd, limits, &|| false)]
    } else {
        std::thread::scope(|scope| {
            let handles: Vec<_> = probes
                .iter()
                .map(|probe| {
                    scope.spawn(move || runner.run(&probe.command, cwd, limits, &|| false))
                })
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
        let verdict = ProbeVerdict::from_output(output)?;
        verdicts.push((probe.name.as_str(), verdict));
    }

    Some(verdicts)
}

impl ProbeVerdict {
    pub(super) fn from_output(output: ProbeOutput) -> Option<Self> {
        match output {
            ProbeOutput::Cancelled => None,
            ProbeOutput::Failed(message) => Some(Self::Failed(rule::first_line(&message))),
            ProbeOutput::Success { stdout } => Some(match rule::parse_output(&stdout) {
                Ok(lines) => Self::Ok(lines),
                Err(message) => Self::Failed(rule::first_line(&message)),
            }),
        }
    }

    pub(super) fn summary(&self) -> String {
        match self {
            Self::Ok(lines) => {
                let unit = if lines.len() == 1 { "item" } else { "items" };
                format!("ok, {} {unit}", lines.len())
            }
            Self::Failed(_) => "read failed".to_owned(),
        }
    }
}

fn commit_report(
    store: &Store,
    caller_seat: &str,
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

        let (mut new_items, new_failures) = collect_changes(tx, caller_seat, verdicts, now)?;
        let total_new = new_items.len();
        let remaining = total_new.saturating_sub(rule::MAX_BATCH_ITEMS);
        new_items.truncate(rule::MAX_BATCH_ITEMS);

        for item in &new_items {
            tx.watch()
                .mark_handed(caller_seat, &item.probe, &item.fingerprint, now)?;
        }

        Ok((new_items, new_failures, remaining))
    })
}

pub(super) fn send_changes(
    tx: &Tx<'_>,
    caller_seat: &str,
    recipient: &str,
    verdicts: &[(&str, ProbeVerdict)],
    now: i64,
) -> Result<bool, Error> {
    let (new_items, new_failures) = collect_changes(tx, recipient, verdicts, now)?;
    let mut entries = Vec::with_capacity(new_items.len() + new_failures.len());
    entries.extend(new_items.into_iter().map(MailEntry::Item));
    entries.extend(new_failures.into_iter().map(MailEntry::Failure));

    if entries.is_empty() {
        return Ok(false);
    }

    let mut next_seq = tx.messages().next_receipt()?;

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
    }

    Ok(true)
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

pub(super) fn unknown_recipient(mut joined: Vec<String>, to: &str) -> Error {
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
