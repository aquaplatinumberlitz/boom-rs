# Spike client stack — macroquad

Spike cho [ADR 0010](../../docs/decisions/0010-client-stack-choice.md) (thay ADR 0007).
Đây **không** phải code production và **không** nằm trong workspace gốc; không merge vào `crates/`.

## Stack

| Thành phần | Phiên bản | Ghi chú |
|---|---|---|
| `macroquad` | 0.4.16, feature `audio` | framework 2D nhẹ; audio dùng `quad-snd` 0.2.8 |
| `image` | 0.25 (chỉ PNG) | nạp sprite đã decode |
| `serde` / `serde_json` | 1 | đọc `scene.json` |
| Rust | 1.99, edition 2024 | theo `rust-toolchain.toml` của repo |

Workspace riêng: `spikes/client-macroquad/Cargo.toml` (members `game`, `tools/prepare-assets`).
`cargo xtask ci` của repo gốc **không** build spike này.

## Chuẩn bị asset (không commit asset game)

`prepare-assets` đọc pack KR bằng `boom-assets` (PR #28), giải mã sprite `CDataI16Alpha` v2,
cắt từng frame theo frame record 48 byte rồi ghi PNG + `scene.json` + OGG vào `local/`
(đã gitignore). Tool không chép pack/asset vào repo.

```sh
# từ repo root; mặc định pack dir là /home/admin/boom-re/boomKR
cargo run -p prepare-assets \
  --manifest-path spikes/client-macroquad/Cargo.toml -- \
  --pack-dir /home/admin/boom-re/boomKR --out local/asset-spike
```

Nguồn entry: `FxMap/Map/Village00`, `FxTile/Tile00/*`, `FxCharacter/Character/C00-Walk*_ALayerStatic`,
`FxDesktop/Bomb/Bomb00`, `FxDesktop/Bomb/BombExplosion004`, `FxSound/MP3/Village`,
`FxSound/Wave/BombExplosion001`. Tên tile/object lấy từ `data/maps/village00-kr.toml`
(phân loại ô của #9), không đoán ID lạ.

Ghi chú kỹ thuật:

- Frame record 48 byte: `f0/f1` = registration/pivot của cell, `f3` = index ảnh,
  `f4..f7` = source rect; xem `docs/specification/assets/MAPS.md` §3. Ý nghĩa `f0/f1` vẫn là
  candidate (Q-003); spike dùng một quy ước trình diễn cố định để hai stack so sánh trên cùng dữ liệu.
- Sheet `BombExplosion004` gốc có nền đen đục và dùng additive. Tool chuyển sang alpha theo độ sáng
  (`alpha = max(r,g,b)`) vì macroquad không có blend per-draw đa nền tảng; đây là xử lý **trình diễn**
  của spike, không phải luật game.
- Frame animation được sắp theo **số** (`#0,#1,…,#10,…`), không theo thứ tự chuỗi; 300 ms chọn
  explosion `#4`. Snapshot/heatmap trong `measurements/` đã tái tạo sau khi sửa (xem PR #37 vòng 2).
- Không có asset dự phòng: thiếu pack/entry thì tool báo lỗi và dừng.

## Chạy

```sh
cargo run --release -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml
```

Phím: `S` đổi 1×/2×, `B` kích hoạt nổ + SFX, `R` reset, `Esc` thoát.
Font: đặt `BOOM_FONT` hoặc dùng font hệ thống (DejaVu Sans / Noto Sans / Segoe UI / Arial).
Audio: BGM `Village` lặp + SFX `BombExplosion001`. Khi audio bật, lỗi đọc/giải mã làm process
exit khác 0 (không còn nuốt thành `None`); chạy smoke test:

```sh
./spikes/client-macroquad/run-audio-smoke.sh
```

Script chỉ kiểm **load/decode** BGM+SFX (pass ở head này, log từng file); playback/timing/âm lượng
chưa đo vì không có soundcard.

## Snapshot xác định

```sh
# Linux headless: Xvfb + Mesa llvmpipe (script tự bật/tắt Xvfb)
./spikes/client-macroquad/run-snapshot.sh local/snap-macroquad-1x.png 1
./spikes/client-macroquad/run-snapshot.sh local/snap-macroquad-2x.png 2
```

Hoặc thủ công:

```sh
__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json \
LIBGL_ALWAYS_SOFTWARE=1 DISPLAY=:99 \
cargo run --release -p spike-macroquad \
  --manifest-path spikes/client-macroquad/Cargo.toml -- \
  --snapshot local/snap-macroquad-1x.png --time-ms 300 --scale 1 --no-audio
```

`--time-ms 300` cố định frame C00/bom/nổ để so pixel. `--no-audio` chỉ dùng cho snapshot.

## Kết quả đo (2026-10-19)

Máy: AMD Ryzen 5 3600 (6 core/12 thread), Linux, Rust 1.99. Build release sạch target,
registry đã warm.

| Chỉ số | macroquad | winit+softbuffer (tham chiếu) |
|---|---|---|
| Thời gian build Linux release | **16,95 s** | 33,99 s |
| Kích thước binary Linux release | **2.634.304 B** | 6.216.808 B |
| Thời gian build Windows GNU release | **15,19 s** | 13,50 s |
| Kích thước `.exe` Windows | 9.156.705 B | 7.966.962 B |
| Số crate transitive (Linux) | **53** | 117 |
| Dòng code game | **691** | 823 |

Thời gian build và số Windows/macOS đo tại head `66e91d7`; vòng sửa fixture/copy shader chỉ đo lại
binary/LOC Linux, pixel và audio.

So pixel scene (bỏ HUD 40 px; 600×520 ở 1×, 1200×1040 ở 2×) với snapshot
winit+softbuffer: 69/312.000 pixel khác (0,022%), sai khác lớn nhất 1/255, toàn bộ nằm ở
vùng khói nổ bán trong suốt (explosion `#4`). `--check-2x-from` cho mismatch 0 trên cả hai stack:
ảnh 2× là nearest expansion trực tiếp của 1×, nhưng đây chỉ là kiểm scale. HUD chữ Việt:
2.560/24.000 pixel khác (chủ yếu khử răng cưa/vị trí glyph, max 132/255); không thiếu glyph.

Fixture pixel tổng hợp (`run-pixel-fixture.sh`, sprite sinh bằng code, expected độc lập) kiểm
pivot/alpha/z-order: macroquad lệch **3.431/312.000 (1,10%) chỉ 1 LSB** ở 1×, **13.724/1.248.000**
ở 2×; winit khớp tuyệt đối. Ngưỡng maxΔ≤1 và lệch≤2%; chi tiết ở
[`measurements/README.md`](measurements/README.md) và ADR 0010 §4.6. Fixture cũng phát hiện
render target alpha bị nhân đôi khi composite; spike sửa bằng copy shader tắt blend ở bước cuối.

So hai stack trên scene KR vẫn là **comparison/regression**, không phải oracle fidelity KR; xem
ma trận tiêu chí ADR 0010 §4.5 và [`measurements/`](measurements/README.md).

## Build 3 OS

```sh
# Linux
cargo build --release -p spike-macroquad --manifest-path spikes/client-macroquad/Cargo.toml

# Windows GNU (đã build thành công .exe)
cargo build --release --target x86_64-pc-windows-gnu \
  -p spike-macroquad --manifest-path spikes/client-macroquad/Cargo.toml

# macOS: check type/compile
cargo check --target x86_64-apple-darwin \
  -p spike-macroquad --manifest-path spikes/client-macroquad/Cargo.toml
```

Trên máy Linux này không có SDK macOS nên `cargo build` cho macOS dừng ở linker
(`cc: unrecognized command-line option '-arch'`); cần runner macOS thật để link/chạy.
Windows đã build được nhưng chưa chạy thử trên Windows.

## Giới hạn của spike

- Scene tĩnh Village00 + một C00 + một bom/nổ + HUD; không có gameplay, mạng, collision,
  animation state machine, hay asset closure đầy đủ.
- Quy ước pivot/anchor là presentation candidate (Q-003), không phải luật KR đã xác nhận. Fixture
  tổng hợp độc lập đã kiểm pivot/alpha/z-order ở mức implementation (macroquad lệch ≤1 LSB), nhưng
  chưa đối chiếu asset/map/pivot của client gốc.
- Macroquad cần copy shader tắt blend khi composite render target ra cửa sổ; nếu dùng blend mặc định,
  alpha target bị nhân đôi và làm tối vùng bán trong suốt (đã sửa trong spike).
- Audio mới kiểm load/decode; playback/timing chưa đo vì không có soundcard. Không đo GPU thật
  (chỉ llvmpipe trong Xvfb).
- Input-held/focus chưa đo; Windows mới build, macOS mới `check`.
- Không có asset game trong repo; mọi ảnh/âm thanh sinh từ pack vào `local/`.
- Không merge code spike vào `crates/`; quyết định stack ở ADR 0010.
