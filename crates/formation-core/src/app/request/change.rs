//! Commit each request change and its envelope together, then run the normal mail wake.

use super::rule::{self, Failure};
use super::{Action, Change, Open};
use crate::app::grant;
use crate::app::mail::{SeatRef, send};
use crate::app::wake;
use crate::composition::Caller;
use crate::runtime::Ctx;
use cw_base::Error;
use formation_store::contract::{Request, RequestAdvice, RequestSettlement, RequestState};
use formation_store::{RosterSeat, Tx};
use std::path::Path;
use time::OffsetDateTime;

struct Mutation<'a> {
    tx: &'a Tx<'a>,
    actor: &'a RosterSeat,
    roster: &'a [RosterSeat],
    now: OffsetDateTime,
}

impl Mutation<'_> {
    fn update(&self, id: &str, action: Action) -> Result<(Request, Option<String>), Error> {
        let mut request = rule::require_request(id, self.tx.requests().get(id)?)?;
        let (action, recipient) = match action {
            Action::Advise { option, note } => {
                self.advise(&mut request, option, Some(note))?;
                let recipient = match &request.state {
                    RequestState::Open { holder } => holder.clone(),
                    RequestState::Settled(_) => None,
                };
                ("received advice", recipient)
            }
            Action::Settle { option, note } => {
                if matches!(request.state, RequestState::Settled(_)) {
                    return Err(Failure::Settled(request.id).into());
                }
                self.settle(&mut request, option, note)?;
                ("settled", Some(request.asker.clone()))
            }
            Action::Pass { option, note } => {
                self.pass(&mut request, option, note)?;
                ("passed", self.actor.reports_to.clone())
            }
        };

        self.tx.requests().update(&request)?;
        self.notify(&request, action, recipient.as_deref())?;
        Ok((request, recipient))
    }

    fn open(&self, input: Open, room: &str) -> Result<(Request, Option<String>), Error> {
        let record = rule::text("record", &input.record)?.to_owned();
        let ask = rule::text("ask", &input.ask)?.to_owned();
        let options = rule::options(&input.options)?;
        rule::require_option(&options, &input.recommendation)?;

        let id = format!(
            "req-{}-{}",
            room_label(room),
            self.tx.requests().new_suffix()?
        );
        let mut request = Request {
            id,
            asker: self.actor.name.clone(),
            class: input.class,
            workspace: self
                .actor
                .workspace
                .as_deref()
                .unwrap_or(grant::ALL_WORKSPACES)
                .to_owned(),
            record,
            ask,
            options,
            recommendation: input.recommendation,
            blocking: input.blocking,
            opened_at: self.now,
            updated_at: self.now,
            state: RequestState::Open {
                holder: self.actor.reports_to.clone(),
            },
            advice: Vec::new(),
        };
        let (action, recipient) = if let Some(option) = input.settle {
            self.settle(&mut request, option, None)?;
            ("opened and settled", Some(request.asker.clone()))
        } else {
            grant::validate_class(&request.class, &self.tx.grants().active()?)?;
            ("opened", self.actor.reports_to.clone())
        };

        self.tx.requests().insert(&request)?;
        self.notify(&request, action, recipient.as_deref())?;
        Ok((request, recipient))
    }

    fn advise(
        &self,
        request: &mut Request,
        option: String,
        note: Option<String>,
    ) -> Result<(), Error> {
        rule::require_option(&request.options, &option)?;
        request.advice.push(RequestAdvice {
            seat: self.actor.name.clone(),
            option,
            note: checked_note(note)?,
            at: self.now,
        });
        Ok(())
    }

    fn settle(
        &self,
        request: &mut Request,
        option: String,
        note: Option<String>,
    ) -> Result<(), Error> {
        let entry = grant::check_authority(
            self.actor,
            &request.class,
            &request.workspace,
            self.roster,
            &self.tx.grants().active()?,
        )?;
        rule::require_option(&request.options, &option)?;
        request.state = RequestState::Settled(RequestSettlement {
            option,
            seat: self.actor.name.clone(),
            grant_id: entry.grant_id,
            limit: entry.limit,
            note: checked_note(note)?,
            at: self.now,
        });
        request.updated_at = self.now;
        Ok(())
    }

    fn pass(
        &self,
        request: &mut Request,
        option: Option<String>,
        note: Option<String>,
    ) -> Result<(), Error> {
        let RequestState::Open { holder } = &request.state else {
            return Err(Failure::Settled(request.id.clone()).into());
        };
        if holder.as_deref() != Some(&self.actor.name) {
            return Err(Failure::NotHolder(
                self.actor.name.clone(),
                request.id.clone(),
                holder.as_deref().unwrap_or("admiral").to_owned(),
            )
            .into());
        }

        if let Some(option) = option {
            self.advise(request, option, note)?;
        } else if note.is_some() {
            return Err(Failure::NoteRequiresRecommendation.into());
        }
        request.state = RequestState::Open {
            holder: self.actor.reports_to.clone(),
        };
        request.updated_at = self.now;
        Ok(())
    }

    fn notify(&self, request: &Request, action: &str, to: Option<&str>) -> Result<(), Error> {
        let Some(to) = to else {
            return Ok(());
        };
        let messages = self.tx.messages();
        let mut sequence = messages.next_receipt()?;
        let id = loop {
            let id = format!("request-{sequence}");
            if messages.prior_delivery(&self.actor.name, &id)?.is_none() {
                break id;
            }
            sequence = sequence
                .checked_add(1)
                .ok_or(Failure::MailIdentifierExhausted)?;
        };
        let body = format!(
            "Request {} {action}. Run `formation request show {}` for details.",
            request.id, request.id,
        );
        send::in_transaction(
            self.tx,
            SeatRef::Name(&self.actor.name),
            self.now.unix_timestamp(),
            to,
            id,
            &body,
        )?;
        Ok(())
    }
}

pub(super) fn run(
    ctx: &Ctx<'_>,
    change: Change,
    queue: &dyn Fn(&str) -> bool,
) -> Result<Request, Error> {
    let room = ctx.mail()?;
    let (request, recipient) = room.store.write(|tx| {
        let roster = grant::require_roster_tx(tx)?;
        let name = joined_caller(tx, &room.caller)?;
        let actor = grant::find_roster_seat(&roster, &name)?;
        let mutation = Mutation {
            tx,
            actor,
            roster: &roster,
            now: (ctx.clock)(),
        };
        match change {
            Change::Open(input) => mutation.open(input, &room.room),
            Change::Existing { id, action } => mutation.update(&id, action),
        }
    })?;

    if let Some(recipient) = recipient {
        wake::after_commit(ctx, &recipient, queue);
    }
    Ok(request)
}

fn joined_caller(tx: &Tx<'_>, caller: &Caller) -> Result<String, Error> {
    match caller {
        Caller::Session(session) => tx
            .seats()
            .by_session(session)?
            .ok_or_else(|| Failure::SeatRequired.into()),
        Caller::Explicit(name) => {
            if tx.seats().seen(name)?.is_none() {
                return Err(Failure::UnjoinedSeat(name.clone()).into());
            }
            Ok(name.clone())
        }
        Caller::None | Caller::Join { .. } => Err(Failure::SeatRequired.into()),
    }
}

fn room_label(room: &str) -> String {
    Path::new(room)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("room")
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() || matches!(letter, '-' | '_') {
                letter
            } else {
                '-'
            }
        })
        .collect()
}

fn checked_note(note: Option<String>) -> Result<Option<String>, Error> {
    note.as_deref()
        .map(|value| rule::text("note", value).map(str::to_owned))
        .transpose()
}
