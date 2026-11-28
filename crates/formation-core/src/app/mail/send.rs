use super::{Error, Outcome, SeatRef, Store, Tx, refused, rule, touch};
use crate::runtime;

/// Joined seats named in an `unknown_recipient` refusal.
const LISTED_SEATS: usize = 32;

pub(super) fn commit(
    store: &Store,
    caller: SeatRef<'_>,
    now: i64,
    to: &str,
    id: String,
    body: &str,
) -> Result<(Outcome, bool), Error> {
    validate(to, &id, body)?;
    store.write(|tx| deliver(tx, caller, now, to, id, body))
}

pub(in crate::app) fn in_transaction(
    tx: &Tx<'_>,
    caller: SeatRef<'_>,
    now: i64,
    to: &str,
    id: String,
    body: &str,
) -> Result<(Outcome, bool), Error> {
    validate(to, &id, body)?;
    deliver(tx, caller, now, to, id, body)
}

fn validate(to: &str, id: &str, body: &str) -> Result<(), Error> {
    rule::name(to)?;
    rule::name(id)?;
    rule::body(body)
}

fn deliver(
    tx: &Tx<'_>,
    caller: SeatRef<'_>,
    now: i64,
    to: &str,
    id: String,
    body: &str,
) -> Result<(Outcome, bool), Error> {
    let sender = touch(tx, caller, now)?;

    let Some(seen) = tx.seats().seen(to)? else {
        return Err(unknown_recipient(tx.seats().names(LISTED_SEATS + 1)?, to));
    };
    let held = runtime::held(seen, now);

    if let Some(prior) = tx.messages().prior_delivery(&sender, &id)? {
        if prior.recipient == to && prior.body == body {
            return Ok((
                Outcome::Sent {
                    id,
                    receipt: prior.receipt,
                    held,
                },
                false,
            ));
        }
        return Err(refused(
            "message_conflict",
            "message ID already used with different content or recipient",
        ));
    }

    let receipt = tx.messages().insert(&sender, to, &id, body, now)?;
    Ok((Outcome::Sent { id, receipt, held }, true))
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
