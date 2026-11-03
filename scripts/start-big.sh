#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT="${1:-$PROJECT_ROOT/data-bigint}"
U128_ROOT="${2:-$PROJECT_ROOT/data}"
THREADS="${3:-$(nproc)}"
TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
[[ "$TARGET_DIR" = /* ]] || TARGET_DIR="$PROJECT_ROOT/$TARGET_DIR"
export CARGO_TARGET_DIR="$TARGET_DIR"
export RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}"
mkdir -p "$ROOT" "$U128_ROOT"
ROOT="$(cd "$ROOT" && pwd -P)"
U128_ROOT="$(cd "$U128_ROOT" && pwd -P)"
cd "$PROJECT_ROOT"
cargo build --release
rm -f "$ROOT/STOP"
nohup setsid "$PROJECT_ROOT/scripts/supervise-big.sh" "$ROOT" "$U128_ROOT" "$THREADS" "$CARGO_TARGET_DIR/release/multi-pal-prime" </dev/null >> "$ROOT/launcher.log" 2>&1 &
echo $! > "$ROOT/supervisor.pid"
sleep 1
kill -0 "$(cat "$ROOT/supervisor.pid")"
echo "bigint_supervisor_pid=$(cat "$ROOT/supervisor.pid") root=$ROOT threads=$THREADS"
