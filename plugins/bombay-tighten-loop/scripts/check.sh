#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

production_lines="$(find crates -path '*/src/*.rs' -type f -print0 | xargs -0 wc -l | tail -1 | awk '{print $1}')"
boolean_tokens="$( (rg -n '\bbool\b' crates/*/src --glob '*.rs' || true) | wc -l | tr -d ' ')"
custom_error_impls="$( (rg -n 'impl([^\n]*)std::error::Error|impl([^\n]*)core::error::Error' crates/*/src --glob '*.rs' || true) | wc -l | tr -d ' ')"

score="$((1000000 - production_lines - (boolean_tokens * 100) - (custom_error_impls * 100)))"
printf 'SCORE: %s\n' "$score"
printf 'production_lines=%s boolean_tokens=%s custom_error_impls=%s\n' \
  "$production_lines" "$boolean_tokens" "$custom_error_impls"

nix flake check -L
