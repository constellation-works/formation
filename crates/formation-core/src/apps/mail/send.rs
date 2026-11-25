use super::{Error, Outcome, Store, refused, rules, touch};
pub(super) fn run(
    store: &Store,
    key: &[u8],
    now: i64,
    to: &str,
    id: String,
    body: &str,
) -> Result<Outcome, Error> {
    rules::name(to)?;
    rules::name(&id)?;
    rules::body(body)?;
    store.write(|tx| {
        let sender = touch(tx, key, now)?;
        if let Some((receipt, recipient, text)) = tx.messages().previous(&sender, &id)? {
            if recipient == to && text == body {
                return Ok(Outcome::Sent(id, receipt));
            }
            return Err(refused(
                "message_conflict",
                "message ID already used with different content or recipient",
            ));
        }
        if !tx.seats().present(to, now - rules::PRESENCE_SECONDS)? {
            return Err(refused(
                "recipient_offline",
                "recipient is offline or unknown",
            ));
        }
        let receipt = tx.messages().insert(&sender, to, &id, body)?;
        Ok(Outcome::Sent(id, receipt))
    })
}
