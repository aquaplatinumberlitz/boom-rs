# Đo spike client stack (ADR 0010)

Số liệu ngày 2026-10-19, máy AMD Ryzen 5 3600 (6 core/12 thread), Linux, Rust 1.99.
Build release **target sạch**, registry đã warm; không tính thời gian tải crate.
Nguồn số: [`metrics.json`](metrics.json) (do các lệnh dưới sinh ra); ảnh:
[`build-size.png`](build-size.png), [`pixel-diff-scene-1x.png`](pixel-diff-scene-1x.png).

## Cách đo

```sh
# Asset dùng chung (không commit): pack KR -> local/asset-spike
cargo run -p prepare-assets \
  --manifest-path spikes/client-macroquad/Cargo.toml -- \
  --pack-dir /home/admin/boom-re/boomKR --out local/asset-spike

# Snapshot xác định: cùng scene, --time-ms 300, tắt audio
./spikes/client-macroquad/run-snapshot.sh local/snap-macroquad-1x.png 1
./spikes/client-winit-softbuffer/run-snapshot.sh local/snap-winit-1x.png 1
./spikes/client-macroquad/run-snapshot.sh local/snap-macroquad-2x.png 2
./spikes/client-winit-softbuffer/run-snapshot.sh local/snap-winit-2x.png 2

# So pixel + heatmap
python3 spikes/client-winit-softbuffer/tools/compare.py \
  --a local/snap-macroquad-1x.png --b local/snap-winit-1x.png \
  --hud 40 --out local/diff-1x.png --json local/diff-1x.json

# Benchmark renderer CPU
cargo run --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml -- \
  --bench 300 --no-audio --scale 1

# Build (sau khi cargo clean target tương ứng)
time cargo build --release -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml
time cargo build --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml

# Windows GNU (full build, không chỉ check)
cargo build --release --target x86_64-pc-windows-gnu -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml
cargo build --release --target x86_64-pc-windows-gnu -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml

# macOS: chỉ check được trên host Linux (thiếu SDK để link)
cargo check --target x86_64-apple-darwin -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml
cargo check --target x86_64-apple-darwin -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml
```

Sinh lại ảnh: `python3 spikes/client-macroquad/measurements/make_charts.py`
(script đọc `local/snap-*` và `metrics.json`; không chứa asset game trong repo).

## Kết quả

| Chỉ số | macroquad | winit+softbuffer |
|---|---|---|
| Build Linux release | **16,95 s** | 33,99 s |
| Binary Linux release | **2.631.744 B** (2,51 MiB) | 6.221.152 B (5,93 MiB) |
| Build Windows GNU release | 15,19 s | **13,50 s** |
| `.exe` Windows | 9.156.705 B | **7.966.962 B** |
| Crate transitive (Linux) | **53** | 117 |
| Dòng code game | **559** | 764 |
| Render CPU 1× | — (GPU) | 3,11 ms/frame |
| Render CPU 2× | — (GPU) | 8,56 ms/frame |

Pixel (bỏ HUD):

| Vùng | Pixel | Khác | Tỉ lệ | Max Δ |
|---|---|---|---|---|
| Scene 1× (600×520) | 312.000 | 127 | 0,041% | 17/255 |
| Scene 2× (1200×1040) | 1.248.000 | 508 | 0,041% | 17/255 |
| HUD chữ Việt 1× | 24.000 | 2.611 | 10,9% | 144/255 |

Toàn bộ 127 pixel khác ở scene 1× nằm trong vùng khói nổ bán trong suốt (alpha blend);
2× đúng 4× diện tích, chứng minh integer scale 1×/2× là nearest-neighbor, không lệch sprite.
HUD khác chủ yếu do khử răng cưa/position của GPU so với blit CPU; ảnh so cho thấy đủ glyph
tiếng Việt, không clipping/placeholder.

## Giới hạn

- Không đo GPU thật: macroquad chạy trên Xvfb + Mesa llvmpipe; số frame time của nó không
  đại diện GPU thật, nên bảng chỉ báo render CPU cho phương án tự dựng.
- Không đo âm thanh (máy không có soundcard); cả hai nạp/khởi động audio không lỗi nhưng
  không nghe được.
- macOS mới `cargo check`; Windows build được nhưng chưa chạy trên Windows.
- Không commit snapshot gốc vì chứa asset game; chỉ commit ảnh đo tổng hợp/heatmap.
