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
export UV_CACHE_DIR="${UV_CACHE_DIR:-$PROJECT_ROOT/.uv-cache}"
export UV_PYTHON_INSTALL_DIR="$PROJECT_ROOT/.runtime/python"
command -v uv >/dev/null 2>&1 || { echo "uv is required to provision Python 3.12" >&2; exit 1; }
if ! PYTHON_BIN="$(uv python find 3.12 2>/dev/null)"; then
  uv python install 3.12
  PYTHON_BIN="$(uv python find 3.12)"
fi
export PYTHONDONTWRITEBYTECODE=1
mkdir -p "$ROOT" "$U128_ROOT"
ROOT="$(cd "$ROOT" && pwd -P)"
U128_ROOT="$(cd "$U128_ROOT" && pwd -P)"
cd "$PROJECT_ROOT"
cargo build --release
rm -f "$ROOT/STOP"
nohup setsid "$PROJECT_ROOT/scripts/supervise-big.sh" "$ROOT" "$U128_ROOT" "$THREADS" "$CARGO_TARGET_DIR/release/multi-pal-prime" "$PYTHON_BIN" </dev/null >> "$ROOT/launcher.log" 2>&1 &
echo $! > "$ROOT/supervisor.pid"
sleep 1
kill -0 "$(cat "$ROOT/supervisor.pid")"
echo "bigint_supervisor_pid=$(cat "$ROOT/supervisor.pid") root=$ROOT threads=$THREADS"
