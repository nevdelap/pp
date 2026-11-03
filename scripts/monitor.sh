#!/usr/bin/env bash
set -u
ROOT="$1"
SEARCH_PID="$2"
PIDFILE="$ROOT/supervisor.pid"
OUT="$ROOT/system_metrics.jsonl"
HZ="$(getconf CLK_TCK 2>/dev/null || echo 100)"
while [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null && kill -0 "$SEARCH_PID" 2>/dev/null; do
  TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  read -r UTIME STIME < <(awk '{print $14,$15}' "/proc/$SEARCH_PID/stat")
  RSS="$(ps -p "$SEARCH_PID" -o rss= | tr -d ' ')"
  CPU="$(awk -v t="$((UTIME+STIME))" -v h="$HZ" 'BEGIN {printf "%.3f",t/h}')"
  PCT="$(ps -p "$SEARCH_PID" -o pcpu= | tr -d ' ')"
  printf '{"timestamp_utc":"%s","pid":%s,"cumulative_cpu_seconds":%s,"cpu_percent_since_start":%s,"rss_kib":%s}\n' "$TS" "$SEARCH_PID" "$CPU" "$PCT" "${RSS:-0}" >> "$OUT"
  sleep 60
done
