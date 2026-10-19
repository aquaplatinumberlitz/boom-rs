# Tiến độ so với catalog KR

> GENERATED — không sửa tay. Chạy `python3 tools/kr_catalog.py`.

Baseline mẫu số: **`KR-00967-V1-scope-1`**.
Nguồn: [features.json](../specification/features.json) và [execution.json](execution.json).
Kế hoạch thực hiện: [REWRITE_PLAN](REWRITE_PLAN.md); danh mục: [FEATURE_CATALOG](../specification/FEATURE_CATALOG.md).

## Số mục đã nghiệm thu

| Phạm vi | Done / tổng | Tỷ lệ số mục |
|---|---:|---:|
| Release 1 | 1 / 37 | 2.70% |
| Toàn catalog cần triển khai | 1 / 112 | 0.89% |

Có **2 mục drop_legacy** hiển thị riêng trong CSV, không cộng vào done hoặc mẫu số.
Đây là tỷ lệ **số mục nghiệm thu**, không phải % công sức, thời gian hay fidelity KR.
Không chia điểm cho in_progress/in_review/blocked. Scope đổi phải đổi revision và giải thích mẫu số.

| Trạng thái | V1 | Toàn catalog cần triển khai |
|---|---:|---:|
| Chưa làm | 2 | 77 |
| Đang làm | 16 | 16 |
| Bị chặn | 0 | 0 |
| Chờ review | 18 | 18 |
| Đã nghiệm thu | 1 | 1 |

## Planning và gate phát hành — không cộng vào feature %

Pha đang hoạt động: **P1, P2**. Chuẩn bị P0–P3: **1/4 pha hoàn tất**; các pha không có độ lớn bằng nhau.
Cổng phát hành: **0/6 passed**. Milestone P8: **Chưa làm**.
Chỉ release khi feature V1 đã nghiệm thu, tất cả gate passed và P8 done.

| Pha | Trạng thái | Owner | Đầu ra | Gate hoàn thành |
|---|---|---|---|---|
| P0 Tổ chức docs/scope | Đã nghiệm thu | integrator | Index, rules, registry, scope và migration | Nguồn chuẩn/link/tool rõ, giữ bằng chứng |
| P1 Tìm thông tin còn thiếu | Đang làm | research | Claim có nguồn trong RULES_V1, fixture và QUESTIONS cập nhật | Câu hỏi chặn kiến trúc có trả lời hoặc phương án kiểm chứng rõ |
| P2 Cấu trúc dự án/interface | Đang làm | architecture | ARCHITECTURE r1, ADR 0004–0008, contracts-r3 trong code, harness scenario, CI ba OS | Boundary/contracts review và giả định quan trọng được ghi nhận |
| P3 Review sẵn sàng | Chưa làm | integrator | Readiness và task prototype | Checklist readiness trong REWRITE_PLAN có kết luận |
| P4 Nền móng/asset/oracle | Chưa làm | assets/RE | Workspace nhỏ, content resolver và oracle fixture | Legacy vẫn chạy, content unknown rõ và schema phù hợp |
| P5 Core/offline | Chưa làm | core | Simulation/replay V1 | Rule scenarios/state hash tái lập và giới hạn nguồn rõ |
| P6 Client hình/tiếng/input | Chưa làm | client | Scene/animation/render/audio/input | Content/presentation contract và acceptance tương ứng |
| P7 Authority/mạng/phòng | Chưa làm | net/backend | Server/client round lifecycle | Authority/convergence, lifecycle và mạng có fixture |
| P8 Tích hợp/QA/phát hành V1 | Chưa làm | integrator/QA | V1 end-to-end trên3OS | Tất cả37feature V1done và6gate releasepassed |
| P9 Mở rộng full feature | Chưa làm | integrator | Các đợt W1–W4 theo registry | Toàn bộ112mục trong baseline hiện hành done; scope mới phải revision riêng |

| Gate | Trạng thái | Owner | Bằng chứng |
|---|---|---|---|
| G1 Content/map | open | assets/RE | — |
| G2 Movement/stats | open | RE/core | — |
| G3 Bomb/damage/items | open | RE/core | — |
| G4 Protocol/state | open | core/net | — |
| G5 UX/localization | open | client/QA | — |
| G6 Release toàn bộ V1 | open | QA/integrator | — |

## Từng feature V1

Owner là luồng; assignee/reviewer là người hoặc agent nhận task cụ thể.
Chi tiết mọi mục, proof/review/dependency nằm trong [CSV](../specification/features.csv) và JSON nguồn.

| ID | Feature | Trạng thái | Owner / assignee | Reviewer | Bước tiếp / blocker | Bằng chứng |
|---|---|---|---|---|---|---|
| V1-001 | Auth tài khoản dự án | Đang làm | backend / opencode-1/dsh-b | Claude/Codex | Logic auth/phiên xong; auth trên wire ở PR #107 (#99) đang sửa review. | [0011-identity-persistence-log-v1.md](<../../docs/decisions/0011-identity-persistence-log-v1.md>); `06058297ddd9`; [auth.rs](<../../crates/boom-server/src/auth.rs>); `14e359ce7954`; `c8b14a81e1e5`; `8759e037c647` |
| V1-002 | Handshake phiên bản/rules/content | Đang làm | net / dsh-a/dsh-b | Claude/Codex | Handshake đạt; Welcome fingerprint ở PR #107. | [handshake.rs](<../../crates/boom-proto/src/dto/handshake.rs>); `e627112e5fe5`; [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1` |
| V1-003 | Một kênh và sảnh danh sách phòng | Đang làm | backend / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-004 | Tạo phòng công khai | Đang làm | backend / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-005 | Vào/rời phòng | Đang làm | backend / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-006 | Chủ phòng và kick | Đang làm | backend / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-007 | Chọn C00/C01 | Đang làm | client / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-008 | Ready/cancel/start | Đang làm | backend / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-009 | Chat trong phòng chờ | Đang làm | backend / dsh-b | Claude/Codex | Server + wire (Addendum B) đạt; còn UI client (#103) và auth wire (#107). | [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `d0a8571a229c`; `3d34d7fdf99b`; `ae70eaec72bc` |
| V1-010 | Start barrier và spawn | Chờ review | net / dsh-a/dsh-b | Claude/Codex | Server authority qua wire đạt (test 2/4 client, delay/loss seed); done khi dependency done. | [lib.rs](<../../crates/boom-server/src/lib.rs>); `fb4ed043822a`; [authority.rs](<../../crates/boom-server/src/authority.rs>); [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `ae70eaec72bc` |
| V1-011 | Di chuyển và ưu tiên input | Đang làm | core / opencode-4 | Claude | Đã có đi/rẽ/va chạm (PR #22); còn ưu tiên giữ nhiều phím/mất focus (V1-033) và hiệu chuẩn tốc độ/bám góc (#11, Q-005). | [movement.rs](<../../crates/boom-core/src/systems/movement.rs>); `fefd5c0d810c` |
| V1-012 | Va chạm tường/gạch/bom | Chờ review | core / opencode-4 | Claude | Đã review và merge PR #22; đặt done khi dependency V1-011 done (AGENTS §6). | [movement.rs](<../../crates/boom-core/src/systems/movement.rs>); [bomb_grace_one_way.toml](<../../fixtures/scenarios/movement/bomb_grace_one_way.toml>); `fefd5c0d810c` |
| V1-013 | Đặt bom và hạn mức | Chờ review | core / opencode-1 | Claude | Đã review và merge PR #19; đặt done khi dependency V1-012 done (AGENTS §6). | [bomb_place.rs](<../../crates/boom-core/src/systems/bomb_place.rs>); [place_one.toml](<../../fixtures/scenarios/bomb_place/place_one.toml>); `4600810eacb6` |
| V1-014 | Fuse/nổ/lan/chặn | Chờ review | core / opencode-2 | Claude | Đã review và merge PR #21; đặt done khi dependency V1-013 done (AGENTS §6). | [explosion.rs](<../../crates/boom-core/src/systems/explosion.rs>); [rays_stop_at_solid.toml](<../../fixtures/scenarios/explosion/rays_stop_at_solid.toml>); `0c9207c4b67b` |
| V1-015 | Nổ dây chuyền | Chờ review | core / opencode-2 | Claude | Đã review và merge PR #21; đặt done khi dependency V1-014 done (AGENTS §6). | [explosion.rs](<../../crates/boom-core/src/systems/explosion.rs>); [chain.toml](<../../fixtures/scenarios/explosion/chain.toml>); `0c9207c4b67b` |
| V1-016 | Phá gạch và drop giới hạn | Chờ review | core / opencode-1 | Claude | Đã review và merge PR #30; đặt done khi V1-015 done. | [blocks.rs](<../../crates/boom-core/src/systems/blocks.rs>); [destroy_and_reveal.toml](<../../fixtures/scenarios/blocks/destroy_and_reveal.toml>); `c0323b13fb39` |
| V1-017 | Item Bomb tăng capacity | Chờ review | core / opencode-2 | Claude | Đã review và merge PR #29; đặt done khi V1-016 done. | [pickup.rs](<../../crates/boom-core/src/systems/pickup.rs>); [bomb.toml](<../../fixtures/scenarios/pickup/bomb.toml>); `5845f1541242` |
| V1-018 | Item Fire tăng tầm | Chờ review | core / opencode-2 | Claude | Đã review và merge PR #29; đặt done khi V1-016 done. | [pickup.rs](<../../crates/boom-core/src/systems/pickup.rs>); [bomb.toml](<../../fixtures/scenarios/pickup/bomb.toml>); `5845f1541242` |
| V1-019 | Item Roller tăng tốc | Chờ review | core / opencode-2 | Claude | Đã review và merge PR #29; đặt done khi V1-016 và V1-011 done. | [pickup.rs](<../../crates/boom-core/src/systems/pickup.rs>); [bomb.toml](<../../fixtures/scenarios/pickup/bomb.toml>); `5845f1541242` |
| V1-020 | Bẫy/bong bóng/chết | Chờ review | core / opencode-3 | Claude | Đã review và merge PR #20; đặt done khi dependency V1-014 done (AGENTS §6). | [life.rs](<../../crates/boom-core/src/systems/life.rs>); [trap_timeout.toml](<../../fixtures/scenarios/life/trap_timeout.toml>); `5b42a6a899a6` |
| V1-021 | Thắng/hòa/hết giờ | Chờ review | core / Claude | Codex | Thắng/hòa/hết giờ ở mode.rs (contracts review #2) + life #20; đặt done khi V1-020 done. | [mode.rs](<../../crates/boom-core/src/mode.rs>); [time_limit_draw.toml](<../../fixtures/scenarios/round/time_limit_draw.toml>); `5b42a6a899a6` |
| V1-022 | Kết quả và chơi vòng tiếp | Chờ review | backend / dsh-a/dsh-b | Claude/Codex | Server authority qua wire đạt (test 2/4 client, delay/loss seed); done khi dependency done. | [lib.rs](<../../crates/boom-server/src/lib.rs>); `fb4ed043822a`; [authority.rs](<../../crates/boom-server/src/authority.rs>); [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `ae70eaec72bc` |
| V1-023 | Disconnect/timeout | Chờ review | net / dsh-a/dsh-b | Claude/Codex | Server authority qua wire đạt (test 2/4 client, delay/loss seed); done khi dependency done. | [lib.rs](<../../crates/boom-server/src/lib.rs>); `fb4ed043822a`; [authority.rs](<../../crates/boom-server/src/authority.rs>); [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `ae70eaec72bc` |
| V1-024 | Input/snapshot authoritative | Đang làm | net / dsh-a/opencode-2 | Claude/Codex | Server xong; client mạng PR #89 (#80) đang sửa review. | [0009-wire-protocol-input-snapshot-codec.md](<../../docs/decisions/0009-wire-protocol-input-snapshot-codec.md>); `743a38af2a27`; [envelope.rs](<../../crates/boom-proto/src/envelope.rs>); [lib.rs](<../../crates/boom-net/src/lib.rs>); [lib.rs](<../../crates/boom-sync/src/lib.rs>); `2af995d8def9`; `e627112e5fe5`; `fb4ed043822a`; `ab4eb5a148a1` |
| V1-025 | Prediction/reconciliation | Chưa làm | net / chưa giao | — | — | — |
| V1-026 | Replay và state hash | Chờ review | core / dsh-b/dsh-a | Claude/Codex | Replay cùng hash 3 OS (CI), từ chối lệch rules/content, server ghi replay round thật kể cả disconnect; done khi V1-024 done. | [main.rs](<../../crates/boom-replay/src/main.rs>); [replay_log.rs](<../../crates/boom-server/src/replay_log.rs>); `5bcafa0f3547`; `956d54b37fb4` |
| V1-027 | QA mạng 4 client | Chưa làm | qa / chưa giao | — | — | — |
| V1-028 | Lưu tài khoản và tùy chọn | Chờ review | backend / opencode-1/opencode-2 | Claude/Codex | Identity bền qua restart + options client đạt; done khi V1-001 done. | [0011-identity-persistence-log-v1.md](<../../docs/decisions/0011-identity-persistence-log-v1.md>); `06058297ddd9`; [sqlite.rs](<../../crates/boom-server/src/identity/sqlite.rs>); [options.rs](<../../crates/boom-client/src/options.rs>); `53aabbfb9023`; `c8b14a81e1e5`; `bf5ca50b338f`; `a3a78b0ec85b` |
| V1-029 | Log/health và vận hành test | Chờ review | backend / dsh-a/opencode-1 | Claude/Codex | Log JSON/health/heartbeat/CLI đạt T1-H1…H7; done khi chạy trong serve thật (#108). | [0011-identity-persistence-log-v1.md](<../../docs/decisions/0011-identity-persistence-log-v1.md>); `06058297ddd9`; [admin.rs](<../../crates/boom-server/src/admin.rs>); [observability.rs](<../../crates/boom-server/src/observability.rs>); `8ffc64ed2421`; `8759e037c647` |
| V1-030 | Map Village00 + dependency closure | Chờ review | assets / opencode-4 | Claude/Codex | Closure + bundle + client vẽ Village00 đạt; done khi chốt. | [RULES_V1.md](<../../docs/specification/gameplay/RULES_V1.md>); `243d230fd7f0`; [closure.rs](<../../crates/boom-assets/src/closure.rs>); [sprite.rs](<../../crates/boom-assets/src/sprite.rs>); `9372f2074939`; `fd2e5d217251`; `282a051a5ce7`; [scene.rs](<../../crates/boom-client/src/scene.rs>); `abae61bba752`; `b3d4d068a3a7`; `4ec8acf9e133` |
| V1-031 | Animation C00/C01 và bomb | Đang làm | client / opencode-3/opencode-4 | Claude/Codex | Clip + phát theo tick đạt; còn presentation journal (chuỗi chết) và clip nổ ở client (#101). | [animation.rs](<../../crates/boom-assets/src/animation.rs>); `389b6ce78fc1`; [anim.rs](<../../crates/boom-client/src/anim.rs>); `5242e48f6127`; `40fb031334dd`; `93288f1f300a` |
| V1-032 | UI luồng V1 bằng asset KR | Đang làm | client / opencode-3 | Claude/Codex | Asset UI trong bundle; màn UI client ở #103. | [closure.rs](<../../crates/boom-assets/src/closure.rs>); `4ec8acf9e133` |
| V1-033 | Input/scale/focus | Đã nghiệm thu | client / dsh-b/opencode-2 | Claude/Codex | Done: mặc định mũi tên+Space (#51), rebind qua options.toml (#69, #77), integer scale + nhả phím khi mất focus (#51); reviewer độc lập Codex; không có dependency. | [input.rs](<../../crates/boom-client/src/input.rs>); `0c5df165ffab`; [options.rs](<../../crates/boom-client/src/options.rs>); `bf5ca50b338f`; `a3a78b0ec85b` |
| V1-034 | Audio/volume | Chờ review | client / dsh-b/dsh-a | Claude/Codex | BGM/SFX theo event, volume/mute, dedup reconcile đạt; done khi V1-025 và V1-031 done. | [audio.rs](<../../crates/boom-client/src/audio.rs>); [AUDIO_V1.md](<../../docs/specification/assets/AUDIO_V1.md>); `6dfdac10040c`; `aace7988256b` |
| V1-035 | Việt hóa UI V1 | Đang làm | client / opencode-3 | Claude/Codex | Chuỗi V1 từ catalog; 42 khóa unknown + 15 review chờ boom-re-l10n; font chưa chọn. | [vi-v1.toml](<../../data/l10n/vi-v1.toml>); [l10n.rs](<../../crates/boom-content/src/l10n.rs>); `71d36a66cd64` |
| V1-036 | Build Windows/Linux/macOS | Đang làm | release / opencode-1 | Claude/Codex | Build phát hành 3 OS ở #102. | — |
| V1-037 | Đối chiếu và báo sai khác | Đang làm | qa / dsh-a | Claude/Codex | Phần 1 đối chiếu server VN xong; còn fixture original/golden riêng. | [VN_ENGINE_DIFF.md](<../../docs/research/findings/VN_ENGINE_DIFF.md>); `440bc8fd5648` |

## Các mục catalog ngoài V1

Các mục này vẫn tính vào mẫu số toàn catalog; chưa tự mở scope triển khai.

| ID | Feature | Trạng thái | Assignee | Bằng chứng |
|---|---|---|---|---|
| KR-F002-L01 | Quảng trường đi bộ và NPC | Chưa làm | — | — |
| KR-F002-L02 | Kênh nhiều instance | Chưa làm | — | — |
| KR-F003-L01 | Battle đội và cứu đồng đội | Chưa làm | — | — |
| KR-F003-L02 | Phòng 5–8 người | Chưa làm | — | — |
| KR-F003-L03 | Password/private/spectator | Chưa làm | — | — |
| KR-F003-L04 | Chọn/random nhiều map và quick-match | Chưa làm | — | — |
| KR-F004-L01 | Consumable kim/khiên/phi tiêu | Chưa làm | — | — |
| KR-F004-L02 | Đẩy/đá/ném bom và block | Chưa làm | — | — |
| KR-F004-L03 | Nhiều nâng cấp/debuff/mount | Chưa làm | — | — |
| KR-F005-L01 | Toàn bộ roster và chỉ số riêng | Chưa làm | — | — |
| KR-F006-L01 | Xem/thử hàng | Chưa làm | — | — |
| KR-F006-L02 | Mua/tặng bằng tiền ảo | Chưa làm | — | — |
| KR-F006-L03 | Ví/ledger/ownership và transaction | Chưa làm | — | — |
| KR-F007-L01 | Hồ sơ thống kê lâu dài | Chưa làm | — | — |
| KR-F007-L02 | Inventory/equip và hạn sử dụng | Chưa làm | — | — |
| KR-F008-L01 | Chat kênh/trận và slash commands | Chưa làm | — | — |
| KR-F008-L02 | Whisper/ignore/mail nếu xác nhận | Chưa làm | — | — |
| KR-F009-L01 | Guild thành viên/quyền | Chưa làm | — | — |
| KR-F009-L02 | Guild hoạt động/xếp hạng | Chưa làm | — | — |
| KR-F010-L01 | Bạn bè/messenger | Chưa làm | — | — |
| KR-F010-L02 | Couple/kết đôi | Chưa làm | — | — |
| KR-F011-L01 | Quest/trigger/progress | Chưa làm | — | — |
| KR-F011-L02 | Daily mission/reset/reward | Chưa làm | — | — |
| KR-F012-L01 | CAPass nhiệm vụ và reward track | Chưa làm | — | — |
| KR-F013-L01 | Ladder và rating | Chưa làm | — | — |
| KR-F013-L02 | Weekly ladder và mùa giải | Chưa làm | — | — |
| KR-F014-L01 | Synthesis/hợp thành | Chưa làm | — | — |
| KR-F014-L02 | Production/chế tạo | Chưa làm | — | — |
| KR-F014-L03 | Resolution/phân giải | Chưa làm | — | — |
| KR-F015-L01 | Cần/mồi và câu cá | Chưa làm | — | — |
| KR-F015-L02 | Tặng/thả/đổi cá | Chưa làm | — | — |
| KR-F016-L01 | Pet ownership/equip | Chưa làm | — | — |
| KR-F016-L02 | Pet skill/random table | Chưa làm | — | — |
| KR-F017-L01 | Tutorial tương tác/NPC | Chưa làm | — | — |
| KR-F018-L01 | HiddenCatch chuẩn và arcade nếu xác nhận | Chưa làm | — | — |
| KR-F019-L01 | AI Battle/difficulty/reward | Chưa làm | — | — |
| KR-F020-L01 | Airplane/bonus stage | Chưa làm | — | — |
| KR-F021-L01 | Baseball hai đội/tính điểm | Chưa làm | — | — |
| KR-F022-L01 | BigBattle roster/map/luật riêng | Chưa làm | — | — |
| KR-F023-L01 | CAHero nghề/kỹ năng/trang bị | Chưa làm | — | — |
| KR-F023-L02 | RPG tiến trình/map riêng | Chưa làm | — | — |
| KR-F024-L01 | Monster AI/collision/spawn | Chưa làm | — | — |
| KR-F024-L02 | Campaign/stage clear/reward | Chưa làm | — | — |
| KR-F025-L01 | SandBox tạo/chia sẻ nội dung nếu xác nhận | Chưa làm | — | — |
| KR-F026-L01 | DIYBomb tùy biến nếu xác nhận | Chưa làm | — | — |
| KR-F027-L01 | ConfusedFight luật/ranking nếu xác nhận | Chưa làm | — | — |
| KR-F028-L01 | IamBoss vai trò/luật riêng | Chưa làm | — | — |
| KR-F029-L01 | SnowyChamp | Chưa làm | — | — |
| KR-F029-L02 | Antarctic | Chưa làm | — | — |
| KR-F030-L01 | MineExplore | Chưa làm | — | — |
| KR-F030-L02 | IamWriggle | Chưa làm | — | — |
| KR-F031-L01 | ItemCollection điều kiện/thưởng | Chưa làm | — | — |
| KR-F032-L01 | Event lịch/reset/reward | Chưa làm | — | — |
| KR-F032-L02 | InstanceReward/LuckyCard | Chưa làm | — | — |
| KR-F033-L01 | Trang phục/layer/slot | Chưa làm | — | — |
| KR-F033-L02 | Trang trí bom/ID/phòng | Chưa làm | — | — |
| KR-F034-L01 | Emote và fanfare | Chưa làm | — | — |
| KR-F035-L01 | Exchange giao dịch/confirm/cancel | Chưa làm | — | — |
| KR-F036-L01 | DTT pool/xác suất | Chưa làm | — | — |
| KR-F036-L02 | DTT season shop | Chưa làm | — | — |
| KR-F037-L01 | CustomMap format/import/export | Chưa làm | — | — |
| KR-F037-L02 | TileCustom/chia sẻ/moderation | Chưa làm | — | — |
| KR-F038-L01 | MonWars | Chưa làm | — | — |
| KR-F039-L01 | Partners và theme liên quan | Chưa làm | — | — |
| KR-F041-L01 | PC-room entitlement/benefits | Chưa làm | — | — |
| KR-F042-L01 | MissionPong | Chưa làm | — | — |
| KR-F042-L02 | CAMission | Chưa làm | — | — |
| KR-F042-L03 | GrowSprout | Chưa làm | — | — |
| KR-F043-L01 | Webview/news/credits ngoài luồng V1 | Chưa làm | — | — |
| KR-F044-L01 | Phần thưởng/EXP/rank cuối trận | Chưa làm | — | — |
| KR-F045-L01 | Mount ride/collision | Chưa làm | — | — |
| KR-F045-L02 | Special status/debuff | Chưa làm | — | — |
| KR-F046-L01 | Captain2/WhiteAngel phân loại | Chưa làm | — | — |
| KR-F046-L02 | Retro/RetroWriggle | Chưa làm | — | — |
| KR-F046-L03 | HoneyButterChip/FullMoon/bnbolympic | Chưa làm | — | — |

## Không triển khai legacy — không tính done

- `KR-F001-L01`: Launcher/NGM/Nexon login
- `KR-F040-L01`: GreenPC/legacy trust status

## Agent phải cập nhật gì

Nhận task theo plan, ghi ID và assignee; sửa trạng thái/proof/review trong JSON nguồn rồi sinh lại report.
Done cần commit đã tích hợp vào HEAD của checkout tích hợp, baseline/spec được chấp nhận, verification passed, reviewer độc lập approved,
bằng chứng tồn tại và dependency done. Validator chỉ kiểm tính nhất quán/tham chiếu; không tự xác minh sự thật của proof.
Planning/research cập nhật execution.json và QUESTIONS tương ứng; không đánh dấu feature done vì đã viết spec hoặc tìm thấy asset.
