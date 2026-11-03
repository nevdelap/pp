#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-/workspace/data}"
find "$ROOT" -maxdepth 1 -name 'part-*.jsonl' -print0 | xargs -0 grep -h '"classification":"prime"' || true
