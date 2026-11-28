use super::{ADMIRAL, Error, Outcome, SeatRef, Store, Tx, refused, rule, touch};
use crate::runtime;

/// Joined seats named in an `unknown_recipient` refusal.
const LISTED_SEATS: usize = 32;

pub(in crate::app) struct Draft<'a> {
    pub(in crate::app) to: &'a str,
    pub(in crate::app) id: String,
    pub(in crate::app) body: &'a str,
    pub(in crate::app) reply_to: Option<i64>,
}

pub(super) fn commit(
    store: &Store,
    caller: SeatRef<'_>,
    now: i64,
    draft: Draft<'_>,
) -> Result<(Outcome, bool), Error> {
    validate(&draft)?;
    store.write(|tx| deliver(tx, caller, now, draft))
}

pub(in crate::app) fn in_transaction(
    tx: &Tx<'_>,
    caller: SeatRef<'_>,
    now: i64,
    draft: Draft<'_>,
) -> Result<(Outcome, bool), Error> {
    validate(&draft)?;
    deliver(tx, caller, now, draft)
}

fn validate(draft: &Draft<'_>) -> Result<(), Error> {
    rule::name(draft.to)?;
    rule::name(&draft.id)?;
    rule::body(draft.body)
}

fn deliver(
    tx: &Tx<'_>,
    caller: SeatRef<'_>,
    now: i64,
    Draft {
        to,
        id,
        body,
        reply_to,
    }: Draft<'_>,
) -> Result<(Outcome, bool), Error> {
    let sender = touch(tx, caller, now)?;
    if to == ADMIRAL {
        only_the_chief(tx, &sender)?;
    }

    let Some(seen) = tx.seats().seen(to)? else {
        return Err(unknown_recipient(tx.seats().names(LISTED_SEATS + 1)?, to));
    };
    let held = runtime::held(seen, now);

    let messages = tx.messages();
    if let Some(prior) = messages.prior_delivery(&sender, &id)? {
        if prior.recipient == to && prior.body == body && prior.reply_to == reply_to {
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
            "message ID already used with different content, recipient or reply receipt",
        ));
    }

    if let Some(reply_to) = reply_to {
        let participant = (!matches!(caller, SeatRef::Admiral)).then_some(sender.as_str());
        if !messages.can_reply_to(reply_to, participant)? {
            return Err(refused(
                "invalid_reply_receipt",
                format!("cannot reply to receipt {reply_to}; choose a receipt from your mail log"),
            ));
        }
    }

    let receipt = messages.insert(&sender, to, &id, body, now)?;
    if let Some(reply_to) = reply_to {
        messages.mark_reply(receipt, reply_to)?;
    }
    if matches!(caller, SeatRef::Admiral) {
        messages.mark_terminal(receipt)?;
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
