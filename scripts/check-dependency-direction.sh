#!/usr/bin/env bash
# scripts/check-dependency-direction.sh — STD-02 R1–R6, R16, R17 for this workspace.
#
# Two layers of checks, neither of which compiles, so the gate runs in
# `make ci-fast`:
#
#   1. Crate edges, read from Cargo's own package graph
#      (`cargo metadata --no-deps --locked`), which names each dependency by
#      its package, so a quoted key or a renamed dependency
#      (`files = { package = "cw-fs" }`) is seen for what it is. Every
#      workspace crate needs a policy below. A normal or build dependency may
#      name only the internal crates in ALLOWED and the cw-kit crates (`cw-*`,
#      from the satellites repository) in KIT, and never an external crate in
#      BANNED. A dev-dependency may name the kit crates in KIT and KIT_DEV;
#      other dev edges are free. A crate without a policy fails the check, so
#      a new member forces a decision. Needs cargo and jq.
#   2. Module edges inside a crate, as grep and grouped-import bans over
#      non-test sources.
#
# The layer order is written down in ARCHITECTURE.md; change it and this
# script in the same commit (STD-02 R6).
set -euo pipefail
cd "$(dirname "$0")/.."

PREFIX="formation"
KIT_PREFIX="cw"
fail=0
err() { echo "dependency-direction: $*" >&2; fail=1; }

# --- 1. crate policies -------------------------------------------------------
# policy <crate>  -> sets ALLOWED (internal deps), KIT (cw-kit deps),
# KIT_DEV (cw-kit crates allowed as dev-dependencies only) and BANNED
# (external deps)
policy() {
  case "$1" in
    formation-store)
      ALLOWED=""
      # The one error type (cw-base) and file access for the data directory
      # (cw-fs). The store is the only crate that names cw-fs.
      KIT="cw-base cw-fs"
      # Test isolation, for its unit tests' private temp roots.
      KIT_DEV="cw-testkit"
      # Persistence never parses arguments, draws on a terminal or installs a
      # log subscriber: those belong to surfaces. It reaches files only
      # through cw-fs, so it needs no file crate.
      BANNED="clap tracing-subscriber anstream anstyle termcolor terminal_size crossterm owo-colors colored tempfile rustix"
      ;;
    formation-core)
      ALLOWED="formation-store"
      # The one error type only: the apps reach files through the store.
      KIT="cw-base"
      # Its integration tests match the kit's file error behind a store
      # failure (`cw_fs::Error::Locked`), so cw-fs is a dev edge only; its
      # sources never name it (module rules below).
      # cw-testkit: test isolation, for its tests' private temp roots.
      KIT_DEV="cw-fs cw-testkit"
      # The apps reach files only through the store, so they need no file or
      # process crate either.
      BANNED="clap tracing-subscriber anstream anstyle termcolor terminal_size crossterm owo-colors colored tempfile rustix"
      ;;
    formation)
      # The binary names only the core; the core re-exports the record shapes
      # it renders and the one error type (cw-base's, as formation_core::Error).
      ALLOWED="formation-core"
      KIT=""
      # Test isolation: the temp root, the isolated child and goldens.
      KIT_DEV="cw-testkit"
      BANNED=""
      ;;
    *) return 1 ;;
  esac
}

contains() { # contains <word> <space-separated list>
  local word="$1" item
  for item in $2; do [[ "$item" == "$word" ]] && return 0; done
  return 1
}

# check_crate_edges: layer 1, on Cargo's package graph. Exits 2 when the
# graph cannot be read, so a broken manifest never passes as "no edges".
check_crate_edges() {
  local tool metadata members edges crate kind dep
  for tool in cargo jq; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      echo "dependency-direction: cannot run: $tool is not installed" >&2
      exit 2
    fi
  done
  if ! metadata=$(cargo metadata --format-version 1 --no-deps --locked 2>/dev/null); then
    echo "dependency-direction: cannot run: cargo metadata failed" >&2
    exit 2
  fi
  # One line per workspace package: its name.
  members=$(jq -er '.packages[].name' <<<"$metadata") || {
    echo "dependency-direction: cannot run: cargo metadata gave no package list" >&2
    exit 2
  }
  # One line per edge: "<crate> <kind> <package>", kind normal, build or dev.
  edges=$(jq -er '.packages[] | .name as $crate
    | .dependencies[] | "\($crate) \(.kind // "normal") \(.name)"' <<<"$metadata") || edges=""
  while IFS= read -r crate; do
    policy "$crate" || err "crate '$crate' has no policy; add one here and a row in ARCHITECTURE.md"
  done <<<"$members"
  while read -r crate kind dep; do
    [[ -n "$crate" ]] || continue
    policy "$crate" || continue
    if [[ "$kind" == "dev" ]]; then
      if [[ "$dep" == "$KIT_PREFIX"-* ]]; then
        contains "$dep" "$KIT $KIT_DEV" ||
          err "$crate must not depend on kit crate $dep, even for tests (allowed: ${KIT:-none} ${KIT_DEV})"
      fi
    elif [[ "$dep" == "$PREFIX" || "$dep" == "$PREFIX"-* ]]; then
      contains "$dep" "$ALLOWED" || err "$crate must not depend on internal crate $dep"
    elif [[ "$dep" == "$KIT_PREFIX"-* ]]; then
      contains "$dep" "$KIT" || err "$crate must not depend on kit crate $dep (allowed: ${KIT:-none})"
    elif contains "$dep" "$BANNED"; then
      err "$crate must not depend on $dep (surface crate in the domain)"
    fi
  done <<<"$edges"
}

# --- 2. module bans ----------------------------------------------------------
# ban <path> <extended-regex> <message>: fail if the pattern appears in
# non-test, non-comment lines of the Rust sources under <path> (a file or a
# directory).
ban() {
  local hits
  hits=$(grep -rnHE --include='*.rs' --exclude-dir=tests "$2" "$1" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' || true)
  if [[ -n "$hits" ]]; then
    echo "$hits" >&2
    err "$3"
  fi
}

# ban_import <path> <prefix> <members_regex> <message>: fail if a grouped
# `use <prefix>::{ ... }` (single- or multi-line) imports any direct member
# matching <members_regex> in non-test, non-comment Rust sources.
ban_import() {
  local target="$1" prefix="$2" members="$3" msg="$4"
  local files=() hits
  while IFS= read -r f; do
    [[ -n "$f" ]] && files+=("$f")
  done < <(find "$target" -name '*.rs' -not -path '*/tests/*' 2>/dev/null | sort)
  [[ ${#files[@]} -eq 0 ]] && return 0
  hits=$(awk -v prefix="$prefix" -v members="$members" '
    FNR == 1 { in_use = 0; depth = 0; skip_next = 0; skip_rest = 0 }
    {
      line = $0
      sub(/\/\/.*/, "", line)
      if (!in_use) {
        if (line ~ "^[[:space:]]*(pub(\\([^)]*\\))?[[:space:]]+)?use[[:space:]]+(::)?" prefix "::\\{") {
          in_use = 1
          depth = 0
          skip_next = 0
          skip_rest = 0
          sub("^[[:space:]]*(pub(\\([^)]*\\))?[[:space:]]+)?use[[:space:]]+(::)?" prefix "::", "", line)
        } else {
          next
        }
      }
      token = ""
      n = length(line)
      for (i = 1; i <= n; i++) {
        c = substr(line, i, 1)
        if (c == "{") {
          if (depth == 1 && token != "" && !skip_next && !skip_rest && token ~ "^(" members ")$") {
            printf "%s:%d:%s\n", FILENAME, FNR, $0
          }
          token = ""
          depth++
        } else if (c == "}") {
          if (depth == 1 && token != "" && !skip_next && !skip_rest && token ~ "^(" members ")$") {
            printf "%s:%d:%s\n", FILENAME, FNR, $0
          }
          token = ""
          depth--
          if (depth <= 0) {
            in_use = 0
            break
          }
        } else if (depth == 1 && !skip_rest && c ~ /[A-Za-z0-9_]/) {
          token = token c
        } else {
          if (depth == 1 && token != "") {
            if (!skip_next && !skip_rest && token ~ "^(" members ")$") {
              printf "%s:%d:%s\n", FILENAME, FNR, $0
            }
            if (token == "as") {
              skip_next = 1
            } else {
              skip_next = 0
            }
            token = ""
          }
          if (depth == 1 && c == ":") {
            skip_rest = 1
          } else if (depth == 1 && c == ",") {
            skip_rest = 0
            skip_next = 0
          }
        }
      }
      if (in_use && depth == 1 && token != "" && !skip_next && !skip_rest && token ~ "^(" members ")$") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0
      }
    }
  ' "${files[@]}" || true)
  if [[ -n "$hits" ]]; then
    echo "$hits" >&2
    err "$msg"
  fi
}

STORE=crates/formation-store/src
CORE=crates/formation-core/src
# The store decodes bytes and the core validates the draft. Host I/O lives in
# the CLI. Check grouped imports as well as fully qualified names.
for layer in "$STORE" "$CORE"; do
  ban "$layer" 'std::(fs|net|process|env)|tempfile::|rustix::|cw_fs\b' "roster libraries must remain independent of host I/O"
  ban_import "$layer" 'std' 'fs|net|process|env' "roster libraries must remain independent of host I/O"
done
ban crates 'cw_testkit' "cw-testkit is restricted to test code"

# Crate edges last: the module bans above run even where Cargo cannot.
check_crate_edges

if [[ "$fail" -eq 0 ]]; then
  echo "dependency-direction: ok"
fi
exit "$fail"
