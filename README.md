<div align="center">
  <h1>Formation</h1>
  <p>Coordinate Claude and Codex sessions by role and workspace.</p>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Scaffold version 0.1.0"></a>
</div>

Formation is a standalone plugin in development. The chief and aide are the operator’s main contacts. Each captain owns a workspace and can work with headless sentries and workers. Seats can join a private local room, see who is online, send durable messages and check their inbox. A sentry can wait for mail on behalf of its parent.

## Quick start

From this checkout, with Rust and Cargo installed:

```sh
cargo run --locked -p formation -- roster check --json < examples/roster.json
```

The example returns `{"version":1,"seats":4,"routes":5}`. It checks names, ownership and the direction of each proposed route. A successful check grants no permissions and starts no sessions.

[Initial design and delivery criteria](docs/design/formation/1_scope.md) · [Inbox quick start](docs/usage.md#local-inbox) · [Roster format and plugin use](docs/usage.md) · [Crate boundaries](ARCHITECTURE.md) · [Development](docs/development.md) · [Feature board](docs/design/FEATURES.md) · [MIT license](LICENSE)
