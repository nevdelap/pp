#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${1:-$PROJECT_ROOT/data}"
TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
[[ "$TARGET_DIR" = /* ]] || TARGET_DIR="$PROJECT_ROOT/$TARGET_DIR"
PIDFILE="$ROOT/supervisor.pid"
if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then echo "running pid=$(cat "$PIDFILE")"; else echo stopped; fi
"$TARGET_DIR/release/multi-pal-prime" progress "$ROOT"
tail -n 3 "$ROOT/telemetry.jsonl" 2>/dev/null || true
