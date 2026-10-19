# Workflow viết lại Rust với nhiều người/agent

Ngày: 2026-10-19. Workflow phân công hiện hành; [AGENTS.md](../../AGENTS.md) giữ quy tắc bắt buộc.
Pha và việc tiếp theo theo [REWRITE_PLAN](REWRITE_PLAN.md), không theo plan cũ trong archive.

Mục tiêu: phát triển engine KR từ client còn lại; VN hỗ trợ đo; phần server thất lạc được
khôi phục theo bằng chứng hoặc thiết kế lại có ghi nhận. Không cần RE xong toàn game mới viết code.

Scope baseline đã chốt: [catalog KR](../specification/FEATURE_CATALOG.md) và [Release 1](SCOPE_V1.md).
46 nhóm bằng chứng được tách thành 114 mục: 37 V1, 75 later, 2 drop legacy. Scope khác với mức xác nhận
runtime; trạng thái 37 mục V1 và gate lấy từ [PROGRESS](PROGRESS.md), không duy trì con số done thủ công ở workflow.

## 1. Tổ chức nhóm

| Vai trò | Sở hữu | Đầu ra bắt buộc |
|---|---|---|
| Lead/integrator | baseline, scope, interface, thứ tự merge | backlog, spec được chấp nhận, build tích hợp chơi được |
| Research KR/CN/TW | guide, patch note, video, thuật ngữ | claim có nguồn, region/date và giới hạn |
| RE/oracle | pack/code/runtime, fixture và phép đo | dữ liệu trích, trace, mức tin cậy và sai khác |
| Assets/client | converter, animation, render/audio/UI | asset manifest và cảnh chạy được |
| Core/net/server | luật trận, command/state, authority | mô phỏng, server và protocol mới |
| Reviewer/QA | kiểm nguồn và implementation độc lập | báo cáo tái lập, replay/diff, lỗi còn mở |

Đây là vai trò, không bắt buộc sáu người cố định. Một người có thể kiêm nhiều vai trò;
với giới hạn ba worker, lead chia thành ba luồng độc lập rồi đổi vai theo milestone.
Task có ranh giới rõ mới giao agent; quyết định sản phẩm và bằng chứng mâu thuẫn cần lead/designer xử lý.
Thêm agent không làm nhanh hơn các phần đang chờ cùng một spec/interface.

## 2. Một đường đi chung cho mọi tính năng

`Question → Evidence → Spec → Implementation → Independent verification → Integrated demo`

1. Lead đặt câu hỏi nhỏ: ví dụ “khiên chặn loại sát thương nào, hết hạn lúc nào?”.
2. Research tìm guide/patch theo tên Hàn, giản thể, phồn thể; RE đối chiếu pack/code/trace song song.
3. Reviewer phân biệt thông tin xác nhận được và thông tin chưa đủ. Lead chấp nhận spec hoặc
   chọn giá trị tạm, ghi rõ phạm vi. Không lấy số đông agent đồng ý làm bằng chứng.
4. Worker triển khai theo bản spec/interface đã ghi revision; spec thay đổi phải báo tác động
   đến core, network, UI và fixtures.
5. Reviewer kiểm theo nguồn/acceptance scenario; test do người viết code tạo không thay thế
   nguồn độc lập. Test meaningful: state transition, thứ tự sự kiện, biên va chạm, replay.
6. Lead merge theo dependency, chạy integration và cập nhật docs trong cùng thay đổi.

Mỗi task chỉ có một owner. Reviewer khác owner khi có thể. Mỗi nhánh/worktree riêng;
không nhiều agent đồng thời ghi cùng file. File dùng chung như Cargo workspace, schema,
rules registry và roadmap do integrator sửa hoặc dành lượt sửa rõ ràng.

## 3. Hợp đồng trước khi chạy song song

Chốt bản tối thiểu, có revision, của các interface sau trước khi worker phụ thuộc vào chúng:

- Asset ID/profile/build, tọa độ, đơn vị và nguồn metadata.
- Input command, SimulationState, SimulationEvent, snapshot/replay format.
- Quyền quyết định của server; dữ liệu client chỉ được gửi như yêu cầu.
- Quy tắc thời gian/số học: thử nghiệm dựa trên phép đo rồi chốt; không giả vờ đã biết tick gốc.
- Schema rule/config: ID ổn định, giá trị, đơn vị, profile, nguồn, trạng thái tạm/chính thức.
- Trình diễn tiêu thụ sự kiện mô phỏng; xác định cách tránh lặp audio/effect khi reconciliation.

Tên/type Rust cụ thể còn có thể đổi. Mục đích là worker kiểm được đầu vào/đầu ra của nhau,
không tự tạo ba mô hình trạng thái khác nhau rồi mới ghép lại.

## 4. Quản lý nguồn luật chơi

Mỗi claim có: `claim_id`, `feature_id`, mô tả, giá trị/đơn vị, tên thuật ngữ gốc,
`source_url` hoặc đường dẫn code/trace, vị trí đoạn/bảng/timestamp,
`region`, `published_date`, `accessed_date`, build được nói tới,
`target_build_match`, mức tin cậy, bằng chứng phản bác và reviewer.

Tách hai trục: **chất lượng nguồn** và **mức phù hợp với build mục tiêu**.
Nguồn official CN chính xác về CN vẫn chưa xác nhận KR. Hai bài copy cùng một bài gốc
không phải hai bằng chứng độc lập. Footer copyright không phải ngày xuất bản.

Trạng thái claim: `candidate → reviewed → accepted / rejected / unresolved`.
Luật được dùng trong engine ghi riêng `measured_vn`, `verified_kr`, `archive_other_region`,
`inferred_kr` hoặc `design_provisional`; một rule có thể liên kết nhiều claim.
Những nhãn này là đề xuất schema, chưa phải file dữ liệu đã triển khai.

Ưu tiên nguồn: dữ liệu/code đúng build + phép đo → guide/patch chính thức đúng khu vực/thời kỳ
→ video có ngữ cảnh → wiki/forum làm đầu mối. Video giúp quan sát nhưng chưa đủ suy ra
tick/subpixel/collision chính xác. Search snippet chỉ là đầu mối nếu chưa đọc được trang.
Nguồn web hữu ích cho luật, không thay thế protocol/server implementation thất lạc.

Với nguồn mâu thuẫn: lập bảng khác biệt theo build/region/event; không lấy trung bình các con số.
Không tìm thấy sau một lượt nghiên cứu có giới hạn thì chuyển sang RE hoặc DESIGN có ghi nhận,
đồng thời mở câu hỏi để kiểm lại; không chặn toàn bộ engine.

## 5. Nguồn web và trạng thái nghiên cứu

Danh sách nguồn chuẩn ở [research/sources](../research/sources/README.md),
backlog ở [QUESTIONS](../research/QUESTIONS.md). Không duy trì bảng nguồn thứ hai trong workflow.
Nguồn KR/CN/TW đều có ngày/build/region và giới hạn; đọc được recipe hoặc hướng dẫn
không đồng nghĩa đã khôi phục luật toàn bộ build KR mục tiêu.

## 6. Milestone và cách chia task

| Mốc | Luồng độc lập có thể chạy cùng lúc | Gate nghiệm thu |
|---|---|---|
| M0 — baseline/spec | Kiểm kê KR; research luật Battle; chuẩn bị phép đo VN | Chốt build, một map, tập item, register nguồn và interface tối thiểu |
| M1 — offline | Asset/render cảnh; core di chuyển/va chạm; RE/QA fixtures | Một nhân vật chạy/rẽ/chạm tường; báo sai khác và độ lặp phép đo |
| M2 — một trận | Bom/nổ/item; animation/audio; reviewer boundary cases | Trận offline hết vòng đời, replay và state hash ổn định theo hợp đồng đã chốt |
| M3 — online 2 người | Authority/session; client reconciliation; QA mạng | Hai client chơi hết trận, không lặp side effect; thử delay/jitter/loss/reconnect |
| M4 — Release 1 | Mở tập map/item đã chốt; lobby/UI; Việt hóa/QA | Luồng phòng đầy đủ, build 3 OS và báo cáo sai khác chấp nhận được |
| M5 — mở rộng | Nhóm theo mode hoặc subsystem có interface rõ | Mỗi mode có spec, fixtures, integration demo và nguồn/thiết kế được ghi nhận |

Research và RE đi trước worker triển khai một phần backlog. Khi phải chờ đo, worker assets/UI
vẫn tiến hành trên dữ liệu đã biết. Không cho hai agent tự viết hai phiên bản core/server song song.
Không cần chuyển hết 51.467 entry trước M1.

## 7. Mẫu task giao người/agent

```text
ID / owner / reviewer:
Baseline commit + branch/worktree:
Mục tiêu cụ thể và phần ngoài phạm vi:
Dependency + revision spec/interface:
Được sửa file/thư mục nào:
Nguồn/claim IDs + điều chưa biết:
Deliverables:
Acceptance scenarios / lệnh kiểm:
Nếu thiếu bằng chứng: báo unknown; chỉ dùng giá trị tạm theo spec.
Bàn giao: commit/diff, docs, kết quả kiểm, giới hạn và bước tích hợp.
```

Ví dụ `RULE-SHIELD-001`: research xác định tác dụng/thời lượng theo KR và CN; RE tìm
Attr/consumer; reviewer xác nhận các nguồn nói cùng loại khiên. Core worker chỉ triển khai
sau khi spec xác định damage nào bị chặn, thời điểm hết hạn, xử lý bẫy và các trường hợp biên.
Test cấu hình tạm phải ghi là test thiết kế tạm, không gắn nhãn tương đương nguyên bản.

## 8. Gate merge và theo dõi tiến độ

Theo [PROGRESS](PROGRESS.md), so trực tiếp với [FEATURE_CATALOG](../specification/FEATURE_CATALOG.md).
Registry `features.json` giữ implementation/tracking; execution.json giữ phase và release gates.
Integrator là writer của hai nguồn chung: ghi ID/assignee/revision khi nhận task, nhập proof/review
khi worker bàn giao, rồi chạy `python3 tools/kr_catalog.py` và `--check`.
Worker không tự sửa report % hoặc đồng thời ghi registry với worker khác.

Done chỉ sau verification+review+integration và dependency done theo AGENTS. In progress/review/blocked
không có credit phần trăm; planning/research không cộng vào % feature. % là tỷ lệ số mục nghiệm thu,
không phải thời gian/công sức/fidelity. Hết một task không đồng nghĩa hết phase hoặc hết V1.

- Worker bàn giao thay đổi nhỏ có demo/fixture; integrator merge theo dependency và giữ main chạy được.
- Dùng test phù hợp: parser với dữ liệu thật/truncated; core với replay và edge case;
  mạng với authority/convergence; hình ảnh theo RGBA/ROI; Việt hóa với glyph/wrap/placeholder.
- Nếu fixture mới chỉ được sinh từ chính implementation, phải ghi đây là regression baseline,
  không phải bằng chứng đúng bản gốc. Người review kiểm nguồn và sự phù hợp build riêng.
- Đánh giá bằng feature chơi được, rule có nguồn và số sai khác chưa giải quyết;
  không dùng số agent, số dòng code hay opcode đã phân loại làm phần trăm hoàn thành engine.
- Sau M2/M3 mới cập nhật lịch từ throughput thật và dependency còn lại.

Đề xuất lượt đầu: giao research Battle/item, asset cho một map và oracle di chuyển chạy song song;
lead chuẩn hóa spec/interface. Kết quả đủ điều kiện mới mở task triển khai core tương ứng.
