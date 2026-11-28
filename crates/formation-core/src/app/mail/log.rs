use super::{ADMIRAL, Error, LogOptions, Outcome, SeatRef, Store, refused};
use formation_store::LogQuery;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn caller(store: &Store, caller: SeatRef<'_>) -> Result<Option<String>, Error> {
    let snapshot = store.read()?;
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
    let caller = caller(store, seat)?;
    let snapshot = store.read()?;
    let query = LogQuery {
        caller: caller.as_deref(),
        seat: options.seat.as_deref(),
        with: options.with.as_deref(),
        since,
        no_watch: options.no_watch,
        limit: options.limit,
        receipt: None,
    };
    let messages = snapshot.messages().log(&query)?;
    Ok(Outcome::Log {
        messages,
        full: options.full,
    })
}

pub(super) fn show(store: &Store, seat: SeatRef<'_>, receipt: i64) -> Result<Outcome, Error> {
    let caller = caller(store, seat)?;
    let snapshot = store.read()?;
    let query = LogQuery {
        caller: None,
        seat: None,
        with: None,
        since: None,
        no_watch: false,
        limit: 1,
        receipt: Some(receipt),
    };
    let message = snapshot
        .messages()
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
    Ok(Outcome::Show(message))
}
