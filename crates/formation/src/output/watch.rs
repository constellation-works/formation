//! Watch command output: wire shape and JSON write.

use formation_core::app::watch::Outcome as WatchOutcome;
use serde_json::{Value, json};
use std::process::ExitCode;

pub(crate) fn payload(out: WatchOutcome) -> Value {
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

pub(crate) fn write(value: &Value) -> ExitCode {
    super::shared::write_json(value)
}
