use super::{Error, Outcome, SeatRef, Store, refused, rule, touch};

pub(super) fn run(
    store: &Store,
    caller: SeatRef<'_>,
    now: i64,
    to: &str,
    id: String,
    body: &str,
) -> Result<Outcome, Error> {
    rule::name(to)?;
    rule::name(&id)?;
    rule::body(body)?;

    store.write(|tx| {
        let sender = touch(tx, caller, now)?;

        if let Some(prior) = tx.messages().prior_delivery(&sender, &id)? {
            if prior.recipient == to && prior.body == body {
                return Ok(Outcome::Sent(id, prior.receipt));
            }
            return Err(refused(
                "message_conflict",
                "message ID already used with different content or recipient",
            ));
        }

        if !tx.seats().present(to, now - rule::PRESENCE_SECONDS)? {
            return Err(refused(
                "recipient_offline",
                "recipient is offline or unknown",
            ));
        }

        let receipt = tx.messages().insert(&sender, to, &id, body)?;
        Ok(Outcome::Sent(id, receipt))
    })
}
