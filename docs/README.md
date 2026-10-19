# Tài liệu dự án Boom KR Rust

**Bắt đầu ở đây.** Quy tắc bắt buộc: [AGENTS.md](../AGENTS.md). Repo `boom-rs` chứa cả docs và Cargo workspace
([ADR 0003](decisions/0003-engine-repo-and-docs-home.md)). Pha hiện tại: research + kiến trúc có hạn,
kèm walking skeleton; xem [REWRITE_PLAN](planning/REWRITE_PLAN.md) và tiến độ ở [PROGRESS](planning/PROGRESS.md).

## Tài liệu điều hành

| Đọc | Mục đích |
|---|---|
| [REWRITE_PLAN](planning/REWRITE_PLAN.md) | Lộ trình P0–P9, hạn pha, dependency, việc tiếp theo |
| [SCOPE V1](planning/SCOPE_V1.md) | Bản đầu làm gì, để sau gì, điều kiện nghiệm thu |
| [ARCHITECTURE](planning/ARCHITECTURE.md) | Crate, boundary, contract, quy tắc code |
| [RULES_V1](specification/gameplay/RULES_V1.md) | Claim luật trận V1 có nguồn và giá trị được chọn |
| [QUESTIONS](research/QUESTIONS.md) | Còn thiếu gì, ai phụ trách, cách đóng |

Đọc thêm đúng task, không đọc mọi tài liệu lịch sử trước mỗi phiên.

## Nguồn chuẩn khác

| Nội dung | Vị trí |
|---|---|
| Catalog chức năng | [Hướng dẫn](specification/FEATURE_CATALOG.md), [JSON nguồn](specification/features.json), [CSV export](specification/features.csv) |
| Trạng thái phase/gates | [execution.json](planning/execution.json), hiển thị trong plan và PROGRESS |
| Cách chia việc team/agents | [TEAM_WORKFLOW](planning/TEAM_WORKFLOW.md) |
| Quyết định có lý do | [Decisions index](decisions/README.md) |
| Nguồn web Hàn–Trung | [Research sources](research/sources/README.md) |
| Tài liệu RE hiện có | [Findings index](research/findings/README.md) |
| Pack/map/item/protocol tham chiếu | [Specification index](specification/README.md) |
| Manifest và ảnh/metadata | [Evidence index](evidence/README.md) |
| Kế hoạch/nhật ký cũ | [Archive index](archive/README.md), chỉ là lịch sử |

Baseline scope: **`KR-00967-V1-scope-1`** — 114 mục, 37 V1, 75 later, 2 drop legacy.
Đây là quyết định phạm vi, **không phải xác nhận runtime hoặc RE hoàn chỉnh client KR**.

## Cập nhật tài liệu

Kết quả nghiên cứu vào RULES_V1/findings/spec và QUESTIONS; quyết định vào ADR; dữ liệu kiểm kê vào evidence.
Thay đổi scope sửa JSON nguồn + SCOPE_V1 rồi sinh lại export. Sau mỗi task chạy
`python3 tools/kr_catalog.py`; `--check` kiểm export/báo cáo không stale.
Đường dẫn cũ tra ở [DOCS_MIGRATION](evidence/DOCS_MIGRATION.json), gồm cả lần chuyển từ `boom-re-l10n` sang repo này.
