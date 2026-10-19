#!/usr/bin/env bash
# Smoke test nạp/giải mã BGM+SFX của spike macroquad (không cần soundcard).
# Exit 0 chỉ khi đọc, kiểm header Ogg và decode được cả hai; lỗi -> exit 2.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

display="${DISPLAY_NUM:-:123}"
mkdir -p /tmp/.X11-unix

export LIBGL_ALWAYS_SOFTWARE=1
export __EGL_VENDOR_LIBRARY_FILENAMES="${__EGL_VENDOR_LIBRARY_FILENAMES:-/usr/share/glvnd/egl_vendor.d/50_mesa.json}"

Xvfb "$display" -screen 0 800x700x24 -nolisten tcp >/tmp/boom-spike-audio-xvfb.log 2>&1 &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
sleep 1

DISPLAY="$display" cargo run --release -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml -- --audio-smoke
