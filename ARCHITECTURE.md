# Formation architecture

Formation starts from the [Constellation Rust CLI template at b8ea03c](https://github.com/constellation-works/rust-cli-template/tree/b8ea03c). The scaffold has one executable and two library crates.

| Layer | Current responsibility |
| --- | --- |
| `formation` | Parse the command, read bounded stdin, call core, render the result and exit status. |
| `formation-core` | Check seat uniqueness, workspace captain ownership and the endpoints of proposed routes. |
| `formation-store` | Decode the versioned roster document and define its exchange types. |

The executable depends on core, which depends on store. The template's [dependency gate](scripts/check-dependency-direction.sh) enforces the direction. The store currently works on bytes in memory. The plugin package exposes one shared roster-checking skill to its host. The [initial design](docs/design/formation/1_scope.md) describes the proposed persistence and adapter boundaries for messaging.
