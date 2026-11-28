---
title: Formation — Admiral seat decisions
owner: claude
created_date: 2026-11-28
updated_date: 2026-11-28
status: Draft
feature: admiral-seat
doc_role: decisions
summary: How a call is judged attached to a terminal, which admiral calls are audited, and why terminal mail carries its own flag.
---

# Formation — Admiral seat decisions

## Any of the three streams makes a terminal call

**Code anchors:** `crates/formation/src/output/common/mod.rs::attached_to_terminal`

The [scope](1_scope.md#who-the-caller-is) makes a call with no agent session and
no `--as` the admiral's when it is attached to a terminal, and the operator's
shell often pipes one stream: `formation mail inbox | less` pipes stdout and
`echo hi | formation mail send` pipes stdin. Formation captures, once per
process, whether stdin, stdout or stderr is a terminal, and any one of them
counts. A script under cron or launchd, or any call whose three streams are
redirected, has none and is refused with `seat_required`.

Cost: A script started from the operator's shell that keeps one stream on the
terminal, such as `out=$(formation mail inbox --room r)`, also resolves as the
admiral. Opening `/dev/tty` would widen that to every process with a
controlling terminal, so the narrower stream probe was kept.

## Admiral reads are audited where the command already writes

**Code anchors:** `crates/formation-core/src/app/mail/mod.rs::admiral_audit`

The scope asks for every admiral call in the audit trail, and
`STD-01@4 §R31` forbids a command that only reports from writing. `mail inbox`
and `mail who` write for every seat, because they renew its presence. For the
admiral, who has no presence, they write the audit event instead. `mail sent`
only reads, for a seat or the admiral, so it records nothing.

Cost: An admiral `mail sent` leaves no trace. It shows only what the admiral
sent, and each send it lists is audited.

## Terminal mail carries its own flag

**Code anchors:** `crates/formation-store/src/driver/sqlite/schema/message.rs::TERMINAL`

`request settle --admiral` and `request reopen --admiral` already store mail
whose sender is `admiral`, entered by a seat that relays an answer given in a
chat. Labelling every `admiral` envelope as sent from a terminal would
mislabel those. An append-only schema step adds `messages.terminal`, set only
when the admiral caller sends; the inbox shows `"terminal": true` and the hook
says the envelope was sent from a terminal.

Cost: Another column on every envelope, read with a fallback for rooms an
earlier build wrote.
