#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-/workspace/data-bigint}"
if [ -f "$ROOT/supervisor.pid" ] && kill -0 "$(cat "$ROOT/supervisor.pid")" 2>/dev/null; then echo "running pid=$(cat "$ROOT/supervisor.pid")"; else echo stopped; fi
if [ -f "$ROOT/final-report.json" ]; then cat "$ROOT/final-report.json"; else echo '{"status":"not_started"}'; fi
tail -n 8 "$ROOT/search.log" 2>/dev/null || true
