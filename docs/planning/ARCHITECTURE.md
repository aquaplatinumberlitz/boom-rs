# Kiến trúc dự án Rust — đang thiết kế

Trạng thái **draft `architecture-r0`**, 2026-10-19; chưa tạo Cargo workspace hoặc chọn dependency.
Baseline sản phẩm: **`KR-00967-V1-scope-1`**. Đọc [REWRITE_PLAN](REWRITE_PLAN.md),
[SCOPE_V1](SCOPE_V1.md), [AGENTS.md](../../AGENTS.md). Đây là nguồn chuẩn về cấu trúc/interface;
scope và policy người chơi thuộc SCOPE_V1, không sao chép thành bộ quy tắc khác ở đây.

## 1. Boundary và cấu trúc đề xuất

Đây là layout dự kiến sau gate readiness; các thư mục `crates/` bên dưới chưa được tạo.
Đặt engine chung trong repo tích hợp được chọn, không xây engine riêng trong `boomkr-re`.

```text
crates/
  boom-core/       # Luật trận, state, simulation, RNG, hash/replay primitives
  boom-content/    # Schema runtime: map/rules/character/animation/content identity
  boom-assets/     # Đọc/chuyển pack, validation, xuất bundle qua CLI
  boom-proto/      # Envelope và DTO giao thức mới, không chứa luật
  boom-client/     # Input, prediction, renderer/audio/UI và kết nối
  boom-server/     # Auth, lobby/room, authority, persistence và vận hành
  boom-replay/     # CLI chạy replay, so state/event, báo sai khác
legacy/           # Vị trí dự kiến cho adapter/oracle cũ sau migration có kiểm chứng
```

Chưa tách render/audio/UI/net thành nhiều crate nhỏ trước khi cần tái sử dụng. Giữ chúng thành module
có interface rõ trong client/server; chỉ tách khi ownership, compile boundary hoặc dependency yêu cầu.
CLI asset có thể nằm cùng package boom-assets để giảm số package ban đầu.

Dependency DAG dự kiến, mũi tên nghĩa là “phụ thuộc vào”:

```text
boom-content → (không phụ thuộc client/server/protocol)
boom-core    → boom-content
boom-assets  → boom-content
boom-proto   → boom-content (chỉ ID/schema identity)
boom-client  → boom-core, boom-content, boom-proto
boom-server  → boom-core, boom-content, boom-proto
boom-replay  → boom-core, boom-content
```

`boom-core` không mở socket, đọc file, gọi wall clock, render hoặc phát âm thanh. `boom-proto`
không phụ thuộc toàn SimulationState: server/client chuyển đổi DTO tại adapter để schema mạng
không khóa layout bộ nhớ core. Runtime tiêu thụ bundle đã validate, không buộc client phải mang pack decoder.
Các dependency này là đề xuất boundary, chưa phải manifest đã được kiểm tra bằng Cargo.

## 2. Legacy oracle được giữ độc lập

Server hiện có nói giao thức client cũ, có giá trị làm fixture và phòng đo; nó không tự trở thành server
production bằng đổi tên crate. Main và worktree l10n có snapshot server khác nhau; chọn baseline trước
khi migration. Không di chuyển `server/` ngay trong pha docs hoặc lấy bản l10n cũ đè main.

Khi triển khai migration, integrator phải kiểm kê command, script, đường dẫn binary và golden test;
giữ cách chạy cũ hoạt động hoặc cập nhật toàn bộ caller trong cùng thay đổi có kiểm chứng.
Không để dependency legacy kéo vào boom-core. Replay mới và trace client cũ có adapter/schema riêng;
log server giả lập chỉ mô tả hành vi giả lập, không chứng minh luật server KR gốc.

## 3. Rules, content và mô hình thời gian

Runtime profile gồm `profile_id`, `schema_revision`, `rules_revision`, `content_hash` và source manifest.
Tách dữ liệu nguồn bất biến khỏi overlay DESIGN của V1. Village00 chỉ có drop tiền trong dữ liệu đã đọc;
profile V1 Bomb/Fire/Roller là overlay có version, không sửa pack gốc hoặc gọi là native KR.

Rule record cần có tên, đơn vị, giá trị/miền giá trị, nguồn/build/region, trạng thái bằng chứng và revision.
Speed-level không tự là px/s; trọng số không tự là xác suất. Dev profile được phép dùng giá trị tạm
có nhãn; profile phát hành phải qua gate tương ứng. Schema phải biểu đạt unknown thay vì âm thầm default.

**Chưa quyết định tick rate, fixed/variable step hoặc số học float/fixed-point.** Core cần bước mô phỏng
có thứ tự xác định cho server/replay, nhưng cách ánh xạ thời gian gốc phải được đo trước khi chốt.
Nếu chọn fixed step mới, đánh giá sai khác chuyển động/collision và ghi DESIGN khi cần; không mặc định
chạy cùng công thức ở tần số khác sẽ giữ nguyên feel. Cross-platform determinism cần fixture/hash thực tế.

Gate thời gian/số học: trace lặp được, đơn vị tọa độ và rounding rõ, cornering/hitbox có kịch bản,
thứ tự update và timer có spec. Các giá trị tạm của emulator không được dùng làm oracle độc lập.

## 4. Hợp đồng tối thiểu — draft `contracts-r0`

Tên dưới đây là vocabulary để chia task, chưa phải API Rust/wire schema đã freeze. Mỗi task phụ thuộc
contract phải ghi revision; thay đổi field/semantics phải review đồng thời core, net và QA.

| Contract | Nội dung tối thiểu | Bất biến cần review |
|---|---|---|
| `InputCommand` | session/round ID, player ID do server bind, command sequence, intended simulation step, hướng held, cạnh nhấn bomb | Không gửi tọa độ/kết quả damage để server tin; sequence dedup; input window hữu hạn |
| `SimulationState` | step/time, round phase, entity ID ổn định, player/bomb/flame/item/block state, timers, RNG state, allocation counters | Đủ để tiếp tục mô phỏng; thứ tự iteration và rounding xác định; không chứa audio/GPU/socket |
| `SimulationEvent` | round ID, step, event ID ổn định, kind, entity IDs, payload cần trình diễn | Một nguyên nhân không sinh hiệu ứng nhiều lần khi replay/reconcile; thứ tự event xác định |
| `Snapshot` | protocol/schema revision, rules/content hash, round/step, authoritative state DTO, input ack/watermark | Client biết lệnh nào đã xử lý; bản đầy đủ khôi phục được state, delta chỉ thêm sau khi có baseline/resync |
| `ReplayHeader` | build/schema/rules/content identity, initial state hoặc seed+setup đầy đủ, roster/spawn, clock model | Seed một mình không đủ nếu content/order khác; từ chối revision không hỗ trợ thay vì phát lại sai âm thầm |
| `ReplayRecord` | step, input được chấp nhận, input neutral/thay thế, sự kiện vận hành ảnh hưởng core, hash checkpoint | Ghi đầu vào thực được server áp dụng, không chỉ packet client gửi; replay không dựa wall clock |

SimulationState còn cần spec RNG algorithm, bounded sampling, entity allocation/order, round seed và
canonical serialization/hash. Hash bỏ presentation/caches, gồm mọi trạng thái ảnh hưởng bước kế tiếp.
Giá trị timer/command window, tick ordering chính xác và codec còn mở; không tự chốt số ở file này.

Phân biệt command phòng đáng tin cậy (`ready/start/leave`) với input trận tần suất cao. Cả hai có request
identity/dedup nhưng không dùng chung chính sách hết hạn. Policy phòng/start/disconnect lấy từ SCOPE_V1.

Tick pipeline dự kiến cần QA review: chọn input hợp lệ/neutral → movement/collision → bomb/item/damage
và timer theo thứ tự được spec → giải hết event cùng step → xét kết quả → snapshot/hash/events.
Đây chưa quyết định thứ tự chi tiết giữa pickup, flame và chain; cần boundary scenarios trước khi coding.

## 5. Net authority, prediction và side effects

Server giữ state chuẩn, bind danh tính vào kết nối, kiểm phase, quyền host, sequence/window và giới hạn
hành động; client không quyết định spawn, item, hit, chết hoặc người thắng. Chia sẻ core không tự tạo
anti-cheat: rate limit, validation, auth/session và lỗi protocol vẫn là trách nhiệm server.

Client lưu input chưa ack và predicted state. Khi nhận snapshot: thay bằng authoritative baseline,
loại input đã ack, chạy lại input còn hiệu lực; presentation nội suy/hiệu chỉnh riêng. Phải chốt phạm vi
prediction (chuyển động hay cả bom) và xử lý remote entities trước khi task renderer/net triển khai độc lập.
Không mặc định rollback toàn trận hoặc tin toàn state remote chỉ để giảm độ giật.

Audio/effect/UI tiêu thụ event qua presentation journal có ID; replay prediction không ghi economy,
log kết quả hoặc gửi notification lần hai. Cần policy cho effect dự đoán bị server từ chối và effect confirmed.
Kết quả round được commit một lần bởi authority. Late snapshot của round cũ bị bỏ theo round identity.

Nghiệm thu mạng dùng fixture SCOPE_V1: RTT/jitter/loss/duplicate/reorder, input trễ/trùng, resync và
loading/disconnect. Ngoài hash hội tụ cần đo correction, input latency và không lặp audio/effect.
Target hội tụ của engine mới không tự chứng minh feel KR giống nguyên bản.

## 6. Asset pipeline, animation và Việt hóa

Pipeline dự kiến: pack manifest/hash → decode có provenance → resolve dependency closure → schema
runtime → bundle/version/hash → validation → client. Tận dụng reader/tool đang có sau khi kiểm coverage,
không port lại chỉ vì đổi ngôn ngữ. Byte decode đúng không chứng minh frame assembly/timing đúng.

Content closure V1 phải nối Village00→tile/object/collision, C00/C01→Attr/animation và mọi state bomb,
flame/item/trapped/dead; không skip field/resource unknown âm thầm. Lưu source entry/hash và mapping
trong manifest; PNG/atlas cần canonical pixel check, còn animation/layout cần scene fixtures riêng.

Renderer nhận presentation snapshot và animation metadata; audio nhận event/mixer policy.
Render cadence tách simulation step. Cần kiểm pivot, clipping, z-order, blend, integer scale, input-to-screen
và âm thanh trên ba OS bằng fixture; không dùng tỷ lệ pixel nền lớn để che lỗi đối tượng quan trọng.

Chuỗi Việt lấy từ catalog/l10n hiện có, ánh xạ key rõ; ảnh có chữ chỉ đưa vào bundle V1 khi có review decision.
Font Unicode, fallback, wrap và focus là contract UI. Golden bản Việt hóa tách golden client KR/VN gốc;
đổi font/nội dung có chủ ý không coi là lỗi fidelity. Chỉ 23 mục pending toàn catalog không có nghĩa mọi
chuỗi/ảnh V1 đã được duyệt; phải resolve tập tài nguyên V1 cụ thể.

## 7. Quyết định còn mở và gate lựa chọn

Chưa chọn library/version từ kế hoạch cũ. ADR phải so phương án, nêu ràng buộc và kết quả thí nghiệm;
tra tài liệu chính thức hiện hành khi bắt đầu chọn dependency/API.

| Quyết định | Phương án cần so | Gate trước khi chốt |
|---|---|---|
| Client stack | Engine có sẵn; hoặc framework/window+GPU với renderer riêng | Một scene V1: sprite alpha/pivot/z-order, text Việt, input, audio, scale và build ba OS; so độ đúng và chi phí bảo trì |
| Transport | Reliable stream đơn giản; hoặc datagram với reliability thích hợp; hybrid chỉ khi có lợi ích đo được | Command/snapshot contract, head-of-line/correction ở fixture mạng, auth/encryption, reconnect và vận hành |
| Persistence | SQLite V1; hoặc PostgreSQL ngay nếu deployment cần | Identity uniqueness/transaction, migration/backup và test backend đã chọn; không mặc định dev/prod hai DB giống semantics |
| Timing/số học | Fixed step/biến thiên có kiểm soát; float/fixed-point theo evidence | Replay cross-OS, movement/collision boundary, sai khác so client tham chiếu và policy DESIGN |
| Prediction | Movement-only; mở rộng bomb/interactions; rollback rộng hơn | Correction budget, state completeness, CPU/memory và side-effect fixture; không mở scope trước benchmark |
| Content format | Schema typed và bundle đóng gói; lựa chọn encoding sau | Version compatibility, provenance, validation, asset closure và rebuild tái lập |

Backend V1 chỉ lưu identity và tùy chọn cần thiết; room/round là ephemeral theo SCOPE_V1. Chưa cần
ledger/economy/guild/job platform. Auth tối thiểu vẫn cần credential/session và secret handling hợp lý;
chi tiết deployment thuộc ADR/task backend, không đưa cấu hình cá nhân vào docs dự án.

## 8. Thứ tự bàn giao kiến trúc trước coding

1. Integrator chọn baseline tích hợp và cách giữ legacy command; assets/RE bàn giao closure/unknown.
2. Core+net+QA review `contracts-r0`, timing/rules profile và kịch bản boundary; ghi revision được chấp nhận.
3. Chọn các phương án cần prototype có giới hạn, acceptance và owner/reviewer; ADR ghi quyết định khi có bằng chứng.
4. Kiểm readiness trong REWRITE_PLAN rồi mới mở task skeleton nhỏ. Workspace mới không tự đồng nghĩa gameplay đã đúng.

Không cần giải các mode later hoặc quyết DB production toàn game để bắt đầu lát cắt V1. Những điểm
chưa biết ảnh hưởng interface phải có owner, cách giải hoặc DESIGN rõ trong QUESTIONS trước khi giao task phụ thuộc.

Chính sách nguồn hiện hành: [VN/KR trusted ngang nhau](../decisions/0002-vn-kr-trusted-sources.md). Dùng trực tiếp bằng chứng VN cho luật chung; giữ provenance và xử lý khác biệt cụ thể.
