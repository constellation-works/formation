.PHONY: help build test goldens fmt fmt-check clippy deny structure standards-check docs-check feature-board ci-fast ci-lint ci install update-plugins clean

CARGO ?= cargo
# Every build uses the committed Cargo.lock as is; a stale lock fails instead
# of being rewritten (STD-05 R23).
LOCKED := --locked
BINARY := formation
INSTALL_BIN_DIR ?= $(HOME)/.cargo/bin
# `make goldens UPDATE=1` rewrites the golden files instead of comparing.
UPDATE ?=
# `STANDARDS_STRICT=1` makes a missing docs/standards/ a failure (CI sets it).
STANDARDS_STRICT ?=
# GitHub Actions sets CI=true. There a missing gate tool fails the gate;
# locally it warns and skips or falls back (see docs/development.md).
CI ?=

# $(call missing_tool,<tool>,<what it provides>): fail under CI, else warn.
define missing_tool
	if [ "$(CI)" = "true" ]; then \
		echo "error: $(1) is not installed; CI requires it for $(2)." >&2; exit 1; \
	fi; \
	echo "warning: $(1) not installed; continuing without $(2) (CI requires it)." >&2; \
	echo "         install: cargo install $(1) --locked" >&2
endef

help:
	@echo "formation make targets"
	@echo ""
	@echo "  make ci-fast          Pre-handoff gate, no compile: fmt, structure, docs and standards"
	@echo "  make ci-lint          clippy -D warnings + cargo-deny (skipped locally if not installed)"
	@echo "  make test             All tests (nextest when installed, else cargo test) + doctests"
	@echo "  make goldens          Compare --help and output goldens with the built binary"
	@echo "  make goldens UPDATE=1 Regenerate goldens after an intended surface change"
	@echo "  make standards-check  Run the vendored standards check (docs/standards/check.sh)"
	@echo "  make feature-board    Regenerate the design feature board"
	@echo "  make ci               Everything above; what CI runs"
	@echo "  make update-plugins   Install the Formation CLI and refresh its Codex and Claude plugins"
	@echo "  make build | fmt | install | clean"

build:
	$(CARGO) build --workspace $(LOCKED)

# ---------------------------------------------------------------------------
# Gates
# ---------------------------------------------------------------------------

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

# Source-only structure checks: dependency direction (STD-02 R1-R6), unit-test
# module reachability (STD-02 R19), terminal/stream ownership (STD-01 R17).
structure:
	./scripts/check-dependency-direction.sh
	./scripts/check-test-modules.sh
	./scripts/check-terminal-guard.sh
	sh scripts/check-private-task-ids.sh

clippy:
	$(CARGO) clippy --workspace --all-targets $(LOCKED) -- -D warnings

# Supply chain (STD-02 R23, STD-05 R23/R24): advisories, licenses, sources.
deny:
	@set -e; if $(CARGO) deny --version >/dev/null 2>&1; then \
		advisory_db="$${CARGO_HOME:-$$HOME/.cargo}/advisory-dbs"; \
		parent="$$advisory_db"; \
		while [ ! -e "$$parent" ] && [ ! -L "$$parent" ]; do parent=$$(dirname "$$parent"); done; \
		if [ -d "$$parent" ] && [ -w "$$parent" ] && [ -x "$$parent" ] \
			&& { [ ! -e "$$advisory_db/db.lock" ] || [ -w "$$advisory_db/db.lock" ]; }; then \
			$(CARGO) deny check; \
		else \
			mkdir -p target/cargo-deny; \
			config=$$(mktemp target/cargo-deny/config.XXXXXXXX); \
			trap 'rm "$$config"' EXIT; \
			awk '{print} /^\[advisories\]$$/ {print "db-path = \"target/cargo-deny/advisory-dbs\""}' deny.toml > "$$config"; \
			$(CARGO) deny check --config "$$config"; \
		fi; \
	else \
		$(call missing_tool,cargo-deny,the supply-chain check); \
	fi

# nextest enforces per-test timeouts (.config/nextest.toml, STD-03 R21/R22)
# and fails a selection that matches no test (STD-04 R10); plain cargo test
# does neither, so the fallback says so. nextest does not run doctests, so
# they are a second leg; the store crate carries at least one (STD-04 R14).
test:
	@if $(CARGO) nextest --version >/dev/null 2>&1; then \
		$(CARGO) nextest run --workspace --all-targets $(LOCKED) --no-tests=fail; \
	else \
		$(call missing_tool,cargo-nextest,per-test timeouts and the zero-test check); \
		$(CARGO) test --workspace --all-targets $(LOCKED); \
	fi
	$(CARGO) test --workspace --doc $(LOCKED)

# Goldens of the shipped surface (STD-01 R24): captured from the built binary.
goldens:
	@if $(CARGO) nextest --version >/dev/null 2>&1; then \
		UPDATE_GOLDENS=$(if $(UPDATE),1,0) $(CARGO) nextest run -p $(BINARY) --test goldens $(LOCKED) --no-tests=fail; \
	else \
		$(call missing_tool,cargo-nextest,the zero-test check); \
		UPDATE_GOLDENS=$(if $(UPDATE),1,0) $(CARGO) test -p $(BINARY) --test goldens $(LOCKED); \
	fi

# The constellation standards this repo adopted are vendored under
# docs/standards/ by operations/scripts/sync-standards.sh, which also writes
# check.sh. Until then this warns and passes, so a fresh copy of the template
# builds; with STANDARDS_STRICT=1 (set in CI) a missing vendor fails.
standards-check:
	@if [ -f docs/standards/check.sh ]; then \
		sh docs/standards/check.sh; \
	elif [ "$(STANDARDS_STRICT)" = "1" ]; then \
		echo "error: standards not vendored yet (no docs/standards/check.sh)." >&2; \
		echo "       from the constellation root: operations/scripts/sync-standards.sh --adopt STD-01,STD-02,STD-03,STD-04,STD-05,STD-06,STD-08,STD-09 <this repo>" >&2; \
		exit 1; \
	else \
		echo "warning: standards not vendored yet (no docs/standards/check.sh); skipping." >&2; \
		echo "         from the constellation root: operations/scripts/sync-standards.sh --adopt STD-01,STD-02,STD-03,STD-04,STD-05,STD-06,STD-08,STD-09 <this repo>" >&2; \
	fi

# Pre-handoff gate for agents: no compile, and every parity check CI runs
# that needs no build (STD-04 R12). Goldens need the binary, so they are
# their own target.
docs-check:
	python3 scripts/check-readme.py
	python3 scripts/generate-feature-board.py --check

feature-board:
	python3 scripts/generate-feature-board.py

ci-fast: fmt-check structure standards-check docs-check

ci-lint: clippy deny

# Full pass. Keep aligned with .github/workflows/ci.yml.
ci: ci-fast ci-lint test goldens

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

install:
	$(CARGO) build --release --bin $(BINARY) $(LOCKED)
	install -d "$(INSTALL_BIN_DIR)"
	install -m 755 target/release/$(BINARY) "$(INSTALL_BIN_DIR)/$(BINARY)"

update-plugins: install
	codex plugin marketplace add "$$PWD"
	codex plugin add formation@formation-local
	@set -eu; \
	plugin_id="formation@formation-local"; \
	marketplace_name="formation-local"; \
	installed_plugins="$$HOME/.claude/plugins/installed_plugins.json"; \
	checkout_sha=$$(git rev-parse HEAD); \
	marketplaces=$$(claude plugin marketplace list --json); \
	marketplace_present=$$(printf '%s\n' "$$marketplaces" | jq -r --arg name "$$marketplace_name" 'any(.[]; .name == $$name)'); \
	if [ "$$marketplace_present" = "true" ]; then \
		claude plugin marketplace update "$$marketplace_name"; \
	else \
		claude plugin marketplace add "$$PWD"; \
	fi; \
	plugins=$$(claude plugin list --json); \
	plugin_present=$$(printf '%s\n' "$$plugins" | jq -r --arg id "$$plugin_id" 'any(.[]; .id == $$id and .scope == "user")'); \
	if [ "$$plugin_present" = "true" ]; then \
		claude plugin update "$$plugin_id" --scope user; \
		installed_sha=$$(jq -r --arg id "$$plugin_id" '[.plugins[$$id][]? | select(.scope == "user") | .gitCommitSha // empty][0] // empty' "$$installed_plugins"); \
		if [ "$$installed_sha" != "$$checkout_sha" ]; then \
			echo "Claude plugin update left gitCommitSha at '$$installed_sha'; reinstalling from this checkout." >&2; \
			claude plugin uninstall "$$plugin_id" --scope user --keep-data; \
			claude plugin install formation --marketplace "$$PWD" --scope user; \
		fi; \
	else \
		claude plugin install formation --marketplace "$$PWD" --scope user; \
	fi; \
	plugin_record=$$(jq -c --arg id "$$plugin_id" '[.plugins[$$id][]? | select(.scope == "user")][0] // null' "$$installed_plugins"); \
	if [ "$$plugin_record" = "null" ]; then echo "error: Claude did not record $$plugin_id in user scope." >&2; exit 1; fi; \
	installed_sha=$$(printf '%s\n' "$$plugin_record" | jq -r '.gitCommitSha // empty'); \
	if [ "$$installed_sha" != "$$checkout_sha" ]; then echo "error: Claude plugin gitCommitSha is '$$installed_sha', expected checkout HEAD '$$checkout_sha'." >&2; exit 1; fi; \
	plugin_path=$$(printf '%s\n' "$$plugin_record" | jq -r '.installPath // empty'); \
	if [ -z "$$plugin_path" ] || [ ! -f "$$plugin_path/hooks/claude.json" ]; then echo "error: Claude plugin cache is missing hooks/claude.json." >&2; exit 1; fi

clean:
	$(CARGO) clean
