# Development

Use a Rust toolchain with Cargo, rustfmt and Clippy. The GitHub Actions workflow pins the CI toolchain and tool versions. The local gates also use jq, Python 3, cargo-nextest and cargo-deny.

Run `make ci` from the repository root. It checks formatting, crate dependencies, tests, stdout ownership, vendored standards, documentation, Clippy, dependency policy, CLI integration tests and output goldens. After an intentional help or output change, run `make goldens UPDATE=1` and review the generated files. Regenerate the design board with `make feature-board`. Cargo commands use the committed lockfile. CI runs on Linux and macOS.

The source-style integration test parses Rust files to check spacing, imports, names and line width alongside stable rustfmt and Clippy. Run it alone with `cargo test --locked -p formation --test source_style`. The store integration tests check names against the live SQLite schema.

Agent integration work targets `agent-main`. Release publication is a separate operator decision.
