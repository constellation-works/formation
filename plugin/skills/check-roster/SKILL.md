---
name: check-roster
description: Check a proposed Formation shift roster for valid seats, workspace ownership and directed routes. Use when the operator supplies a roster file to validate.
---

1. Run `formation roster check --json < "$roster_file"` with the supplied file path.
2. Report the seat and route counts on success or the checker’s diagnostic on failure.

A valid draft grants no messaging or tool permissions. If the binary is unavailable, report that the Formation CLI is required.
