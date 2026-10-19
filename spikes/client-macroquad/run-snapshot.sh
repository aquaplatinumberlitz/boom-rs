#!/usr/bin/env bash
# Render một snapshot xác định của spike macroquad trên Xvfb + Mesa llvmpipe.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

out="${1:-local/snap-macroquad-1x.png}"
scale="${2:-1}"
display="${DISPLAY_NUM:-:123}"

export LIBGL_ALWAYS_SOFTWARE=1
export __EGL_VENDOR_LIBRARY_FILENAMES="${__EGL_VENDOR_LIBRARY_FILENAMES:-/usr/share/glvnd/egl_vendor.d/50_mesa.json}"

Xvfb "$display" -screen 0 "$((600 * scale + 64))x$((560 * scale + 64))x24" -nolisten tcp \
  >/tmp/boom-spike-xvfb.log 2>&1 &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
sleep 1

DISPLAY="$display" cargo run --release -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml -- \
  --snapshot "$out" --time-ms 300 --scale "$scale" --no-audio
