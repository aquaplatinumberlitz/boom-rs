# Kiến trúc dự án Rust

Revision **`architecture-r1`**, 2026-10-19 (r0 là draft trước khi có repo). Baseline `KR-00967-V1-scope-1`.
Nguồn chuẩn về cấu trúc crate, boundary, contract và quy tắc code. Scope/policy người chơi thuộc
[SCOPE_V1](SCOPE_V1.md); quyết định có lý do ở ADR 0003–0008.

## 1. Repo và crate

Repo `boom-rs` ([ADR 0003](../decisions/0003-engine-repo-and-docs-home.md)):

```text
boom-rs/
  AGENTS.md, CLAUDE.md, opencode.json   # quy tắc
  docs/                                 # tài liệu điều hành (xem docs/README.md)
  tools/                                # tool docs/catalog (Python)
  Cargo.toml, rust-toolchain.toml       # workspace, toolchain ghim phiên bản
  .cargo/config.toml                    # alias `cargo xtask`
  crates/
    boom-core/      # [có] luật trận: Mode, System, SimulationEvent, state, RNG, hash; thuần, không IO
      src/systems/  #      mỗi cơ chế gameplay một module (movement; bom/item/nhốt sẽ thêm)
    boom-content/   # [có] schema rules profile + map runtime; nạp/validate TOML
    boom-assets/    # [có] đọc định dạng legacy (.map) → schema content; CLI
    boom-replay/    # [có] CLI chạy/kiểm replay, so hash checkpoint
    boom-scenario/  # [có] định dạng dựng trận chung + harness boundary scenario
    boom-proto/     # [khung, đợt 2] envelope + DTO giao thức mới (ADR 0009), không chứa luật
    boom-net/       # [khung, đợt 2] trait transport, framing, session (ADR 0006)
    boom-room/      # [có] máy trạng thái phòng/vòng đời thuần (SCOPE_V1 §3), API ở §5.4
    boom-client/    # [khung, đợt 2] macroquad (ADR 0010): input, prediction, render/audio/UI; đọc bundle, không đọc pack
    boom-server/    # [khung, đợt 2] auth, lobby, authority, persistence
  xtask/                                # `cargo xtask ci`, golden replay
  data/rules/                           # rules profile có provenance (TOML)
  data/overlays/, data/items/, data/events/  # [dự kiến] overlay luật, hiệu ứng item, liveops (ADR 0008)
  fixtures/replays/                     # replay + hash golden, chạy trên CI ba OS
  fixtures/scenarios/<system>/          # boundary scenario theo system (active/pending)
  .github/workflows/ci.yml              # CI ba OS (cần remote để chạy)
```

Crate `[dự kiến]` chỉ được tạo khi task của pha đó mở. Không tách render/audio/UI thành crate riêng
trước khi có nhu cầu tái dùng hoặc ownership rõ.

Dependency (mũi tên = phụ thuộc vào):

```text
boom-content → (serde, toml)              # không phụ thuộc core/client/server
boom-core    → boom-content
boom-assets  → boom-content
boom-scenario → boom-core, boom-content
boom-replay  → boom-core, boom-content, boom-scenario
boom-proto   → boom-content (ID/schema identity)
boom-net     → boom-proto
boom-room    → boom-content
boom-client  → boom-core, boom-content, boom-proto, boom-net
boom-server  → boom-core, boom-content, boom-proto, boom-net, boom-room
```

`boom-proto` không phụ thuộc `SimulationState`: server/client đổi DTO ở adapter để schema mạng
không khóa layout bộ nhớ core. Runtime tiêu thụ bundle đã validate, client không mang pack decoder.

Dependency nền hiện có: `serde` (derive) và `toml` trong `boom-content` để đọc profile; `postcard` 1.x trong
`boom-proto` (ADR 0009); `macroquad` =0.4.16 (feature `audio`) trong `boom-client` (ADR 0010).
Thêm dependency mới ghi lý do trong commit; dependency nền ghi vào đây.

## 2. Mở rộng tính năng, mode và event

Theo [ADR 0008](../decisions/0008-extensibility-model.md). Tóm tắt luồng một tick:

```text
input đã chấp nhận ─▶ Mode.systems() theo thứ tự ─▶ SimulationState (có kiểu, được hash)
                              │                              │
                              ▼                              ▼
                     EventSink: SimulationEvent{id=(tick,seq), kind}   Mode.outcome() → RoundEnded
                              │
          ┌───────────────────┼───────────────────────┐
          ▼                   ▼                       ▼
  presentation journal   server: event miền      replay/golden: event_digest
  (âm thanh, hiệu ứng)   → stats/quest/thưởng
```

| Thêm gì | Ở đâu |
|---|---|
| Cơ chế gameplay | `boom-core/src/systems/<tên>.rs` implement `System`, thêm vào `systems()` của mode cần |
| Mode | Type implement `Mode`, đăng ký trong `modes()`; ID ghi trong replay/protocol |
| Thông số | Rule trong `data/rules/*.toml` + claim RULES_V1 |
| Event/season | `RulesOverlay` (luật) + dữ liệu liveops (lịch, thưởng, cờ) ở server |
| Phản ứng theo trận | Consumer `SimulationEvent` ngoài core, dedup theo event ID + round ID |
| Item mới | Dữ liệu item → primitive hiệu ứng có sẵn; primitive mới chỉ khi cần |
| Tính năng ngoài trận | Module backend theo domain, nhận event miền; ledger idempotent |

## 3. Quy tắc code

- Toolchain ghim trong `rust-toolchain.toml`; edition 2024; `[workspace.lints]` chung.
- `unsafe` bị cấm ở mọi crate hiện có.
- `boom-core` ([ADR 0004](../decisions/0004-time-numeric-model.md)): không float
  (`clippy::float_arithmetic` deny), không `HashMap/HashSet`, `Instant`, `SystemTime`
  (`crates/boom-core/clippy.toml`), không IO/socket/wall clock/render/audio.
- Mọi giá trị luật lấy từ rules profile; code core không chứa hằng số luật chơi.
- `cargo xtask ci` = fmt check + clippy `-D warnings` + test (gồm mọi scenario) + kiểm replay golden + docs check.
- Mọi thay đổi vào `main` qua pull request theo AGENTS §5 (CI ba OS xanh; reviewer độc lập trừ PR sổ tiến độ).

## 4. Rules profile, content và mô hình thời gian

Profile TOML ở `data/rules/` (overlay ở `RulesOverlay`, ADR 0008) gồm `profile_id`, `schema`, `release` và từng rule
`{ value, unit, source, status, claims }`. Nhãn `source`/`status` theo AGENTS §4; `claims` trỏ ID trong
[RULES_V1](../specification/gameplay/RULES_V1.md). `boom-content` từ chối khi nạp nếu:

- `status = "unknown"` mà có `value`, hoặc thiếu `value` khi status khác unknown;
- profile `release = true` còn rule `candidate` hoặc `unknown`;
- rule thiếu claim (trừ `design` thuần kiến trúc như tick).

Định danh profile là hash `fnv1a64-v0` của encoding chuẩn các rule (không phải hash file thô, để CRLF/LF
không đổi định danh). Replay ghi định danh này và từ chối profile khác.

Map runtime (`boom-content::MapGrid`) gồm kích thước, ô `(tile_id, obj_id)` theo build và spawn.
Loại ô (đi được, khối cứng, khối mềm) là bảng phân loại riêng theo build; ID chưa có trong bảng là
`Unknown`, và core **từ chối** dựng thế giới có ô Unknown thay vì đoán. Fixture tự viết dùng bản đồ ASCII
với loại ô đã biết. Đọc `.map` legacy hỗ trợ cả header VN (spawn +76) và KR (+80) theo R-003.

Thời gian: fixed step, `sim.ticks_per_second` từ profile; tọa độ `Mpx` (1/1000 px); ms → tick làm tròn lên
một lần khi nạp (ADR 0004). Hiệu chuẩn tick gốc và speed-level còn mở (Q-004, Q-005).

## 5. Contracts — `contracts-r4`

Revision **`contracts-r4`**, 2026-10-19 (r3 đã review ở #2; r4 chỉ thêm getter cho snapshot, chờ review ở PR đợt 2). Code là nguồn chuẩn của kiểu dữ liệu;
mục này giữ quy tắc. Review đổi gì thì integrator ra revision mới và báo các task bị ảnh hưởng.

| Revision | Thay đổi | Lý do |
|---|---|---|
| r1 | Bản đầu: state theo module, event V1, pipeline FFA, quyền ghi, harness scenario | START-C |
| r2 | `check_invariants(&World)`: ID duy nhất xuyên bom/item; ô, owner, grace, `ItemKind` đối chiếu World/Rules; người chơi trong lưới. `new_round` từ chối chỉ số vượt trần (`CoreError::InvalidStat`) và kiểm bất biến sau `on_round_start` (`CoreError::Contract`) | Review #2: ba lỗi P2 (ID trùng, tham chiếu sai, lượt dựng với chỉ số sai) |
| r3 | `check_invariants` kiểm độ dài dữ liệu lưới trước mọi truy cập ô; `Field::kind/hidden` không panic khi lệch; `World::new` từ chối địa hình có số ô khác kích thước, lưới rỗng hoặc spawn ngoài lưới (`CoreError::BadTerrain`). System không bị ảnh hưởng | Review #2 lần 2: lưới khai báo 3 ô chứa 2 ô vẫn Ok, chứa 0 ô thì panic |
| r4 | `Rng::state()`/`Rng::from_state()`, `IdAllocator::peek_next()`/`IdAllocator::from_next()` để snapshot khôi phục RNG và allocator | ADR 0009 §4.4 |

### 5.1 Kiểu trong `boom-core`

| Contract | Nơi định nghĩa | Ghi chú |
|---|---|---|
| `PlayerInput { dir, place_bomb }` | `sim.rs` | Input **đã được authority chấp nhận** cho một tick; `place_bomb` là cạnh nhấn, không phải giữ |
| `PlayerSetup { id, stats: CharacterStats }` | `sim.rs`, `state/player.rs` | `Stat { value, max }` cho speed/bombs/range, lấy từ BomberAttr qua content |
| `SimulationState` | `state/mod.rs` | `tick, phase, rng, ids, players, field, bombs, flames, items`; mỗi phần tự hash trong file của nó |
| `EntityId`, `IdAllocator` | `state/mod.rs` | ID bom/item cấp tăng dần từ 1 trong lượt; chỉ cấp qua `ids.next_id()` |
| `Player`, `Life`, `Stat` | `state/player.rs` | `Life::Trapped { since }` vẫn tính là chưa chết |
| `Field` | `state/field.rs` | Loại ô đổi được trong lượt + item giấu trong khối mềm |
| `Bomb { id, owner, cell, range, explodes_at, grace }` | `state/bomb.rs` | Tầm chụp lúc đặt (R-032); `grace` = người được phép đi ra khỏi bom |
| `Flame { cell, until, source, owner }` | `state/flame.rs` | `until` là tick cuối còn lửa (bao gồm); mỗi ô một flame |
| `FloorItem`, `ItemKind` | `state/item.rs` | `ItemKind(i)` = chỉ số trong `Rules::items` (V1: Bomb, Fire, Roller) |
| `Rules` | `rules.rs` | Mọi giá trị luật V1, đổi từ profile; thêm rule mới qua integrator |
| `EventKind` (10 loại V1) | `event.rs` | `name()` + `fields()` là encoding chung cho digest, scenario, log |
| `Mode`, `System` | `mode.rs`, `system.rs` | `System::on_round_start` cho việc đầu lượt (giấu item) |

`SimulationState::check_invariants(&World)` (`state/invariants.rs`) kiểm: danh sách sắp đúng; một bom/item/
flame mỗi ô; ID entity duy nhất trên mọi loại và do allocator cấp; bom/item trên ô nền, lửa trên nền hoặc
khối mềm; số ô dữ liệu khớp kích thước lưới (kiểm trước mọi truy cập ô); owner/grace là người chơi có thật; `ItemKind` có trong `Rules::items`; người chơi trong lưới;
chỉ số không vượt trần; item giấu chỉ trong khối mềm. Hàm không panic với state sai. `new_round` gọi nó
trước khi trả state; harness scenario và replay gọi sau mỗi tick, nên system vi phạm bị bắt ở tick gây lỗi.

### 5.2 Thứ tự trong một tick (pipeline FFA)

`movement → bomb_place → explosion → blocks → pickup → life`, rồi `tick += 1`, rồi `Mode::outcome`.
Đầu lượt: lưới từ địa hình → hoán vị spawn theo seed → `on_round_start` từng system theo thứ tự trên.
RNG được dùng đúng theo thứ tự này. Đổi thứ tự là đổi luật: chỉ integrator, kèm scenario và bless golden.

Hệ quả đã chốt: bom đặt trong step N nằm ở ô người chơi **sau khi** di chuyển của step N; bom có
`explodes_at = N` nổ trong step N; lửa sinh trong step N phá khối, nhốt người ngay trong step N;
kết quả lượt xét sau khi mọi system đã chạy. Còn mở (quyết trong task tương ứng, ghi vào RULES_V1):
lửa có phá item đang nằm trên nền không; nhặt item khi đã chạm trần thì item mất hay ở lại.

### 5.3 Ma trận quyền ghi

Bảng chi tiết ở đầu `crates/boom-core/src/systems/mod.rs`. Tóm tắt: mỗi field công khai chỉ có một
system được ghi (movement → vị trí/hướng và xóa `grace`; bomb_place → thêm bom; explosion → xóa bom, lửa;
blocks → lưới, item lộ/bị phá; pickup → xóa item, chỉ số; life → `Player.life`). Đọc thì tự do.
Cần ghi field của system khác → mở issue cho integrator, không sửa thẳng.

### 5.4 Contract mạng và replay (chưa có code)

Revision **`proto-r1`**, 2026-10-19, **Accepted** theo
[ADR 0009](../decisions/0009-wire-protocol-input-snapshot-codec.md) (PR #24). `boom-proto` chưa được tạo;
integrator mở task triển khai ở P7. Khi có code, kiểu DTO là nguồn chuẩn; mục này giữ quy tắc và bất biến review.

| Contract | Nội dung tối thiểu | Bất biến cần review |
|---|---|---|
| Envelope | 16 byte: magic `BOOM`, `wire_schema=1`, `kind`, `flags`, `payload_len`, reserved; payload postcard | Từ chối magic/schema/kind/flags/len sai trước khi decode; frame tối đa 64 KiB; message lạ không được đoán |
| Handshake (V1-002) | `wire_schema`, `state_schema`, `rules_profile_id`, `rules_hash`, `content_sha256`, capability | Khớp chính xác ở V1; server so với profile/bundle của mình, không dùng hash client để chọn nội dung; lệch bị từ chối trước sảnh |
| `InputCommand` / batch | `round_id`, player do server bind, `ack_snapshot`, tối đa 8 lệnh `{seq, target_tick, dir|bomb}` | Seq tăng liên tục **trong batch** (lỗ hổng giữa batch là gap); **payload bất biến theo `seq` sau lần gửi đầu**, chỉ lệnh chưa gửi mới được tính lại `target_tick`; **dedup trước kiểm cửa sổ**, lệnh `seq <= applied_seq` bỏ qua dù `target_tick` cũ; lệnh quá khứ ở đầu hàng được retire + ack (không áp dụng); cửa sổ trễ 100 ms/trước 200 ms; gap timeout 100 ms chỉ retire sequence thiếu, không bỏ lệnh pending; `InputRejected` mang `applied_seq`/`server_tick`/`first_seq`+`command_count` để xác định batch bị reject và resync; rate limit; authority đổi thành `PlayerInput` |
| `Snapshot` | `snapshot_seq`, round/tick, `state_schema`, `rules_hash`, `content_epoch`, phase, RNG/allocator, ack input, players/field/bombs/flames/items | **Đầy đủ**, khôi phục được `SimulationState`; client dựng lại và gọi `check_invariants`; bỏ snapshot cũ/khác round; chưa có delta |
| `ReplayHeader` | schema, profile ID + hash, mode, map, seed, roster/spawn, `hash_algo` | Từ chối revision/profile không khớp. Bản file hiện tại: `boom-replay` schema 1 |
| `ReplayRecord` | tick, `PlayerInput` **sau scheduler** (kể cả neutral/expired), checkpoint hash | Ghi input thực được áp dụng; không ghi raw batch/`target_tick`; không dựa wall clock |
| Event ra ngoài | `round_id` + `EventId` + `name` + `fields` | Consumer dedup theo `(round_id, EventId)`; snapshot không phát lại event |

Quy tắc kèm theo:

- Input mạng chỉ là **đề nghị**; server chuyển thành `PlayerInput` sau khi kiểm round, sequence, cửa sổ và
  rate limit. Client không gửi tọa độ/máu/item/kết quả. Lệnh trùng/đến muộn/ngoài cửa sổ không được làm đổi
  state; lệnh hết hạn/gap được ghi nhận là neutral trong replay. `applied_seq` là sequence đã **giải quyết**
  (áp dụng hoặc retire), không phải "đã áp dụng input"; ack dùng giá trị này.
- **Liveness:** dedup trước kiểm cửa sổ; lệnh quá khứ ở đầu hàng được retire ngay khi batch được chấp nhận
  + ack; gap timeout chỉ retire sequence thiếu rồi xử lý lệnh pending; `OutOfWindow` chỉ dành cho
  `target_tick` quá tương lai; `InputRejected` mang `applied_seq`/`server_tick` để client bỏ lệnh cũ, tách
  lệnh quá tương lai và resend. **Payload bất biến theo `seq` sau lần gửi đầu**: client chỉ tính lại
  `target_tick` cho lệnh chưa gửi; lệnh có thể đã buffer resend y nguyên (không retarget), còn lệnh chắc
  chắn chưa buffer được bỏ và thay bằng seq mới, lỗ hổng do gap timeout retire. Không nhánh nào giữ lệnh quá
  hạn ở đầu hàng mà không tiến ack.
- `Snapshot` V1 là full state, không delta. Kích thước Village00 4 người ước tính **829 B** điển hình và
  **5 692 B** trần theo miền V1 được enforce, dưới `MAX_SNAPSHOT_BYTES=16 KiB` (ADR 0009 §4.2); đây là
  design, chưa đo runtime. Trần dùng varint postcard đúng (u32 5 B, u64 10 B, cell 2 B, Vec prefix) và
  miền `15×13`/count được enforce. Toạ độ `x`/`y` là `Mpx` (1/1000 px, **không** phải chỉ số ô): domain
  `0 <= x < width·tile_mpx`, `0 <= y < height·tile_mpx` (V1: `x<600 000`, `y<520 000`) nên zigzag 3 B;
  nếu decoder không enforce domain, cận `i32` 5 B cho tổng **845/5 708 B**.
- `snapshot_seq`/`round_id`/`content_epoch`/`rules_hash` phải khớp phiên; `content_sha256` đầy đủ chỉ ở
  handshake. Client bỏ snapshot cũ/trùng và xin `SnapshotRequest` khi cần resync.
- Client dự đoán di chuyển của mình từ snapshot confirmed, replay lệnh chưa ack; bom chỉ là bóng chờ, còn
  nổ/nhốt/chết/item chỉ theo event server (ADR 0005). Snapshot đầy đủ làm resync đơn giản; delta sau này
  cần baseline/resync và ADR mới.
- `INPUT_*`/`SNAPSHOT_INTERVAL`/`MAX_*` là **tham số giao thức** trong `ProtocolLimits` có revision, không
  phải luật chơi và không nằm trong `Rules`; tick/round vẫn đọc từ `Rules`. Không rải hằng số trong code.
- Snapshot cần getter RNG state và allocator next từ core (contracts-r4, integrator); chưa có thì chưa thể
  hiện thực snapshot đầy đủ. Không sửa `crates/boom-core` trong task #14.
- Lệnh phòng tin cậy (`ready/start/leave`, kind dành riêng) tách khỏi input trận tần suất cao; cả hai có
  identity/dedup, chính sách hết hạn khác nhau. Policy phòng/start/disconnect lấy từ SCOPE_V1.
- Index ADR và dòng "Codec giao thức" ở §8 do integrator cập nhật khi nhận ADR 0009; task #14 chỉ sửa ADR
  mới và mục này theo phạm vi issue.

#### API `boom-room` (đề xuất task #13, integrator duyệt)

`boom-room` là state machine thuần, không socket/đồng hồ/lưu trữ. Entry point là `Lobby::apply(RoomCommand)`:

- Server bind `PlayerId` và `RequestId` vào lệnh. Cùng request thành công gần nhất trả
  `Outcome::Duplicate` (không đổi state, không phát event); request cũ hơn trả `StaleRequest`.
  Lệnh lỗi không đổi state nên không ghi request ID, có thể thử lại cùng ID sau khi sửa điều kiện.
- `RoomPolicy { max_players, min_players_to_start }` do server truyền vào, không có giá trị mặc định
  trong crate; task này chưa thêm khóa `room.*` vào `data/rules` vì ngoài phạm vi file. P7 phải nối
  policy từ rules profile/SCOPE_V1 §3 trước khi dùng thật.
- `Lobby` bảo đảm một người chỉ ở một phòng, cấp `RoomId`/`RoundId` tăng dần; join chiếm slot thấp nhất;
  host kế nhiệm là người vào sớm nhất còn lại.
- Pha `Waiting`/`Started`; start chốt `RoundMember { player, slot, character }` đúng một lần.
  `Loading`/`Playing`/`Results`, loading barrier, disconnect, chat chờ và spawn thuộc V1-009/010/022/023.
- Roster hoặc nhân vật đổi khi `Waiting` phát `ReadyReset` và hủy toàn bộ ready; host cũng phải ready
  (khác client gốc có chủ ý, RULES_V1 R-063). Lệnh start lặp không tạo lượt thứ hai.

### 5.5 Boundary scenario

Kịch bản nghiệm thu ở `fixtures/scenarios/<system>/*.toml`, chạy bằng `cargo test -p boom-scenario`
(định dạng ở `crates/boom-scenario/src/scenario.rs`). `active` phải qua; `pending` là acceptance khởi điểm
cho feature chưa làm. Scenario dựng sẵn được bom/item/lửa/item giấu, nên mỗi system test độc lập với
system khác. Worker nhận feature chuyển scenario của mình sang `active`, thêm ca biên, và reviewer
kiểm scenario đối chiếu RULES_V1 chứ không chỉ đối chiếu code.

## 6. Authority, prediction và side effect

Theo [ADR 0005](../decisions/0005-authority-and-prediction.md): server quyết mọi kết quả; client dự đoán
di chuyển của chính mình, chạy lại input chưa ack khi nhận snapshot; bom chỉ có bóng "chờ" ở presentation;
người khác được nội suy. Chia sẻ core không tự tạo anti-cheat: rate limit, validation, auth/session vẫn
là trách nhiệm server.

Audio/effect/UI tiêu thụ event qua presentation journal có ID; replay dự đoán không ghi economy, log kết quả
hoặc gửi thông báo lần hai. Kết quả lượt commit một lần bởi authority; snapshot trễ của lượt cũ bị bỏ
theo round ID. Nghiệm thu mạng dùng fixture SCOPE_V1 (RTT/jitter/loss/duplicate/reorder, resync,
loading/disconnect); ngoài hash hội tụ còn đo correction, input latency và không lặp audio/effect.

## 7. Asset pipeline, animation và Việt hóa

Pipeline: pack manifest/hash → decode có provenance → resolve dependency closure → schema runtime →
bundle/version/hash → validation → client. P4 port đọc pack sang `boom-assets` dựa trên định dạng đã RE
(`boom-re-l10n/tools/idf.py`, `boomkr-re/rust/krpack`), viết lại, không copy code. Byte decode đúng không
chứng minh frame assembly/timing đúng.

Content closure V1 nối Village00 → tile/object/collision, C00/C01 → Attr/animation và mọi state bom,
lửa, item, nhốt, chết; không bỏ qua field/tài nguyên unknown âm thầm. PNG/atlas cần kiểm pixel chuẩn,
animation/layout cần scene fixture riêng. Renderer nhận presentation snapshot + metadata animation;
render cadence tách tick mô phỏng.

Chuỗi Việt lấy từ catalog `boom-re-l10n/l10n/`, ánh xạ key rõ; ảnh có chữ chỉ vào bundle V1 khi có quyết định
review. Font Unicode, fallback, wrap, focus là contract UI. Golden bản Việt hóa tách golden client gốc.

## 8. Quyết định còn mở

| Quyết định | Trạng thái | Gate |
|---|---|---|
| Client stack | **ADR 0010: macroquad, có điều kiện** | Còn: build+run thật 3 OS, input-held/focus, playback audio (P6) |
| Persistence | Mở (Q-011); SQLite là ứng viên V1 | Uniqueness/transaction, migration/backup, test backend thật |
| Codec giao thức | **ADR 0009** | Triển khai `boom-proto`/`boom-net` ở P7 |
| Định dạng bundle content | Mở (P4) | Provenance, validation, asset closure, rebuild tái lập |
| Thứ tự chi tiết trong tick | Mở (START-C, Q-006) | Boundary scenario pickup/lửa/chain/chết cùng tick |

Đã quyết: thời gian/số học (ADR 0004), authority/prediction (ADR 0005), transport mặc định (ADR 0006),
mô hình mở rộng (ADR 0008).
Backend V1 chỉ lưu identity và tùy chọn; room/round là ephemeral theo SCOPE_V1.
