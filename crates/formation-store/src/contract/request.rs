//! Durable requests and the evidence recorded with them.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// One option in a request.
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestOption {
    /// Option label, reused by recommendations and settlements.
    pub label: String,
    /// Option text.
    pub text: String,
}

/// Advice added to a request after it was opened.
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestAdvice {
    /// Recommending seat.
    pub seat: String,
    /// Recommended option label.
    pub option: String,
    /// Explanation, when supplied.
    pub note: Option<String>,
    /// When the recommendation was made.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// The choice and authority recorded at settlement.
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestSettlement {
    /// Chosen option label.
    pub option: String,
    /// Seat that made the choice.
    pub seat: String,
    /// Winning room grant, or absent for a catalog entry.
    pub grant_id: Option<i64>,
    /// Condition attached to the winning entry when the choice was made.
    pub limit: Option<String>,
    /// Explanation, when supplied.
    pub note: Option<String>,
    /// Source of an admiral answer, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Seat that entered an admiral answer, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entered_by: Option<String>,
    /// When the choice was made.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Who can act as the current holder of an open request.
#[derive(Debug)]
pub enum RequestState {
    /// An open request. An absent seat means that it waits on the admiral.
    Open {
        /// Applied seat currently holding the request.
        holder: Option<String>,
    },
    /// A completed decision with its authority preserved.
    Settled(RequestSettlement),
}

/// A request stored in the room that holds it.
#[derive(Debug)]
pub struct Request {
    /// Stable identifier bearing the originating room's name.
    pub id: String,
    /// Seat that opened the request.
    pub asker: String,
    /// Class from the authority catalog.
    pub class: String,
    /// Workspace in which the choice is made.
    pub workspace: String,
    /// Pointer to the task, pull request or other record.
    pub record: String,
    /// One-sentence question.
    pub ask: String,
    /// Choices available to the holder, in the asker's order.
    pub options: Vec<RequestOption>,
    /// Option label the asker originally recommended.
    pub recommendation: String,
    /// Whether work is waiting on this decision.
    pub blocking: bool,
    /// When the request was opened.
    pub opened_at: OffsetDateTime,
    /// When the holder or state last changed.
    pub updated_at: OffsetDateTime,
    /// Current holder or completed decision.
    pub state: RequestState,
    /// Recommendations added by seats, in recording order.
    pub advice: Vec<RequestAdvice>,
    /// Prior settlements overturned by the admiral.
    pub overturned: Vec<RequestSettlement>,
}
