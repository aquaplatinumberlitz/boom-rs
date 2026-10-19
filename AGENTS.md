# Quy tắc chung cho mọi agent của dự án Boom KR Rust

Đây là nguồn quy tắc chung của dự án, áp dụng cho agent điều phối, worker, reviewer và phiên tiếp nối.
Codex đọc `AGENTS.md`; adapter Claude/OpenCode phải trỏ về file này, không sao chép một bộ luật khác.
Chỉ dẫn trực tiếp mới hơn của người dùng và chỉ dẫn nền tảng có thứ tự ưu tiên cao hơn.

## 1. Đọc trước khi làm

Mỗi phiên và mỗi worker **bắt buộc đọc**:

1. `AGENTS.md` — file này.
2. `docs/README.md` — bản đồ tài liệu và nguồn chuẩn.
3. `docs/planning/REWRITE_PLAN.md` — kế hoạch tổng hiện hành, pha, dependency và việc tiếp theo;
   đây là bản thay thế trực tiếp cho REWRITE_PLAN_2026-10-19 đã archive.
4. `docs/planning/SCOPE_V1.md` — phạm vi được chốt.

Sau đó đọc `docs/planning/PROGRESS.md` để biết phase/feature đã nghiệm thu, đang làm, blocked và
chưa làm; đây là báo cáo sinh từ dữ liệu, không phải một kế hoạch thứ hai.
Task feature phải tra ID trong `docs/specification/FEATURE_CATALOG.md` và registry trước khi nhận việc.

Task kiến trúc đọc thêm `docs/planning/ARCHITECTURE.md` và các ADR liên quan.
Task tìm nguồn/RE đọc `docs/research/QUESTIONS.md` và spec/bằng chứng liên quan.
Task sửa catalog đọc `docs/specification/features.json`; CSV là export, không phải nguồn sửa tay.
Khi bàn giao, nêu baseline, phạm vi file, tài liệu đã đọc, kết quả kiểm và điều còn thiếu.
Agent điều phối phải đưa quy tắc đọc này vào task giao cho worker, kể cả khi worker không tự load AGENTS.
Sau khi đổi rules, agent đang chạy phải đọc lại; adapter là cơ chế nạp hướng dẫn, không phải khóa kỹ thuật.
Không báo “mọi agent đã tuân thủ” nếu chỉ mới tạo file; kiểm việc đọc trong bàn giao/task thực tế.

## 2. Mục tiêu hiện tại

**Lên kế hoạch → tìm thêm thông tin → chốt cấu trúc dự án trước khi viết lại engine Rust.**
Được làm trong pha hiện tại: tổ chức docs, catalog/scope, research, RE có mục tiêu, fixture,
tool kiểm kê/validation và thiết kế interface/kiến trúc. Chưa tự mở sprint viết gameplay/server/client
production hoặc dựng workspace engine lớn khi task được giao chỉ là planning/research.
Lệnh triển khai engine mới từ người dùng sẽ mở rộng phạm vi; không yêu cầu hỏi lại các quyết định đã có.

Target là client KR `00967`. Theo quyết định người dùng ngày 2026-10-19, **VN là trusted source ngang KR**: dùng trực tiếp bằng chứng VN đã xác nhận để chốt luật chung và thiết kế/triển khai, không bắt buộc RE lại trên KR.
Giữ provenance VN/KR để truy vết; khác biệt cụ thể về map/ID/asset/mode/thông số được xử lý theo build.
Xem [quyết định nguồn](docs/decisions/0002-vn-kr-trusted-sources.md).
Giữ baseline `KR-00967-V1-scope-1` trừ khi nhiệm vụ được giao yêu cầu thay đổi scope.
Không đợi hiểu hết mọi mode mới lập kiến trúc, nhưng không biến unknown thành hằng số gốc.

## 3. Một thông tin — một nguồn chuẩn

| Nội dung | Nguồn sửa chính |
|---|---|
| Điều hướng | `docs/README.md` |
| Kế hoạch tổng, pha, mốc, việc tiếp theo | `docs/planning/REWRITE_PLAN.md` |
| Scope V1, policy và nghiệm thu | `docs/planning/SCOPE_V1.md` |
| Cấu trúc dự án và interface | `docs/planning/ARCHITECTURE.md` |
| Câu hỏi, owner, hướng tìm | `docs/research/QUESTIONS.md` |
| Catalog và scope từng mục | `docs/specification/features.json` |
| Trạng thái implementation, assignee, proof/review của feature | `docs/specification/features.json` |
| Trạng thái phase và release gates | `docs/planning/execution.json` |
| Báo cáo %/danh sách trạng thái | `docs/planning/PROGRESS.md` — generated, không sửa tay |
| Quyết định có lý do | `docs/decisions/` |
| Kết quả khảo sát kỹ thuật | `docs/research/findings/` hoặc spec đúng chủ đề |
| Manifest/metadata | `docs/evidence/` |
| Nhật ký/kế hoạch hết hiệu lực | `docs/archive/` |

Không tạo thêm MASTER_PLAN/FINAL_PLAN/NEW_PLAN hoặc một file tổng kết sau mỗi phiên.
Cập nhật nguồn chuẩn đang có. Chỉ tạo tài liệu mới khi có chủ đề độc lập, có link trong index
và không lặp lại nội dung đã được quản lý ở nơi khác. Không dùng archive làm scope hiện hành.
Spec chỉ ghi phần đã hiểu; lịch sử thử nghiệm dài nằm trong archive và được dẫn lại khi cần.
Docs viết tiếng Việt rõ ràng; ID/type/API giữ tên kỹ thuật ổn định.

## 4. Bằng chứng và điều chưa biết

Mỗi kết luận phân biệt **quan sát trực tiếp / suy luận / nguồn khác build-region / DESIGN / unknown**.
Asset/name/class tồn tại không chứng minh tính năng đang bật hoặc chạy được.
Log fakeserver không chứng minh luật server gốc. Test tự sinh từ implementation chỉ là regression,
không phải oracle độc lập. Tốc độ level không tự tương đương px/s; weight không tự là phần trăm.

Nguồn web ghi URL, tiêu đề/locator, ngày truy cập, ngày xuất bản nếu có, region/build/event,
kết luận được hỗ trợ và giới hạn. Ưu tiên nguồn chính thức; không trộn PC với mobile hoặc game khác.
Hai bài sao chép cùng nguồn không là hai bằng chứng độc lập. Không tìm thấy phải ghi ranh giới tìm kiếm,
không kết luận không tồn tại. Không sao chép toàn bài; lưu ghi chú ngắn và artifact đúng phạm vi cần thiết.
Giá trị tạm phải có provenance, đơn vị và trạng thái; không gắn `verified_kr` khi chỉ đo được VN.

## 5. Làm việc nhiều người/agent

Mỗi task có ID, mục tiêu, baseline commit/worktree, owner, reviewer, file được sửa, dependency,
đầu ra và cách nghiệm thu. Worker chỉ sửa phạm vi được giao; không sửa file chung ngoài lượt được phân.
Một file/shared schema chỉ có một owner tại một thời điểm. Điều phối giữ root docs, registry và ADR index.
Đọc và giữ thay đổi đang có; không reset/clean/checkout đè file, không sửa binary/pack/IDA database gốc.
Repo `boomkr-re` giữ nghiên cứu KR; worktree `boom-re-l10n` giữ thay đổi hiện hành tới khi tích hợp.
Không copy code giữa repo hoặc coi worktree l10n là snapshot server main mới nhất.

Task hẹp có thể chạy song song; dependency và thay đổi interface phải được chốt theo revision.
Worker trả bằng chứng, diff, kiểm chứng và unknown; reviewer kiểm độc lập khi có thể.
Không lấy số agent đồng ý hoặc số dòng code làm thước đo đúng/hoàn thành.
Không tự merge/push/deploy vượt yêu cầu task; commit và tích hợp phải giữ nguyên thay đổi của người khác.
Chỉ stage file thuộc task được giao và kiểm `git diff --cached` trước commit; không gom cả repo
bằng `git add .`/`git add -A` khi có worker khác đang làm trong workspace chung.

### Cập nhật tiến độ bắt buộc

- Nhận task triển khai bằng feature ID, ghi assignee cụ thể và baseline/spec revision đã chấp nhận.
  Integrator/owner registry áp dụng cập nhật trước/sau task; worker bàn giao status/proof theo ID,
  không cho nhiều worker ghi đồng thời vào JSON chung. execution.json cũng có một owner ghi.
- Trạng thái hợp lệ: `not_started → in_progress → in_review → done`; dùng `blocked` kèm lý do/bước tiếp.
  Khi sửa lại sau review hoặc đổi dependency, đưa mục liên quan về trạng thái cần review thay vì giữ done.
- `done` phải có integration commit đã nằm trong HEAD của checkout tích hợp, acceptance verification passed (command/result),
  evidence files/commit, reviewer độc lập approved, không còn blocker và tất cả dependency done.
  Validator kiểm record/ref, reviewer kiểm sự thật của bằng chứng. Không tự nhận done vì có code/spec/asset.
  Kiểm và cập nhật registry ở checkout tích hợp; commit chỉ có trên nhánh worker chưa merge không đủ.
- Planning/research cập nhật phase/gate trong execution.json và QUESTIONS; không tăng % feature vì viết docs.
- Chạy `python3 tools/kr_catalog.py` để sinh CSV, PROGRESS và trạng thái phase trong kế hoạch tổng;
  chạy lại với `--check` trước bàn giao để phát hiện export/report stale.
- % là tỷ lệ số mục nghiệm thu: V1 `done/37`, catalog `done/112` ở baseline hiện hành; loại riêng2drop.
  Không cộng điểm tùy ý cho in_progress/in_review/blocked và không gọi đó là % thời gian/công sức/fidelity.
- Đổi scope cần baseline revision và giải thích mẫu số; không bỏ mục khó để tự tăng %. Review lại các mục
  bị ảnh hưởng khi spec/rules/dependency thay đổi. Không báo plan/release hoàn tất khi gate còn mở.

## 6. Gate chất lượng và tài liệu

- Sửa JSON nguồn rồi sinh lại CSV bằng tool; chạy validator phù hợp.
- Khi đổi docs/catalog, chạy `python3 tools/docs_check.py`, `python3 tools/kr_catalog.py --check`
  và `git diff --check` từ gốc worktree hiện hành.
- Đổi đường dẫn phải sửa link, tool và command liên quan; kiểm links và `git diff --check`.
- Nội dung được chuyển/archive phải có bản đồ migration; không xóa bằng chứng để làm docs gọn.
- Không ghi cấu hình IDA/MCP hoặc thiết lập cá nhân vào docs dự án.
- Kết quả planning không được đánh dấu implementation done; build xanh không đồng nghĩa fidelity đạt.
- Scope phát hiện thêm chỉ cập nhật backlog; không tự mở rộng V1.
- Không thêm dependency/engine/toolchain hoặc chạy game chỉ để trả lời câu hỏi docs nếu không cần.
- Bàn giao phải cập nhật nguồn chuẩn và việc tiếp theo; không để kết quả chỉ tồn tại trong chat.

Việc chưa đủ bằng chứng được ghi trong QUESTIONS với owner và cách xử lý tiếp theo.
Không dừng để hỏi lại khi đã có đủ ủy quyền; tiếp tục phần độc lập và báo rõ giới hạn thực tế.
