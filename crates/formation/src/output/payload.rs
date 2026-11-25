//! Wire payloads preserved independently of the application's outcome types.

use formation_core::app::mail::{Outcome, SentMessage};
use formation_core::app::watch::Outcome as WatchOutcome;
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
        Outcome::SentHistory { messages, total } => sent_history(&messages, total),
        Outcome::Ack(receipt) => json!({"acknowledged": receipt}),
    }
}

pub(super) fn sent_history(messages: &[SentMessage], total: i64) -> Value {
    json!({
        "messages": messages,
        "total": total,
        "truncated": is_truncated(messages.len(), total),
    })
}

pub(super) fn is_truncated(count: usize, total: i64) -> bool {
    i64::try_from(count).is_ok_and(|count| count < total)
}

pub(crate) fn prompt(text: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": text,
        },
    })
}

pub(crate) fn stop(text: &str) -> Value {
    json!({"decision": "block", "reason": text})
}

pub(crate) fn watch(out: WatchOutcome) -> Value {
    match out {
        WatchOutcome::Report {
            items,
            failed,
            remaining,
            expired,
        } => json!({
            "items": items
                .into_iter()
                .map(|item| json!({
                    "probe": item.probe,
                    "fingerprint": item.fingerprint,
                    "text": item.text,
                }))
                .collect::<Vec<_>>(),
            "failed": failed
                .into_iter()
                .map(|failure| json!({
                    "probe": failure.probe,
                    "error": failure.error,
                }))
                .collect::<Vec<_>>(),
            "remaining": remaining,
            "expired": expired,
        }),
        WatchOutcome::Mailed {
            to,
            envelopes,
            items,
            failed,
            expired,
        } => json!({
            "to": to,
            "envelopes": envelopes,
            "items": items,
            "failed": failed,
            "expired": expired,
        }),
    }
}

pub(crate) fn error(error: &formation_core::Error) -> Value {
    json!({
        "code": error.code(),
        "error": error.to_string(),
    })
}
