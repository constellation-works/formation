# Formation architecture

Formation follows the app layout in the [Constellation Rust CLI template](https://github.com/constellation-works/rust-cli-template/tree/e67dfde). The workspace pins the shared kit crates to the template's satellites revision. [The dependency gate](scripts/check-dependency-direction.sh) checks crate edges, module boundaries, schema ownership and the presence of the scaffold.

## Follow a message

1. The [executable](crates/formation/src/main.rs) captures the environment once. The [mail handler](crates/formation/src/commands/mail.rs) applies explicit paths and translates clap arguments into a typed app command. The output module owns JSON payloads, rendering and diagnostics.
2. The [mail app](crates/formation-core/src/apps/mail/mod.rs) enters through the [shared runtime](crates/formation-core/src/runtime/exec.rs). Composition resolves the room and identity path. The runtime supplies context, the clock and a guard that records the outcome of join, send and acknowledge commands. Its audit event contains command metadata and excludes message bodies and credentials.
3. The [send flow](crates/formation-core/src/apps/mail/send.rs) validates input, authenticates the seat and checks replay and recipient presence in the same write transaction that inserts the envelope. The inbox flow releases each snapshot before sleeping. Presence renewals for inbox and who calls are operational heartbeats.
4. The [store](crates/formation-store/src/store.rs) exposes snapshots and transactions. Each repository owns one record's queries and mappings. The SQLite driver opens the database through `cw-sqlite`; its [schema](crates/formation-store/src/driver/sqlite/schema/mod.rs) lists append-only steps from modules for seats, messages and audit events. The identity repository uses `cw-fs` to create and read private keys. Callers never receive a database connection.

The offline [roster app](crates/formation-core/src/apps/roster/mod.rs) follows the same dispatch boundary with no room or identity. Its record types live in the store's contracts module. A malformed roster or an offline check cannot create inbox state.

## Adding a command

Add arguments and presentation mapping in the CLI, a command, outcome and flow under the owning core app, and record operations in the appropriate store repository. Keep policy in the app and infrastructure in the shared kit. Run the dependency gate and exercise the command through its public entry point. [Development gates](docs/development.md)
