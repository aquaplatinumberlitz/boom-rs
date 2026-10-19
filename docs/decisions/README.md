# Decisions — quyết định có lý do

ADR ghi một quyết định, lý do, phương án đã cân nhắc, hệ quả và gate kiểm chứng.
Chỉ tạo ADR cho quyết định có tác động lâu dài; không biến mỗi task nhỏ thành ADR.

| ID | Chủ đề | Trạng thái |
|---|---|---|
| [0002](0002-vn-kr-trusted-sources.md) | VN/KR là trusted source ngang nhau; dùng trực tiếp luật chung | Accepted — chỉ đạo người dùng 2026-10-19 |
| [0001](0001-project-baseline.md) | Target, scope V1, pha và nguồn chuẩn docs | Accepted — quyết định dự án theo chỉ đạo người dùng |

Các lựa chọn chưa quyết định (time model/số học, render stack, transport, storage) nằm trong
ARCHITECTURE + QUESTIONS cho tới khi đủ dữ liệu. Không ghi accepted chỉ vì có đề xuất trong plan cũ.

Khi cần ADR mới: ID, status, bối cảnh, quyết định, alternatives, consequences, evidence/limits,
người/nguồn chốt và việc xác minh tiếp. Thay đổi quyết định cũ dùng superseding ADR và giữ lịch sử.
