#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${1:-$PROJECT_ROOT/data}"
THREADS="${2:-16}"
TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
[[ "$TARGET_DIR" = /* ]] || TARGET_DIR="$PROJECT_ROOT/$TARGET_DIR"
export CARGO_TARGET_DIR="$TARGET_DIR"
export RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}"
mkdir -p "$ROOT"
ROOT="$(cd "$ROOT" && pwd -P)"
cd "$PROJECT_ROOT"
cargo build --release
exec flock -n "$ROOT/search.lock" "$PROJECT_ROOT/scripts/supervise.sh" "$ROOT" "$THREADS" "$CARGO_TARGET_DIR/release/multi-pal-prime"
