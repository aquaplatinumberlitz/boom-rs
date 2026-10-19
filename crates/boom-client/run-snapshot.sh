#!/usr/bin/env bash
# Snapshot xác định của boom-client trên Xvfb + Mesa llvmpipe (không cần asset game).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

out="${1:-local/boom-client-snapshot.png}"
scale="${2:-2}"
display="${DISPLAY_NUM:-:123}"

mkdir -p /tmp/.X11-unix
export LIBGL_ALWAYS_SOFTWARE=1
export __EGL_VENDOR_LIBRARY_FILENAMES="${__EGL_VENDOR_LIBRARY_FILENAMES:-/usr/share/glvnd/egl_vendor.d/50_mesa.json}"

Xvfb "$display" -screen 0 "$((600 * scale + 64))x$((560 * scale + 64))x24" -nolisten tcp \
  >/tmp/boom-client-xvfb.log 2>&1 &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
sleep 1

DISPLAY="$display" cargo run --release --locked -p boom-client -- \
  --snapshot "$out" --ticks 60 --scale "$scale"
