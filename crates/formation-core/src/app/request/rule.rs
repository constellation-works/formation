//! Request input rules and read filters.

use super::Filter;
use crate::app::grant::ALL_WORKSPACES;
use cw_base::{Error, Kind};
use formation_store::RosterSeat;
use formation_store::contract::{ADMIRAL, Request, RequestOption, RequestState, identifier};
use std::collections::BTreeSet;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const MAX_TEXT_BYTES: usize = 4096;

const MIN_OPTIONS: usize = 2;

const MAX_OPTIONS: usize = 4;

const MAX_LABEL_BYTES: usize = 16;

/// Roles whose requests the chief holds first.
const CHIEF_ROUTED: [&str; 3] = ["captain", "advisor", "aide"];

#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("pass --as <seat> or join this room from your session before changing a request")]
    SeatRequired,
    #[error("seat `{0}` has not joined this room; run `formation mail join` first")]
    UnjoinedSeat(String),
    #[error("no request {0:?} in this room; use `formation request list` to see ids")]
    UnknownRequest(String),
    #[error("request `{0}` is already settled")]
    Settled(String),
    #[error("seat `{0}` does not hold request `{1}` (holder: {2})")]
    NotHolder(String, String, String),
    #[error("`--{0}` must contain one line of text of at most {MAX_TEXT_BYTES} bytes")]
    InvalidText(&'static str),
    #[error("provide two to four `--option 'label: text'` flags with distinct ASCII labels")]
    InvalidOptions,
    #[error("option {0:?} is unknown; choose one of {1}")]
    UnknownOption(String, String),
    #[error("workspace {0:?} is not in the applied roster; choose `all`{1}")]
    UnknownWorkspace(String, String),
    #[error("`--note` on a pass requires `--recommend <option>`")]
    NoteRequiresRecommendation,
    #[error("invalid `--since` {0:?}; use an RFC 3339 timestamp with an offset")]
    InvalidSince(String),
    #[error("a joined seat must pass --admiral to reopen a request")]
    AdmiralRequired,
    #[error("request `{0}` is not settled")]
    NotSettled(String),
    #[error("the current roster has no chief")]
    NoChief,
    #[error("the current roster has more than one chief ({0}); name one as `reports_to` for `{1}`")]
    AmbiguousChief(String, String),
    #[error("`{0}` reports to `{1}`, which is not a chief seat; a request must go to the chief")]
    ParentNotChief(String, String),
    #[error("could not reserve an envelope identifier for the request")]
    MailIdentifierExhausted,
}

impl From<Failure> for Error {
    fn from(failure: Failure) -> Self {
        let code = match &failure {
            Failure::AdmiralRequired => "admiral_required",
            Failure::NotSettled(_) => "request_not_settled",
            Failure::NoChief => "chief_required",
            Failure::AmbiguousChief(..) => "chief_ambiguous",
            Failure::ParentNotChief(..) => "request_parent_not_chief",
            Failure::SeatRequired => "seat_required",
            Failure::UnjoinedSeat(_) => "unknown_seat",
            Failure::UnknownRequest(_) => "unknown_request",
            Failure::Settled(_) => "request_settled",
            Failure::NotHolder(..) => "request_not_holder",
            Failure::InvalidText(_) => "invalid_request_text",
            Failure::InvalidOptions => "invalid_options",
            Failure::UnknownOption(..) => "unknown_option",
            Failure::UnknownWorkspace(..) => "unknown_workspace",
            Failure::NoteRequiresRecommendation => "note_requires_recommendation",
            Failure::InvalidSince(_) => "invalid_since",
            Failure::MailIdentifierExhausted => {
                return Self::new(Kind::Internal, "mail_identifier_exhausted", failure);
            }
        };
        Self::new(Kind::Refused, code, failure)
    }
}

pub(super) struct ListFilter {
    waiting: Option<String>,
    settled_by: Option<String>,
    since: Option<OffsetDateTime>,
}

impl ListFilter {
    pub(super) fn parse(filter: Filter) -> Result<Self, Error> {
        let since = filter
            .since
            .map(|value| {
                OffsetDateTime::parse(&value, &Rfc3339).map_err(|_| Failure::InvalidSince(value))
            })
            .transpose()?;
        Ok(Self {
            waiting: filter.waiting,
            settled_by: filter.settled_by,
            since,
        })
    }

    pub(super) fn matches(&self, request: &Request) -> bool {
        match &request.state {
            RequestState::Open { holder } => {
                if self.settled_by.as_ref().is_some_and(|seat| {
                    !request.overturned.iter().any(|entry| {
                        &entry.seat == seat && self.since.is_none_or(|since| entry.at >= since)
                    })
                }) || self.waiting.as_ref().is_some_and(|name| {
                    if name == ADMIRAL {
                        holder.as_deref().is_some_and(|seat| seat != ADMIRAL)
                    } else {
                        holder.as_deref() != Some(name.as_str())
                    }
                }) {
                    return false;
                }
            }
            RequestState::Settled(settlement) => {
                if self.waiting.is_some()
                    || self.settled_by.as_ref().is_some_and(|seat| {
                        seat != &settlement.seat
                            && !request.overturned.iter().any(|prior| {
                                &prior.seat == seat
                                    && self.since.is_none_or(|since| prior.at >= since)
                            })
                    })
                {
                    return false;
                }
                if self.settled_by.is_some() {
                    return self.settled_by.as_ref().is_some_and(|seat| {
                        (seat == &settlement.seat
                            && self.since.is_none_or(|since| settlement.at >= since))
                            || request.overturned.iter().any(|prior| {
                                seat == &prior.seat
                                    && self.since.is_none_or(|since| prior.at >= since)
                            })
                    });
                }
            }
        }

        self.since.is_none_or(|since| {
            request.updated_at >= since || request.advice.iter().any(|advice| advice.at >= since)
        })
    }
}

/// Pick who holds a request the asker opens.
///
/// A captain, advisor or aide goes to the chief: its explicit parent when that seat is a
/// chief, otherwise the sole chief in the roster. Other seats keep their parent.
pub(super) fn holder(asker: &RosterSeat, roster: &[RosterSeat]) -> Result<Option<String>, Error> {
    if !CHIEF_ROUTED.contains(&asker.role.as_str()) {
        return Ok(asker.reports_to.clone());
    }
    let is_chief = |name: &str| {
        roster
            .iter()
            .any(|seat| seat.name == name && seat.role == "chief")
    };
    if let Some(parent) = &asker.reports_to {
        if is_chief(parent) {
            return Ok(Some(parent.clone()));
        }
        return Err(Failure::ParentNotChief(asker.name.clone(), parent.clone()).into());
    }
    let mut chiefs = roster.iter().filter(|seat| seat.role == "chief");
    match (chiefs.next(), chiefs.next()) {
        (None, _) => Err(Failure::NoChief.into()),
        (Some(chief), None) => Ok(Some(chief.name.clone())),
        (Some(first), Some(second)) => Err(Failure::AmbiguousChief(
            format!("{}, {}", first.name, second.name),
            asker.name.clone(),
        )
        .into()),
    }
}

/// Choose the workspace a new request targets.
///
/// An explicit `all` or a workspace some applied seat belongs to is kept; anything else is
/// refused. With no value the request keeps the asker's own workspace, or `all` without one.
pub(super) fn workspace(
    asker: &RosterSeat,
    roster: &[RosterSeat],
    requested: Option<String>,
) -> Result<String, Error> {
    let Some(requested) = requested else {
        return Ok(asker
            .workspace
            .as_deref()
            .unwrap_or(ALL_WORKSPACES)
            .to_owned());
    };
    let known: BTreeSet<&str> = roster
        .iter()
        .filter_map(|seat| seat.workspace.as_deref())
        .collect();
    if requested == ALL_WORKSPACES || known.contains(requested.as_str()) {
        return Ok(requested);
    }
    let names = known
        .iter()
        .map(|name| format!(", `{name}`"))
        .collect::<String>();
    Err(Failure::UnknownWorkspace(requested, names).into())
}

pub(super) fn require_request(id: &str, record: Option<Request>) -> Result<Request, Error> {
    record.ok_or_else(|| Failure::UnknownRequest(id.to_owned()).into())
}

pub(super) fn text<'a>(field: &'static str, value: &'a str) -> Result<&'a str, Error> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(Failure::InvalidText(field).into());
    }
    Ok(value)
}

pub(super) fn options(raw: &[String]) -> Result<Vec<RequestOption>, Error> {
    if !(MIN_OPTIONS..=MAX_OPTIONS).contains(&raw.len()) {
        return Err(Failure::InvalidOptions.into());
    }
    let mut options: Vec<RequestOption> = Vec::new();
    for value in raw {
        let (label, body) = value.split_once(':').ok_or(Failure::InvalidOptions)?;
        let label = label.trim();
        if label.len() > MAX_LABEL_BYTES
            || !identifier(label)
            || options.iter().any(|option| option.label == label)
        {
            return Err(Failure::InvalidOptions.into());
        }
        options.push(RequestOption {
            label: label.to_owned(),
            text: text("option", body)?.to_owned(),
        });
    }
    Ok(options)
}

pub(super) fn require_option(options: &[RequestOption], label: &str) -> Result<(), Error> {
    if options.iter().any(|option| option.label == label) {
        return Ok(());
    }
    let labels = options
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(Failure::UnknownOption(label.to_owned(), labels).into())
}
