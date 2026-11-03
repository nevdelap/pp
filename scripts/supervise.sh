#!/usr/bin/env bash
set -u
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$1"
THREADS="$2"
BIN="$3"
while true; do
  printf '{"event":"supervisor_start","timestamp_utc":"%s"}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$ROOT/supervisor.jsonl"
  "$BIN" run "$ROOT" "$THREADS" >> "$ROOT/search.log" 2>&1 &
  WORKER="$!"
  "$PROJECT_ROOT/scripts/monitor.sh" "$ROOT" "$WORKER" >> "$ROOT/monitor.log" 2>&1 &
  MONITOR="$!"
  wait "$WORKER"
  STATUS=$?
  kill "$MONITOR" 2>/dev/null || true
  wait "$MONITOR" 2>/dev/null || true
  printf '{"event":"worker_exit","timestamp_utc":"%s","exit_code":%s}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$STATUS" >> "$ROOT/supervisor.jsonl"
  [ "$STATUS" -eq 0 ] && exit 0
  sleep 3
done
