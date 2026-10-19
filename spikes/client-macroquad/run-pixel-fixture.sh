#!/usr/bin/env bash
# Fixture pixel tổng hợp cho macroquad: sprite sinh bằng code, expected độc lập, 1×/2×.
# Không dùng asset game; ảnh sinh vào local/ (gitignored).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

fixture="local/pixel-fixture"
python3 spikes/client-macroquad/measurements/pixel_fixture.py --out "$fixture"

display="${DISPLAY_NUM:-:123}"
mkdir -p /tmp/.X11-unix
export LIBGL_ALWAYS_SOFTWARE=1
export __EGL_VENDOR_LIBRARY_FILENAMES="${__EGL_VENDOR_LIBRARY_FILENAMES:-/usr/share/glvnd/egl_vendor.d/50_mesa.json}"

Xvfb "$display" -screen 0 1400x1250x24 -nolisten tcp >/tmp/boom-spike-fixture-xvfb.log 2>&1 &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
sleep 1

for scale in 1 2; do
  actual="$fixture/actual-macroquad-${scale}x.png"
  DISPLAY="$display" cargo run --release --locked -p spike-macroquad \
    --manifest-path spikes/client-macroquad/Cargo.toml -- \
    --assets "$fixture" --snapshot "$actual" --time-ms 0 --scale "$scale" --no-audio
  if [ "$scale" -eq 1 ]; then
    pixels=312000
    hud=40
  else
    pixels=1248000
    hud=80
  fi
  python3 spikes/client-winit-softbuffer/tools/compare.py \
    --a "$fixture/expected-${scale}x.png" --b "$actual" \
    --hud "$hud" --json "$fixture/diff-macroquad-${scale}x.json" \
    --out "$fixture/diff-macroquad-${scale}x.png" \
    --fail-different-gt $((pixels / 50)) --fail-max-delta-gt 1
done

echo "pixel fixture macroquad PASS (1×/2×; ngưỡng maxΔ≤1, lệch≤2%)"
