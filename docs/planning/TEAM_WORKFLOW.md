# Workflow viết lại Rust với nhiều người/agent

Cập nhật 2026-10-19. [AGENTS.md](../../AGENTS.md) giữ quy tắc bắt buộc; file này giải thích cách chia việc.
Pha, hạn và việc tiếp theo ở [REWRITE_PLAN](REWRITE_PLAN.md) — không có hệ mốc thứ hai ở đây.

## 1. Vai trò

| Vai trò | Sở hữu | Đầu ra bắt buộc |
|---|---|---|
| Lead/integrator | baseline, scope, interface, workspace root, thứ tự merge | backlog, spec được chấp nhận, `main` luôn xanh |
| Research KR/CN/TW | guide, patch note, video, thuật ngữ | claim có nguồn, region/date và giới hạn |
| RE/oracle | pack/code/runtime, fixture và phép đo | dữ liệu trích, trace, claim VN/KR |
| Assets/client | asset pipeline, animation, render/audio/UI | manifest, bundle, scene chạy được |
| Core/net/server | luật trận, command/state, authority | mô phỏng, server và protocol mới |
| Reviewer/QA | kiểm nguồn và implementation độc lập | báo cáo tái lập, replay/diff, lỗi còn mở |

Một người có thể kiêm nhiều vai trò. Với tối đa ba worker, lead chia ba luồng độc lập (xem bảng luồng
theo pha trong REWRITE_PLAN). Thêm agent không làm nhanh phần đang chờ cùng một spec/interface;
không cho hai agent viết hai phiên bản core/server song song.

## 2. Một đường đi chung cho mọi tính năng

`Question → Claim → Spec/profile → Implementation → Independent verification → Integrated demo`

1. Lead đặt câu hỏi nhỏ, ví dụ "khiên chặn loại sát thương nào, hết hạn lúc nào?".
2. Research tìm guide/patch theo tên Hàn, giản thể, phồn thể; RE đối chiếu pack/code/trace song song.
3. Kết quả vào [RULES_V1](../specification/gameplay/RULES_V1.md) (hoặc spec đúng chủ đề) dưới dạng claim
   có nguồn/trạng thái. Lead chọn giá trị: `confirmed`, hoặc `design` có claim dẫn tới; ghi vào `data/rules/`.
4. Worker triển khai theo spec/contract đã ghi revision; spec đổi phải báo tác động tới core, net, UI, fixture.
5. Reviewer kiểm theo nguồn và acceptance; test do người viết code tạo không thay thế nguồn độc lập.
6. Integrator merge theo dependency, chạy integration, cập nhật registry/docs trong cùng thay đổi.

## 3. Hợp đồng trước khi chạy song song

Chốt revision tối thiểu của: asset ID/profile/build và đơn vị tọa độ; `InputCommand`, `SimulationState`,
`SimulationEvent`, `Snapshot`, replay; quyền quyết định của server; schema rules profile.
Tên type Rust có thể đổi; mục đích là worker kiểm được đầu vào/đầu ra của nhau.
Nguồn: [ARCHITECTURE](ARCHITECTURE.md) và ADR 0004–0006.

## 4. Claim luật chơi

Mỗi claim có: ID (`R-xxx`), mô tả, giá trị/đơn vị, tên thuật ngữ gốc, nguồn (nhãn AGENTS §4),
locator (URL hoặc đường dẫn code/trace + vị trí), region/build, ngày truy cập/xuất bản nếu là web,
trạng thái và bằng chứng phản bác nếu có. Một giá trị trong profile có thể dẫn nhiều claim.

Ưu tiên nguồn: dữ liệu/code client (KR hoặc VN) + phép đo → guide/patch chính thức đúng khu vực/thời kỳ
→ video có ngữ cảnh → wiki/forum/OSS làm đầu mối. Video không đủ suy tick/subpixel/collision chính xác.
Nguồn mâu thuẫn: lập bảng khác biệt theo build/region/event, không lấy trung bình.
Không tìm thấy sau một lượt có giới hạn → RE hoặc `design` có claim, mở câu hỏi kiểm lại; không chặn engine.

## 5. Làm việc với code Rust

- Mỗi worker một worktree: `git -C /home/admin/boom-rs worktree add ../boom-rs-wt/<task-id> -b <task-id>`.
- Crate có owner theo task; đổi API công khai của crate khác phải qua integrator.
- `boom-core` giữ quy tắc determinism (ADR 0004): không float, không `HashMap`, không wall clock/IO.
- Mỗi rule mới trong core lấy giá trị từ profile; không có hằng số luật trong code.
- Test có ý nghĩa: state transition, thứ tự event, biên va chạm, replay hash. Fixture sinh từ implementation
  là regression baseline, ghi rõ như vậy.
- Trước bàn giao: `cargo xtask ci` xanh; đổi hành vi core thì cập nhật golden replay có giải thích trong commit.
- Fixture cần dữ liệu game không commit; test đọc đường dẫn từ biến môi trường và tự bỏ qua khi thiếu.

## 6. Mẫu task

```text
ID / owner / reviewer:
Baseline commit + worktree/nhánh:
Feature ID / câu hỏi / claim liên quan:
Mục tiêu và phần ngoài phạm vi:
Dependency + revision spec/contract:
File/thư mục được sửa:
Deliverables:
Acceptance scenarios / lệnh kiểm:
Nếu thiếu bằng chứng: ghi unknown hoặc design có claim, không tự đặt hằng số.
Bàn giao: commit/diff, docs đã cập nhật, kết quả kiểm, giới hạn, bước tích hợp.
```

Ví dụ `RULE-SHIELD-001`: research xác định tác dụng/thời lượng theo KR và CN; RE tìm Attr/consumer;
reviewer xác nhận các nguồn nói cùng loại khiên. Core chỉ triển khai sau khi claim xác định damage nào bị chặn,
thời điểm hết hạn và các trường hợp biên.

## 7. Theo dõi tiến độ

Integrator là người ghi duy nhất của `features.json` và `execution.json`: ghi ID/assignee/revision khi
nhận task, nhập proof/review khi worker bàn giao, rồi chạy `python3 tools/kr_catalog.py` và `--check`.
Định nghĩa done và % ở AGENTS §6. Đánh giá bằng feature chơi được, rule có nguồn và số sai khác chưa giải;
không dùng số agent, số dòng code hay opcode làm phần trăm.
