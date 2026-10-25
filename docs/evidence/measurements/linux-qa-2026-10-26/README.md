# Nghiệm thu Linux V1-036 — gói phát hành trên máy thật (2026-10-26)

Phạm vi theo chỉ đạo chủ dự án 25/10: **Linux trước**, cấu hình gọn, không benchmark; Windows/macOS hoãn.
Không kèm ảnh/bundle/asset game trong repo (ảnh chụp chỉ giữ local).

## Môi trường
- Máy tham chiếu RELEASE §"Cấu hình Linux gọn" (Ryzen 5 3600, 32 GB, RTX 3060), Wayland + XWayland,
  PipeWire (PulseAudio 1.6.9), soundcard thật. Không phải minimum hardware.
- Mã nguồn: `main` `9085a0c` (sau #306, #308). Build `cargo build --release` client/server/assets.

## Lần 1 (gói từ `21a0496`, trước sửa) — phát hiện #305
| Bước | Kết quả |
|---|---|
| pack → check → tar.gz → unpack → smoke 10 ca | đạt |
| `boom-server serve` từ gốc gói theo README | **lỗi**: `content.load_fail map_profile_load_failed` (gói thiếu `data/maps/village00-kr.toml`) |
| 2 client `--connect` theo README | **lỗi**: `ContentMismatch` (client chỉ đọc `BOOM_CONTENT_SHA256`) — thoát trước khi mở cửa sổ |
| Có cửa sổ/âm thanh (đặt tạm env + map) | chủ dự án xác nhận bằng mắt: có 2 cửa sổ + tiếng; **nhân vật giật, sprite/animation thiếu** |

Nguyên nhân và sửa: [#305](https://github.com/aquaplatinumberlitz/boom-rs/issues/305) — PR #306 (gói kèm
`data/maps`, handshake dùng `content_hash` của bundle, README quickstart), PR #308 (nhân vật vẽ theo pixel,
đủ hai lớp `static`+`adjust`, neo theo offset f0/f1, z-order theo y chân).

## Lần 2 (gói từ `9085a0c`, sau sửa)
| Bước | Kết quả |
|---|---|
| pack → `check_package check` (12 file, không pack/bundle/asset game) → tar.gz (SHA-256 `784a3fa9…c776f`) → unpack → smoke 10 ca | đạt |
| Dựng bundle theo README vào `data/bundle/village00` (`content_hash` `74903a59…5003`) | đạt |
| `bin/boom-server serve` mặc định từ gốc gói | đạt (`server.start`, `content.load_ok`) |
| 2 client `--connect --register --auto-ready`, **không biến môi trường** | đạt: `auth.register`×2, `round.loading`×2, `round.start`, `round.playing`; không lỗi bắt tay/panic |
| Ảnh client trong gói (`--demo --snapshot`, local) | nhân vật C00/C01 đủ thân, đứng đúng ô, che đúng theo y chân (đối chiếu VN `battle_village01`) |

## Giới hạn
- `--snapshot` đôi khi trả ảnh trắng ở lần chạy đầu (1/2 lần, cùng binary) — lỗi chụp của công cụ QA,
  không phải hình chơi; theo dõi riêng.
- Nhịp đổi frame animation vẫn DESIGN 100 ms (R-118); nhịp gốc đang đo ở #311.
- Không đo minimum hardware/FPS; Windows/macOS hoãn (#300) ⇒ V1-036 toàn phần và G6 **chưa** ký.
