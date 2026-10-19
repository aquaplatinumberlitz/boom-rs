# 0009 — Giao thức V1: envelope có revision, InputCommand/Snapshot đầy đủ và codec

Status: **Proposed** — chờ review #14, 2026-10-19. Chưa tạo crate `boom-proto`; ADR accepted rồi
integrator mở task codec theo thứ tự dependency.

## Bối cảnh

`contracts-r3` đã có `PlayerInput` là input **authority đã chấp nhận** cho một tick
([ARCHITECTURE §5.1](../planning/ARCHITECTURE.md#51-kiểu-trong-boom-core)) và pipeline FFA. Chưa có kiểu
trên dây, envelope, snapshot hay codec. V1-024 cần chốt các contract đó trước khi viết `boom-proto` /
`boom-net`; V1-002 cần handshake phiên bản/rules/content. Task #14 chỉ làm ADR và mục §5.4, không tạo crate.

Ràng buộc đã có:

- Server quyết mọi kết quả; client chỉ dự đoán di chuyển của mình ([ADR 0005](0005-authority-and-prediction.md)).
- V1 dùng TCP sau một trait transport, frame có độ dài tiền tố, input gửi kèm ack và cửa sổ input chưa ack
  ([ADR 0006](0006-transport-default.md)). Application vẫn dedup, không dựa vào TCP.
- Fixed step, số nguyên, không float/`HashMap`/wall clock trong core; tick lấy từ `Rules`
  ([ADR 0004](0004-time-numeric-model.md), [ARCHITECTURE §4](../planning/ARCHITECTURE.md#4-rules-profile-content-và-mô-hình-thời-gian)).
- Mọi contract có revision; thay đổi thêm field là additive, đổi nghĩa là tăng revision
  ([ADR 0008](0008-extensibility-model.md)).
- Nghiệm thu V1 yêu cầu packet/input trùng, trễ, ngoài cửa sổ không làm đổi state, và bốn client hội tụ
  ([SCOPE_V1 §7](../planning/SCOPE_V1.md#7-bộ-nghiệm-thu-tích-hợp) mục 5–6).

Client KR/VN cũ dùng frame/obfuscate/opcode riêng ([LEGACY_VN_PROTOCOL](../specification/networking/LEGACY_VN_PROTOCOL.md));
giao thức mới không tái sử dụng các lớp đó. Hai đầu đều là Rust, không cần tương thích client cũ.

## Quyết định 1 — revision, handshake và phạm vi phiên bản

Có bốn định danh khác nhau, không gộp:

| Định danh | Giá trị V1 | Ý nghĩa |
|---|---|---|
| `wire_schema` | `1` | Cách đọc envelope + codec payload + nghĩa message. Đổi cách đọc là tăng số này. |
| `state_schema` | `1` | Bố cục DTO `Snapshot` và cách dựng lại `SimulationState`. |
| `rules_profile_id` + `rules_hash` | lấy từ `boom-content` | `profile_id` để đọc, `rules_hash` (`fnv1a64-v0`) để từ chối profile khác. |
| `content_sha256` | 32 byte | Hash manifest content đã chuẩn hóa; chỉ trao đổi đầy đủ trong handshake. |

- V1 chỉ chấp nhận **khớp chính xác** `wire_schema = 1`; không có chuyện client cũ nói chuyện với server mới.
  Khi cần tương thích nhiều revision, mở ADR mới và thêm negotiation; không suy diễn từ sự có mặt của
  `wire_schema`.
- `Hello` (C→S) mang `wire_schema`, `state_schema`, `rules_profile_id`, `rules_hash`, `content_sha256`,
  build client và danh sách capability. `Welcome` (S→C) trả `session_id`, `player_id`, các định danh server
  chấp nhận, `content_epoch`, `snapshot_interval_ms` và `protocol_limits_version`.
- Server so với profile/bundle **của mình**; không dùng hash client khai để chọn nội dung. Lệch revision,
  rules hash hoặc content hash bị từ chối trước khi vào sảnh (V1-002).
- `content_epoch` là số do server cấp cho một lần nạp content, chỉ có nghĩa trong phiên. Snapshot dùng epoch
  thay vì lặp 32 byte hash; handshake giữ hash đầy đủ. `content_epoch` không phải định danh vĩnh viễn của
  content, không dùng trong replay.
- Nếu không đọc được `wire_schema` (magic sai hoặc schema lạ), server đóng kết nối và ghi log; chỉ gửi
  `Reject` khi còn đọc được envelope schema 1. Client hiển thị lỗi cập nhật, không thử đoán.

Handshake không thay thế auth/session (V1-001, V1-028) và không tự tạo kênh tin cậy; TLS/lớp mã hóa thuộc
task backend ([ADR 0006](0006-transport-default.md)).

## Quyết định 2 — envelope 16 byte, tách khỏi payload

Mỗi frame transport (đã có độ dài tiền tố theo ADR 0006) chứa đúng một envelope + payload:

| Offset | Byte | Trường | Giá trị V1 |
|---:|---:|---|---|
| 0 | 4 | `magic` | `42 4F 4F 4D` (`BOOM`) |
| 4 | 2 | `wire_schema` | `u16` little-endian, `1` |
| 6 | 2 | `kind` | `u16` little-endian, bảng dưới |
| 8 | 2 | `flags` | `0`; bit lạ phải bị từ chối |
| 10 | 4 | `payload_len` | `u32` little-endian, `<= 65_520` |
| 14 | 2 | `reserved` | `0` |

`payload_len` phải khớp độ dài payload của frame; sai lệch là lỗi giao thức, không cắt bớt/đệm. Envelope
không mang `round_id`, vì lệnh phòng và handshake không có round. `round_id` nằm trong payload của message
thuộc trận.

| Kind | Tên | Chiều | Dùng ở V1 |
|---:|---|---|---|
| `0x0001` | `Hello` | C→S | Handshake |
| `0x0002` | `Welcome` | S→C | Handshake |
| `0x0003` | `Reject` | S→C | Lỗi handshake/session, có mã lý do |
| `0x000E` | `Ping` | hai chiều | Đo RTT/liveness, payload là `u64` không dấu |
| `0x000F` | `Pong` | hai chiều | Trả nguyên `Ping` |
| `0x0010` | `InputBatch` | C→S | Input trận, có cửa sổ dư thừa |
| `0x0011` | `InputRejected` | S→C | Lệnh ngoài cửa sổ/sequence/rate limit |
| `0x0020` | `Snapshot` | S→C | Snapshot đầy đủ |
| `0x0021` | `SnapshotRequest` | C→S | Xin snapshot đầy đủ để resync |
| `0x0030` | `RoundEvents` | S→C | Event của round, có `EventId` |
| `0x0040` | `RoomCommand` | C→S | Dành cho #13/backend; chưa định nghĩa trong ADR này |
| `0x0041` | `RoomState` | S→C | Dành cho #13/backend; chưa định nghĩa trong ADR này |

Message mới chỉ được gửi khi capability tương ứng được hai bên chọn trong handshake. V1 chỉ bật các kind
đã định nghĩa ở trên; gặp kind lạ thì `Reject`/đóng, không bỏ qua rồi đoán.

Giới hạn khung: `MAX_FRAME_BYTES = 64 KiB` gồm envelope; `MAX_SNAPSHOT_BYTES = 16 KiB`;
`MAX_INPUT_BATCH_BYTES = 1 KiB`. Đây là giới hạn giao thức (design), không phải luật chơi; giá trị nằm trong
`ProtocolLimits` có revision, không rải hằng số trong code.

## Quyết định 3 — `InputCommand` và cửa sổ input dư thừa

### 3.1 DTO

`InputBatch` (kind `0x0010`):

| Trường | Kiểu | Ghi chú |
|---|---|---|
| `round_id` | `u32` | Round server đã chốt; lệch round bị từ chối |
| `player` | `u8` | Chỉ để đối chiếu; server **bind** người chơi theo session, không route theo trường này |
| `ack_snapshot` | `u32` | `snapshot_seq` lớn nhất client đã áp dụng đủ; dùng để cắt cửa sổ và đo RTT |
| `commands` | mảng 1..8 | Sắp `seq` tăng liên tục, không trùng |

Mỗi `InputCommand`:

| Trường | Kiểu | Ghi chú |
|---|---|---|
| `seq` | `u32` | Bắt đầu từ 1 cho mỗi `(round_id, player)`, tăng 1 mỗi lệnh |
| `target_tick` | `u32` | Tick authority mà client dự đoán lệnh sẽ có hiệu lực |
| `bits` | `u8` | bit 0–2: hướng 0=None, 1=Up, 2=Down, 3=Left, 4=Right; bit 3: cạnh nhấn bom; bit 4–7 phải 0 |

`bits` giữ đúng một byte để lệnh nhỏ và dễ debug. Hướng là **đang giữ**, không phải delta; cạnh bom chỉ có
nghĩa trong tick đầu tiên lệnh được áp dụng. Không gửi tọa độ, máu, item, kill hay kết quả.

### 3.2 Cửa sổ dư thừa và resend

- Mỗi batch chứa tối đa 8 lệnh liên tiếp bắt đầu từ lệnh chưa được ack nhỏ nhất, không phải 8 lệnh mới nhất.
  Nhờ vậy một lệnh bị mất/không áp dụng ở giữa vẫn được gửi lại; lệnh đã áp dụng chỉ tốn vài byte và được
  dedup.
- Client gửi lại cùng cửa sổ cho tới khi `Snapshot.acks[player].seq` vượt qua lệnh đó. Không có cơ chế
  "gửi một lần rồi thôi".
- `ack_snapshot` không cấp quyền gì; server không dùng nó để bỏ qua validation. Nó giúp client/server tính
  RTT và biết snapshot nào đã tới.
- Batch lặp, lệnh lặp trong cùng batch, hoặc lệnh đã áp dụng là **idempotent**: trả ack hiện tại, không đổi
  state, không phát event.

### 3.3 Scheduler phía server (quy tắc dedup/hết hạn)

Server giữ cho mỗi `(round, player)`:

- `applied_seq`: sequence cao nhất đã xử lý xong (0 trước lệnh đầu).
- `pending`: lệnh có `seq > applied_seq`, sắp theo `seq`; tối đa `MAX_INPUT_GAP` lệnh.
- `last_held_dir` và tick cuối nhận lệnh, để giữ hướng khi không có lệnh mới.

Khi nhận batch:

1. Kiểm `round_id`, session/player, số lệnh, thứ tự `seq` tăng liên tục, `bits` reserved = 0,
   `target_tick` trong `[server_tick - INPUT_LATE, server_tick + INPUT_LEAD]`.
2. `seq <= applied_seq`: lệnh cũ/trùng → bỏ, không đổi state. `seq` đã có trong `pending`: giữ bản đầu;
   nếu payload khác bản đầu thì từ chối batch (`InputRejected::BadSequence`), vì cùng sequence phải cùng lệnh.
3. `seq > applied_seq + MAX_INPUT_GAP`: từ chối batch, không cấp phát vô hạn; client phải resync.
4. Chèn lệnh mới vào `pending`.
5. Mỗi tick, lấy các lệnh liên tiếp từ `applied_seq + 1` có `target_tick <= tick`:
   - nếu `target_tick < tick - INPUT_LATE`: lệnh **hết hạn**, bỏ hiệu lực (coi như neutral), vẫn tăng
     `applied_seq` để không chặn lệnh sau; ghi nhận `input_expired` để log/QA;
   - nếu còn trong cửa sổ: gộp các lệnh đã chín thành **một** `PlayerInput` cho tick hiện tại:
     `dir` = hướng của lệnh mới nhất, `place_bomb` = có ít nhất một cạnh bom trong các lệnh đã chín.
     Nhiều cạnh bom trong cùng tick chỉ tạo tối đa một `place_bomb`; cạnh thừa bị bỏ và đếm.
6. Nếu `applied_seq + 1` bị thiếu quá `INPUT_GAP_TIMEOUT`, bỏ qua các sequence thiếu (coi là neutral),
   tăng `applied_seq` tới sequence có trong `pending`, ghi `input_gap`. TCP tin cậy có thứ tự nên gap bình
   thường chỉ do lỗi client/batch bị từ chối; resend 8 lệnh phải lấp được gap.
7. Nếu không có lệnh mới trong `INPUT_STALE`, `dir` trở về neutral và `place_bomb = false`; đây là "input
   mới là neutral" của SCOPE_V1 §3, **không** phải ngắt kết nối. Ngắt quá 10 giây do V1-023 xử lý, không
   phải hằng số giao thức này.
8. `Snapshot.acks[player] = { last_seq: applied_seq, applied_tick: tick }` sau khi áp dụng input của tick.

Replay chỉ ghi `PlayerInput` **sau scheduler** cho từng tick, kể cả neutral/expired. Không ghi raw batch,
`target_tick` hay `ack_snapshot`: đến muộn/sớm/trùng không được làm replay khác đi. Vì vậy một lần chạy lại
chỉ cần tick + `PlayerInput` đã chốt.

### 3.4 Giá trị V1 (design, chưa phải luật KR)

Các ngưỡng dưới đây là **tham số giao thức** trong `ProtocolLimits`, không nằm trong `Rules` và không phải
claim luật chơi. Tick/round vẫn lấy từ `Rules` (`sim.ticks_per_second`, `round.duration_ms`); ms đổi sang
tick bằng `ms_to_ticks` một lần khi nạp, theo ADR 0004.

| Tham số | V1 | Ở 60 Hz | Lý do |
|---|---:|---:|---|
| `INPUT_REDUNDANCY` | 8 lệnh | ≈133 ms | Phủ RTT 100 ms + jitter 30 ms của SCOPE_V1; đổi sang datagram không đổi contract |
| `INPUT_LATE` | 100 ms | 6 tick | Lệnh tới muộn hơn bị coi là hết hạn, không tua state |
| `INPUT_LEAD` | 200 ms | 12 tick | Chặn client gửi tick tương lai xa để giữ buffer hữu hạn |
| `INPUT_GAP_TIMEOUT` | 100 ms | 6 tick | Đủ cho một vòng resend; gap lâu hơn coi là neutral, không treo trận |
| `INPUT_STALE` | 150 ms | 9 tick | Giữ hướng tối đa 2–3 RTT một chiều rồi neutral |
| `MAX_INPUT_GAP` | 16 lệnh | — | Trần buffer; vượt là lỗi giao thức |
| `MAX_INPUT_BATCHES_PER_S` | 120 | 2/tick | Client có thể gửi theo frame render; chặn spam |
| `SNAPSHOT_INTERVAL` | 50 ms | 3 tick | 20 Hz; đủ cho nội suy và reconcile ở RTT 100 ms |

Các số này là `design`, chưa có trace server gốc (server gốc thất lạc). V1-027/Q-010 phải đo p95 input
latency và correction; nếu vượt ngân sách thì sửa `ProtocolLimits`/ADR 0006, không sửa lén trong code.

## Quyết định 4 — `Snapshot` đầy đủ, chưa delta

V1 gửi **toàn bộ state** trong mỗi snapshot. Không có baseline/delta trong V1: một snapshot bất kỳ là điểm
resync độc lập, nên không cần chain, không sợ mất một delta, và client mới/desync chỉ cần snapshot mới nhất.
Delta chỉ được thêm sau khi có baseline + resync + test mất gói; việc đó cần ADR mới.

### 4.1 DTO

| Khối | Trường | Ghi chú |
|---|---|---|
| Header | `snapshot_seq u32`, `round_id u32`, `tick u32`, `state_schema u16`, `rules_hash u64`, `content_epoch u32`, `phase`, `rng_state u64`, `next_entity_id u32` | `snapshot_seq` tăng từ 1 trong round; `phase` = Playing / Ended(Draw) / Ended(Winner u8) |
| `acks` | `Vec<{ player u8, last_seq u32, applied_tick u32 }>` | Một mục/người chơi, sắp theo `player`; đây là ack của input đã áp dụng, không phải ack TCP |
| `players` | `Vec<{ id u8, x i32, y i32, facing u8, speed Stat, bombs Stat, range Stat, life }>` | `Stat = {value u8, max u8}`; `life` = Alive / Trapped{since u32} / Dead |
| `field` | `{ width u16, height u16, kinds_bits Vec<u8>, hidden Vec<{cell u16, kind u8}> }` | `kinds_bits` 2 bit/ô, bit thấp trước (ô 0 ở bit 0–1), hàng rồi cột, 0=Floor, 1=Solid, 2=Breakable, 3=không hợp lệ; `hidden` sắp theo `cell`, chỉ trên ô Breakable |
| `bombs` | `Vec<{ id u32, owner u8, cell u16, range u8, explodes_at u32, grace Vec<u8> }>` | Sắp theo `id`; `grace` sắp tăng |
| `flames` | `Vec<{ cell u16, until u32, source u32, owner u8 }>` | Sắp theo `cell`, mỗi ô tối đa một flame |
| `items` | `Vec<{ id u32, kind u8, cell u16 }>` | Sắp theo `id`, mỗi ô tối đa một item |

Thứ tự field trong bảng là thứ tự encode (postcard struct); đổi thứ tự/kiểu field là tăng `state_schema`.
`cell` = `y * width + x`; dùng `u16` để không phụ thuộc map 15×13. `kind` của item là chỉ số trong
`Rules::items`, không phải ID asset. `rng_state` là state SplitMix64 hiện tại, không phải seed; `next_entity_id`
là giá trị allocator sẽ cấp tiếp theo, không suy từ max ID đã thấy vì entity đã chết/nhặt có thể đã bị xóa.

Snapshot **không** chứa event. Event đi trong `RoundEvents` cùng stream TCP, có `round_id + EventId`; consumer
dedup theo `(round_id, EventId)` (ADR 0008). Snapshot không phát lại âm thanh/hiệu ứng.

### 4.2 Kích thước Village00, 4 người

Village00 KR là 15×13 = 195 ô (R-002). Bảng dưới là **trần trên theo từng trường** của DTO ở trên, dùng
postcard (mục 5) với các giả định ghi rõ; không phải số đo runtime.

Giả định cho cột "điển hình": 4 người, 8 bom (2/người), 20 ô lửa, 6 item trên nền, 40 item còn giấu.
Cột "trần" dùng độ dài **tối đa** của từng trường (ID/sequence 4–5 byte, ô 2 byte); bom tối đa theo
4 × 10 = 40 (C00 max bombs), lửa/items tối đa 195 mỗi loại. Cột "điển hình" cũng dùng cách tính cận trên
tương ứng, nên là **cận trên** của trạng thái đại diện đó, không phải số byte thực
của một frame cụ thể.

| Khối | Công thức trần | Điển hình | Trần cấu trúc |
|---|---|---:|---:|
| Envelope | 16 | 16 | 16 |
| Header snapshot | round/seq/tick/hash/… | 32 | 43 |
| `acks` | `1 + 4·(1+4+4)` | 21 | 37 |
| `players` | `4·(1+3+3+1+2+2+2+1)` | 60 | 68 |
| `field` | `2+2+(1+49)+(1+3·H)` | 175 (H=40) | 640 (H=195) |
| `bombs` | `1+B·(4+1+2+1+4+1+4)` | 137 (B=8) | 681 (B=40) |
| `flames` | `1+F·(2+4+4+1)` | 221 (F=20) | 2 146 (F=195) |
| `items` | `1+I·(4+1+2)` | 43 (I=6) | 1 366 (I=195) |
| **Tổng** | | **≈705 B** | **≈4,9 KiB** |

Với 20 Hz: điển hình ≈14 KB/s/client, 4 client ≈56 KB/s; trần lý thuyết ≈100 KB/s/client, ≈400 KB/s cho 4
client. `MAX_SNAPSHOT_BYTES = 16 KiB` còn dư so với trần cấu trúc; nếu map/mode sau này vượt, phải tăng
revision/limit có review, không nới âm thầm.

Số liệu này là `design`; Q-002 chưa chốt số ô Breakable thật của Village00, nên H là tham số. Khi có bảng
phân loại ô, thay H bằng số thật và ghi lại trong ADR/QA. Không dùng bảng này làm bằng chứng fidelity KR.

### 4.3 Kiểm và resync

- Client kiểm `payload_len <= MAX_SNAPSHOT_BYTES`, `snapshot_seq > last_seen`, `round_id` khớp, `state_schema`
  khớp, `rules_hash`/`content_epoch` khớp phiên.
- Dựng `World` từ content/rules đã handshake; dựng lại `SimulationState`; gọi
  `SimulationState::check_invariants(&World)` (contracts-r3) trước khi áp dụng. Snapshot sai bất biến bị bỏ,
  không panic, không áp dụng một phần; client gửi `SnapshotRequest`.
- Client bỏ snapshot cũ/trùng; snapshot mới thay toàn bộ state confirmed, rồi replay các lệnh chưa ack.
- `SnapshotRequest` = `{ round_id, last_snapshot_seq }`; server trả snapshot đầy đủ kế tiếp, không cần chờ kỳ.
- Server gửi snapshot đầu sau khi round bắt đầu và snapshot định kỳ.
- Không có "snapshot delta"; nếu sau này thêm, phải có `baseline_seq`, resync và test mất baseline trước.

### 4.4 Yêu cầu core khi mở `boom-proto` (không sửa trong task này)

DTO cần đọc/ghi vài field hiện chưa có API công khai trong `boom-core`:

- `Rng`: cần getter state (hoặc `SimulationState::rng_state()`) để đưa vào snapshot.
- `IdAllocator`: cần getter `next` và cách dựng lại allocator từ `next_entity_id`; hiện chỉ có `next_id()`.
- `Field`: có thể dựng lại bằng `Field::from_terrain` + `set_kind`/`set_hidden`, nên chưa cần API mới;
  nếu muốn tránh 195 lần gọi thì thêm constructor có kiểm.

Đây là thay đổi **contract core** (contracts-r4) do integrator làm, kèm test/invariant; ADR này chỉ ghi yêu cầu,
không sửa `crates/boom-core` trong PR #14. Không có các getter này thì `boom-proto` không thể snapshot đầy đủ
mà không suy đoán.

## Quyết định 5 — codec: envelope viết tay + payload postcard

### 5.1 So sánh

| Tiêu chí | postcard 1.x | protobuf/prost | Tự viết toàn bộ |
|---|---|---|---|
| Kích thước | Varint, rất gần trần lý thuyết; 705 B cận trên điển hình ở trên | Tốt nhưng có tag/field number; repeated packed | Tốt nhất nếu tối ưu bit, nhưng công sức cao |
| Revision/schema | Không self-describing, không hỗ trợ schema evolution; phải bump `wire_schema` | Có field number, unknown field, evolution tốt hơn | Tự đặt revision; không có gì tự động |
| Determinism | Encoder tuần tự, ổn định; decoder chấp nhận varint không canonical | Thứ tự field/map và default value làm canonical không bảo đảm | Kiểm soát tuyệt đối |
| Dependency/codegen | `postcard` + `serde`; không build.rs, không codegen | `prost` + `prost-build`/protoc; thêm bước build | Không dependency, nhưng nhiều code encode/decode |
| Debug | Không tự mô tả; cần dump tool/golden vector | `.proto` dễ đọc, nhưng cần tool | Tự viết dump |
| Hai đầu Rust | Phù hợp; `no_std`, không unsafe | Phù hợp nhưng nặng hơn | Phù hợp |
| Rủi ro decode | Cần giới hạn count/frame, tránh cấp phát theo length độc hại | Length-delimited trong frame; vẫn cần giới hạn | Tự chịu trách nhiệm bounds/fuzz |

Bincode 2 cũng đã được cân nhắc; nó tương đương postcard về kích thước/determinism nhưng có cấu hình
varint/derive riêng và khác biệt format 1↔2 dễ nhầm. Postcard 1.x có spec wire công khai, ổn định từ v1.0,
`no_std`, đủ cho V1; bincode không tạo lợi thế rõ nên không chọn. Spec tham chiếu:
[Postcard Wire Format](https://postcard.jamesmunns.com/wire-format), truy cập 2026-10-19.

Chọn **postcard 1.x cho payload**, **envelope viết tay 16 byte**. Lý do:

- Giao thức mới, hai đầu Rust, không cần protobuf đa ngôn ngữ; `postcard` nhỏ, `no_std`, không codegen,
  encoder ổn định và tiết kiệm byte.
- Envelope cần magic/schema/kind/len cố định, dễ kiểm trước khi gọi decoder; viết tay 16 byte rõ ràng hơn
  ép postcard vào header.
- Không có nhu cầu schema evolution trong V1; revision khớp chính xác, thay đổi là bump `wire_schema`. Nếu
  sau này cần nhiều client version, ADR mới có thể thêm negotiation hoặc chuyển codec.
- Tự viết toàn bộ không đem lại lợi ích đủ lớn so với rủi ro viết tay parser cho input không tin cậy; chỉ
  xem lại nếu postcard không đáp ứng test kích thước/ổn định.

Khi mở `boom-proto`, thêm `postcard` và `serde` vào `Cargo.toml` của crate đó, ghim phiên bản, ghi lý do
trong commit; đây là dependency nền phải ghi vào ARCHITECTURE. Không serialize trực tiếp `SimulationState`
hay `EventKind`: `boom-proto` có DTO riêng, adapter ở `boom-net`/`boom-server`/`boom-client` chuyển đổi
(`ARCHITECTURE §1`). Nhờ vậy layout bộ nhớ core không bị khóa vào dây.

### 5.2 Quy tắc decode/encode

- Encode: envelope trước, payload sau; postcard canonical do encoder tạo. Không dùng float; DTO chỉ số
  nguyên/enum/bool/Vec. Field order cố định theo DTO, không dựa vào thứ tự `HashMap`.
- Decode: kiểm magic/schema/kind/flags/reserved/len trước; giới hạn frame/snapshot/batch; kiểm số phần tử
  trước khi cấp phát (dùng fixed-capacity hoặc kiểm length prefix; không để length độc hại cấp phát vô hạn);
  từ chối trailing byte; kiểm `seq`/sort/invariant sau decode.
- Postcard decoder chấp nhận varint không canonical; runtime chỉ dùng giá trị đã decode, không hash raw byte.
  Golden test dùng encoder canonical. Không dùng wire byte làm khóa idempotency/ledger.
- Mọi lỗi decode là `Reject`/`InputRejected`/đóng session theo mức; không panic, không áp dụng một phần.

### 5.3 Ví dụ encode/decode

Ví dụ `InputBatch`: `round_id = 1`, `player = 0`, `ack_snapshot = 10`, hai lệnh:

- `seq = 7`, `target_tick = 120`, `bits = 0x08` (dir None, nhấn bom);
- `seq = 8`, `target_tick = 121`, `bits = 0x04` (dir Right, không nhấn bom).

Payload postcard (field order như DTO 3.1):

```text
01 00 0A 02 07 78 08 08 79 04
```

Giải thích: `01` round 1; `00` player 0; `0A` ack 10; `02` hai lệnh; `07` seq 7; `78` target 120;
`08` bits; `08` seq 8; `79` target 121; `04` bits. Envelope 16 byte:

```text
42 4F 4F 4D 01 00 10 00 00 00 0A 00 00 00 00 00
```

`42 4F 4F 4D` = "BOOM"; `01 00` schema 1; `10 00` kind `0x0010`; `00 00` flags 0; `0A 00 00 00`
payload_len 10; `00 00` reserved. Tổng frame 26 byte.

Decode giả định server tick 118: `[118-6, 118+12] = [112, 130]`, cả 120/121 trong cửa sổ; batch hợp lệ,
hai lệnh liên tiếp, reserved bits 0. Server buffer tới tick 120/121, gộp thành `PlayerInput` tại tick tương
ứng; `Snapshot.acks[0].seq` sau đó là 8. Nếu batch tới khi server tick 130 thì `target_tick=120 < 130-6=124`,
batch bị `InputRejected::OutOfWindow` ngay, không vào `pending`, state không đổi. Trường hợp lệnh đã vào
`pending` rồi mới quá hạn vì gap/head-of-line được xử lý ở bước 5: bỏ hiệu lực, ack tiến và ghi expired.

Ví dụ này cũng là golden vector đầu tiên cho `boom-proto`; khi codec được viết, test phải tạo đúng byte này
và decode ngược, trên ba OS.

## Tác động tới prediction/reconciliation (ADR 0005)

1. Client giữ state confirmed = snapshot mới nhất; danh sách lệnh chưa ack (`seq` + `target_tick` + input).
2. Khi có snapshot: kiểm/áp state confirmed; bỏ lệnh có `seq <= acks[player].seq`; replay các lệnh còn lại
   từ `snapshot.tick` bằng chính `boom-core` với `PlayerInput` đã chuyển đổi, các người chơi khác giữ theo
   snapshot (không ngoại suy). Đây là correction; hiển thị mượt là việc presentation, không sửa core.
3. Trong lúc chờ, client dự đoán **chỉ di chuyển của mình**; bom chỉ là bóng chờ, nổ/nhốt/chết/item chỉ
   hiện theo `RoundEvents`/snapshot server. Không phát âm thanh/hiệu ứng từ state dự đoán.
4. Client gửi cửa sổ 8 lệnh chưa ack mỗi batch; `target_tick = snapshot.tick + số tick local kể từ snapshot + 1`
   (xấp xỉ; server clamp theo cửa sổ). Nếu lệnh bị expired/rejected, snapshot ack cho biết và client bỏ nó;
   không giữ mãi lệnh cũ.
5. Event dedup theo `(round_id, EventId)`; snapshot không phát lại event. Replay dự đoán không ghi economy/log
   (ADR 0008).
6. Nếu p95 correction/input latency vượt ngân sách V1-027, xem lại `INPUT_*`/`SNAPSHOT_INTERVAL`; nếu vẫn
   vượt, đảo transport theo ADR 0006 hoặc mở ADR mới. Không đổi ngầm hằng số trong code.

## Hệ quả

- `boom-proto` (mở sau khi ADR accepted) giữ DTO/envelope/codec + golden vector; `boom-net` giữ frame/session/
  rate limit; `boom-server` giữ scheduler/round; `boom-client` giữ prediction/reconcile.
- Test bắt buộc: round-trip mọi message/kind; reject magic/schema/len/flags/reserved; duplicate/stale/out-of-window/
  gap/rate-limit; snapshot full dựng lại đúng `SimulationState` và qua `check_invariants`; kích thước dưới trần;
  cùng byte trên Windows/Linux/macOS.
- `ProtocolLimits` có revision riêng; `Rules` vẫn là nguồn tick/round. Không đưa hằng số luật vào code.
- `ReplayHeader`/`ReplayRecord` hiện tại không đổi trong PR này. Khi V1-026 mở, thêm `content_sha256`/checkpoint
  theo contract replay; raw input mạng không vào replay.
- Index ADR (`docs/decisions/README.md`) và dòng "Codec giao thức" ở `ARCHITECTURE §8` do integrator cập nhật
  khi nhận ADR; PR #14 chỉ sửa ADR mới + §5.4 theo phạm vi issue.

## Phương án đã cân nhắc

| Phương án | Lý do không chọn |
|---|---|
| Client-authoritative như legacy | Trái ADR 0005; không đáp ứng acceptance V1-024 |
| UDP/datagram mặc định | ADR 0006 chọn TCP trước; contract input dư thừa đã chuẩn bị để đảo sau |
| Chỉ gửi lệnh mới nhất, không dư thừa | Mất một lệnh là kẹt sequence/gap; trái ADR 0006 |
| Delta snapshot ngay từ V1 | Chưa có baseline/resync/test mất baseline; full snapshot đơn giản và đủ cho map 195 ô |
| JSON/MessagePack | Lớn hơn nhiều, dễ dính float/không ổn định thứ tự; không phù hợp 20 Hz |
| Protobuf ngay | Schema evolution tốt nhưng nặng/codegen, canonical không bảo đảm; chưa cần đa ngôn ngữ |
| Tự viết codec toàn bộ | Rủi ro parser cho input không tin cậy, không lợi bằng postcard ở quy mô này |

## Bằng chứng, giới hạn và điều kiện đảo

- Bằng chứng bối cảnh: ADR 0004/0005/0006/0008, contracts-r3, `LEGACY_VN_PROTOCOL`, R-066 (legacy P2P thay bằng
  snapshot). Không có trace server gốc cho giao thức mới; đây là **design** của dự án, không phải tuyên bố
  server KR/VN làm vậy.
- Kích thước snapshot là ước lượng cấu trúc, không phải đo runtime; phụ thuộc số ô Breakable thật (Q-002),
  cách chọn item và codec. Khi có `boom-proto`, test golden phải ghi số byte thật.
- Các ngưỡng input/snapshot là `design`; Q-010/V1-027 đo p95. Không dùng số này làm luật gameplay.
- `content_sha256` giả định manifest content canonical từ P4; nếu pipeline chọn hash khác, V1-002 và ADR này
  phải sửa trước khi code.
- Không có mã hóa/authentication trong envelope; TLS/session thuộc task backend. Không coi hash client là
  nguồn tin cậy.
- Điều kiện đảo: postcard không đạt kích thước/ổn định/security test → tự viết codec với cùng DTO contract,
  ADR mới; cần delta trước V1-027 → ADR mới có baseline/resync; p95 vượt ngân sách → chỉnh `ProtocolLimits`
  hoặc ADR 0006; content hash đổi → sửa V1-002/ADR này.

## Còn thiếu / câu hỏi mở

- Q-002: số ô Breakable/item giấu thật của Village00 để thay H trong bảng kích thước.
- Q-010/V1-027: ngân sách correction/input latency p95, `INPUT_*`/`SNAPSHOT_INTERVAL` cuối cùng.
- V1-002/P4: định dạng manifest content và canonical hash (ADR này giả định SHA-256).
- Contract core: getter RNG state và allocator next (mục 4.4), cần contracts-r4 trước khi `boom-proto` snapshot.
- Room command/state, auth, TLS, reconnect: ngoài phạm vi ADR này; #13/backend mở task riêng.
