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
`boom-qa/src/lib.rs`: cùng API `predict/apply_snapshot/window/acknowledge`, không chép thuật toán. `RoomFlow` cũng
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
