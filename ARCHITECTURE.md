# Formation architecture

Formation follows the app layout in the [Constellation Rust CLI template](https://github.com/constellation-works/rust-cli-template/tree/e67dfde). The workspace pins the shared kit crates to one satellites revision. [The dependency gate](scripts/check-dependency-direction.sh) checks crate edges, module boundaries, schema ownership and the presence of the scaffold.

## Follow a message

1. The [executable](crates/formation/src/main.rs) captures the environment once. The [mail handler](crates/formation/src/command/mail/mod.rs) applies `--room` and `--as` options and translates clap arguments into a typed app command. The output module owns JSON payloads, rendering and diagnostics.
2. The [mail app](crates/formation-core/src/app/mail/mod.rs) enters through the [shared runtime](crates/formation-core/src/runtime/exec.rs). Composition resolves the room (name or path), caller session id and seat. The runtime supplies context, the clock and a guard that records the outcome of join, send and acknowledge commands. Its audit event contains command metadata and excludes message bodies.
3. The [send flow](crates/formation-core/src/app/mail/send.rs) validates input, refreshes the sender, and resolves recipient membership, replay and `held` presence in the same write transaction that inserts the envelope. After committing new mail, ordinary send and the leased watch reader use the same [wake decision](crates/formation-core/src/app/wake.rs). The CLI launches the bounded queue command after the wake claim commits. The inbox flow releases each snapshot before sleeping. Presence renewals for inbox and who calls are operational heartbeats.
4. The [store](crates/formation-store/src/store.rs) exposes snapshots, transactions and session pointer files. Each repository owns one record's queries and mappings. The SQLite driver opens the database through `cw-sqlite`; its [schema](crates/formation-store/src/driver/sqlite/schema/mod.rs) lists append-only steps from modules for seats, messages and audit events. The session repository uses `cw-fs` to read and write private session files. A join from a provider session holds that session's advisory lock while it replaces the pointer and, if the join fails, puts it back. That lock is taken before the room database's write lock, and no path takes the two in the other order (STD-03 §R3). Callers never receive a database connection. Existing room files retain their published table names, seats and messages, and upgrade in place for compatibility with the original inbox format (STD-12 §R1).

The [roster app](crates/formation-core/src/app/roster/mod.rs) follows the same
dispatch boundary. Validation runs offline. List discovers rooms through
composition and opens a strict snapshot of each, without creating or updating
SQLite sidecars. Close and apply check the same room marker inside their
write transactions. Mail and roster use the same presence rule.

## Adding a command

Add arguments and presentation mapping in the CLI, a command, outcome and flow under the owning core app, and record operations in the appropriate store repository. Keep policy in the app and infrastructure in the shared kit. Run the dependency gate and exercise the command through its public entry point. [Development gates](docs/development.md)
