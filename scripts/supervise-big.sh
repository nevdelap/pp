#!/usr/bin/env bash
set -u
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT="$1"
U128_ROOT="$2"
THREADS="$3"
BIN="$4"
PYTHON_BIN="$5"
mkdir -p "$ROOT"
cd "$PROJECT_ROOT"
while true; do
  [ -e "$ROOT/STOP" ] && exit 0
  printf '{"event":"big_supervisor_start","timestamp_utc":"%s"}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$ROOT/supervisor.jsonl"
  "$PYTHON_BIN" "$PROJECT_ROOT/scripts/big-coordinator.py" "$BIN" "$ROOT" "$U128_ROOT" "$THREADS" >> "$ROOT/search.log" 2>&1
  STATUS=$?
  printf '{"event":"big_coordinator_exit","timestamp_utc":"%s","exit_code":%s}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$STATUS" >> "$ROOT/supervisor.jsonl"
  [ -e "$ROOT/STOP" ] && exit 0
  [ "$STATUS" -eq 0 ] && exit 0
  sleep 3
done
