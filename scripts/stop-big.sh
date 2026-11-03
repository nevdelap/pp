#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-/workspace/data-bigint}"
PIDFILE="$ROOT/supervisor.pid"
[ -f "$PIDFILE" ] || { echo "no supervisor PID file at $PIDFILE" >&2; exit 1; }
touch "$ROOT/STOP"
PID="$(cat "$PIDFILE")"
kill -TERM -- "-$PID" 2>/dev/null || kill -TERM "$PID" 2>/dev/null || true
echo "requested stop for BigInt supervisor $PID; completed ranges remain checkpointed"
