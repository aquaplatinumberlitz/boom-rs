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
    boom-proto/     # [dự kiến P7] envelope + DTO giao thức mới, không chứa luật
    boom-net/       # [dự kiến P7] trait transport, framing, session (ADR 0006)
    boom-room/      # [có, rỗng] máy trạng thái phòng/vòng đời thuần (SCOPE_V1 §3), chờ task backend
    boom-client/    # [dự kiến P6] input, prediction, render/audio/UI (ADR 0007)
    boom-server/    # [dự kiến P7] auth, lobby, authority, persistence
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

Dependency nền hiện có: `serde` (derive) và `toml` trong `boom-content` để đọc profile.
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
- Mọi thay đổi vào `main` qua pull request; CI ba OS phải xanh (TEAM_WORKFLOW §5).

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

## 5. Contracts — `contracts-r2`

Revision **`contracts-r2`**, 2026-10-19, chờ review lại ở issue #2. Code là nguồn chuẩn của kiểu dữ liệu;
mục này giữ quy tắc. Review đổi gì thì integrator ra revision mới và báo các task bị ảnh hưởng.

| Revision | Thay đổi | Lý do |
|---|---|---|
| r1 | Bản đầu: state theo module, event V1, pipeline FFA, quyền ghi, harness scenario | START-C |
| r2 | `check_invariants(&World)`: ID duy nhất xuyên bom/item; ô, owner, grace, `ItemKind` đối chiếu World/Rules; người chơi trong lưới. `new_round` từ chối chỉ số vượt trần (`CoreError::InvalidStat`) và kiểm bất biến sau `on_round_start` (`CoreError::Contract`) | Review #2: ba lỗi P2 (ID trùng, tham chiếu sai, lượt dựng với chỉ số sai) |

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
khối mềm; owner/grace là người chơi có thật; `ItemKind` có trong `Rules::items`; người chơi trong lưới;
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

| Contract | Nội dung tối thiểu | Bất biến cần review |
|---|---|---|
| `InputCommand` | round ID, player ID do server bind, sequence, tick dự định, hướng giữ, cạnh nhấn bom, ack snapshot | Không gửi tọa độ/kết quả; dedup theo sequence; cửa sổ input hữu hạn (ADR 0006). Authority đổi thành `PlayerInput` |
| `Snapshot` | schema revision, rules/content hash, round/tick, state DTO, input ack | Bản đầy đủ khôi phục được `SimulationState`; delta chỉ thêm sau khi có baseline/resync |
| `ReplayHeader` | schema, profile ID + hash, mode, map, seed, roster/spawn, `hash_algo` | Từ chối revision/profile không khớp. Bản file hiện tại: `boom-replay` schema 1 |
| `ReplayRecord` | tick, input được server áp dụng (kể cả neutral), checkpoint hash | Ghi input thực được áp dụng; không dựa wall clock |
| Event ra ngoài | `round_id` + `EventId` + `name` + `fields` | Consumer dedup theo `(round_id, EventId)` |

Lệnh phòng tin cậy (`ready/start/leave`) tách khỏi input trận tần suất cao; cả hai có identity/dedup,
chính sách hết hạn khác nhau. Policy phòng/start/disconnect lấy từ SCOPE_V1.

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
| Client stack | ADR 0007 Proposed — spike | Một scene V1 trên 3 OS, so độ đúng pixel và chi phí bảo trì |
| Persistence | Mở (Q-011); SQLite là ứng viên V1 | Uniqueness/transaction, migration/backup, test backend thật |
| Codec giao thức | Mở (START-C/P7) | Version compatibility, kích thước snapshot, dễ debug |
| Định dạng bundle content | Mở (P4) | Provenance, validation, asset closure, rebuild tái lập |
| Thứ tự chi tiết trong tick | Mở (START-C, Q-006) | Boundary scenario pickup/lửa/chain/chết cùng tick |

Đã quyết: thời gian/số học (ADR 0004), authority/prediction (ADR 0005), transport mặc định (ADR 0006),
mô hình mở rộng (ADR 0008).
Backend V1 chỉ lưu identity và tùy chọn; room/round là ephemeral theo SCOPE_V1.
