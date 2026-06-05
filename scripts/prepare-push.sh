#!/usr/bin/env bash
# Run fmt, clippy, and check before pushing.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
export RUSTFLAGS="${RUSTFLAGS:--Dwarnings}"

echo "==> cargo fmt --all"
cargo fmt --all

echo "==> cargo test --all"
cargo test --all

echo "==> cargo clippy --workspace --all-targets"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo check --workspace"
cargo check --workspace

echo "prepare-push: all checks passed"
