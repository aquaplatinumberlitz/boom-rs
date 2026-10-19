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

**Integrator hiện tại:** Claude (agent Claude Code trong phiên của chủ dự án), chủ dự án chỉ định 2026-10-19;
thao tác GitHub dưới tài khoản `aquaplatinumberlitz`. Integrator không tự review code mình viết: thay đổi
contract do integrator làm vẫn cần reviewer khác.

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

**Bảng việc** là GitHub issues của repo `aquaplatinumberlitz/boom-rs` (nhãn `task`, `review`, `wave:N`,
`stream:*`). Mỗi issue theo mẫu §6: feature ID, file được sửa, dependency, acceptance.

Vòng đời một task:

1. Worker comment "nhận" trên issue; integrator ghi assignee/baseline vào `features.json` (`in_progress`).
2. Worker tạo worktree riêng: `git -C /home/admin/boom-rs worktree add ../boom-rs-wt/<task> -b <task>`,
   chạy `cargo xtask setup` một lần (hook pre-push chạy `cargo xtask ci`).
3. Chỉ sửa file trong phạm vi issue. Cần đổi file dùng chung (`state/`, `event.rs`, `rules.rs`, `mode.rs`,
   `sim.rs`, `Cargo.toml` gốc, `features.json`, `execution.json`) → hỏi integrator trong issue.
4. Chuyển scenario của feature từ `pending` sang `active`, thêm ca biên; scenario là acceptance, reviewer
   đối chiếu nó với RULES_V1/SCOPE_V1.
5. Mở PR theo mẫu (`.github/pull_request_template.md`), CI ba OS xanh, reviewer khác người làm approve.
6. Integrator merge PR (`gh pr merge`) theo thứ tự dependency, rồi cập nhật `features.json` bằng một PR
   sổ tiến độ (tự merge được theo AGENTS §5). Integrator cũng mở PR cho thay đổi của chính mình.

Ranh giới để chạy song song không đụng nhau:

| Vùng | Owner | Ghi chú |
|---|---|---|
| `crates/boom-core/src/systems/<system>.rs` + `fixtures/scenarios/<system>/` | worker của system đó | Ma trận quyền ghi ở `systems/mod.rs` |
| `crates/boom-core/src/{state/,event.rs,rules.rs,mode.rs,sim.rs}` | integrator | Contract; đổi = contracts-rN |
| `crates/boom-assets`, `data/maps/` | assets | |
| `crates/boom-room` | backend | Thuần, không socket |
| `spikes/` | client | Ngoài workspace, không merge vào `crates/` |
| `docs/specification/gameplay/RULES_V1.md` | RE + worker liên quan | Mỗi PR chỉ thêm/sửa claim thuộc feature của mình |
| `features.json`, `execution.json`, `AGENTS.md`, ADR index, `.github/` | integrator | Người ghi duy nhất |

Quy tắc code:

- `boom-core` giữ quy tắc determinism (ADR 0004): không float, không `HashMap`, không wall clock/IO.
- Mỗi giá trị luật lấy từ `Rules`; không có hằng số luật trong code.
- Test có ý nghĩa: state transition, thứ tự event, biên va chạm, replay hash. Fixture sinh từ implementation
  là regression baseline, ghi rõ như vậy.
- Đổi hành vi core có chủ đích: `cargo xtask bless` và giải thích trong PR.
- Fixture cần dữ liệu game không commit; test đọc đường dẫn từ biến môi trường và tự bỏ qua khi thiếu.
- Không khóa nhánh `main` trên GitHub (quyết định chủ dự án 2026-10-19). Thay bằng quy tắc AGENTS §5:
  mọi thay đổi, kể cả của integrator, qua PR; hook pre-push từ chối push thẳng `main`. Loại PR integrator
  được tự merge (sổ tiến độ, file generated, sửa link/chính tả) và loại cần reviewer độc lập ghi ở AGENTS §5.

## 6. Mẫu task và cách giao cho agent

Mẫu issue: `.github/ISSUE_TEMPLATE/task.yml`. Nội dung tối thiểu:

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

Lời giao việc cho một agent (dán nguyên, thay số issue):

```text
Làm task GitHub issue #N của repo /home/admin/boom-rs (gh issue view N).
Trước khi làm: đọc AGENTS.md, docs/README.md, docs/planning/REWRITE_PLAN.md, docs/planning/SCOPE_V1.md
và tài liệu issue chỉ định. Tạo worktree ../boom-rs-wt/<task> từ main, chạy cargo xtask setup.
Chỉ sửa file trong phạm vi issue; cần sửa file khác thì dừng và hỏi trong issue.
Xong: cargo xtask ci xanh, scenario liên quan active, PR theo mẫu, báo lại link PR và điều còn thiếu.
Không push lên main, không merge.
```

Ví dụ `RULE-SHIELD-001`: research xác định tác dụng/thời lượng theo KR và CN; RE tìm Attr/consumer;
reviewer xác nhận các nguồn nói cùng loại khiên. Core chỉ triển khai sau khi claim xác định damage nào bị chặn,
thời điểm hết hạn và các trường hợp biên.

## 7. Theo dõi tiến độ

Integrator là người ghi duy nhất của `features.json` và `execution.json`: ghi ID/assignee/revision khi
nhận task, nhập proof/review khi worker bàn giao (PR đã merge, commit tích hợp, lệnh kiểm), rồi chạy
`python3 tools/kr_catalog.py` và `--check`. `tracking.next_action` của feature trỏ số issue đang giữ việc.

Thứ tự merge đợt 1 khi nhiều PR cùng sẵn sàng: contract review (#2, hiện contracts-r3) → bomb_place, explosion → blocks,
pickup, life → movement (phần grace cần bomb_place) → các scenario liên hệ chéo (`own_bomb_full_cycle`,
`destroy_and_reveal`) chuyển active. Các luồng assets/RE/backend/client/l10n merge độc lập.
Định nghĩa done và % ở AGENTS §6. Đánh giá bằng feature chơi được, rule có nguồn và số sai khác chưa giải;
không dùng số agent, số dòng code hay opcode làm phần trăm.
