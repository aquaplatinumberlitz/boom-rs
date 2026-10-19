# Decisions — quyết định có lý do

ADR ghi một quyết định, lý do, phương án đã cân nhắc, hệ quả và điều kiện đảo/kiểm chứng.
Chỉ tạo ADR cho quyết định có tác động lâu dài; không biến mỗi task nhỏ thành ADR.

| ID | Chủ đề | Trạng thái |
|---|---|---|
| [0010](0010-client-stack-choice.md) | Client stack: macroquad (có điều kiện), winit+softbuffer làm tham chiếu | Accepted có điều kiện — review #37 (Codex + Claude) |
| [0009](0009-wire-protocol-input-snapshot-codec.md) | Giao thức InputCommand/Snapshot và codec | Accepted — review #24 (Codex) |
| [0008](0008-extensibility-model.md) | Mô hình mở rộng: Mode/System/Event, overlay luật, dữ liệu item/liveops | Accepted — yêu cầu người dùng 2026-10-19 |
| [0007](0007-client-stack-spike.md) | Client stack chọn bằng spike một scene V1 | Superseded bởi 0010 |
| [0006](0006-transport-default.md) | Transport mặc định: TCP sau trait, input dư thừa | Accepted — mặc định có thể đảo |
| [0005](0005-authority-and-prediction.md) | Server quyết kết quả; client dự đoán di chuyển của mình | Accepted — mặc định có thể đảo |
| [0004](0004-time-numeric-model.md) | Fixed step, số nguyên, RNG/hash riêng trong core; mở walking skeleton | Accepted — mặc định có thể đảo |
| [0003](0003-engine-repo-and-docs-home.md) | Repo `boom-rs` chứa engine và docs | Accepted — chỉ đạo người dùng 2026-10-19 |
| [0002](0002-vn-kr-trusted-sources.md) | VN/KR là nguồn tin cậy ngang nhau | Accepted — chỉ đạo người dùng 2026-10-19 |
| [0001](0001-project-baseline.md) | Target, scope V1, pha và nguồn chuẩn docs | Accepted, superseded một phần bởi 0002–0004 |

Quyết định còn mở (persistence, định dạng bundle) nằm ở ARCHITECTURE §8.
"Mặc định có thể đảo" nghĩa là team làm theo ngay; điều kiện đảo ghi trong ADR, đảo bằng ADR mới.

Khi cần ADR mới: ID, status, bối cảnh, quyết định, phương án, hệ quả, bằng chứng/giới hạn,
người/nguồn chốt và điều kiện đảo. Thay quyết định cũ bằng ADR mới và giữ lịch sử.
