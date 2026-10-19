# ADR 0001 — Baseline dự án và giai đoạn trước triển khai

Status: **Accepted**, 2026-10-19; **superseded một phần**: chính sách VN bởi [0002](0002-vn-kr-trusted-sources.md),
vị trí repo/docs bởi [0003](0003-engine-repo-and-docs-home.md), pha "chưa dựng engine" bởi [0004](0004-time-numeric-model.md).
Nguồn: người dùng yêu cầu chốt catalog/V1 rồi tổ chức docs,
lên plan, tìm thêm thông tin và thiết kế cấu trúc trước khi viết lại engine; yêu cầu rule chung cho mọi agent.

## Quyết định

- Target KR `00967`; VN hỗ trợ tham chiếu, không tự xác nhận tương đương KR.
- Giữ scope `KR-00967-V1-scope-1` ở SCOPE_V1 và registry: 37 V1, 75 later, 2 drop legacy.
- Giai đoạn hiện tại planning/research/architecture; chưa tự dựng engine production trong task docs.
- Một nguồn quy tắc là root AGENTS.md; adapter chỉ dẫn tới đó. Một trang bắt đầu là docs/README.md.
- REWRITE_PLAN, SCOPE_V1, QUESTIONS, ARCHITECTURE điều hành hiện tại; kế hoạch cũ và nhật ký vào archive.
- Catalog JSON là nguồn sửa; CSV và bảng scope là export. Dữ liệu quét nằm trong evidence.

## Phương án cân nhắc

Giữ tất cả file root và thêm một plan mới dễ gây đọc nhầm phiên bản. Xóa tài liệu cũ làm mất bằng chứng.
Chọn phân loại + index + archive + migration map, giữ nội dung và sửa link/tool.

## Hệ quả và giới hạn

Mọi agent đọc quy tắc và nguồn chuẩn trước làm; không tự lấy archived plan làm lệnh mở scope.
Scope được chốt không chứng minh runtime KR hoặc readiness phát hành. Timing, collision, content closure,
transport và storage vẫn cần nghiên cứu/thiết kế; không được nhận diện là quyết định accepted bởi ADR này.
Các worktree khác chưa tự nhận thay đổi; cần tích hợp có kiểm baseline, tránh ghi đè server mới hơn.
