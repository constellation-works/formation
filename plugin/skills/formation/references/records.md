# Decisions, open items and recurring issues

Use Scribe when configured. Keep the work and formal decisions in Orbit and point to them from the Scribe entry. Search active entries before raising or filing anything. Record the actual author, related session and task. For example:

```sh
scribe entry list --tag open-item --status active
scribe entry add --tag open-item --author "$model" --project "$project" --related "session:$transcript" --related "task:$task" "$item"
scribe entry list --tag issue --status active
```

An open item identifies the asker, blocking question, full options, recommendation with reason, where the answer must be entered, and affected record. Give the admiral the item ID. Append their exact answer, time and source using `scribe entry update`. Relay it to the owner. The owner retires the item with the result only after carrying the decision out. The aide can list and update items.

Record a durable design, policy or grant under the decision tag. Include who decided, options shown, recommendation, rationale and record. Quote the admiral and give their reason only if stated. A chief or captain records its own reasoning. Supersede a reversal with a new entry. A choice limited to one task stays on that task.

An issue stores the exact recurring fingerprint, what is known, owner and evidence. Append to the matching issue. An accepted issue stays active with its justification, lift condition and the accepted topic (`scribe entry update <id> --add-topic accepted`). Apply and cite it until that condition changes. Notify the sentry of the bounded mute. For a fix, retire with the task or commit. For a decision, open an item pointing to the issue.

Reach for group history with `scribe search --group <id> --kind message` for admiral words and `scribe ask --group <id>` for sourced synthesis.
