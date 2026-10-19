# Đo spike client stack (ADR 0010)

Số liệu ngày 2026-10-19, máy AMD Ryzen 5 3600 (6 core/12 thread), Linux, Rust 1.99.
Vòng sửa #2 (frame animation theo số + audio smoke) đã tái tạo snapshot/heatmap và số pixel;
thời gian build, số Windows/macOS giữ từ head `66e91d7` vì host này không build lại hai target đó.
Nguồn số: [`metrics.json`](metrics.json); ảnh:
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

# Kiểm nearest expansion trực tiếp cho từng stack (không suy từ 4× diện tích)
python3 spikes/client-winit-softbuffer/tools/compare.py \
  --a local/snap-macroquad-2x.png --b local/snap-macroquad-2x.png \
  --hud 80 --check-2x-from local/snap-macroquad-1x.png
python3 spikes/client-winit-softbuffer/tools/compare.py \
  --a local/snap-winit-2x.png --b local/snap-winit-2x.png \
  --hud 80 --check-2x-from local/snap-winit-1x.png

# Audio smoke: đọc, kiểm header Ogg và decode BGM+SFX; không cần soundcard.
# Exit 0 chỉ khi cả hai OK; thiếu/hỏng file phải exit khác 0.
./spikes/client-macroquad/run-audio-smoke.sh
./spikes/client-winit-softbuffer/run-audio-smoke.sh

# Ca âm: bỏ/đổi tên SFX trong một bản copy asset rồi chạy --audio-smoke;
# log phải ghi rõ "audio smoke FAIL" và process exit 2.

# Benchmark renderer CPU
cargo run --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml -- \
  --bench 300 --no-audio --scale 1

# Build (sau khi cargo clean target tương ứng)
time cargo build --release -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml
time cargo build --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml

# Windows GNU (full build, không chỉ check; số trong bảng từ head 66e91d7)
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
| Binary Linux release | **2.631.328 B** (2,51 MiB) | 6.216.616 B (5,93 MiB) |
| Build Windows GNU release | 15,19 s | **13,50 s** |
| `.exe` Windows | 9.156.705 B | **7.966.962 B** |
| Crate transitive (Linux) | **53** | 117 |
| Dòng code game | **591** | 798 |
| Render CPU 1× | — (GPU) | 2,782 ms/frame |
| Render CPU 2× | — (GPU) | 7,926 ms/frame |

Thời gian build và số Windows/macOS đo tại head `66e91d7`; vòng sửa frame/audio chỉ đo lại
binary/LOC Linux và pixel/audio. Không build lại Windows/macOS trên host này.

Pixel (bỏ HUD):

| Vùng | Pixel | Khác | Tỉ lệ | Max Δ |
|---|---|---|---|---|
| Scene 1× (600×520) | 312.000 | 143 | 0,046% | 17/255 |
| Scene 2× (1200×1040) | 1.248.000 | 572 | 0,046% | 17/255 |
| HUD chữ Việt 1× | 24.000 | 2.611 | 10,9% | 144/255 |

Toàn bộ 143 pixel khác ở scene 1× nằm trong vùng khói nổ bán trong suốt (bbox x 352–367,
y 278–292, frame explosion `#4` tại `--time-ms 300`); alpha blend GPU khác blit CPU. `--check-2x-from`
cho mismatch 0 trên cả hai stack: ảnh 2× là nearest expansion trực tiếp của 1×. Đây chỉ là kiểm scale,
không chứng minh pivot/alpha/z-order. HUD khác chủ yếu do khử răng cưa/vị trí glyph; ảnh so cho thấy
đủ glyph tiếng Việt, không clipping/placeholder.

Audio smoke: cả macroquad và winit+softbuffer **load/decode PASS** cho BGM (270.601 B) và SFX
(23.168 B); ca thiếu SFX exit 2 với log `audio smoke FAIL`. Playback/timing/âm lượng **chưa đo** vì
không có soundcard. Đây là khác biệt với claim cũ "nạp/khởi động không lỗi" suy từ process không lỗi.

## Giới hạn

- So hai renderer là **comparison/regression**, không phải oracle fidelity KR: hai bên dùng chung
  decoder, quy ước anchor/sort và chuyển additive→alpha, nên lỗi chung về pivot/layer/blend không bị bắt.
  Cần fixture expected pixel độc lập hoặc ảnh client gốc/decoded `i16.py` (chỉ so cục bộ, không commit
  ảnh game) trước khi kết luận đạt pivot/alpha/z-order.
- Không đo GPU thật: macroquad chạy trên Xvfb + Mesa llvmpipe; frame time không đại diện GPU thật.
- Input-held/focus chưa đo; spike chỉ xử lý cạnh `B/S/R/Esc`.
- macOS mới `cargo check`; Windows build được nhưng chưa chạy trên Windows; gate build+run 3 OS chưa đạt.
- Không commit snapshot gốc vì chứa asset game; chỉ commit ảnh đo tổng hợp/heatmap.
