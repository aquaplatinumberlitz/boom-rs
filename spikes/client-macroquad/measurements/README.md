# Đo spike client stack (ADR 0010)

Số liệu ngày 2026-10-19, máy AMD Ryzen 5 3600 (6 core/12 thread), Linux, Rust 1.99.
Vòng sửa #3 (fixture pixel độc lập + copy shader) đã tái tạo snapshot/heatmap, số pixel và fixture;
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

# Fixture tổng hợp: sprite sinh bằng code, expected độc lập, so 1×/2× từng renderer
# (script tự sinh local/pixel-fixture, render và fail nếu maxΔ>1 hoặc lệch>2%)
./spikes/client-macroquad/run-pixel-fixture.sh
./spikes/client-winit-softbuffer/run-pixel-fixture.sh

# Audio smoke: đọc, kiểm header Ogg và decode BGM+SFX; không cần soundcard.
# Exit 0 chỉ khi cả hai OK; thiếu/hỏng file phải exit khác 0.
./spikes/client-macroquad/run-audio-smoke.sh
./spikes/client-winit-softbuffer/run-audio-smoke.sh

# Ca âm: bỏ/đổi tên SFX trong một bản copy asset rồi chạy --audio-smoke;
# macroquad phải exit 2 + "audio smoke FAIL"; winit phải exit 1 + tên file/lỗi.

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
| Binary Linux release | **2.634.304 B** (2,51 MiB) | 6.216.808 B (5,93 MiB) |
| Build Windows GNU release | 15,19 s | **13,50 s** |
| `.exe` Windows | 9.156.705 B | **7.966.962 B** |
| Crate transitive (Linux) | **53** | 117 |
| Dòng code game | **691** | 823 |
| Render CPU 1× | — (GPU) | 2,782 ms/frame |
| Render CPU 2× | — (GPU) | 7,926 ms/frame |

Thời gian build và số Windows/macOS đo tại head `66e91d7`; vòng sửa fixture/copy shader chỉ đo lại
binary/LOC Linux, pixel và audio. Không build lại Windows/macOS trên host này.

Pixel (bỏ HUD):

| Vùng | Pixel | Khác | Tỉ lệ | Max Δ |
|---|---|---|---|---|
| Scene 1× (600×520) | 312.000 | 69 | 0,022% | 1/255 |
| Scene 2× (1200×1040) | 1.248.000 | 276 | 0,022% | 1/255 |
| HUD chữ Việt 1× | 24.000 | 2.560 | 10,7% | 132/255 |

Toàn bộ 69 pixel khác ở scene 1× nằm trong vùng khói nổ bán trong suốt (bbox x 352–367,
y 279–292, frame explosion `#4` tại `--time-ms 300`), chỉ lệch 1 LSB. `--check-2x-from`
cho mismatch 0 trên cả hai stack: ảnh 2× là nearest expansion trực tiếp của 1×. Đây chỉ là kiểm scale;
pivot/alpha/z-order được kiểm bằng fixture độc lập bên dưới. HUD khác chủ yếu do khử răng cưa/vị trí
glyph; ảnh so cho thấy đủ glyph tiếng Việt, không clipping/placeholder.

## Fixture pixel tổng hợp (expected độc lập)

`pixel_fixture.py` sinh sprite **bằng code** (không dùng asset game): bốn ảnh 100×80 có pattern màu
và alpha 64/96/128/160/200/255; scene 4×4 với ba object chồng theo y (z-order) và anchor bất đối
xứng `[12,37]`, `[50,58]`, `[88,60]`. Expected do compositor Python riêng tính:
`out = (src·a + dst·(255-a) + 127) // 255`; additive được quy đổi trước nên fixture chỉ kiểm
straight alpha (hai spike không có blend additive per-draw).

| Stack | 1× khác | Max Δ | 2× khác | Max Δ |
|---|---:|---:|---:|---:|
| macroquad | 3.431/312.000 (1,10%) | 1 | 13.724/1.248.000 (1,10%) | 1 |
| winit+softbuffer | 0/312.000 | 0 | 0/1.248.000 | 0 |

Ngưỡng: **max Δ ≤ 1** và **số pixel lệch ≤ 2%**. Lý do: compositor dùng làm tròn integer
`+127/255`, GPU có thể lệch 1 LSB khi blend; Δ≥2 hoặc lệch >2% là lỗi hệ thống (pivot/layer/alpha).
Kiểm bằng `run-pixel-fixture.sh` của từng spike; script fail nếu vượt ngưỡng. Fixture này kiểm
implementation trên dữ liệu tổng hợp, **không phải** oracle fidelity KR.

Audio smoke: cả macroquad và winit+softbuffer **load/decode PASS** cho BGM (270.601 B) và SFX
(23.168 B); ca thiếu SFX: macroquad exit 2 + `audio smoke FAIL`, winit exit 1 + tên file/lỗi.
Playback/timing/âm lượng **chưa đo** vì
không có soundcard. Đây là khác biệt với claim cũ "nạp/khởi động không lỗi" suy từ process không lỗi.

## Giới hạn

- So hai renderer trên scene KR là **comparison/regression**, không phải oracle fidelity KR. Fixture
  tổng hợp đã kiểm độc lập pivot/alpha/z-order ở mức implementation (đạt ngưỡng ở trên), nhưng chưa
  đối chiếu asset/map/pivot Q-003 của client gốc; cần ảnh client gốc/decoded `i16.py` cục bộ
  (không commit ảnh game) nếu muốn kết luận fidelity.
- Macroquad cần copy shader tắt blend khi composite render target ra cửa sổ; nếu dùng blend mặc định,
  alpha target bị nhân đôi và làm tối vùng bán trong suốt (đã phát hiện qua fixture và sửa trong spike).
- Không đo GPU thật: macroquad chạy trên Xvfb + Mesa llvmpipe; frame time không đại diện GPU thật.
- Input-held/focus chưa đo; spike chỉ xử lý cạnh `B/S/R/Esc`.
- macOS mới `cargo check`; Windows build được nhưng chưa chạy trên Windows; gate build+run 3 OS chưa đạt.
- Không commit snapshot gốc vì chứa asset game; chỉ commit ảnh đo tổng hợp/heatmap.
