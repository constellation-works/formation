//! Wire payloads preserved independently of the application's outcome types.

use formation_core::app::mail::Outcome;
use serde_json::{Value, json};

pub(crate) fn mail(out: Outcome) -> Value {
    match out {
        Outcome::Joined {
            seat,
            replaced_session,
        } => json!({
            "seat": seat,
            "rebound": replaced_session.is_some(),
            "replaced_session": replaced_session,
        }),
        Outcome::Seats(seats) => json!({"seats": seats
            .into_iter()
            .map(|seat| json!({
                "name": seat.name,
                "unread": seat.unread,
                "held": seat.held,
                "seen": seat.seen,
            }))
            .collect::<Vec<_>>()}),
        Outcome::Sent { id, receipt, held } => {
            json!({"id": id, "receipt": receipt, "held": held})
        }
        Outcome::Inbox(messages) => json!({"messages": messages}),
        Outcome::Ack(receipt) => json!({"acknowledged": receipt}),
    }
}

pub(crate) fn error(error: &formation_core::Error) -> Value {
    json!({
        "code": error.code(),
        "error": error.to_string(),
    })
}
