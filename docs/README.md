# Tài liệu dự án Boom KR Rust

**Bắt đầu ở đây.** Pha hiện tại: **planning, research và chốt kiến trúc**, chưa triển khai engine Rust mới.
Quy tắc bắt buộc cho mọi agent: [AGENTS.md](../AGENTS.md).

**Làm theo [REWRITE_PLAN](planning/REWRITE_PLAN.md); xem đã làm tới đâu tại
[PROGRESS](planning/PROGRESS.md).** Báo cáo so trực tiếp với [FEATURE_CATALOG](specification/FEATURE_CATALOG.md):
done/V1 và done/toàn catalog, các mục blocked/review, phase và gate phát hành tách riêng.

## Bốn tài liệu điều hành

| Đọc | Mục đích |
|---|---|
| [REWRITE_PLAN — kế hoạch tổng](planning/REWRITE_PLAN.md) | Toàn lộ trình planning→V1→full feature, dependency, trạng thái và việc tiếp theo; thay trực tiếp plan đã archive |
| [SCOPE V1](planning/SCOPE_V1.md) | Bản đầu làm gì, để sau gì và điều kiện nghiệm thu |
| [QUESTIONS](research/QUESTIONS.md) | Thiếu thông tin gì, tìm ở đâu, ai phụ trách |
| [ARCHITECTURE](planning/ARCHITECTURE.md) | Cấu trúc dự kiến và interface cần chốt trước khi viết engine |

Đọc thêm đúng task, không đọc mọi tài liệu lịch sử trước mỗi phiên.

## Nguồn chuẩn

| Nội dung | Vị trí |
|---|---|
| Catalog chức năng | [Hướng dẫn](specification/FEATURE_CATALOG.md), [JSON nguồn](specification/features.json), [CSV export](specification/features.csv) |
| Trạng thái phase/gates | [execution.json](planning/execution.json), được hiển thị trong plan và PROGRESS |
| Tiến độ so với catalog | [PROGRESS](planning/PROGRESS.md), generated từ trạng thái/proof/review trong JSON nguồn |
| Cách chia việc team/agents | [TEAM_WORKFLOW](planning/TEAM_WORKFLOW.md); AGENTS.md giữ quy tắc chung |
| Nguồn web Hàn–Trung | [Research sources](research/sources/README.md) |
| Tài liệu RE hiện có | [Findings index](research/findings/README.md) |
| Pack/map/item/protocol tham chiếu | [Specification index](specification/README.md) |
| Quyết định kiến trúc/sản phẩm | [Decisions index](decisions/README.md) |
| Manifest và ảnh/metadata | [Evidence index](evidence/README.md) |
| Kế hoạch/nhật ký cũ | [Archive index](archive/README.md), chỉ dùng làm lịch sử/bằng chứng |

Baseline scope: **`KR-00967-V1-scope-1`** — 114 mục, 37 V1, 75 later, 2 drop legacy.
Đây là quyết định phạm vi dự án; **không phải xác nhận runtime hoặc RE hoàn chỉnh client KR**.
VN là trusted source ngang KR; protocol/server cũ không phải engine Rust mới.

## Cập nhật tài liệu

Mỗi thông tin có một nguồn chuẩn. Không tạo thêm plan/tổng kết mới sau mỗi phiên.
Kết quả nghiên cứu nhập vào QUESTIONS + findings/spec; quyết định ghi ADR; dữ liệu kiểm kê vào evidence.
Thay đổi scope sửa JSON nguồn, SCOPE_V1 và sinh lại CSV. Đổi đường dẫn phải kiểm link/tool.
Sau mỗi task cập nhật trạng thái/proof/review theo AGENTS rồi chạy `python3 tools/kr_catalog.py`.
Không sửa phần trăm bằng tay; `--check` kiểm báo cáo và exports có khớp nguồn không.

Các file cũ đã được chuyển, không xóa nội dung: [bản đồ migration](evidence/DOCS_MIGRATION.json).
Thay đổi hiện ở worktree `boom-re-l10n`; các worktree khác chưa được tích hợp, xem
[trạng thái baseline](planning/REWRITE_PLAN.md#baseline-và-worktree).

Chính sách nguồn hiện hành: [VN/KR trusted ngang nhau](decisions/0002-vn-kr-trusted-sources.md). Dùng trực tiếp bằng chứng VN cho luật chung; giữ provenance và xử lý khác biệt cụ thể.
