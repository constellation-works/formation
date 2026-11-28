use super::thread::Threads;
use super::{ADMIRAL, Error, LogOptions, Outcome, SeatRef, Store, refused};
use formation_store::{LogQuery, Snapshot};
use std::collections::HashMap;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn caller(snapshot: &Snapshot, caller: SeatRef<'_>) -> Result<Option<String>, Error> {
    match caller {
        SeatRef::Admiral => Ok(None),
        SeatRef::Name(name) if name == ADMIRAL => Ok(None),
        SeatRef::Name(name) if snapshot.seats().contains(name)? => Ok(Some(name.to_owned())),
        SeatRef::Name(_) => Err(refused("unknown_seat", "seat has not joined this room")),
        SeatRef::Session(session) => {
            snapshot
                .seats()
                .by_session(session)?
                .map(Some)
                .ok_or_else(|| {
                    refused(
                        "seat_required",
                        "seat required: pass --as <seat> or join this room from your session",
                    )
                })
        }
    }
}

pub(super) fn run(store: &Store, seat: SeatRef<'_>, options: LogOptions) -> Result<Outcome, Error> {
    if options.limit < 1 {
        return Err(refused(
            "invalid_limit",
            "--limit must be a positive integer",
        ));
    }
    let since = options
        .since
        .as_deref()
        .map(|value| {
            OffsetDateTime::parse(value, &Rfc3339)
                .map(|time| time.unix_timestamp() + i64::from(time.nanosecond() > 0))
                .map_err(|_| refused("invalid_since", "--since must be an RFC 3339 timestamp"))
        })
        .transpose()?;
    let snapshot = store.read_strict()?;
    let caller = caller(&snapshot, seat)?;
    let query = LogQuery {
        caller: caller.as_deref(),
        seat: options.seat.as_deref(),
        with: options.with.as_deref(),
        since,
        no_watch: options.no_watch,
        limit: if options.threads {
            i64::MAX
        } else {
            options.limit
        },
        receipts: None,
    };
    let history = snapshot.messages();
    if options.threads {
        let threads = Threads::new(history.thread_links()?);
        let limit = usize::try_from(options.limit).unwrap_or(usize::MAX);
        let mut positions = HashMap::new();
        // Count every matching thread, but only load envelopes from the newest ones.
        let messages = history.log_matching(&query, |receipt| {
            let next = positions.len();
            *positions.entry(threads.root(receipt)).or_insert(next) < limit
        })?;
        return Ok(Outcome::Threads {
            threads: threads.group(messages),
            total: positions.len(),
            full: options.full,
        });
    }
    Ok(Outcome::Log {
        messages: history.log(&query)?,
        full: options.full,
    })
}

pub(super) fn show(store: &Store, seat: SeatRef<'_>, receipt: i64) -> Result<Outcome, Error> {
    let snapshot = store.read_strict()?;
    let caller = caller(&snapshot, seat)?;
    let history = snapshot.messages();
    let mut query = LogQuery {
        caller: None,
        seat: None,
        with: None,
        since: None,
        no_watch: false,
        limit: 1,
        receipts: Some(std::slice::from_ref(&receipt)),
    };
    let message = history
        .log(&query)?
        .pop()
        .ok_or_else(|| refused("unknown_receipt", "receipt not found"))?;
    if caller
        .as_ref()
        .is_some_and(|seat| seat != &message.sender && seat != &message.recipient)
    {
        return Err(refused(
            "mail_forbidden",
            "receipt is not addressed to or sent by this seat",
        ));
    }
    let others = Threads::new(history.thread_links()?).others(receipt);
    let mut thread = Vec::new();
    if !others.is_empty() {
        query.caller = caller.as_deref();
        query.receipts = Some(&others);
        query.limit = i64::MAX;
        thread = history.log(&query)?;
        thread.reverse();
    }
    let selected = thread.partition_point(|message| message.receipt < receipt);
    thread.insert(selected, message);
    Ok(Outcome::Show { selected, thread })
}
