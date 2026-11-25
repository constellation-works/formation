# Formation architecture

Formation starts from the [Constellation Rust CLI template at b8ea03c](https://github.com/constellation-works/rust-cli-template/tree/b8ea03c). The scaffold has one executable and two library crates.

| Layer | Current responsibility |
| --- | --- |
| `formation` | Parse the command, read bounded stdin and private seat identity files, call core, render the result and exit status. |
| `formation-core` | Check seat and message identifiers, validate rosters and coordinate bounded inbox waits. |
| `formation-store` | Decode the roster, store seat bindings and presence, and transact message sends and acknowledgments in SQLite. |

The executable depends on core, which depends on store. The template's [dependency gate](scripts/check-dependency-direction.sh) enforces the direction. Each room has its own local SQLite database. The CLI supplies a digest of the private identity to the store. Both host packages expose the roster and inbox skills.
