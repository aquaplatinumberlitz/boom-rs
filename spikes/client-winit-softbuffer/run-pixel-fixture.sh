#!/usr/bin/env bash
# Fixture pixel tổng hợp cho winit+softbuffer: sprite sinh bằng code, expected độc lập, 1×/2×.
# Không dùng asset game; ảnh sinh vào local/ (gitignored).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

fixture="local/pixel-fixture"
python3 spikes/client-macroquad/measurements/pixel_fixture.py --out "$fixture"

for scale in 1 2; do
  actual="$fixture/actual-winit-${scale}x.png"
  cargo run --release --locked -p spike-winit-softbuffer \
    --manifest-path spikes/client-winit-softbuffer/Cargo.toml -- \
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
    --hud "$hud" --json "$fixture/diff-winit-${scale}x.json" \
    --out "$fixture/diff-winit-${scale}x.png" \
    --fail-different-gt $((pixels / 50)) --fail-max-delta-gt 1
done

echo "pixel fixture winit+softbuffer PASS (1×/2×; ngưỡng maxΔ≤1, lệch≤2%)"
