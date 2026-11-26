# Formation agent guide

Formation is a standalone plugin for coordinating desktop and headless agent sessions.

## Starting from the template

Keep the template architecture as Formation grows. The CLI maps arguments and outcomes, core apps own decisions, the runtime and composition supply their context, and the store owns record access. Following the same path for a small command gives the next command a working example and avoids the repeated refactors we encountered in Scribe and Orbit. Trace changes against [ARCHITECTURE.md](ARCHITECTURE.md) and run the dependency gate before committing.

## Rules

- A roster check grants no routes. Qualify wake on the exact receiving host before exposing it as a capability.
- Start with the [Formation overview](docs/design/formation/1_overview.md) for roles, units and authority. Update the [shift-messaging scope](docs/design/shift-messaging/1_scope.md) when a proposed delivery capability becomes verified.

## Branching

- Integrate on `agent-main`.

## Gates

- Run the [development gates](docs/development.md) before handoff.

## Code

- Keep the [crate boundaries](ARCHITECTURE.md) intact.

<!-- constellation-standards:begin -->
<!-- Managed by the constellation's operations/scripts/sync-standards.sh; edits inside this block are overwritten. -->
## Constellation standards

This repository adopts these constellation standards, vendored read-only in `docs/standards/` (each a directory: `STD-nn.md` holds the rules, `why.md` and `checks.md` the reasons and gates):

- `STD-01@4` — [docs/standards/STD-01-cli-surface/STD-01.md](docs/standards/STD-01-cli-surface/STD-01.md): CLI surface — command grammar, inputs and effects, output modes, errors, help, and golden-tested public surfaces
- `STD-02@8` — [docs/standards/STD-02-rust-architecture-and-errors/STD-02.md](docs/standards/STD-02-rust-architecture-and-errors/STD-02.md): Rust architecture and errors — layering, visibility, typed errors, failure semantics, idiom and naming, logging, and…
- `STD-03@2` — [docs/standards/STD-03-concurrency-and-process-safety/STD-03.md](docs/standards/STD-03-concurrency-and-process-safety/STD-03.md): Concurrency and process safety — locks, channels, durable writes, state evolution, subprocess lifecycles, destructive…
- `STD-04@2` — [docs/standards/STD-04-testing-and-verification/STD-04.md](docs/standards/STD-04-testing-and-verification/STD-04.md): Testing and verification — test design, hermetic tests, honest gates, docs as contracts, and evidence-based…
- `STD-05@1` — [docs/standards/STD-05-security-boundaries/STD-05.md](docs/standards/STD-05-security-boundaries/STD-05.md): Security boundaries — authority, filesystem containment, child environments, redaction, local network surfaces, supply…
- `STD-06@2` — [docs/standards/STD-06-design-docs-and-feature-tracking/STD-06.md](docs/standards/STD-06-design-docs-and-feature-tracking/STD-06.md): Design docs and feature tracking — one folder per feature, a delivery stage separate from doc approval, a generated…
- `STD-08@3` — [docs/standards/STD-08-economy/STD-08.md](docs/standards/STD-08-economy/STD-08.md): Economy — bounded agent context, delta-only records, one channel per ask, bookkeeping off the page, one-line errors…
- `STD-09@3` — [docs/standards/STD-09-project-docs/STD-09.md](docs/standards/STD-09-project-docs/STD-09.md): Project docs — a shared structure for the README and common repository documentation
- `STD-11@3` — [docs/standards/STD-11-agent-context/STD-11.md](docs/standards/STD-11-agent-context/STD-11.md): Agent context files — skills and instruction files loaded by an agent harness
- `STD-12@1` — [docs/standards/STD-12-source-style/STD-12.md](docs/standards/STD-12-source-style/STD-12.md): Source style — names, vertical spacing, line width and the formatter gate, as a language-neutral core with one profile…

Follow them; they are normative. To deviate from a rule, record a decision in `docs/design/<feature>/4_decisions.md` citing `STD-nn@<version> §Rn`; never edit `docs/standards/` (`sh docs/standards/check.sh` enforces this).
Reviewers check every change against the adopted standards and report violations as `STD-nn §Rn` with file:line evidence.
<!-- constellation-standards:end -->
