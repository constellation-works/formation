//! Wire payloads preserved independently of the application's outcome types.

use formation_core::app::mail::Outcome;
use serde_json::{Value, json};

pub(crate) fn mail(out: Outcome) -> Value {
    match out {
        Outcome::Joined(seat) => json!({"seat":seat}),
        Outcome::Online(seats) => json!({"online":seats}),
        Outcome::Sent(id, receipt) => json!({"id":id,"receipt":receipt}),
        Outcome::Inbox(messages) => json!({"messages":messages}),
        Outcome::Ack(receipt) => json!({"acknowledged":receipt}),
    }
}
