# 0007 — Client stack: chọn bằng spike một scene V1

Status: **Superseded** bởi [0010](0010-client-stack-choice.md) (2026-10-19); trước đó Proposed. Chưa chọn thư viện; ADR này chốt cách chọn và hạn chọn.

## Ràng buộc

Sprite RLE alpha với pivot/canvas 48 byte, z-order và integer scale; chữ tiếng Việt Unicode;
âm thanh BGM + SFX theo event; input bàn phím có xử lý mất focus; build Windows/Linux/macOS;
vòng render tách khỏi tick mô phỏng. Không cần 3D, không cần editor.

## Phương án đưa vào spike

| Phương án | Điểm cần đo |
|---|---|
| Framework 2D nhẹ thuần Rust (kiểu macroquad) | Đủ kiểm soát vòng lặp, chất lượng text Việt, audio |
| Window + GPU tự dựng (winit + wgpu) + thư viện text/audio riêng | Chi phí tự viết renderer/UI so với độ chính xác pixel |
| Thư viện C đa nền (SDL3 binding) | Build/đóng gói 3 OS, chất lượng binding |
| Engine đầy đủ (Bevy) | Độ chồng chéo ECS với core, tốc độ thay đổi API, thời gian build |

Tra tài liệu chính thức và phiên bản hiện hành khi bắt đầu spike; không lấy phiên bản từ plan cũ.

## Spike và tiêu chí chọn

Một scene: Village00 tĩnh + C00 đi 4 hướng + một bom nổ + một dòng chữ Việt có dấu + BGM/SFX,
integer scale 1×/2×, build chạy được trên 3 OS. So: độ đúng pixel (pivot, alpha, z-order), độ dễ bảo trì,
thời gian build, kích thước bản phát hành. Tối đa hai phương án, mỗi phương án time-box 3 ngày.
Kết quả ghi thành ADR thay thế file này (Accepted), kèm ảnh và số đo.

Spike chỉ mở sau khi content closure Village00 (START-A/Q-002) đủ dựng scene; trước đó chỉ dùng ảnh decode.
