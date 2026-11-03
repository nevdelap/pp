#!/usr/bin/env bash
set -euo pipefail
FILE="${1:-/home/agentbox/www/pp-review.html}"
LOG="${2:-/tmp/pp-review-watch.jsonl}"
if [ ! -f "$FILE" ]; then echo "missing review file: $FILE" >&2; exit 1; fi
LAST="$(sha256sum "$FILE" | awk '{print $1}')"
printf '{"event":"watch_started","timestamp_utc":"%s","path":"%s","sha256":"%s"}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$FILE" "$LAST" >> "$LOG"
while true; do
  sleep 5
  CURRENT="$(sha256sum "$FILE" | awk '{print $1}')"
  if [ "$CURRENT" != "$LAST" ]; then
    printf '{"event":"file_changed","timestamp_utc":"%s","path":"%s","old_sha256":"%s","new_sha256":"%s"}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$FILE" "$LAST" "$CURRENT" >> "$LOG"
    LAST="$CURRENT"
  fi
done
