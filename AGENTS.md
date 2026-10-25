# Quy tắc chung — dự án Boom KR Rust (`boom-rs`)

Nguồn quy tắc duy nhất cho agent điều phối, worker, reviewer và người. `CLAUDE.md`/`opencode.json`
chỉ trỏ về file này. Chỉ dẫn trực tiếp mới hơn của người dùng được ưu tiên hơn file này.

## 1. Đọc trước khi làm

Bắt buộc mỗi phiên/worker: `AGENTS.md` → `docs/README.md` → `docs/planning/REWRITE_PLAN.md`
→ `docs/planning/SCOPE_V1.md`, rồi `docs/planning/PROGRESS.md` (generated).
Theo loại task, đọc thêm:

- Feature: ID trong `docs/specification/FEATURE_CATALOG.md` và `features.json`.
- Kiến trúc/code Rust: `docs/planning/ARCHITECTURE.md` và ADR liên quan trong `docs/decisions/`.
- Luật chơi/research: `docs/specification/gameplay/RULES_V1.md`, `docs/research/QUESTIONS.md`.

Điều phối chép quy tắc đọc này vào mọi task giao worker. Bàn giao nêu: baseline commit, file đã sửa,
tài liệu đã đọc, lệnh kiểm + kết quả, điều còn thiếu. Rules đổi thì agent đang chạy phải đọc lại.

## 2. Mục tiêu và pha hiện tại

Viết lại client + server Boom bằng Rust; target nội dung là client KR `00967`, scope
`KR-00967-V1-scope-1` ([SCOPE_V1](docs/planning/SCOPE_V1.md)). Pha và hạn của pha nằm ở REWRITE_PLAN.
Được làm ngay: research có mục tiêu, spec/contract/ADR, và walking skeleton trong `crates/` theo
[ARCHITECTURE](docs/planning/ARCHITECTURE.md). Gameplay production, mạng và client mở theo task
có feature ID sau readiness P3. Lệnh trực tiếp của người dùng mở rộng phạm vi; không hỏi lại quyết định đã có.

## 3. Một thông tin — một nguồn chuẩn

| Nội dung | Nguồn sửa |
|---|---|
| Điều hướng | `docs/README.md` |
| Kế hoạch, pha, hạn, việc tiếp theo | `docs/planning/REWRITE_PLAN.md` |
| Scope V1, policy, nghiệm thu | `docs/planning/SCOPE_V1.md` |
| Cấu trúc crate, interface, quy tắc code | `docs/planning/ARCHITECTURE.md` |
| Claim luật chơi và bằng chứng | `docs/specification/gameplay/RULES_V1.md` |
| Giá trị luật engine dùng | `data/rules/*.toml` (mỗi giá trị trỏ claim) |
| Câu hỏi mở | `docs/research/QUESTIONS.md` |
| Catalog, scope và trạng thái từng feature | `docs/specification/features.json` |
| Trạng thái pha và release gate | `docs/planning/execution.json` |
| Báo cáo % | `docs/planning/PROGRESS.md` — generated, không sửa tay |
| Quyết định có lý do | `docs/decisions/` |
| Manifest/metadata | `docs/evidence/` |
| Lịch sử, plan hết hiệu lực | `docs/archive/` — không dùng làm scope |

Không tạo MASTER_PLAN/FINAL_PLAN/plan theo phiên hoặc file tổng kết phiên; sửa nguồn chuẩn.
Tài liệu mới chỉ khi chủ đề độc lập và có link từ index. Docs viết tiếng Việt; ID/type/API giữ tên kỹ thuật.

## 4. Bằng chứng

Mỗi claim và mỗi giá trị luật có **nguồn** và **trạng thái**:

- Nguồn: `kr_static`, `kr_runtime`, `vn_static`, `vn_runtime`, `web`, `oss`, `emu_choice`, `design`.
- Trạng thái: `confirmed`, `candidate`, `design`, `unknown` (unknown không mang giá trị).

Ưu tiên giá trị cùng đại lượng: KR 00967 `confirmed` → VN VBNB_116 `confirmed` →
BnB `5c108e8` được chọn `design` → `design` riêng ([ADR 0013](docs/decisions/0013-rule-source-precedence.md)).
`candidate` không đè nguồn confirmed; ngoại lệ chỉ số KR candidate có lý do riêng trong ADR 0013.
VN và KR là nguồn tin cậy ngang nhau ([ADR 0002](docs/decisions/0002-vn-kr-trusted-sources.md)).
Claim VN `confirmed` dùng trực tiếp cho luật chung; khác biệt cụ thể theo build (ID, offset, map, mode)
ghi theo profile. `web`, `oss` và `emu_choice` (giá trị server giả lập tự đặt) chỉ là `candidate`
cho tới khi được đối chiếu hoặc chọn thành `design`.

Trình diễn (clip, frame, thời gian, lớp, hiệu ứng, chuyển động, UI) không tự đặt luật phát: RE consumer →
quay/đo VN → `design` ghi nợ; PR hình ảnh kèm so sánh cảnh đối chiếu local ([ADR 0014](docs/decisions/0014-presentation-fidelity.md)).
Không suy diễn quá bằng chứng: asset tồn tại ≠ tính năng chạy; log server giả lập ≠ luật server gốc;
test sinh từ implementation chỉ là regression; speed level ≠ px/s; weight ≠ phần trăm.
Nguồn web ghi URL, tiêu đề, ngày truy cập/xuất bản, region/build và giới hạn; hai bài chép cùng nguồn
là một bằng chứng; không tìm thấy phải ghi phạm vi đã tìm, không kết luận là không tồn tại.

## 5. Làm việc nhiều người/agent

- Task là GitHub issue (nhãn `task`) có ID, mục tiêu, baseline, owner, reviewer, file được sửa, dependency,
  đầu ra, cách nghiệm thu (quy trình ở [TEAM_WORKFLOW](docs/planning/TEAM_WORKFLOW.md) §5–6).
- Đội và quyền: [TEAM_WORKFLOW](docs/planning/TEAM_WORKFLOW.md) §1 — integrator chính Claude, co-integrator Codex,
  worker OpenCode và dsh. Một file chỉ có một owner tại một thời điểm. Integrator giữ: `AGENTS.md`,
  `docs/README.md`, `features.json`, `execution.json` (chỉ integrator chính ghi), index ADR, `Cargo.toml` gốc,
  `rust-toolchain.toml`, các `clippy.toml`.
- Mọi thay đổi vào `main` qua pull request, **kể cả của integrator**; không push thẳng `main`
  (hook pre-push chặn; bật bằng `cargo xtask setup`). Merge khi CI ba OS xanh và:
  - code, contract, rules/claim, ADR, AGENTS hoặc quy trình: reviewer khác người làm đã approve
    (comment `APPROVE` kèm head SHA, vì mọi agent dùng chung tài khoản GitHub — TEAM_WORKFLOW §1);
  - chỉ sổ tiến độ (`features.json` tracking, `execution.json`, file generated) hoặc sửa link/chính tả
    không đổi nghĩa: integrator tự merge được, ghi lý do trong PR.
  Không khóa nhánh trên GitHub (quyết định chủ dự án 2026-10-19); quy tắc này + hook thay thế.
- Worker code chạy trong worktree/nhánh riêng; integrator merge theo dependency.
- Không reset/clean/checkout đè thay đổi người khác; không sửa binary, pack hoặc IDA database.
- Legacy server, tool RE và pack nằm ở `boom-re`, `boom-re-l10n`, `boomkr-re`; tham chiếu bằng đường dẫn,
  không copy code legacy vào `crates/`.
- Chỉ stage file của task, kiểm `git diff --cached`; không `git add -A` khi có agent khác.
  Không tự push/merge/deploy vượt yêu cầu task.

## 6. Tiến độ

- Trạng thái: `not_started → in_progress → in_review → done`; `blocked` kèm lý do và bước tiếp.
  Sửa lại sau review hoặc đổi dependency thì trả mục về `in_review`, không giữ `done`.
- `done` cần: integration commit có trong HEAD repo này, verification passed (lệnh + kết quả),
  evidence, reviewer độc lập approved, không blocker, mọi dependency done. Validator chỉ kiểm record;
  reviewer kiểm sự thật.
- % = số mục nghiệm thu (V1 `done/37`, catalog `done/112`); không cộng điểm cho trạng thái dở,
  không phải % công sức/fidelity. Docs/research không tăng % feature.
- Đổi scope cần baseline revision mới; không bỏ mục khó để tăng %.

## 7. Gate trước bàn giao

- Docs/catalog: `python3 tools/docs_check.py`, `python3 tools/kr_catalog.py --check`, `git diff --check`.
- Code: `cargo xtask ci` (fmt, clippy `-D warnings`, test gồm mọi scenario, replay golden, docs check,
  test tool tiến độ `tools/test_kr_tracking.py`).
- Scenario của feature (`fixtures/scenarios/`) là acceptance: chuyển `pending` → `active` khi feature xong.
- Sửa JSON nguồn rồi sinh lại export bằng `python3 tools/kr_catalog.py`; không sửa CSV/PROGRESS tay.
- Đổi đường dẫn phải sửa link/tool; nội dung chuyển/archive phải có bản đồ migration.
- Thêm dependency Rust phải ghi lý do trong commit; dependency nền ghi vào ARCHITECTURE.
- Không ghi cấu hình IDA/MCP hoặc thiết lập cá nhân vào repo. Build xanh ≠ fidelity đạt.
- Kết quả phải nằm trong nguồn chuẩn, không chỉ trong chat. Thiếu bằng chứng → QUESTIONS có owner và bước tiếp.
