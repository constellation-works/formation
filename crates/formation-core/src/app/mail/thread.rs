//! Group envelopes by explicit links and summarize the members visible in a view.

use super::{LogMessage, Thread};
use formation_store::contract::ThreadLink;
use std::collections::{BTreeSet, HashMap};

pub(super) struct Threads {
    roots: HashMap<i64, i64>,
}

impl Threads {
    pub(super) fn new(links: Vec<ThreadLink>) -> Self {
        let mut requests = HashMap::new();
        let mut threads = Self {
            roots: HashMap::new(),
        };
        // Replies point back to an earlier receipt. Request notifications share the
        // first receipt for that request, which later replies to those notifications inherit.
        for link in links {
            let root = if let Some(reply_to) = link.reply_to {
                threads.root(reply_to)
            } else if let Some(request_id) = link.request_id {
                *requests.entry(request_id).or_insert(link.receipt)
            } else {
                continue;
            };
            if link.receipt != root {
                threads.roots.insert(link.receipt, root);
            }
        }
        threads
    }

    pub(super) fn others(&self, receipt: i64) -> Vec<i64> {
        let root = self.root(receipt);
        let mut others: Vec<i64> = self
            .roots
            .iter()
            .filter_map(|(&member, &group)| (group == root && member != receipt).then_some(member))
            .collect();
        if root != receipt {
            others.push(root);
        }
        others
    }

    pub(super) fn group(&self, messages: Vec<LogMessage>) -> Vec<Thread> {
        let mut positions = HashMap::new();
        let mut groups: Vec<Vec<LogMessage>> = Vec::new();
        // The input is newest first, so the first encounter fixes each thread's order.
        for message in messages {
            let index = *positions
                .entry(self.root(message.receipt))
                .or_insert_with(|| {
                    groups.push(Vec::new());
                    groups.len() - 1
                });
            groups[index].push(message);
        }
        groups
            .into_iter()
            .map(|mut messages| {
                messages.reverse();
                let seats: BTreeSet<&str> = messages
                    .iter()
                    .flat_map(|message| [message.sender.as_str(), message.recipient.as_str()])
                    .collect();
                let seats = seats.into_iter().map(str::to_owned).collect();
                Thread { seats, messages }
            })
            .collect()
    }

    pub(super) fn root(&self, receipt: i64) -> i64 {
        self.roots.get(&receipt).copied().unwrap_or(receipt)
    }
}
