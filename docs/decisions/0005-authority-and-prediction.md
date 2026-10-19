# 0005 — Phân quyền quyết định và phạm vi prediction V1

Status: **Accepted (mặc định có thể đảo)**, 2026-10-19. Trả lời phần chiến lược của Q-008/Q-010;
codec và thông số cửa sổ input vẫn mở.

## Bối cảnh — client gốc là mô hình lai

Bằng chứng VN runtime/static ([VN battle runtime](../research/findings/VN_BATTLE_RUNTIME_REFERENCE.md),
`boom-re/docs/oc_e-game.md` §5a):

| Hành vi | Bên quyết định trong legacy |
|---|---|
| Di chuyển | Client; không có gói C→S cho di chuyển |
| Đặt bom | Client gửi 111; server cấp ID bom (152) |
| Thời điểm nổ | **Server** gửi 151; client vẽ vụ nổ khi nhận |
| Vỡ gạch | Client tự tính, báo 135 |
| Item trong gạch | **Server** gửi trước trận (553) |
| Nhặt item | Client xin (117), **server** xác nhận (158) |
| Bị nhốt, chết, cứu, chọc | Client tự tính, báo 120/11, 120/13, 120/12 |
| Vị trí người chơi khác | P2P qua UDP giữa các client (bảng peer trong 146) |

Vì vậy V1 server-authoritative không xa bản gốc như tưởng: fuse và item vốn do server quyết.
Phần thay đổi cảm giác lớn nhất là di chuyển, nhốt và chết, vốn do client tự quyết ngay lập tức.

## Quyết định

- Server quyết mọi kết quả gameplay: vị trí, bom, nổ, gạch, item, nhốt, chết, thắng (như SCOPE_V1).
- Client **dự đoán di chuyển của chính mình** và chạy lại input chưa ack khi nhận snapshot.
- Đặt bom: client hiện ngay một bóng "chờ xác nhận" ở ô hiện tại (presentation, không vào state dự đoán);
  bom thật xuất hiện theo event của server. Bị từ chối thì bỏ bóng chờ, không phát âm thanh nổ.
- Nổ, nhốt, chết, item: chỉ hiển thị theo event server (có event ID; không phát lại khi reconcile).
- Người chơi khác: nội suy giữa hai snapshot, không ngoại suy.

## Điều kiện đảo/điều chỉnh

Đánh giá bằng fixture mạng của SCOPE_V1 (RTT 100 ms, jitter 0–30 ms, loss 1%):

- Độ trễ cảm nhận khi đặt bom hoặc khi bị nhốt bị playtest đánh giá kém → mở rộng prediction sang bom
  (cần rollback đầy đủ state bom/flame) bằng ADR mới.
- Correction di chuyển nhìn thấy được thường xuyên → xem lại thứ tự áp input và cửa sổ input.

## Hệ quả

`SimulationState` phải chạy được cả trên client (prediction) lẫn server; core không biết vai trò.
Presentation journal phân biệt event dự đoán và event đã xác nhận.
