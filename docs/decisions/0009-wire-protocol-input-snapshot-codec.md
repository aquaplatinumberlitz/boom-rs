# 0009 — Giao thức V1: envelope có revision, InputCommand/Snapshot đầy đủ và codec

Status: **Accepted**, 2026-10-19 — Codex approve và merge PR #24 (issue #14). Chưa tạo crate `boom-proto`;
integrator mở task triển khai codec ở P7 theo thứ tự dependency.

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
| `0x0011` | `InputRejected` | S→C | Lệnh quá tương lai/sequence/rate limit; mang ack để resync |
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
| `commands` | mảng 1..8 | Trong batch: `seq` tăng liên tục, không trùng; giữa các batch có thể có lỗ hổng do bỏ lệnh (server xử lý như gap) |

Mỗi `InputCommand`:

| Trường | Kiểu | Ghi chú |
|---|---|---|
| `seq` | `u32` | Bắt đầu từ 1 cho mỗi `(round_id, player)`, tăng 1 mỗi lệnh |
| `target_tick` | `u32` | Tick authority mà client dự đoán lệnh sẽ có hiệu lực |
| `bits` | `u8` | bit 0–2: hướng 0=None, 1=Up, 2=Down, 3=Left, 4=Right; bit 3: cạnh nhấn bom; bit 4–7 phải 0 |

`bits` giữ đúng một byte để lệnh nhỏ và dễ debug. Hướng là **đang giữ**, không phải delta; cạnh bom chỉ có
nghĩa trong tick đầu tiên lệnh được áp dụng. Không gửi tọa độ, máu, item, kill hay kết quả.

`InputRejected` (kind `0x0011`, S→C):

| Trường | Kiểu | Ghi chú |
|---|---|---|
| `round_id` | `u32` | Round bị từ chối; lệch round không vào scheduler |
| `reason` | `u8` | 0 `BadSequence`, 1 `OutOfWindow` (chỉ quá tương lai), 2 `GapTooLarge`, 3 `RateLimited` |
| `applied_seq` | `u32` | Ack hiện tại sau dedup/retire; client bỏ mọi lệnh `seq <=` giá trị này |
| `first_seq` | `u32` | `seq` đầu tiên của batch bị từ chối; `0` khi batch sai cấu trúc không xác định được |
| `command_count` | `u8` | Số lệnh của batch bị từ chối (`0` nếu không xác định); dải `first_seq..first_seq+command_count-1` là batch đã bị reject |
| `server_tick` | `u32` | Tick server lúc từ chối; client dùng để phân loại quá khứ/tương lai và quyết định chờ/bỏ lệnh chắc chắn chưa buffer, không dùng để sửa payload lệnh đã gửi |

`InputRejected` không áp dụng input và không thay đổi state; nó là tín hiệu resync sequence, không phải
"hủy cả cửa sổ". Client xử lý như một ack (§3.2).

### 3.2 Cửa sổ dư thừa và resend

- Mỗi batch chứa tối đa 8 lệnh liên tiếp **trong batch**, bắt đầu từ lệnh chưa được ack nhỏ nhất còn giữ,
  không phải 8 lệnh mới nhất. Nhờ vậy một lệnh bị mất/không áp dụng ở giữa vẫn được gửi lại; lệnh đã áp
  dụng chỉ tốn vài byte và được dedup. Nếu client đã bỏ một seq (chắc chắn chưa buffer), batch kế tiếp bắt
  đầu ở seq lớn hơn; lỗ hổng đó là gap hợp lệ và do gap timeout retire.
- Ack (`Snapshot.acks[player].last_seq` hoặc `InputRejected.applied_seq`) là `applied_seq` đã **giải quyết**:
  đã áp dụng **hoặc** đã retire vì quá hạn/gap. Client bỏ mọi lệnh `seq <=` ack; lệnh quá hạn không bao giờ
  bị giữ lại chỉ vì chưa từng vào `pending`.
- Client gửi lại cùng cửa sổ cho tới khi ack vượt qua lệnh đó. Không có cơ chế "gửi một lần rồi thôi".
- `ack_snapshot` không cấp quyền gì; server không dùng nó để bỏ qua validation. Nó giúp client/server tính
  RTT và biết snapshot nào đã tới.
- **Payload bất biến theo `seq`:** `seq` được cấp khi lệnh lần đầu vào một batch gửi; sau đó
  `(target_tick, bits)` của seq đó không được đổi. Client chỉ tính/chỉnh `target_tick` cho lệnh **chưa
  gửi**; input thay thế nhận `seq` mới. Vì vậy không có "retarget" một seq có thể đã vào `pending` của
  server.
- Khi nhận `InputRejected` (hoặc snapshot có ack mới), client coi `applied_seq` là ack: bỏ lệnh `seq <=`
  ack. Với lệnh **đã gửi** còn lại, client **resend y nguyên payload**, không đổi `target_tick` và không
  bỏ chỉ vì `target_tick < server_tick - INPUT_LATE`; lệnh quá hạn sẽ được server retire và ack, client có
  thể ngừng dự đoán trước nhưng vẫn giữ payload để resend tới khi ack. Lệnh **chưa gửi** được tính
  `target_tick` mới (nếu còn dùng) hoặc bỏ tự do. Không resend nguyên batch lỗi.
- Client được bỏ một lệnh **đã gửi** chỉ khi chắc chắn server chưa buffer nó. Một batch bị
  `InputRejected` bị từ chối trước khi chèn mọi lệnh **mới** trong batch (§3.3 bước 1–3), nên lệnh chỉ
  từng xuất hiện trong đúng batch đó là chắc chắn chưa buffer; client được bỏ nó (không tái sử dụng `seq`)
  hoặc resend. `InputRejected.first_seq`/`command_count` xác định đúng dải batch bị reject; nếu hai trường
  là `0` (không xác định), client coi mọi lệnh liên quan là **có thể đã buffer**. Nếu lệnh từng nằm trong
  batch khác, hoặc client không chắc, coi là **có thể đã buffer**: chỉ resend y nguyên tới khi ack. Bỏ một
  seq tạo lỗ hổng; gap timeout/late retire của server đóng lỗ hổng đó, client không tự nhảy ack.
- Lệnh quá tương lai trong batch bị `OutOfWindow` **không được chèn**, nhưng cũng không được kéo theo lệnh
  quá khứ ở đầu hàng: client tách lệnh quá tương lai ra, chờ tới khi nó vào cửa sổ rồi gửi lại y nguyên
  (hoặc bỏ nếu chắc chắn chưa buffer, dùng seq mới cho input thay thế) và gửi phần còn lại để retire/ack
  tiến. Không gộp một lệnh quá tương lai với lệnh mới làm reject cả batch.
- Batch lặp lại hoặc lệnh đã áp dụng là **idempotent**: không đổi state, không phát event; ack hiện tại đi
  trong snapshot kế tiếp. Lệnh trùng ngay trong một batch là batch sai cấu trúc (`seq` phải tăng liên tục)
  và bị `InputRejected::BadSequence`.

### 3.3 Scheduler phía server (dedup, retire, gap)

Server giữ cho mỗi `(round, player)`:

- `applied_seq`: sequence cao nhất đã **giải quyết liên tục** (0 trước lệnh đầu). "Giải quyết" = đã áp dụng
  hoặc đã retire vì quá hạn/gap; đây là giá trị ack, không có nghĩa input đã được áp dụng.
- `pending`: lệnh có `seq > applied_seq`, sắp theo `seq`; tối đa `MAX_INPUT_GAP` lệnh.
- `gap_since_tick`: tick đầu tiên thấy `applied_seq + 1` thiếu trong khi đã buffer lệnh cao hơn; `None` khi
  không có gap.
- `last_held_dir` và tick cuối nhận lệnh, để giữ hướng khi không có lệnh mới.

Xử lý một batch ở `server_tick` theo đúng thứ tự sau; **dedup trước khi kiểm cửa sổ**. Batch là **atomic**
với lệnh mới: hoặc chèn tất cả lệnh mới hợp lệ, hoặc từ chối và không chèn lệnh mới nào; reject không
retire:

1. **Kiểm cấu trúc batch** (lỗi thì không sửa state): `round_id`, session/player, 1..8 lệnh, `seq` tăng
   liên tục **trong batch**, không trùng, `bits` reserved = 0. Lỗ hổng giữa các batch là gap hợp lệ, không
   phải lỗi cấu trúc. Không kiểm `target_tick` ở bước này.
2. **Dedup theo sequence**:
   - `seq <= applied_seq`: lệnh đã giải quyết (kể cả ack chưa tới client) → bỏ qua hoàn toàn, **không** kiểm
     `target_tick` và không đổi state. Nhờ vậy lệnh cũ trong batch resend không làm hỏng lệnh mới.
   - `seq` đã có trong `pending`: giữ bản đầu; payload giống → bỏ qua; payload khác → từ chối batch
     (`InputRejected::BadSequence`), không sửa state, vì cùng sequence phải cùng lệnh. Bản trong `pending`
     thắng; batch bị từ chối trước khi chèn các lệnh mới, client xử lý theo §3.4(d).
   - Còn lại là lệnh mới.
3. **Kiểm lệnh mới**: `target_tick > server_tick + INPUT_LEAD` → từ chối batch
   (`InputRejected::OutOfWindow`, nghĩa "quá tương lai"), không sửa state và không chèn bất kỳ lệnh mới nào
   của batch; `seq > applied_seq + MAX_INPUT_GAP` → từ chối (`InputRejected::GapTooLarge`). Lệnh có
   `target_tick < server_tick - INPUT_LATE` **không** bị từ chối; nó sẽ được retire, không áp dụng.
   `InputRejected` mang `applied_seq`/`server_tick`; client tách lệnh quá tương lai ra và gửi lại phần còn
   lại (nhất là lệnh quá khứ ở đầu hàng) để ack tiến, không gộp chúng mãi thành một batch bị reject.
4. **Chèn** các lệnh mới hợp lệ vào `pending`. Miền `seq` ở bước 3 bảo đảm `pending` không vượt
   `MAX_INPUT_GAP` sequence.
5. **Retire đầu hàng** ngay sau khi chèn: lặp khi lệnh kế tiếp `applied_seq + 1` có
   `target_tick < server_tick - INPUT_LATE` → bỏ khỏi `pending`, tăng `applied_seq`, ghi `input_expired`.
   Retire không tạo `PlayerInput` và không phát event; nó chỉ tiến ack. Dừng khi lệnh kế tiếp còn trong cửa
   sổ hoặc `applied_seq + 1` đang thiếu. Bước này chỉ chạy khi batch được chấp nhận; batch bị reject ở
   bước 1–3 không retire, nên client phải tách lệnh để lần gửi sau retire.
6. **Mỗi tick, gap timeout trước**: `gap_since_tick` được đặt khi buffer một lệnh có `seq > applied_seq + 1`
   trong lúc `applied_seq + 1` không có trong `pending` (hoặc khi ack tiến tới một lỗ). Mốc bắt đầu là tick
   đầu tiên thấy thiếu, không phải tick `applied_seq` dừng; resend/duplicate không reset mốc, và mốc được
   xóa khi `applied_seq + 1` đã có trong `pending` hoặc `pending` rỗng. Khi
   `server_tick - gap_since_tick >= INPUT_GAP_TIMEOUT` và `pending` không rỗng:
   - chỉ retire các sequence **thiếu** từ `applied_seq + 1` đến `min(pending.seq) - 1`, ghi `input_gap`;
   - đặt `applied_seq = min(pending.seq) - 1`, rồi xử lý lệnh đầu `pending` theo bước 5/7: nếu đã chín và
     còn cửa sổ thì áp dụng; nếu quá hạn thì retire; nếu còn tương lai thì giữ trong `pending`;
   - **không** đặt `applied_seq` tới một sequence đang có trong `pending` khi chưa áp dụng/retire nó.
   Nếu `pending` rỗng, không có gap timeout; ack giữ nguyên và client resend.
7. **Mỗi tick, áp dụng**: lấy các lệnh liên tiếp từ `applied_seq + 1` có `target_tick <= tick`:
   - `target_tick < tick - INPUT_LATE`: lệnh **hết hạn**, retire như bước 5 (coi như neutral), ack tiến;
   - còn trong cửa sổ: gộp các lệnh đã chín thành **một** `PlayerInput` cho tick hiện tại:
     `dir` = hướng của lệnh mới nhất, `place_bomb` = có ít nhất một cạnh bom trong các lệnh đã chín.
     Nhiều cạnh bom trong cùng tick chỉ tạo tối đa một `place_bomb`; cạnh thừa bị bỏ và đếm.
8. **Hết input**: nếu không có lệnh mới trong `INPUT_STALE`, `dir` trở về neutral và `place_bomb = false`;
   đây là "input mới là neutral" của SCOPE_V1 §3, **không** phải ngắt kết nối. Ngắt quá 10 giây do V1-023
   xử lý, không phải hằng số giao thức này.
9. `Snapshot.acks[player] = { last_seq: applied_seq, applied_tick: tick }` sau khi áp dụng/retire input của
   tick.

**Vì sao luôn tiến được:** mọi lệnh đã giải quyết đều nằm trong `applied_seq` và được ack; lệnh quá hạn ở
đầu hàng được retire ngay khi batch được chấp nhận, không cần chờ vào `pending`; gap timeout chỉ bỏ các
sequence thực sự thiếu; `InputRejected` mang `applied_seq`/`server_tick` để client bỏ lệnh cũ, tách lệnh quá
tương lai và resend y nguyên payload từ lệnh chưa ack nhỏ nhất. Payload bất biến theo `seq` nên resend
không tạo `BadSequence` với lệnh đang `pending`; không có nhánh nào giữ một lệnh quá hạn ở đầu hàng mà
không tiến ack, và không nhánh nào yêu cầu client sửa `target_tick` của lệnh có thể đã buffer.

Replay chỉ ghi `PlayerInput` **sau scheduler** cho từng tick, kể cả neutral/expired. Không ghi raw batch,
`target_tick` hay `ack_snapshot`: đến muộn/sớm/trùng không được làm replay khác đi. Vì vậy một lần chạy lại
chỉ cần tick + `PlayerInput` đã chốt. Lệnh bị retire ở đầu vào không tạo thêm `PlayerInput`; tick đó ghi
neutral nếu không có lệnh khác.

### 3.4 Ví dụ dedup, retire, gap và phục hồi sau reject

Giả sử `INPUT_LATE=6` tick, `INPUT_LEAD=12`, `INPUT_GAP_TIMEOUT=6`, `server_tick=130`.

**(a) Lệnh đầu tiên đã quá hạn.** `applied_seq=0`; client gửi `seq=1,target_tick=120`.
`120 < 130-6=124`, nhưng đây là lệnh đầu hàng: server retire seq 1, đặt `applied_seq=1`, ghi
`input_expired`, không áp dụng input. Snapshot ack `last_seq=1`; client bỏ seq 1 và resend từ seq 2. Batch
không bị từ chối chỉ vì `target_tick` cũ. Nếu batch là `[1:120, 2:131]`, server retire 1 rồi giữ 2; seq 2
áp dụng khi tới tick 131. Đây là đường thoát khỏi trạng thái "ack=0 nhưng mọi batch đều chứa seq 1 quá hạn".

**(b) Duplicate cũ + lệnh mới.** `applied_seq=5`, `pending` rỗng; client chưa nhận ack nên gửi
`[4:110, 5:118, 6:131]` tại tick 130. seq 4/5 `<= applied_seq` được dedup, **không** kiểm `target_tick`;
seq 6 nằm trong `[124,142]` nên vào pending. Batch hợp lệ; ack vẫn 5 tới khi seq 6 chín. Nếu kiểm cửa sổ
trước dedup, batch bị từ chối vì 4/5 quá hạn và lệnh mới 6 không bao giờ vào — đây là lỗi phải tránh.

**(c) Gap timeout không bỏ lệnh pending.** `applied_seq=0`, `pending={2: target_tick=130}`; gap bắt đầu
tick 120. Tại tick 126 (`126-120 >= 6`), server chỉ retire sequence thiếu là 1, đặt `applied_seq=1`, ack=1;
seq 2 vẫn pending. Tại tick 130, seq 2 chín và được áp dụng, ack=2. Không được đặt `applied_seq=2` ngay
sau timeout vì như vậy seq 2 bị bỏ dù chưa áp dụng.

**(d) Reject → phục hồi, không retarget lệnh có thể đã buffer.** `applied_seq=8`; client gửi
`seq=9,target_tick=200` tại tick 130 (`200 > 142`). Server trả
`InputRejected{reason=OutOfWindow, applied_seq=8, first_seq=9, command_count=1, server_tick=130}` và không
chèn lệnh mới nào của batch. `first_seq`/`command_count` xác định đúng batch bị reject.
Nếu seq 9 mới chỉ xuất hiện trong đúng batch này, client biết chắc nó chưa buffer: có thể (i) giữ seq 9 và
resend y nguyên khi server tick tiến đủ để 200 vào cửa sổ, hoặc (ii) bỏ seq 9, dùng seq 10 với
`target_tick` mới; lỗ hổng seq 9 do gap timeout retire. Nếu seq 9 từng nằm trong batch khác (có thể đã
buffer), client **không đổi payload**: resend y nguyên và chờ tới cửa sổ, không gửi kèm lệnh mới làm reject
cả batch. Client coi 8 là ack (bỏ seq <=8), tách lệnh quá tương lai khỏi lệnh quá khứ/đầu hàng, và gửi
`SnapshotRequest` nếu state chưa chắc. Không resend nguyên batch lỗi. Tương tự với
`GapTooLarge`/`BadSequence`/`RateLimited`: ack trong reject là điểm bắt đầu resend, không phải "bỏ toàn bộ
cửa sổ". Riêng `BadSequence` do payload khác bản đã có trong `pending` (lỗi client, không xảy ra với quy tắc
bất biến): server giữ bản `pending` đầu tiên và không chèn lệnh mới; client bỏ bản local của seq đó (bản
server thắng), không gửi payload khác cho cùng seq, resync state bằng snapshot và tiếp tục bằng seq mới.
Snapshot không mang payload `pending`, nên client không cố khôi phục nó; lệnh gốc trong `pending` vẫn được
server áp dụng/retire theo `target_tick` của nó.

**(e) Snapshot ack tiến khi còn lệnh tương lai pending + lệnh mới.** Server tick 130, `applied_seq=5`,
`pending={6: target_tick=132}`. Snapshot tick 130/ack 5 tới client; client bỏ seq <=5, **không** đổi seq 6
(vẫn `target_tick=132`) và resend `[6:132, 7:133]`, trong đó 7 là input mới với seq mới. Server dedup 6 y
nguyên, chèn 7; tick 132 áp dụng 6, tick 133 áp dụng 7, ack tiến. Nếu snapshot có ack=6 (6 đã áp dụng) trong
khi 7 còn pending, client bỏ 6, giữ 7 y nguyên và gửi input mới là seq 8; resend `[7,8]`. Không nhánh nào
đổi `target_tick` của 6/7 sau khi đã gửi; nếu lệnh mới có `target_tick` sớm hơn lệnh pending phía trước,
server vẫn áp dụng theo thứ tự `seq` và có thể dồn lệnh mới sang tick sau, nên client nên gán `target_tick`
không giảm dần theo `seq`.

`SnapshotRequest` chỉ là resync **state** (snapshot trả kèm `acks`); nó không thay thế retire/ack. Liveness
sequence đến từ retire ở bước 5/6 và ack trong `InputRejected`, nên không phải chờ snapshot mới tiến được.

### 3.5 Giá trị V1 (design, chưa phải luật KR)

Các ngưỡng dưới đây là **tham số giao thức** trong `ProtocolLimits`, không nằm trong `Rules` và không phải
claim luật chơi. Tick/round vẫn lấy từ `Rules` (`sim.ticks_per_second`, `round.duration_ms`); ms đổi sang
tick bằng `ms_to_ticks` một lần khi nạp, theo ADR 0004.

| Tham số | V1 | Ở 60 Hz | Lý do |
|---|---:|---:|---|
| `INPUT_REDUNDANCY` | 8 lệnh | ≈133 ms | Phủ RTT 100 ms + jitter 30 ms của SCOPE_V1; đổi sang datagram không đổi contract |
| `INPUT_LATE` | 100 ms | 6 tick | Lệnh tới muộn hơn bị retire (không áp dụng), ack tiến; không tua state |
| `INPUT_LEAD` | 200 ms | 12 tick | Chặn client gửi tick tương lai xa để giữ buffer hữu hạn |
| `INPUT_GAP_TIMEOUT` | 100 ms | 6 tick | Chỉ retire sequence thiếu; lệnh đang pending không bị bỏ, không treo trận |
| `INPUT_STALE` | 150 ms | 9 tick | Giữ hướng tối đa 2–3 RTT một chiều rồi neutral |
| `MAX_INPUT_GAP` | 16 lệnh | — | Trần buffer; vượt là lỗi giao thức |
| `MAX_INPUT_BATCHES_PER_S` | 120 | 2/tick | Client có thể gửi theo frame render; chặn spam |
| `SNAPSHOT_INTERVAL` | 50 ms | 3 tick | 20 Hz; đủ cho nội suy và reconcile ở RTT 100 ms |

Các số này là `design`, chưa có trace server gốc (server gốc thất lạc). V1-027/Q-010 phải đo p95 input
latency và correction; nếu vượt ngân sách thì sửa `ProtocolLimits`/ADR 0006, không sửa lén trong code.
Các ngưỡng không đổi nghĩa khi retire: `INPUT_GAP_TIMEOUT` chỉ áp dụng cho sequence **thiếu**, không bỏ lệnh
đang có trong `pending`; `OutOfWindow` chỉ dành cho `target_tick` quá **tương lai**, còn quá khứ được
retire + ack.

## Quyết định 4 — `Snapshot` đầy đủ, chưa delta

V1 gửi **toàn bộ state** trong mỗi snapshot. Không có baseline/delta trong V1: một snapshot bất kỳ là điểm
resync độc lập, nên không cần chain, không sợ mất một delta, và client mới/desync chỉ cần snapshot mới nhất.
Delta chỉ được thêm sau khi có baseline + resync + test mất gói; việc đó cần ADR mới.

### 4.1 DTO

| Khối | Trường | Ghi chú |
|---|---|---|
| Header | `snapshot_seq u32`, `round_id u32`, `tick u32`, `state_schema u16`, `rules_hash u64`, `content_epoch u32`, `phase`, `rng_state u64`, `next_entity_id u32` | `snapshot_seq` tăng từ 1 trong round; `phase` = Playing / Ended(Draw) / Ended(Winner u8) |
| `acks` | `Vec<{ player u8, last_seq u32, applied_tick u32 }>` | Một mục/người chơi, sắp theo `player`; `last_seq` là sequence đã **giải quyết** (áp dụng hoặc retire), không phải ack TCP |
| `players` | `Vec<{ id u8, x i32, y i32, facing u8, speed Stat, bombs Stat, range Stat, life }>` | `x`/`y` là `Mpx` (1/1000 px), **không** phải chỉ số ô; `Stat = {value u8, max u8}`; `life` = Alive / Trapped{since u32} / Dead |
| `field` | `{ width u16, height u16, kinds_bits Vec<u8>, hidden Vec<{cell u16, kind u8}> }` | `kinds_bits` 2 bit/ô, bit thấp trước (ô 0 ở bit 0–1), hàng rồi cột, 0=Floor, 1=Solid, 2=Breakable, 3=không hợp lệ; `hidden` sắp theo `cell`, chỉ trên ô Breakable |
| `bombs` | `Vec<{ id u32, owner u8, cell u16, range u8, explodes_at u32, grace Vec<u8> }>` | Sắp theo `id`; `grace` sắp tăng |
| `flames` | `Vec<{ cell u16, until u32, source u32, owner u8 }>` | Sắp theo `cell`, mỗi ô tối đa một flame |
| `items` | `Vec<{ id u32, kind u8, cell u16 }>` | Sắp theo `id`, mỗi ô tối đa một item |

Thứ tự field trong bảng là thứ tự encode (postcard struct); đổi thứ tự/kiểu field là tăng `state_schema`.
`cell` = `y * width + x`; dùng `u16` để không phụ thuộc map 15×13. `kind` của item là chỉ số trong
`Rules::items`, không phải ID asset. `rng_state` là state SplitMix64 hiện tại, không phải seed; `next_entity_id`
là giá trị allocator sẽ cấp tiếp theo, không suy từ max ID đã thấy vì entity đã chết/nhặt có thể đã bị xóa.
`x`/`y` giữ nguyên tọa độ `Mpx` của `contracts-r3` để dựng lại đúng `SimulationState` (kể cả vị trí đang di
chuyển); không nén về chỉ số ô. Domain được kiểm theo `width`/`height`/`Rules::tile` ở §4.2, không phải
`0..14`/`0..12`.

Snapshot **không** chứa event. Event đi trong `RoundEvents` cùng stream TCP, có `round_id + EventId`; consumer
dedup theo `(round_id, EventId)` (ADR 0008). Snapshot không phát lại âm thanh/hiệu ứng.

### 4.2 Kích thước Village00, 4 người

Village00 KR là 15×13 = 195 ô (R-002). Bảng dưới là **trần theo miền giá trị V1 được enforce**, dùng postcard
(mục 5); không phải số đo runtime và không phải cận kiểu postcard khi bỏ enforce.

Miền enforce (decoder kiểm trước khi cấp phát; client gọi `check_invariants` trước khi áp dụng):

- 2–4 người; `acks.len() <= 4`, `players.len() <= 4`; `grace.len() <= 4`.
- `width=15`, `height=13`, `cell < 195`; `hidden` chỉ trên ô Breakable.
- Tọa độ người chơi giữ đúng `Mpx` của `contracts-r3`: `0 <= x < width·tile_mpx`,
  `0 <= y < height·tile_mpx`, với `tile_mpx = grid.tile_px·1000` từ `Rules` (V1: 40 px → 40 000).
  V1 enforce `x < 600 000`, `y < 520 000`; tâm ô `index·tile_mpx + tile_mpx/2` (ví dụ spawn (13,11)
  → `x=540 000`, `y=460 000`) nằm trong miền này. Decoder dùng `Rules` của phiên, không hard-code 15/13
  và không đổi sang chỉ số ô.
- `bombs.len() <= 40` (4 × C00 max bombs 10); `flames.len() <= 195`; `items.len() <= 195`;
  `hidden.len() <= 195`.
- Varint: `u8` = 1 B; `u16` cell 0..194 = 2 B; `u32` tối đa 5 B; `u64` tối đa 10 B; `i32` `Mpx`
  trong miền trên là không âm, zigzag max `2·599 999 = 1 199 998` (x) và `2·519 999 = 1 039 998` (y)
  nên 3 B; nếu không enforce miền, cận kiểu `i32` là 5 B. `Stat` = 2 B; `life` max
  `Trapped{since u32}` = 1+5 = 6 B; `phase` lồng nhau tối đa 3 B. `Vec` prefix là `varint(usize)`:
  count 4/8/20/40 = 1 B, count 195 = 2 B.

Giả định "điển hình": 4 người, 8 bom (2/người), 20 ô lửa, 6 item trên nền, 40 item còn giấu. Cột
"điển hình" dùng count của trạng thái đại diện nhưng vẫn lấy cận trên từng trường, nên là **cận trên** của
trạng thái đó, không phải số byte thực của một frame cụ thể.

| Khối | Công thức trần (miền V1) | Điển hình (B) | Trần (B) |
|---|---|---:|---:|
| Envelope | `16` | 16 | 16 |
| Header snapshot | `5+5+5+3+10+5+3+10+5` | 51 | 51 |
| `acks` | `1 + 4·(1+5+5)` | 45 | 45 |
| `players` | `1 + 4·(1+3+3+1+2+2+2+6)` | 81 | 81 |
| `field` | `1+1+(1+49)+(P_H+3·H)` | 173 (H=40, P_H=1) | 639 (H=195, P_H=2) |
| `bombs` | `1 + B·(5+1+2+1+5+1+4)` | 153 (B=8) | 761 (B=40) |
| `flames` | `P_F + F·(2+5+5+1)` | 261 (F=20, P_F=1) | 2 537 (F=195, P_F=2) |
| `items` | `P_I + I·(5+1+2)` | 49 (I=6, P_I=1) | 1 562 (I=195, P_I=2) |
| **Tổng** | | **829** | **5 692** |

`P_H`/`P_F`/`P_I` là varint length prefix của Vec (1 byte khi count < 128, 2 byte khi count = 195).
`field` gồm `width` (1 B), `height` (1 B), `kinds_bits` (`1 + ceil(15·13/4) = 1+49`) và `hidden`
(`P_H + H·(2+1)`). `players` mỗi người: `id` 1, `x` 3, `y` 3, `facing` 1, ba `Stat` 2+2+2, `life` 6.
`bombs` gồm `id u32` 5, `owner` 1, `cell` 2, `range` 1, `explodes_at u32` 5, `grace`
(prefix 1 + tối đa 4 ID = 5). `flames` mỗi ô: `cell` 2, `until` 5, `source` 5, `owner` 1. `items` mỗi mục:
`id` 5, `kind` 1, `cell` 2.

Nếu **không** enforce miền `Mpx` trên, cận kiểu `i32` 5 B cho `x/y` làm `players` thành
`1+4·(1+5+5+1+2+2+2+6) = 97 B`, tổng điển hình/trần là **845/5 708 B**; bảng chính dùng miền enforce 3 B.
Nếu `cell u16` không được kiểm, cận 3 B cũng làm `flames`/`items`/`field` tăng. Decoder phải kiểm
`width`/`height`/`cell`/`x`/`y`/count trước khi cấp phát; nếu map sau này khác 15×13 hoặc `tile_px` khác,
phải tính lại bảng và `MAX_SNAPSHOT_BYTES`, không dùng bảng này cho map khác.

Với 20 Hz: điển hình `829·20 = 16 580 B/s ≈16,2 KiB/s/client`, 4 client `≈64,8 KiB/s`; trần
`5 692·20 = 113 840 B/s ≈111,2 KiB/s/client`, 4 client `≈444,7 KiB/s`. Chưa tính overhead TCP/frame
transport và các message khác. `MAX_SNAPSHOT_BYTES = 16 KiB` còn dư so với trần 5 692 B; nếu map/mode sau
này vượt, phải tăng revision/limit có review, không nới âm thầm.

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
| Kích thước | Varint, rất gần trần lý thuyết; 829 B cận trên điển hình, 5 692 B trần ở trên | Tốt nhưng có tag/field number; repeated packed | Tốt nhất nếu tối ưu bit, nhưng công sức cao |
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
ứng; `Snapshot.acks[0].last_seq` sau đó là 8.

Nếu batch tới khi server tick 130, cả 120/121 đều `< 130-6=124`: đây là lệnh **quá khứ**, không phải
`OutOfWindow`. Server retire seq 7 rồi 8 ở đầu hàng, tăng `applied_seq` lên 8, ghi `input_expired`, không
áp dụng input; snapshot ack `last_seq=8` để client bỏ chúng. Nếu batch là `[7:120, 8:131]` thì retire 7,
giữ 8 và áp dụng 8 khi tới tick 131. `OutOfWindow` chỉ dùng khi `target_tick > server_tick+INPUT_LEAD`
(ví dụ 200 tại tick 130); khi đó `InputRejected` mang `applied_seq`/`server_tick` để client bỏ lệnh cũ,
tách lệnh quá tương lai và resend y nguyên payload (chỉ lệnh chưa gửi mới nhận `target_tick` mới), không
khóa cửa sổ. Ví dụ liveness đầy đủ ở §3.4.

Ví dụ này cũng là golden vector đầu tiên cho `boom-proto`; khi codec được viết, test phải tạo đúng byte này
và decode ngược, trên ba OS.

## Tác động tới prediction/reconciliation (ADR 0005)

1. Client giữ state confirmed = snapshot mới nhất; danh sách lệnh chưa ack (`seq` + `target_tick` + input).
2. Khi có snapshot: kiểm/áp state confirmed; bỏ lệnh có `seq <= acks[player].last_seq`; replay các lệnh
   còn lại từ `snapshot.tick` bằng chính `boom-core` với `PlayerInput` đã chuyển đổi, các người chơi khác giữ
   theo snapshot (không ngoại suy). Đây là correction; hiển thị mượt là việc presentation, không sửa core.
3. Trong lúc chờ, client dự đoán **chỉ di chuyển của mình**; bom chỉ là bóng chờ, nổ/nhốt/chết/item chỉ
   hiện theo `RoundEvents`/snapshot server. Không phát âm thanh/hiệu ứng từ state dự đoán.
4. Client gửi cửa sổ 8 lệnh chưa ack mỗi batch; `target_tick` chỉ gán khi lệnh lần đầu vào batch, theo
   `snapshot.tick + số tick local kể từ snapshot + 1` (xấp xỉ; server clamp theo cửa sổ). Sau khi gửi,
   payload bất biến: khi nhận snapshot/reject, client bỏ mọi lệnh `seq <= applied_seq`, resend y nguyên các
   lệnh còn lại (kể cả đã quá `INPUT_LATE`; server sẽ retire + ack), và chỉ cấp `seq` mới cho input mới.
   Không đổi `target_tick` của lệnh có thể đã vào `pending`; nếu chắc chắn lệnh chưa buffer (chỉ từng ở
   batch bị reject) thì được bỏ nó và dùng seq mới, lỗ hổng do gap timeout retire.
5. Event dedup theo `(round_id, EventId)`; snapshot không phát lại event. Replay dự đoán không ghi economy/log
   (ADR 0008).
6. Nếu p95 correction/input latency vượt ngân sách V1-027, xem lại `INPUT_*`/`SNAPSHOT_INTERVAL`; nếu vẫn
   vượt, đảo transport theo ADR 0006 hoặc mở ADR mới. Không đổi ngầm hằng số trong code.

## Hệ quả

- `boom-proto` (mở sau khi ADR accepted) giữ DTO/envelope/codec + golden vector; `boom-net` giữ frame/session/
  rate limit; `boom-server` giữ scheduler/round; `boom-client` giữ prediction/reconcile.
- Test bắt buộc: round-trip mọi message/kind; reject magic/schema/len/flags/reserved; duplicate/stale/out-of-window
  (chỉ quá tương lai)/gap/rate-limit; batch duplicate cũ + lệnh mới; late retire + ack; gap timeout chỉ retire
  sequence thiếu; reject → resync; snapshot full dựng lại đúng `SimulationState` và qua `check_invariants`;
  kích thước dưới trần; cùng byte trên Windows/Linux/macOS.
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
