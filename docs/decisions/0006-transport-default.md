# 0006 — Transport mặc định V1: luồng tin cậy có thứ tự sau một trait

Status: **Accepted (mặc định có thể đảo)**, 2026-10-19. Trả lời phần lựa chọn của Q-010;
reconnect, mã hóa và vận hành vẫn thuộc task net/backend.

## Quyết định

- V1 dùng một kết nối TCP mỗi client, `TCP_NODELAY`, frame có độ dài tiền tố; TLS hoặc lớp mã hóa
  tương đương quyết ở task backend.
- Mọi code gameplay/phòng đi qua trait transport trong crate `boom-net` (chưa tạo); không gọi socket trực tiếp
  từ `boom-client`/`boom-server`.
- Input gửi kèm ack snapshot gần nhất và một cửa sổ input chưa ack (dư thừa có chủ đích), để khi đổi sang
  datagram không phải đổi contract.
- Application vẫn dedup theo sequence (SCOPE_V1 nghiệm thu #5–6), không dựa vào TCP.

## Lý do

≤ 4 người, dữ liệu nhỏ, cần nhanh có end-to-end. TCP cho reconnect/firewall đơn giản.
Rủi ro head-of-line khi mất gói được đo, không suy đoán.

## Điều kiện đảo

Chạy fixture loss 1% + jitter 30 ms: nếu correction/độ trễ input tại p95 vượt ngân sách do QA đặt
trong task V1-027 → chuyển sang transport datagram có reliability theo kênh, giữ nguyên contract
`InputCommand`/`Snapshot`. Ghi ADR mới kèm số đo.
