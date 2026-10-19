#!/usr/bin/env bash
# Smoke test nạp/giải mã BGM+SFX của spike winit+softbuffer (không cần soundcard).
# Exit 0 chỉ khi đọc, kiểm header Ogg và decode được cả hai; lỗi -> exit 1/2.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo run --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml -- --audio-smoke
