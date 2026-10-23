# QA mạng V1-027 — hồ sơ mạng và cách đo

Tài liệu này mô tả **hồ sơ mạng** dùng cho harness QA headless (`crates/boom-qa`) và cách
harness đo độ trễ, kiểm tra hội tụ state. Không phải luật chơi: mọi giá trị ở đây là **tham số
fixture kiểm thử**, `source = "design"` trong profile rules của harness, không sinh claim `R-xxx`
nào trong [RULES_V1](gameplay/RULES_V1.md).

## Phạm vi

| Mục | Nội dung |
|---|---|
| Server | `boom-server` thật, identity SQLite trong thư mục tạm (ADR 0011 §10) |
| Client | Bot headless dùng `Predictor` của client cho input/prediction/reconciliation; lifecycle qua DTO |
| Kết nối | `boom-net::MemoryTransport`, hai chiều, lỗi mạng theo seed |
| Luật trận | Profile rules riêng của harness (`crates/boom-qa/rules/*.toml`) |

Harness dùng trực tiếp source chuẩn `boom-client/src/net/prediction.rs` qua `#[path]` trong
`boom-qa/src/lib.rs`: cùng API `predict/predict_at/apply_snapshot/window/window_for_send/acknowledge`,
không chép thuật toán. `RoomFlow` cũng
include source client chung để kiểm fingerprint/roster/round cũ trước side effect.
`boom-client` hiện chỉ có binary và #166 không mở quyền sửa Cargo/lib client; cách include này
không kéo renderer/GPU vào QA. Tên public `DirectDtoBot`/factory giữ tương thích; phần lifecycle
vẫn nói DTO, còn input dùng predictor thật và resend cửa sổ lệnh chưa ack với payload bất biến.
Chưa dùng trọn `NetClient`/`ClientSession` cho auth/phòng: không coi phép đo này là nghiệm thu
retry/timeout của facade đó.

### Thay implementation bot

Harness **không** tự dựng bot: nó gọi `boom_qa::bot::BotFactory` cho từng bot của lần chạy.

| Entry point | Bot dùng | Ai dùng |
|---|---|---|
| `boom_qa::harness::run(&RunConfig)` | `DirectDtoBotFactory` (predictor chung + lifecycle DTO) | CLI `boom-qa`, test |
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

### Định danh người chơi: `account_id`, không phải `PlayerId` của kết nối

Mọi trường `player` trên dây (`RoomState.host`, `RoomState.members[].player`,
`RoundStart.roster[].player`) mang **account_id** do server cấp trong `AuthOk`, **không** phải
`PlayerId` của kết nối (ADR 0009 Addendum B §B.2). Bot vì vậy:

- học `account_id` của mình từ `AuthOk` và dùng nó để biết mình có phải host
  (`RoomState.host == account_id` của bot) và để lấy **slot** của mình trong roster
  (`members[].player == account_id` ⇒ lấy `members[].slot`);
- `account_id` **không** trùng với `PlayerId` mà harness gán cho kết nối: server cấp
  `account_id` theo thứ tự đăng ký tài khoản, nên hai bot có thể đổi thứ tự. So sánh nhầm hai
  định danh này khiến bot tưởng mình là host ⇒ gửi `Start` liên tục và bị từ chối `NotHost`
  ⇒ phòng treo, lượt không bắt đầu.

Harness cũng tra `applied_seq` của server (khoá theo `account_id`) bằng `account_id` mà bot
học được, nếu không thì không đo được độ trễ input nào.

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

`latency_ms` là **thời gian lưu hành lệnh trong phép đo QA**, không phải RTT mạng: từ lúc bot
**tạo** sequence (`sent_at_ms`) tới lần poll của harness thấy `applied_seq` của server đã đạt hoặc
vượt sequence đó (`applied_seq >= seq`; server áp dụng **hoặc** retire).

- Bot lấy `sent_at_ms = now_ms` của lần `send_input` (`crates/boom-qa/src/bot.rs:1103-1157`); sau
  `Predictor::predict` và `Predictor::window` (`crates/boom-client/src/net/prediction.rs:450-465`)
  cắt cửa sổ tối đa 8 lệnh (`ProtocolLimits::max_commands`), bot mới ghi `stats.sent` (dòng
  1148-1155). Mốc là `now_ms` của cùng lần gọi, **không phải mốc "first-wire"**: nếu hàng chờ lớn
  hơn 8 thì sequence mới có thể chưa nằm trong batch vừa mã hoá và chưa lên dây ở mốc đó. Phép đo
  hiện tại không chứng minh được seq đã vào batch hay đã lên dây.
- `input.sent` đếm **batch** (`commands_sent` tăng một lần mỗi `send_input`), còn
  `input.resolved` đếm **sequence**. Hai cột khác đơn vị, không trừ trực tiếp cho nhau.
- Harness đọc `applied_seq` mỗi vòng poll (`crates/boom-qa/src/harness.rs:699-733`).
  `applied_seq` tiến khi `InputTrack::apply_tick` áp dụng/retire (`crates/boom-server/src/input.rs:281-345`)
  **hoặc** khi `InputTrack::submit` retire đầu hàng (`input.rs:271-277`).
- Chỉ retire phát `InternalServerEvent::InputRetired` từ `apply_tick` mới được harness loại khỏi
  mẫu. Retire ngay trong `submit` (trường `InputAck.retired`) bị `Server::handle_frame` bỏ qua
  bằng `Ok(_)` (`crates/boom-server/src/server.rs:786-803`), không có event/log, nên **vẫn được
  tính** là mẫu độ trễ và không xuất hiện trong `input.retired`. Vì vậy `retired = 0` chỉ nói
  "không có retire ở `apply_tick`", không chứng minh "không có lệnh nào bị retire".
- `p50`/`p95`/`max`/`mean` là phân vị của tập mẫu đó. RTT `Ping`/`Pong` (`rtt_ms`) là phép đo
  riêng, ghép theo nonce, và là số dùng khi cần nói về độ trễ mạng.

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

### Correction cùng tick (schema báo cáo 2)

Bot lưu vị trí dự đoán của chính mình theo tick. Khi nhận snapshot mới được Predictor chấp nhận,
so tọa độ server với vị trí đã dự đoán **ở cùng tick**, dùng khoảng cách Manhattan `|dx| + |dy|`
đơn vị mpx. Bao gồm mẫu 0; snapshot trùng/cũ và tick chưa từng dự đoán không tạo mẫu. Vì vậy
`corrections.count` đếm mẫu khác 0; `amplitude_mpx` có count/min/mean/p50/p95/max; không có mẫu
thì phân vị là null. Đây là sai số dự đoán lịch sử trước reconciliation, không phải khoảng nhảy
hình giữa hai frame và không đo bằng ms.

### Drain và recovery

Sau khi mọi bot đã có hash/ack, harness tiếp tục bơm cả bốn chiều nhận của client và server,
chặn lệnh phòng mới để không tự mở lượt tiếp. Khoảng drain tối thiểu bằng hai lần
`one_way_delay + jitter + reorder_window`, cộng bốn bước poll. Chỉ thành công khi server về
Waiting; timeout không được báo drained.

`--scenario disconnect` đóng kết nối client cuối sau tick 10 của Playing; `liveness` bỏ riêng
Pong của client đó tới khi server đóng phiên. Harness mở phiên mới, auth lại cùng tài khoản,
vào phòng ở lượt sau theo policy V1 (không nhập lại giữa trận). Lượt bị ngắt vẫn ghi thiếu hash,
không che lỗi bằng hash lấy từ server. `recovered` chỉ true khi lượt sau có đủ bốn hash khớp và
đã drain; `converged` vẫn giữ nghĩa mọi lượt đều hội tụ, nên có thể false ở kịch bản recovery.

## Chạy

```sh
# Hồ sơ nghiệm thu: 4 bot, 2 lượt, seed 42
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42

# Ghi JSON ra file
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42 --json-out qa.json
```

Lệnh in ra **bảng markdown** rồi **JSON** của cùng một báo cáo. Mã thoát khác 0 khi hết ngân
sách thời gian, chưa drain, hoặc không hội tụ (với recovery: không phục hồi ở lượt sau).

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

### Kết quả ma trận 2026-10-23 (opencode-4)

Đo ngày 2026-10-23 (UTC) trên Linux 7.2.9-2-cachyos, `rustc`/`cargo` 1.99.0. Build/commit:
`14318549297226fbe416aad452c87f67c0792a36` (`origin/main`, cây làm việc sạch trước khi chạy).
Binary debug build bằng `cargo build -p boom-qa`; mỗi tổ hợp chạy tuần tự bằng:

```sh
cargo run -p boom-qa -- --profile scope-v1 --clients <2|4> --seed <1|2|3> \
  --json-out local/qa-network-matrix/<clients>-<seed>.json
```

`--rounds` để mặc định theo hồ sơ (`rounds = 2`). Stdout+stderr của từng run lưu vào
`local/qa-network-matrix/<clients>-<seed>.log`, exit code vào file `.exit` cùng thư mục
(`local/` nằm trong `.gitignore`). Sáu tổ hợp đều exit 0; không run nào `timed_out`,
`drained = false` hoặc thiếu hash. `recovered = false` ở cả sáu run vì scenario là `Steady`
(`fault_round = null`), không phải lỗi phục hồi.

| Clients | Seed | Exit | Thời gian (ms) | p95 input (ms) [mẫu] | p95 RTT (ms) [mẫu] | Batch input bị từ chối | Hội tụ | Drained | Timed out |
|---:|---:|---:|---:|---:|---:|---:|---|---|---|
| 2 | 1 | 0 | 42523 | 1581 [3624] | 199 [41] | 28 | có | có | không |
| 2 | 2 | 0 | 41913 | 1672 [3627] | 215 [39] | 28 | có | có | không |
| 2 | 3 | 0 | 42223 | 1565 [3657] | 197 [40] | 31 | có | có | không |
| 4 | 1 | 0 | 42634 | 1676 [7243] | 192 [80] | 62 | có | có | không |
| 4 | 2 | 0 | 42726 | 1587 [7224] | 198 [83] | 62 | có | có | không |
| 4 | 3 | 0 | 42264 | 1595 [7256] | 185 [83] | 62 | có | có | không |

Mọi phân vị đều có mẫu (`count > 0`), không có `null`; `snapshot_missed = 0` và
`input.retired = 0` ở cả sáu run. Cột "Batch input bị từ chối" là số `input.rejected` do
schema báo cáo; harness chỉ coi `timed_out`, chưa `drained` hoặc không hội tụ là lỗi, nên các
giá trị này không làm exit code khác 0 và không được dùng để suy nguyên nhân ở đây.

| Clients | Seed | Lượt | Bot | Hash server | Hash bot | Hội tụ |
|---:|---:|---:|---:|---|---|---|
| 2 | 1 | 1 | 2 | `14226134489770241462` | `bot0` = `14226134489770241462`<br>`bot1` = `14226134489770241462` | có |
| 2 | 1 | 2 | 2 | `15829727555954839341` | `bot0` = `15829727555954839341`<br>`bot1` = `15829727555954839341` | có |
| 2 | 2 | 1 | 2 | `3357971480158021122` | `bot0` = `3357971480158021122`<br>`bot1` = `3357971480158021122` | có |
| 2 | 2 | 2 | 2 | `8429043139095620009` | `bot0` = `8429043139095620009`<br>`bot1` = `8429043139095620009` | có |
| 2 | 3 | 1 | 2 | `3357971480158021122` | `bot0` = `3357971480158021122`<br>`bot1` = `3357971480158021122` | có |
| 2 | 3 | 2 | 2 | `8551430651842991812` | `bot0` = `8551430651842991812`<br>`bot1` = `8551430651842991812` | có |
| 4 | 1 | 1 | 4 | `14832401785224703741` | `bot0` = `14832401785224703741`<br>`bot1` = `14832401785224703741`<br>`bot2` = `14832401785224703741`<br>`bot3` = `14832401785224703741` | có |
| 4 | 1 | 2 | 4 | `2300611997934512629` | `bot0` = `2300611997934512629`<br>`bot1` = `2300611997934512629`<br>`bot2` = `2300611997934512629`<br>`bot3` = `2300611997934512629` | có |
| 4 | 2 | 1 | 4 | `13925574154528887970` | `bot0` = `13925574154528887970`<br>`bot1` = `13925574154528887970`<br>`bot2` = `13925574154528887970`<br>`bot3` = `13925574154528887970` | có |
| 4 | 2 | 2 | 4 | `2300611997934512629` | `bot0` = `2300611997934512629`<br>`bot1` = `2300611997934512629`<br>`bot2` = `2300611997934512629`<br>`bot3` = `2300611997934512629` | có |
| 4 | 3 | 1 | 4 | `10931218994403125820` | `bot0` = `10931218994403125820`<br>`bot1` = `10931218994403125820`<br>`bot2` = `10931218994403125820`<br>`bot3` = `10931218994403125820` | có |
| 4 | 3 | 2 | 4 | `833496084450221151` | `bot0` = `833496084450221151`<br>`bot1` = `833496084450221151`<br>`bot2` = `833496084450221151`<br>`bot3` = `833496084450221151` | có |

Nguyên liệu thô để tái tạo (không commit, `local/` ignored):

| Run | JSON sha256 | Log sha256 |
|---|---|---|
| `2-1` | `d0030f57e4d15596d4463afaa8c4387f5c51fb8a3d9844b9866138dd76a3539e` | `ca705225f4356c43e8b0791e85d8bf3010ec6d0e25f1281a2c82029ae1e524f9` |
| `2-2` | `96d1457a1790f012cb3a19632a35b95ac2919272f3691401e1b14337e8004fc6` | `098fb0f3a6e6046cbbe476c8b6bc70f4d3f659142a7551bf0087c2dbd24b3127` |
| `2-3` | `5ade7c283a088a60b5abc658f718b9f60806e1f645243418419e5746c46e5ed2` | `bb155f70788a759811f9472a769c0efea8f808d48a79cd019b05f407b19049f2` |
| `4-1` | `c735bb8a41a600d0046005326a82fc833accf7ea8e8fcfb014aa189887c7588a` | `86a055979acf7ee566e8c30d7277d261f7f11d129ec8dd0d2e1dccf624ebb4d0` |
| `4-2` | `eafd0de2f0cb1f6e0062f357a7941c9922b2d6d56eb926ec93a21f9529238ae0` | `40d2544b5ceac35fb610e9518986e0f57415b148301e8b58042dc3bbbf2826c2` |
| `4-3` | `6c651a3c0362fe527dade6659d845d926fdddb777ed3bed58a13e50813e9ef9d` | `a314789268a2a9105480d4f9b8c3359cd293a9c7302f373db85b5d21f49196c2` |

Kết quả trên chỉ là quan sát của bản Rust tại commit nêu trên với fixture `qa-village`/`qa-net`;
không đánh giá gate G4 và không kết luận map/rules hay fidelity client KR.

### Chẩn đoán ma trận 2026-10-23 (#215)

Baseline phân tích: `743e8ea9cdf0b975eb55d814f84bfe75ad746ffc` (main). Số đo gốc của PR #212 nằm ở
`/home/admin/boom-rs-wt/issue-210/local/qa-network-matrix/` (readonly; `local/` không commit): sáu
cặp `<clients>-<seed>.json/.log/.exit`, sha256 ở bảng trên. Code của sáu run ở `1431854…`;
`git diff 1431854..743e8ea` chỉ đổi docs/tools, không đổi `crates/`, nên chẩn đoán dưới đây áp dụng
cho baseline này.

Để lấy lý do `InputRejected` (sáu log cũ tắt `BOOM_QA_TRACE`), chạy **một** lần bổ sung cùng hồ sơ,
2 bot, seed 1, có trace:

```sh
BOOM_QA_TRACE=1 cargo run -p boom-qa -- --profile scope-v1 --clients 2 --seed 1 \
  --json-out local/issue-215-trace/2-1.json 2> local/issue-215-trace/2-1.trace
```

Kết quả: exit 0, 42.439 ms, `converged=true`, `drained=true`, `snapshot_missed=0`,
`p50/p95 input = 847/1699 ms` (3.600 mẫu), `rejected=31`, `retired=0`, RTT p50/p95 = 154/206 ms.
Vật liệu local (không commit): `local/issue-215-trace/2-1.json` sha256 `96c60db26adc…`,
`2-1.trace` sha256 `f3617e420d4c…`. Lần chạy này **không** thay ma trận sáu tổ hợp và không dùng để
tuyên bố pass.

#### `InputRejected` được phân loại gì

- Sáu log cũ **không** có lý do: chỉ có bảng/JSON, `input.rejected` 28/28/31/62/62/62 là **số batch**,
  không kèm reason. Không thể phân loại từ sáu file này.
- Run trace 2/1: 31/31 batch mang reason DTO `BadSequence` (bot0 15, bot1 16).
- Cả 31 rơi thành hai cụm ngay sau `RoundFinished`: dòng 2529 (`t=21497`) rồi 2539–2593, và dòng
  5133 (`t=42120`) rồi 5144–5200. Không có batch nào bị từ chối giữa lượt. Cụm thứ nhất kết thúc
  trước `RoundStart round=2` (dòng 2625).
- `BadSequence` trên dây **không** phải chẩn đoán cuối: `Server::send_input_rejected`
  (`crates/boom-server/src/server.rs:1471-1476`) chỉ ánh xạ `OutOfWindow`, `GapTooLarge`,
  `RateLimited`; mọi `InputError` khác — kể cả `NotPlaying`, `WrongRound`,
  `NonContiguousSequence`, hay `BadSequence` do payload khác — đều thành `BadSequence`.
  `Authority::submit_input` trả `NotPlaying` khi pha không phải `Playing`
  (`crates/boom-server/src/authority.rs:733-735`), và `RoundFinished` chuyển pha sang `Results`
  (`authority.rs:822-833`). Thứ tự trace vì vậy khớp với giả thuyết **input tới sau khi lượt đã kết
  thúc (`NotPlaying`)**, không phải trùng sequence/payload giữa lượt.
- Chưa chứng minh: server không log biến thể `InputError` thật, cũng không log pha lúc reject. Vì vậy
  **reason thật = unknown**; chỉ kết luận được lớp DTO là `BadSequence` và vị trí là quanh chuyển
  lượt. Chênh lệch 28 so với 31 là khác biệt thời điểm, không phải hằng số.
- Phép kiểm tiếp theo: log `InputError` (discriminant) + `phase`/`round_id` tại
  `send_input_rejected`, hoặc thêm reason DTO riêng cho `NotPlaying` (đổi protocol → task riêng);
  chạy lại đúng 2/1 và 4/1 để so. Không sửa predictor/server trong task này.

#### "Độ trễ input" p95 ~1,6 s là mốc nào

Chuỗi đo thực tế:

1. `send_input` lấy `sent_at_ms = now_ms`, rồi `Predictor::predict` tạo seq và `Predictor::window`
   cắt tối đa 8 lệnh; sau đó bot mới ghi `stats.sent` (`bot.rs:1103-1157`, dòng ghi 1148-1155;
   `prediction.rs:450-465`). Mốc là thời điểm vào `send_input`, **không phải first-wire**; seq mới
   **có thể chưa nằm trong batch** nếu hàng chờ lớn hơn 8. Phép đo chưa có mốc "lần đầu lên dây"
   hay kích thước hàng chờ.
2. `target_tick` được gán lúc **tạo** lệnh bằng tick của state dự đoán (`prediction.rs:411-443`),
   không phải lúc lệnh lần đầu vào batch. State dự đoán đã replay các lệnh chưa ack
   (`prediction.rs:507-557`), nên tick này nằm trước tick server một khoảng xấp xỉ hàng lệnh đang
   chờ. ADR 0009 §3.2 mô tả `target_tick` gán khi lệnh lần đầu vào batch; đây là khác biệt code với
   mô tả đó, cần task riêng để quyết định, không sửa trong task này.
3. Harness chỉ đọc `applied_seq` mỗi vòng poll (`harness.rs:699-733`); `applied_seq` tiến khi server
   áp dụng/retire tại `apply_tick` (`input.rs:281-345`) hoặc retire tại `submit`
   (`input.rs:271-277`, `authority.rs:771-777`). Mẫu bao trùm khoảng từ lúc tạo lệnh tới lúc giải
   quyết; khoảng đó **có thể** gồm thời gian chờ trong hàng đợi client/cửa sổ lệnh, một chiều mạng
   và thời gian chờ tới `target_tick`/retire. Nó không phải "gửi→nhận", không phải "áp dụng xong",
   và không phải RTT.
4. `retired` chỉ đếm retire ở `apply_tick`. Retire trong `submit` bị bỏ (`server.rs:795`), nên một
   phần mẫu có thể là lệnh **không hề được áp dụng**; tỉ lệ này chưa đo được.

Cơ chế trên cho thấy `latency_ms` **có thể** gồm thời gian chờ trong hàng đợi client/cửa sổ lệnh và
chờ `target_tick`, không chỉ RTT. Nhưng phép đo chưa có `first-wire`, kích thước hàng chờ hay tỉ lệ
mẫu applied/retired, nên **không suy được tỷ trọng** của từng phần; nguyên nhân định lượng của
p50/p95 vẫn là `unknown`. Không dùng p50 hay mean để suy số lệnh "đang bay" khi chưa đo trực tiếp.
RTT p50 156–167 ms (sáu run gốc) / 154 ms (trace) là phép đo `Ping`/`Pong` riêng, không mâu thuẫn
với p50 input ~0,8 s.

#### Kết luận được và không được

- Được: sáu tổ hợp exit 0, hội tụ hash, drained, không timeout; `input.rejected` là số batch bị từ
  chối, và trong run trace đều là `BadSequence` quanh chuyển lượt; p95 input 1,56–1,70 s là **thời
  gian lưu hành theo định nghĩa của harness**, không phải RTT.
- Không được: coi p95 ~1,6 s là "ngân sách latency đã đạt", coi hội tụ hash là bằng chứng latency,
  hoặc so trực tiếp với RTT 100 ms của hồ sơ. Không có ngưỡng latency nào được chốt trong SCOPE_V1
  hay task này; báo cáo **không** đặt pass/fail cho latency.
- Không được: quy p95 cho hàng đợi client hay `target_tick` như kết luận định lượng. Cơ chế đó có
  thể góp phần, nhưng chưa đo `first-wire`/queue size/tỉ lệ applied-retired nên tỷ trọng còn
  `unknown`.
- Không được: khẳng định nguyên nhân `InputRejected` là `NotPlaying` hay lỗi predictor/server; biến
  thể `InputError` chưa được log. Không được khẳng định mọi mẫu latency đều đã được áp dụng, vì
  retire trong `submit` không quan sát được.
- Phép kiểm tiếp theo (theo thứ tự): (a) log `InputError` + pha/round khi reject; (b) phát
  `InputAck.retired` của `submit` thành event/log và tách mẫu applied/retired; (c) ghi mốc **lần
  đầu lệnh thực sự nằm trong batch** để tách queue khỏi wire; (d) chỉ sau đó mới bàn ngưỡng latency.
  Việc (a)/(b) nằm ở `boom-server` — ngoài phạm vi #215, cần task riêng; không sửa predictor/server
  ở đây.

### Xử lý #226: target_tick lúc gửi và đếm retired theo sequence (2026-10-24)

Baseline `fa1291baa638bd07f5c3b21c753378acca7b44c7`. Task #226 (V1-027/G4) tách lỗi producer
QA khỏi client thật, repro nhỏ trước khi sửa.

**Nguyên nhân** (xác nhận bằng trace tạm ghi `target_tick`, tick server lúc submit và lúc
apply/retire; trace đã gỡ, không có trong code):

1. `DirectDtoBot::send_input` lấy `target_tick` do `Predictor::predict` gán **lúc tạo lệnh**, bằng
   tick của state dự đoán. State đó dựng từ snapshot nên cũ hơn tick authority; với RTT hồ sơ
   `scope-v1` (100 ms + jitter 0–30 ms, đo thực p50 ~158 ms), target cũ hơn `INPUT_LATE` (6 tick,
   100 ms @60 Hz) ngay sau vài chục lệnh.
2. Lỗi thứ hai làm trầm trọng: cửa sổ batch tối đa 8 lệnh chưa ack. Khi số lệnh chưa ack vượt 8,
   lệnh mới **chưa vào batch** nhiều chu kỳ (20 ms/lệnh), nhưng `target_tick` vẫn giữ giá trị lúc
   tạo; tới lần gửi đầu tiên thì đã quá hạn. Đây là khác biệt với ADR 0009 §3.2 ("chỉ tính/chỉnh
   `target_tick` cho lệnh **chưa gửi**").
3. `BotConfig.one_way_delay_ms` và `input_lead_ticks` của hồ sơ bị bỏ không dùng trong
   `send_input`/predictor.
4. Harness đếm `retired` theo **event** `InputRetired`, không chỉ sequence đã resolved và đã gửi,
   nên `retired` có thể lệch đơn vị với `resolved`.

**Sửa (bot QA, phần client thật ở §r2 dưới)** (giữ payload bất biến sau lần gửi đầu, ADR 0009 §3.2/§3.4(e)):

- `Predictor::predict_at(input, target_tick)` (API additive; `predict` gọi lại với tick state dự
  đoán) cho caller truyền tick authority ước lượng; dãy `target_tick` được kẹp không giảm theo
  `seq`.
- `PendingCommand.sent` + `Predictor::window_for_send(max, target_tick)`: lệnh **chưa gửi** được
  gán/chỉnh `target_tick` tại đúng lần đầu vào batch; lệnh đã gửi resend y nguyên payload.
- `DirectDtoBot` ước lượng tick server lúc gửi bằng `snapshot.tick + số tick trôi từ lúc nhận +
  one_way_delay_ms + input_lead_ticks`, rồi ghi mốc `sent` tại **first-wire** cho mọi lệnh lần đầu
  vào batch (không chỉ lệnh vừa tạo).
- Harness chỉ tăng `retired` khi sequence đó đã resolved và có trong `sent`; do đó
  `applied = resolved - retired` cùng đơn vị sequence.

**Đo trước/sau** (cùng seed 1, `scope-v1`; số liệu thô trong `local/issue-226/`, không commit):

| Lần chạy | Lệnh | `sent` (batch) | `resolved` | `retired` | `applied` | Latency count / p50 / p95 | RTT p50/p95 |
|---|---|---:|---:|---:|---:|---|---|
| Trước fix, 2 bot, 1 round (`before-scope-v1-2-1-r1.json`, sha256 `998ead32…`) | 2 bot | 1991 | 1825 | 1823* | 2 | 2 / 54 / 66 ms | 156 / 197 ms |
| Sau fix, 2 bot, 2 round (`after-scope-v1-2-1.json`, sha256 `43910ebb…`) | 2 bot | 1800 | 1567 | 0 | 1567 | 1567 / 88 / 113 ms | 158 / 195 ms |
| Sau fix, 4 bot, 2 round (`after-scope-v1-4-1.json`, sha256 `a4ad28f3…`) | 4 bot | 3735 | 3287 | 0 | 3287 | 3287 / 88 / 111 ms | 147 / 193 ms |

\* Trước fix harness đếm `retired` theo event; sau fix đếm theo sequence đã resolved. Vì vậy cột
`retired` trước fix chỉ dùng để thấy gần như toàn bộ lệnh bị retire, không so trực tiếp từng đơn vị
với sau fix. Mỗi batch hiện tạo đúng một lệnh mới nên `created = sent`; `resolved < created` ở lần
sau là các lệnh còn chờ khi round kết thúc, không phải retired.

Lệnh:

```text
cargo run -p boom-qa -- --profile scope-v1 --clients 2 --seed 1 --json-out local/issue-226/after-scope-v1-2-1.json
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 1 --json-out local/issue-226/after-scope-v1-4-1.json
```

Cả hai lần sau fix exit 0, `converged=true`, `drained=true`, `timed_out=false`; latency count bằng
đúng `applied`, nên mọi mẫu latency là lệnh thực sự được áp dụng (không còn mẫu retire). Đây là
bằng chứng **hồi quy** cho #226, không phải nghiệm thu G4/V1-027: chỉ chạy hai tổ hợp theo yêu cầu
task, chưa chạy lại ma trận sáu tổ hợp, chưa đặt ngưỡng latency/tỉ lệ pass mới, và không sửa
`INPUT_LATE`/`INPUT_LEAD`/profile để đạt số.

**Giới hạn:** trace xác nhận nguyên nhân là instrumentation tạm, không lưu trong repo; kết luận dựa
trên mã hiện tại + artifact JSON/log ở `local/`. Latency vẫn là first-wire → `applied_seq`, gồm
thời gian bay và chờ `target_tick`, không phải RTT và không phải thời điểm áp dụng chính xác.
`InputRejected` vẫn chỉ phân loại DTO `BadSequence` như #215; task này không đổi protocol/server.

### #226 r2 (2026-10-24) — client thật dùng clock ước lượng, regression có thời gian điều khiển

Phần trên chỉ sửa bot QA; **client sản phẩm** vẫn dùng đường cũ. Lượt r2 nối facade client:

- `NetClient::predict` lấy `target_tick` từ `input_target_tick`: tick snapshot đã áp + số tick
  trôi từ lúc nhận + RTT gần nhất. RTT đo bằng `Ping`/`Pong` của chính client trên đường input
  (`input_ping_retry`/`record_input_pong`, EWMA 1/4); **không** đọc `one_way_delay_ms` của fixture.
- `NetClient::send_window` gọi `window_for_send(max, target_tick)`: lệnh chưa gửi được stamp
  `target_tick` tại lần đầu vào batch; lệnh đã gửi giữ nguyên payload khi resend (ADR 0009 §3.2).
- Test `client_predict_stamps_target_tick_from_clock_and_resend_keeps_payload` dùng snapshot/RTT
  đặt trước, kiểm target vượt tick confirmed và payload resend byte-identical.
- Test `producer_batch_moves_and_places_bomb_with_controlled_fixture_clock` dùng fixture có trễ
  **100 ms tường minh**, target tick do test đặt; đi qua `NetClient::predict`→`resend`→batch→
  `Authority`, rồi kiểm người chơi di chuyển, bom được đặt, không có `InputRetired`. Test cũ
  `scope_latency_fixture_applies_input_instead_of_retiring_it` (Real + ngưỡng 10 %) và test tự
  gọi `state.step` đã bị bỏ; **không** dùng `TimingMode::Virtual` một mình làm bằng chứng delay
  100 ms.
- `ScheduledHarness` giữ `InputRetired` theo `round` của event (không lấy round hiện tại của
  authority); `input.retired`/`resolved` cùng đơn vị sequence.

**Giới hạn:** `input_target_override` và clock override chỉ là test hook cho fixture đồng hồ ảo;
production luôn dùng `Instant::now` và target ước lượng thật. Clock test điều khiển chung
snapshot_received_at, mốc gửi/nhận Ping và target để hai lần chạy cùng seed có cùng thứ tự frame.
Ước lượng dùng đồng hồ thật + RTT đo được, chưa có lead/budget latency mới; không suy từ test
fixture sang SLA hay nghiệm thu V1-027. Số đo cũ ở bảng trên vẫn là của bot QA trước r2.

**Đo tham chiếu r2 (một run, không phải ma trận):** `scope-v1`, 2 bot, seed 1, head `f292fd7`
(log/JSON `.re-work/qa-226-scope-v1-2-1.*`, không commit): exit 0, `converged=true`,
`drained=true`, `timed_out=false`; `sent=2087` batch, `resolved=1831` sequence, `retired=0`,
latency p50/p95 = 87/112 ms (1831 mẫu), RTT p50/p95 = 158/194 ms (22 mẫu), `rejected=28` batch,
`room_rejected=1`. Đây chỉ là quan sát hồi quy trên Linux local, **không** đặt ngưỡng latency,
không thay ma trận sáu tổ hợp và không chứng minh G4/V1-027.

## Giới hạn

- Harness đo **bản Rust hiện tại**, không kết luận gì về độ trùng khớp với client/server gốc
  (SCOPE_V1 §7.8).
- Snapshot Ended chỉ chứng minh state/hash, không thay bảng kết quả/rank. Bot thiếu bảng sẽ
  gửi `RoundResultRequest` theo [ADR 0009 §B.7](../decisions/0009-wire-protocol-input-snapshot-codec.md#b7-2026-10-22--recovery-roundresult),
  chỉ ack khi đã nhận bảng authority và hash cuối. Server giữ đúng bảng commit tới Start kế tiếp.
- Bản đồ và profile rules của harness là **fixture kiểm thử**, không phải map `FxMap/Map/Village00`
  của V1 (SCOPE_V1 §1) và không phải profile phát hành.
- Recovery test không yêu cầu client đã rời có hash của lượt bị ngắt; cần đủ bốn hash ở lượt sau.

## Nghiệm thu phần 2 (#166)

CI chạy bốn test trong `boom-qa/src/harness.rs::recovery_tests`: bốn predictor drain/hội tụ,
disconnect giữa trận rồi auth/rejoin, mất Pong tới hết liveness rồi auth/rejoin, và mất frame
RoundResult đầu của cả bốn bot rồi xin lại kết quả/drain/hội tụ.
Timer test là design vận hành: Ping 50 ms, timeout 500 ms; không đổi timer mặc định profile.

Sửa tính tất định #178 (baseline `26435d0`, đối chiếu kỹ thuật #158): ba recovery test
chọn `TimingMode::Virtual`, tiến `step_ms` cố định sau rào IO. Reader server xác nhận đã enqueue
frame qua bộ đếm tại lần `recv_frame` kế tiếp; harness chờ từng lần gửi để thứ tự giữa bot
không phụ thuộc scheduler. `Server::flush_outbound` đặt marker nội bộ sau queue writer;
marker không đi trên wire và không tiêu thụ PRNG lỗi mạng. Bot đọc tuần tự sau marker để
reader không tiến đồng hồ MemoryTransport giữa lúc writer còn gửi cùng batch. Reconnect
chờ EOF đã enqueue và writer cũ đóng trước khi tái dùng ID kết nối. Trần chờ rào 10 giây
chỉ phát hiện thread kẹt; không quyết định tick, retry hay liveness.

Nguyên nhân CI đỏ: thời gian thật tiếp tục chạy trong khi reader/writer chưa xử lý xong,
đặc biệt khoảng drain reconnect 100 ms có thể làm các bot khác lỡ Pong dưới timer 500 ms.
Thứ tự frame/retry thay đổi kéo theo dòng PRNG và thời điểm hoàn tất lượt;
ngân sách 20 giây thật không bảo đảm hoàn tất dù hash lượt sau đã hội tụ. `auth_failed`
không tự chứng minh account còn phiên cũ: bot reset về Register nên đăng ký lại tài khoản
đã có sẽ nhận UsernameUnavailable. Bot đã nhận AuthOk nay dùng Login khi mở lại phiên;
ca recovery kiểm không có auth failure, chỉ bot bị tiêm lỗi reconnect đúng một lần.
Trace chế độ ảo còn xác định `RoomRejected::StaleRequest`: reset request ID về 0 trong khi
lobby giữ mốc dedup theo account qua disconnect. Bot nay giữ request ID tăng liên tục
qua reconnect; regression kiểm đồng thời Login và request ID không lùi.

Assertion hội tụ giữ nguyên và bổ sung so sánh **toàn bộ RunReport** của hai lần cùng seed
cho mỗi scenario. Budget 20.000 ms nay là thời gian fixture. MemoryTransport Virtual
vẫn giữ loss/trùng/đảo thứ tự theo seed, nhưng giao ngay khi reader hỏi; các số đo ảo không
là benchmark RTT thật hoặc bằng chứng delay theo tick authority. Profile đo thủ công vẫn
chạy Real như trước; phần drain 100 ms của chế độ đó chưa được thay bằng rào.

Kiểm chứng local bản sửa #178: `CARGO_TARGET_DIR=$PWD/target cargo xtask ci` exit 0;
lặp 30 lần `taskset -c 0 cargo test -p boom-qa --lib recovery_tests` đạt 30/30
(3 test/lượt, mỗi scenario so sánh hai báo cáo); `git diff --check` sạch.
CI ba OS và review độc lập được theo dõi trên PR, chưa coi local pass là nghiệm thu V1.

Auth vẫn qua SQLite + AuthService/Argon2id thật, nhưng tham số hash fixture là 64 KiB/1 vòng/1 luồng
để auth không chặn poll lâu hơn timer liveness thử nghiệm. Không đổi cấu hình server production;
không dùng báo cáo này để benchmark auth hoặc đánh giá độ bền mật khẩu.

Đo dài thủ công, giữ profile mạng SCOPE_V1:

```sh
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42 --json-out qa.json
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42 --scenario disconnect --json-out qa-disconnect.json
cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42 --scenario liveness --server-ping-ms 100 --liveness-ms 2000 --json-out qa-liveness.json
```

Chưa đóng V1-027 chỉ từ test nhanh; bộ đo dài nhiều seed vẫn là điều kiện nghiệm thu release.

### Kết quả đo thủ công 2026-10-22 (Codex)

Baseline code `eff7f94c258e154c60a7c36a60ceaefdf7f5deba` + thay đổi #166, Linux,
4 bot, seed 42, profile `scope-v1`, 2 lượt hoàn tất mỗi bot; CLI như trên, ba lệnh đều exit 0.
Liveness dùng Ping 100 ms / timeout 2000 ms; hai kịch bản khác giữ timer profile.

| Scenario | Thời gian (ms) | Mẫu sai số | p50 / p95 (mpx) | Lượt:số hash | Kết quả |
|---|---|---|---|---|---|
| Steady | 47329 | 6135 | 4000 / 6000 | 1:4/4, 2:4/4 | PASS |
| Disconnect | 69964 | 8301 | 4000 / 6000 | 1:3/4, 2:4/4, 3:4/4 | PASS |
| Liveness | 70794 | 8574 | 4000 / 6000 | 1:3/4, 2:4/4, 3:4/4 | PASS |

Lượt 1 của recovery thiếu hash bot3 theo chủ đích; lượt 2/3 có đủ bốn hash khớp server.
`converged=false` cho toàn lần recovery giữ nguyên sự thiếu đó; `recovered=true`, `drained=true`,
`timed_out=false`. Steady có cả hai lượt hội tụ. Correction là phép đo sai số lịch sử đã nêu,
không là bằng chứng cảm giác điều khiển hoặc fidelity KR. Chưa chạy đủ ma trận 2/4 bot × 3 seed;
đường xin lại bảng RoundResult được kiểm riêng bởi test recovery §B.7, không thuộc phép đo cũ này.


Chạy fault mất bảng kết quả (cùng profile/seed có lỗi mạng):

```sh
cargo run -p boom-qa -- --profile fast --clients 4 --seed 7 --scenario round-result-loss
```

Scenario bỏ đúng frame `RoundResult` đầu ở mỗi bot; các phản hồi xin lại đi qua mạng bình thường.
Bot không được ack chỉ từ snapshot Ended. Test `mat_round_result_bon_bot_xin_lai_va_hoi_tu`
kiểm đủ bốn hash, ack kết quả và drain, không thay đổi cách tính % feature.


### Kiểm recovery RoundResult 2026-10-22 (Codex)

Baseline `26435d0cbb232a2e2657dd9aa01346ed71570172` + thay đổi recovery #166, Linux.
Lệnh đo: `cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42 --scenario round-result-loss`.
Exit 0, 47.305 ms: RTT 100 ms, jitter 0–30 ms, loss/trùng/đảo thứ tự mỗi loại 1%.
Cả bốn bot đã nhận bảng và ack 2 lượt; mỗi lượt có đủ 4/4 hash khớp server;
`recovered=true`, `drained=true`, `converged=true`, `timed_out=false`, `snapshot_missed=0`.
Correction có 6.115 mẫu (gồm 0), p50/p95 = 4.000/6.000 mpx; đây là regression của bản Rust,
không phải nghiệm thu fidelity hay ngân sách latency. Bộ đo dài này là một seed/bốn bot,
không thay ma trận release 2/4 bot × 3 seed.


Kiểm lặp cùng thay đổi recovery: **20/20 vòng PASS**, mỗi vòng chạy:

```sh
cargo test -p boom-client net::tests
cargo test -p boom-server recovery_round_result
cargo test -p boom-qa recovery_tests
```

Mỗi vòng gồm 29 test client mạng, 2 test server retention/quyền và 4 test QA recovery.
Có test mất frame đầu và nhận lại đúng bảng, duplicate, backoff/dừng retry, kết quả cũ,
Waiting, hết retention sau Start, hủy phòng/khác phòng, account ngoài roster và rejoin
bằng connection khác. Đây là regression của implementation theo contract Claude chốt.
