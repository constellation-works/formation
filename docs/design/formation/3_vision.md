---
title: Formation directions
owner: codex
created_date: 2026-11-16
updated_date: 2026-11-24
status: Draft
feature: formation
doc_role: vision
summary: Open delivery and staffing questions, the evidence they need and the longer-term direction accepted for exploration.
---

# Formation directions

## Reach the chat that owns the decision

The next question is whether a chief can send a message while a captain is idle and have that exact captain start a turn, then receive a reply in the original chief chat. The sentry already has a bounded way to read the mail. Success needs a receiving host build, seat, message receipt and new turn in the intended chat before human input. Qualify each direction independently. [Shift-messaging design and trial](../shift-messaging/1_scope.md)

If a host can offer pending mail only on the next user turn, that capability is still useful. An operator should be able to tell whether a receipt means stored, handled, or observed to start a turn. An adapter would also need to enforce the approved binding and route when it delivers queued mail.

## Spend judgment where it changes the outcome

The parked [portable-kit proposal](references/portable.md) describes a script that recognizes repeated observations and a sentry that interprets unfamiliar ones. CI is a small test case. Can repeated failures be suppressed without hiding a recurrence after recovery? Can the responsible captain still see every item that needs a decision? These questions can be tested using the local room and existing CI reader.

## Adjust staffing to demand

Daniel accepted self-regulating formations as a direction for exploration. In the [original vision](https://github.com/constellation-works/formation/blob/520d0d03181bfb68ea15347093f2a4b831c6c298/docs/design/formation/3_vision.md), an incident has a goal, budget and deadline. It starts with the least expensive responder that can settle it, escalates when necessary and releases capacity when the work ends. Repeated incidents should become cheaper to resolve as people capture what worked.

Today the operator selects units, a captain requests reinforcement, and the chief records shift outcomes. Before automating those choices, use close-out records to see which escalations needed the operator, how often a watch found actionable work and what adding a seat cost. Automated triggers, capacity budgets and the component that would own them remain open design choices. [Current close-out measures](../../../plugin/skills/formation/references/close-out.md)
