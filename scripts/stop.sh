#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-/workspace/data}"
PIDFILE="$ROOT/supervisor.pid"
if [ -f "$PIDFILE" ]; then
  PID="$(cat "$PIDFILE")"
  kill -TERM -- "-$PID" 2>/dev/null || true
  echo "sent TERM to supervisor $PID; committed partitions are safe and incomplete partitions will be retried"
else
  echo "no supervisor PID file at $PIDFILE" >&2; exit 1
fi
