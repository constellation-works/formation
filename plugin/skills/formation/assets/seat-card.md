# Seat card

Fill one card per seat from the day's values and paste it as the seat's first message. Omit any line the role does not use. Append role-specific lines (`order`, `task`, `done`, `subject`, `sources`, `specialists`, `roster`) when that role needs them. Keep `skill` as an absolute path to the directory containing `SKILL.md`.

```markdown
You are `<seat>`, <role> of `<ws>` in formation `<group>`.
Read the Formation skill, then `references/role-<role>.md`, and follow its First turn.

skill:        /abs/path/to/skills/formation
house rules:  /abs/path/to/house.md        (or: none)
unit:         <solo | duo | trio | squad | squad (repair mode off)> (<line-up>)
group / room: <group>
record:       <task id>, crew `<crew>`
workspace:    <hm_owner/ws_name>, repo <owner/name>, base <base_branch>
reports to:   <parent seat> (<session id>)
may message:  <allowed seat names and ids>
repair mode:  <on | off>
attention:    <yours (no resident) | resident <mandate id>>
handoff:      <two or three lines from the last group's summary>
open items:   <attention counts by kind; what waits for the admiral>
```
