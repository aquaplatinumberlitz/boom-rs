# QA mạng V1-027 — hồ sơ mạng và cách đo

Tài liệu này mô tả **hồ sơ mạng** dùng cho harness QA headless (`crates/boom-qa`) và cách
harness đo độ trễ, kiểm tra hội tụ state. Không phải luật chơi: mọi giá trị ở đây là **tham số
fixture kiểm thử**, `source = "design"` trong profile rules của harness, không sinh claim `R-xxx`
nào trong [RULES_V1](gameplay/RULES_V1.md).

## Phạm vi

| Mục | Nội dung |
|---|---|
| Server | `boom-server` thật, identity SQLite trong thư mục tạm (ADR 0011 §10) |
| Client | Bot headless nói DTO trực tiếp (`boom-proto`), không dự đoán state |
| Kết nối | `boom-net::MemoryTransport`, hai chiều, lỗi mạng theo seed |
| Luật trận | Profile rules riêng của harness (`crates/boom-qa/rules/*.toml`) |

Harness **không** dùng client thật (`boom-client`): giai đoạn 1 bot nói DTO trực tiếp. Điểm thay
thế là trait `boom_qa::bot::BotClient`, để khi module net của `boom-client` sẵn sàng thì thêm một
implementation mà không phải đổi harness.

### Thay implementation bot

Harness **không** tự dựng bot: nó gọi `boom_qa::bot::BotFactory` cho từng bot của lần chạy.

| Entry point | Bot dùng | Ai dùng |
|---|---|---|
| `boom_qa::harness::run(&RunConfig)` | `DirectDtoBotFactory` (bot nói DTO trực tiếp) | CLI `boom-qa`, test |
| `boom_qa::harness::run_with(&RunConfig, &dyn BotFactory)` | factory do bên ngoài cài | implementation bot khác (ví dụ bot dự đoán state) |

`BotFactory::create(&BotConfig) -> Box<dyn BotClient>` nhận cấu hình harness đã tính sẵn cho bot
đó (tên, tài khoản, `PlayerId`, vai trò host, thế giới, profile rules, nhịp input/ping…), nên
implementation bên ngoài chỉ cần implement `BotClient` (`on_frame`, `on_tick`, `stats`, `phase`,
`reset`) là chạy được trong harness. Test `harness_runs_an_external_bot_implementation` trong
`crates/boom-qa/tests/harness.rs` truyền một implementation khác từ integration test và kiểm
lượt vẫn hội tụ hash.

## Hồ sơ mạng

Hồ sơ nằm trong `crates/boom-qa/profiles/*.toml`:

| Thông số | `scope-v1` | `fast` | Nguồn |
|---|---|---|---|
| RTT mục tiêu | 100 ms | 20 ms | SCOPE_V1 §7.6 |
| Trễ một chiều (RTT/2) | 50 ms | 10 ms | Do `boom-net` mô phỏng trễ một chiều |
| Jitter | 0–30 ms | 0–5 ms | SCOPE_V1 §7.6 |
| Mất frame | 10‰ (1 %) | 10‰ (1 %) | SCOPE_V1 §7.6 |
| Trùng frame | 10‰ (1 %) | 10‰ (1 %) | SCOPE_V1 §7.6 yêu cầu "duplicate/reorder theo fixture" |
| Đảo thứ tự | 10‰ (1 %) | 10‰ (1 %) | SCOPE_V1 §7.6 |
| Cửa sổ đảo thứ tự | 30 ms | 10 ms | Lựa chọn của hồ sơ (design) |
| `Ping` server (giữ phiên) | 5 s | 5 s | Tham số vận hành của harness (design) |
| Hạn liveness server | 60 s | 60 s | Tham số vận hành của harness (design) |

`scope-v1` là hồ sơ dùng để nghiệm thu V1-027 (2 và 4 bot, 2 lượt, 3 seed). `fast` là hồ sơ
chạy nhanh cho test CI.

Tỉ lệ trùng/đảo thứ tự **không** do SCOPE_V1 chốt cụ thể ("duplicate/reorder theo fixture"),
nên hồ sơ tự đặt 1 % và ghi rõ ở đây là lựa chọn của hồ sơ, không phải giá trị đã xác nhận.

### Vì sao trùng/đảo thứ tự vẫn hội tụ

`boom-net::MemoryTransport` mô phỏng lỗi ở **tầng frame**, nên mọi frame server→bot đều có thể
bị mất, trùng hoặc tới trễ. Bot vì vậy phải idempotent:

- `Auth` chỉ gửi **một lần mỗi phiên**. Server từ chối `Auth` lần hai sau `AuthOk`
  (ADR 0009 Addendum A §A.1) và đóng phiên, nên `Welcome` trùng do mạng nhân bản frame cũng gây
  ra điều đó. Chỉ khi server **trả lời** `AuthFail` thì bot mới thử lại (`Register` → `Login` khi
  tên đã có).
- Mỗi lượt chỉ kết thúc **một lần**: `RoundResult` và snapshot `Phase::Ended` đều có thể tới
  (trùng hoặc đảo thứ tự), nên bot gộp chúng qua tập lượt đã xử lý.
- Lệnh phòng (`Create`/`Join`) **được gửi lại** sau `retry_ms` nếu chưa có phản hồi, vì lệnh có
  thể bị mất. Lệnh lặp là idempotent ở phía server: `PlayerAlreadyInRoom` được hiểu là "đã ở
  trong phòng" chứ không phải lỗi, và bot **dừng** gửi `Join` để không lặp vô hạn — bot chờ
  `RoomState` để biết mình ở phòng nào. Nếu `RoomState` không tới trong
  `room_state_deadline_ms` (server chỉ đẩy `RoomState` khi phòng đổi), bot rời phòng đó rồi
  vào lại.
- **View phòng phải được làm tươi**: `RoomState` là frame **đẩy** (push), nên một bản bị mất sẽ
  khiến bot giữ view cũ mãi mãi (ví dụ host không thấy "phòng đủ ready" nên không start). Bot
  vì vậy gửi lại `SetReady{true}` mỗi `retry_ms`: lệnh này idempotent và mọi lệnh phòng thành
  công đều khiến server đẩy lại `RoomState` cho mọi thành viên ⇒ view luôn còn tươi.
- Host chỉ gửi `Start` khi **đã thấy** phòng đủ số bot của lần chạy và tất cả ready: start sớm
  sẽ bỏ sót bot chưa vào phòng, bot đó không có state hash của lượt ⇒ báo cáo báo không hội tụ.
- `InputBatch.player` là **slot trong phòng** mà server cấp, đọc từ `RoomState`/`RoundStart`
  roster, không phải slot session. Server bind người chơi theo session rồi so `batch.player` với
  slot trong roster của lượt; gửi sai slot thì mọi batch bị từ chối.

### Giữ phiên sống (liveness)

Server đóng phiên khi **không nhận được `Pong`** trong `liveness_timeout_ms`, và nó chỉ làm
mới hạn đó khi nhận `Pong` — tức là khi trả lời `Ping` của server, không phải khi client gửi
`Ping` riêng. Mặc định server là `Ping` 15 s / liveness 30 s: **mất đúng một `Pong`** là đủ để
phiên chết, trong khi hồ sơ mạng cố ý mất 1 % frame.

Harness vì vậy đặt lại hai tham số này theo hồ sơ (`server_ping_interval_ms`,
`server_liveness_timeout_ms`): `Ping` 5 s, liveness 60 s ⇒ phải mất liên tiếp nhiều `Pong` mới
giết phiên.

### Mở lại phiên

Khi server đóng phiên (mất mạng, hết hạn liveness…), harness mở lại phiên cho bot: đóng nửa
ghi cũ (server thấy EOF và thả session cũ), **dẹp hết frame tồn đọng của server**, rồi mới
đăng ký session mới. Bước dẹp là bắt buộc: `Inbound::Closed` mang theo `PlayerId`, nếu tới sau
khi session mới đăng ký thì nó đóng nhầm session mới (phiên mới chết ngay sau khi vừa mở).

## Đo độ trễ

Harness đo **độ trễ input**: khoảng thời gian từ lúc bot gửi lệnh (sequence `seq`) tới lúc
server **giải quyết** sequence đó (áp dụng hoặc retire).

- Bot ghi mốc gửi theo `(lượt, sequence)`; sequence đánh số lại theo từng lượt (ADR 0009 §3.1),
  nên khoá phải theo lượt.
- Harness đọc `applied_seq` của từng bot mỗi vòng poll; sequence vừa được giải quyết là mốc
  "server đã xử lý".
- Lệnh bị **retire** (quá hạn cửa sổ) không tính vào độ trễ: server đã loại nó, không áp dụng.
- Báo cáo có `p50`/`p95`/`max`/`mean` của tập mẫu độ trễ, cùng `p50`/`p95` RTT đo bằng
  `Ping`/`Pong`.

### RTT ghép theo nonce của `Ping`

`Ping` mang một nonce và server trả lại `Pong` **cùng nonce** (ADR 0009 §2). Bot giữ bảng
`nonce → mốc gửi` cho mọi ping **đang chờ** và chỉ ghi mẫu RTT khi `Pong` khớp đúng một ping
trong bảng:

- `Pong` của ping cũ tới **sau** `Pong` của ping mới (đảo thứ tự) không được ghi mẫu của
  ping mới, và cũng không được xoá ping đang chờ — nếu không, `Pong` thật của ping mới sẽ bị
  bỏ mất.
- `Pong` trùng (mạng nhân bản frame) của một ping đã đo không ghi thêm mẫu.
- `Pong` mất không làm hỏng ping sau: mỗi ping giữ mốc riêng, nên `Pong` tới muộn vẫn ghi
  được mẫu của chính nó; ping quá hạn (không có `Pong` sau `10` chu kỳ ping) thì bị bỏ khỏi
  bảng chờ.
- Nonce tăng đơn điệu và **không dùng lại** kể cả sau khi mở lại phiên, nên `Pong` của phiên
  cũ không khớp nhầm ping của phiên mới.

Vì `Timing::Real`, transport **ngủ đúng số ms ảo** nên độ trễ đo được là độ trễ thật, không
phải số ảo.

## Kiểm tra hội tụ state

Mỗi lượt kết thúc, server gửi `RoundResult`; bot **xin snapshot** (`SnapshotRequest`) để dựng
lại `SimulationState` đầy đủ (`boom-sync::from_snapshot`) và tự tính `state.hash()`. Hash bot dựng
lại phải bằng hash server lúc lượt kết thúc.

Vì server **xoá lượt** khi mọi người ack, bot chỉ gửi `ResultAck` **sau khi đã có** state cuối
lượt. Nếu hết hạn mà chưa có snapshot, bot vẫn ack (để lượt kết thúc) và ghi nhận
`snapshot_missed` — đó là lỗi thật của mạng, không phải lỗi hội tụ.

Báo cáo `converged` của một lượt = mọi bot đều có hash và hash đó bằng hash server.

## Chạy

```sh
# Hồ sơ nghiệm thu: 4 bot, 2 lượt, seed 42
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42

# Ghi JSON ra file
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42 --json-out qa.json
```

Lệnh in ra **bảng markdown** rồi **JSON** của cùng một báo cáo. Mã thoát khác 0 khi hết ngân
sách thời gian, hoặc có lượt không hội tụ.

Chẩn đoán chi tiết: đặt `BOOM_QA_TRACE=1` để in sự kiện server và từng frame bot nhận/gửi.

## Nghiệm thu V1-027 phần 1

Lệnh nghiệm thu (2 và 4 bot, 2 lượt, 3 seed):

```sh
for c in 2 4; do for s in 1 2 3; do
  cargo run -p boom-qa -- --profile scope-v1 --clients "$c" --seed "$s"
done; done
```

Mỗi lần chạy phải: không panic, mã thoát 0, mọi lượt có `converged: true` với **đủ số bot**
(2 hoặc 4 bot đều có hash trong bảng "Hội tụ state hash"), và báo cáo có `p95` độ trễ input.

Test nhanh chạy trong `cargo xtask ci` (`crates/boom-qa/tests/harness.rs`): hồ sơ `fast`
(lượt ngắn, mạng vẫn có mất/trùng/đảo thứ tự), 2 bot và 4 bot, 1 lượt — kiểm mọi bot đều vào
phòng và dựng lại được hash của lượt.

## Giới hạn

- Harness đo **bản Rust hiện tại**, không kết luận gì về độ trùng khớp với client/server gốc
  (SCOPE_V1 §7.8).
- Giai đoạn 1 bot **không dự đoán state**, nên `corrections` luôn bằng 0 và không đo được biên độ
  sửa. Muốn đo correction phải dùng implementation bot dùng module net của `boom-client`
  (task tiếp theo của V1-027). `boom-client` hiện là crate **chỉ có binary** (không có `lib.rs`),
  nên task đó cần tách phần `net` ra target lib (hoặc chuyển sang crate dùng chung) — việc này
  ngoài phạm vi file của issue này.
- Bản đồ và profile rules của harness là **fixture kiểm thử**, không phải map `FxMap/Map/Village00`
  của V1 (SCOPE_V1 §1) và không phải profile phát hành.
- Harness **chưa** kịch bản "mất kết nối giữa trận" (SCOPE_V1 §7.7): theo policy V1, người chơi
  mất kết nối giữa vòng không được nhập lại, nên bot mất kết nối giữa lượt sẽ không có state hash
  của lượt đó ⇒ lượt bị báo không hội tụ và lần chạy hết ngân sách. Kịch bản này thuộc task
  khác của V1-023.
