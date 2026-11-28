use super::{ADMIRAL, Error, Outcome, SeatRef, Store, Tx, refused, rule, touch};
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
    if to == ADMIRAL {
        only_the_chief(tx, &sender)?;
    }

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
    if matches!(caller, SeatRef::Admiral) {
        tx.messages().mark_terminal(receipt)?;
    }
    Ok((Outcome::Sent { id, receipt, held }, true))
}

/// Only the seat holding the chief's rank in the applied roster mails the admiral; every other
/// seat reaches the admiral through a request.
fn only_the_chief(tx: &Tx<'_>, sender: &str) -> Result<(), Error> {
    let roster = tx.roster().seats()?;
    if roster.is_empty() {
        return Err(refused(
            "no_roster",
            "no roster applied to this room, so it has no chief to mail the admiral; \
             run `formation roster apply --room <room>` first",
        ));
    }
    if roster
        .iter()
        .any(|seat| seat.name == sender && seat.role == "chief")
    {
        return Ok(());
    }
    Err(refused(
        "admiral_via_chief",
        "only the chief mails the admiral; open a request with `formation request open` instead",
    ))
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
