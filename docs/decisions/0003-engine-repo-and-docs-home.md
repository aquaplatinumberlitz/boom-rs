# 0003 — Repo engine `boom-rs` là nơi tích hợp code và docs dự án

Status: **Accepted**, 2026-10-19, theo chỉ đạo người dùng (chấp nhận góp ý review kế hoạch và chọn
"chuyển docs sang boom-rs ngay"). Thay phần vị trí repo/docs trong [0001](0001-project-baseline.md).

## Bối cảnh

Docs điều hành nằm trên nhánh Việt hóa `claude/l10n-kr-vi` của `boom-re-l10n`; main `boom-re` vẫn có
plan cũ (`docs/MASTER_PLAN.md`, `docs/kr-re-plan.md`) và chưa có AGENTS; `boomkr-re/docs/rust-rewrite.md`
còn trỏ tới plan đã archive. ARCHITECTURE để ngỏ "repo tích hợp được chọn". Agent mở nhầm checkout
sẽ đọc kế hoạch cũ, còn engine không có chỗ đặt.

## Quyết định

- Tạo repo mới `/home/admin/boom-rs` (nhánh `main`), chứa: `AGENTS.md` (quy tắc duy nhất), `docs/`
  (toàn bộ cây docs đã tổ chức), tool docs (`docs_check`, `kr_catalog`, `kr_tracking`), Cargo workspace
  `crates/`, `xtask/`, `data/rules/` và `fixtures/`.
- Import từ `boom-re-l10n@9bde2ceacd82cd542f871fcd42f18d29e6e5705d` nguyên văn (commit import riêng),
  các thay đổi sau đó là commit riêng để review được. Bản đồ ở [DOCS_MIGRATION](../evidence/DOCS_MIGRATION.json).
- `boom-re-l10n` giữ việc Việt hóa (`l10n/`, tool `l10n_*`, gallery ảnh) và tool đọc pack
  (`idf.py`, `kr_feature_inventory.py`, `kr_v1_content.py`); docs dự án ở đó được thay bằng stub trỏ về đây.
- `boom-re` (main) giữ server giả lập VN, RE notebook và oracle legacy; `boomkr-re` giữ nghiên cứu KR.
  Hai repo này chỉ thêm banner trỏ về repo này, không bị di chuyển code.
- Router `/home/admin/AGENTS.md` trỏ về `boom-rs/AGENTS.md`.

## Phương án đã cân nhắc

| Phương án | Lý do không chọn |
|---|---|
| Giữ docs ở l10n, engine ở repo khác | Hai nơi điều hành; nhánh l10n có mục đích khác và có agent khác commit |
| Đưa docs + engine vào main `boom-re` | Main chứa exe, IDA DB, Wine prefix và `server/` legacy; dễ kéo legacy vào engine và làm CI nặng |
| Repo mới (chọn) | Ranh giới sạch: engine không phụ thuộc dữ liệu có bản quyền; legacy vẫn chạy như cũ |

## Hệ quả

- Validator tiến độ kiểm commit trong repo này; bằng chứng P0 từ l10n được ghi lại bằng commit import.
- Fixture dùng dữ liệu game (pack, map) không commit; test đọc qua biến môi trường và tự bỏ qua khi thiếu.
- Agent đang chạy trong l10n phải đọc lại rules; kết quả research mới nhập vào đây.
- Chưa có remote; push/CI từ xa cần người dùng tạo remote.
