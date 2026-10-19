#!/usr/bin/env bash
# Render một snapshot xác định của spike winit+softbuffer (offscreen, không cần X).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

out="${1:-local/snap-winit-1x.png}"
scale="${2:-1}"

cargo run --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml -- \
  --snapshot "$out" --time-ms 300 --scale "$scale" --no-audio
