# Spike client stack — winit + softbuffer + fontdue

Spike cho [ADR 0010](../../docs/decisions/0010-client-stack-choice.md) (thay ADR 0007).
Phương án "tự dựng window/renderer/text" chạy CPU framebuffer, không dùng GPU.
**Không** phải code production, **không** nằm trong workspace gốc, không merge vào `crates/`.

## Stack

| Thành phần | Phiên bản | Ghi chú |
|---|---|---|
| `winit` | 0.30 | cửa sổ/event loop |
| `softbuffer` | 0.4 | trình bày framebuffer CPU |
| `fontdue` | 0.9 | rasterize chữ Việt |
| `quad-snd` | 0.2.8 | audio dùng chung với spike macroquad để cô lập phần render/text |
| `image` | 0.25 (chỉ PNG) | nạp sprite |
| `serde` / `serde_json` | 1 | đọc `scene.json` |
| Rust | 1.99, edition 2024 | |

Workspace riêng: `spikes/client-winit-softbuffer/Cargo.toml` (member `game`).

## Chuẩn bị asset

Dùng chung asset với spike macroquad (cùng `scene.json`):

```sh
cargo run -p prepare-assets \
  --manifest-path spikes/client-macroquad/Cargo.toml -- \
  --pack-dir /home/admin/boom-re/boomKR --out local/asset-spike
```

Không commit asset; `local/` đã gitignore. Chi tiết decode/ghi chú ở
[README của spike macroquad](../client-macroquad/README.md).

## Chạy

```sh
cargo run --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml
```

Phím: `S` đổi 1×/2×, `B` kích hoạt nổ + SFX, `R` reset, `Esc` thoát.
Font: `BOOM_FONT` hoặc font hệ thống. Audio im lặng nếu không có thiết bị.

## Snapshot và đo

```sh
./spikes/client-winit-softbuffer/run-snapshot.sh local/snap-winit-1x.png 1
./spikes/client-winit-softbuffer/run-snapshot.sh local/snap-winit-2x.png 2
```

Snapshot chạy hoàn toàn offscreen (không cần X/Wayland). So hai snapshot:

```sh
python3 spikes/client-winit-softbuffer/tools/compare.py \
  --a local/snap-macroquad-1x.png --b local/snap-winit-1x.png \
  --hud 40 --out local/diff-1x.png
```

Benchmark renderer (release, cùng scene, không tính cửa sổ):

```sh
cargo run --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml -- \
  --bench 300 --no-audio --scale 1
```

## Kết quả đo (2026-10-19)

Máy: AMD Ryzen 5 3600 (6 core/12 thread), Linux, Rust 1.99. Build release sạch target,
registry đã warm.

| Chỉ số | winit+softbuffer | macroquad (tham chiếu) |
|---|---|---|
| Build Linux release | 33,99 s | **16,95 s** |
| Binary Linux release | 6.221.152 B | **2.631.744 B** |
| Build Windows GNU release | **13,50 s** | 15,19 s |
| `.exe` Windows | **7.966.962 B** | 9.156.705 B |
| Crate transitive (Linux) | 117 | **53** |
| Dòng code game | 764 | **559** |
| Render CPU 1× (600×560) | 3,32 ms/frame | — (GPU) |
| Render CPU 2× (1200×1120) | 8,85 ms/frame | — (GPU) |

So pixel scene với macroquad: 127/312.000 pixel khác ở 1× (0,041%), lớn nhất 17/255,
toàn bộ ở khói nổ bán trong suốt; 2× là 508/1.248.000. HUD chữ Việt: 2.611/24.000 pixel khác (chủ yếu khử răng cưa/vị trí glyph, max 144/255);
không thiếu glyph.

Chi tiết/ảnh ở [`../client-macroquad/measurements/`](../client-macroquad/measurements/README.md).

## Build 3 OS

```sh
# Linux
cargo build --release -p spike-winit-softbuffer \
  --manifest-path spikes/client-winit-softbuffer/Cargo.toml

# Windows GNU (đã build thành công .exe)
cargo build --release --target x86_64-pc-windows-gnu \
  -p spike-winit-softbuffer --manifest-path spikes/client-winit-softbuffer/Cargo.toml

# macOS: check compile (không link được trên host Linux thiếu SDK)
cargo check --target x86_64-apple-darwin \
  -p spike-winit-softbuffer --manifest-path spikes/client-winit-softbuffer/Cargo.toml
```

Windows đã build được nhưng chưa chạy thử trên Windows; macOS mới `check`, cần runner macOS
thật để link/chạy. `winit` + `softbuffer` + `quad-snd` đều qua `cargo check` cho cả hai target.

## Giới hạn

- Renderer CPU tự viết: blit RGBA straight-alpha, text layout không shaping/ligature (tiếng Việt
  precomposed vẫn đủ dấu), không có GPU effect/blend mode.
- Audio dùng chung `quad-snd` với macroquad nên spike này **không** so sánh backend audio;
  nó so chi phí render/text/UI tự dựng.
- Quy ước pivot/anchor là presentation candidate (Q-003), không phải luật KR.
- Không có gameplay/mạng/collision; không commit asset game; không merge vào `crates/`.
