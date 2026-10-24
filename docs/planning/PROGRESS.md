# Tiến độ so với catalog KR

> GENERATED — không sửa tay. Chạy `python3 tools/kr_catalog.py`.

Baseline mẫu số: **`KR-00967-V1-scope-1`**.
Nguồn: [features.json](../specification/features.json) và [execution.json](execution.json).
Kế hoạch thực hiện: [REWRITE_PLAN](REWRITE_PLAN.md); danh mục: [FEATURE_CATALOG](../specification/FEATURE_CATALOG.md).

## Số mục đã nghiệm thu

| Phạm vi | Done / tổng | Tỷ lệ số mục |
|---|---:|---:|
| Release 1 | 36 / 37 | 97.30% |
| Toàn catalog cần triển khai | 36 / 112 | 32.14% |

Có **2 mục drop_legacy** hiển thị riêng trong CSV, không cộng vào done hoặc mẫu số.
Đây là tỷ lệ **số mục nghiệm thu**, không phải % công sức, thời gian hay fidelity KR.
Không chia điểm cho in_progress/in_review/blocked. Scope đổi phải đổi revision và giải thích mẫu số.

| Trạng thái | V1 | Toàn catalog cần triển khai |
|---|---:|---:|
| Chưa làm | 0 | 75 |
| Đang làm | 0 | 0 |
| Bị chặn | 0 | 0 |
| Chờ review | 1 | 1 |
| Đã nghiệm thu | 36 | 36 |

## Planning và gate phát hành — không cộng vào feature %

Pha đang hoạt động: **P1, P2, P3, P4, P5, P6, P7, P8**. Chuẩn bị P0–P3: **1/4 pha hoàn tất**; các pha không có độ lớn bằng nhau.
Cổng phát hành: **2/6 passed**. Milestone P8: **Đang làm**.
Chỉ release khi feature V1 đã nghiệm thu, tất cả gate passed và P8 done.

| Pha | Trạng thái | Owner | Đầu ra | Gate hoàn thành |
|---|---|---|---|---|
| P0 Tổ chức docs/scope | Đã nghiệm thu | integrator | Index, rules, registry, scope và migration | Nguồn chuẩn/link/tool rõ, giữ bằng chứng |
| P1 Tìm thông tin còn thiếu | Đang làm | research | Claim có nguồn trong RULES_V1, fixture và QUESTIONS cập nhật | Câu hỏi chặn kiến trúc có trả lời hoặc phương án kiểm chứng rõ |
| P2 Cấu trúc dự án/interface | Chờ review | architecture | ARCHITECTURE, contracts-r4, ADR protocol/client stack, harness scenario và CI ba OS | Boundary/contracts review và giả định quan trọng được ghi nhận |
| P3 Review sẵn sàng | Đang làm | integrator | Readiness và task prototype | Checklist readiness trong REWRITE_PLAN có kết luận |
| P4 Nền móng/asset/oracle | Chờ review | assets/RE | Workspace nhỏ, content resolver và oracle fixture | Legacy vẫn chạy, content unknown rõ và schema phù hợp |
| P5 Core/offline | Chờ review | core | Simulation/replay V1 | Rule scenarios/state hash tái lập và giới hạn nguồn rõ |
| P6 Client hình/tiếng/input | Đang làm | client | Scene/animation/render/audio/input | Content/presentation contract và acceptance tương ứng |
| P7 Authority/mạng/phòng | Đang làm | net/backend | Server/client round lifecycle | Authority/convergence, lifecycle và mạng có fixture |
| P8 Tích hợp/QA/phát hành V1 | Đang làm | integrator/QA | V1 end-to-end trên3OS | Tất cả37feature V1done và6gate releasepassed |
| P9 Mở rộng full feature | Chưa làm | integrator | Các đợt W1–W4 theo registry | Toàn bộ112mục trong baseline hiện hành done; scope mới phải revision riêng |

| Gate | Trạng thái | Owner | Bằng chứng |
|---|---|---|---|
| G1 Content/map | passed | assets/RE | [BUNDLES.md](<../../docs/specification/assets/BUNDLES.md>); [village00-kr.toml](<../../data/maps/village00-kr.toml>); `891c7c7d3d76` |
| G2 Movement/stats | in_review | RE/core | [0013-rule-source-precedence.md](<../../docs/decisions/0013-rule-source-precedence.md>); [RULES_V1.md](<../../docs/specification/gameplay/RULES_V1.md>); [v1-dev.toml](<../../data/rules/v1-dev.toml>); [README.md](<../../docs/evidence/measurements/vn-player-grace-2026-10-24/README.md>); `b69ca8b9de2f` |
| G3 Bomb/damage/items | in_review | RE/core | [RULES_V1.md](<../../docs/specification/gameplay/RULES_V1.md>); [v1-dev.toml](<../../data/rules/v1-dev.toml>); [warmup_freezes_round.toml](<../../fixtures/scenarios/round/warmup_freezes_round.toml>); `bcd6362b64ff`; [README.md](<../../docs/evidence/measurements/vn-flame-window-2026-10-24/README.md>); `b69ca8b9de2f` |
| G4 Protocol/state | passed | core/net | [QA_NETWORK.md](<../../docs/specification/QA_NETWORK.md>); `743e8ea9cdf0`; `4c17d85d0b66`; `e0dcdfc31e8e`; `ed16335c889e`; [0009-wire-protocol-input-snapshot-codec.md](<../../docs/decisions/0009-wire-protocol-input-snapshot-codec.md>); [tests.rs](<../../crates/boom-client/src/net/tests.rs>) |
| G5 UX/localization | in_review | client/QA | [UI_STRINGS_V1.md](<../../docs/specification/localization/UI_STRINGS_V1.md>); [l10n_review_v1.py](<../../tools/l10n_review_v1.py>); [AUDIO_V1.md](<../../docs/specification/assets/AUDIO_V1.md>); `2b86a87fb4f4`; [ui-vi-v1.toml](<../../data/assets/ui-vi-v1.toml>); `43739e2fa6ab`; [README.md](<../../docs/evidence/measurements/rust-client-ux-2026-10-24/README.md>); `796284d422ba` |
| G6 Release toàn bộ V1 | blocked | QA/integrator | [PROGRESS.md](<../../docs/planning/PROGRESS.md>); `303ab49a07c2`; [README.md](<../../docs/evidence/measurements/rust-client-ux-2026-10-24/README.md>); [package-qa.json](<../../docs/evidence/measurements/rust-client-ux-2026-10-24/package-qa.json>) |

## Từng feature V1

Owner là luồng; assignee/reviewer là người hoặc agent nhận task cụ thể.
Chi tiết mọi mục, proof/review/dependency nằm trong [CSV](../specification/features.csv) và JSON nguồn.

| ID | Feature | Trạng thái | Owner / assignee | Reviewer | Bước tiếp / blocker | Bằng chứng |
|---|---|---|---|---|---|---|
| V1-001 | Auth tài khoản dự án | Đã nghiệm thu | backend / opencode-1/dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Không thiếu acceptance tạo/login, sai thông tin, một phiên. Claude ghi integration commit/proof/review hiện hành; không cần chờ UI V1-032 để nghiệm thu logic auth này. | [0011-identity-persistence-log-v1.md](<../../docs/decisions/0011-identity-persistence-log-v1.md>); `06058297ddd9`; [auth.rs](<../../crates/boom-server/src/auth.rs>); `14e359ce7954`; `c8b14a81e1e5`; `8759e037c647`; `c8fbd31b7c8a`; `ee779e43c9bc` |
| V1-002 | Handshake phiên bản/rules/content | Đã nghiệm thu | net / dsh-a/dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Acceptance mismatch protocol/rules/content có code và test; chưa cần thêm code để đóng mục này. Chờ ghi V1-001 done; cập nhật proof #117/#136 thay next_action cũ. | [handshake.rs](<../../crates/boom-proto/src/dto/handshake.rs>); `e627112e5fe5`; [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `c8fbd31b7c8a`; `ee779e43c9bc` |
| V1-003 | Một kênh và sảnh danh sách phòng | Đã nghiệm thu | backend / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Luồng auth→một sảnh và danh sách/số người tạo/vào/rời có bằng chứng ở server + NetClient/NetBackend. Chờ dependency. Việc gọi UI mạng từ binary còn thiếu thuộc V1-032, không suy rằng test NetBackend chứng minh màn hình t | [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-004 | Tạo phòng công khai | Đã nghiệm thu | backend / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: ID phòng duy nhất, một phòng/account và 4 slot đã có regression. Chờ V1-003 done; không thiếu code riêng theo acceptance này. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-005 | Vào/rời phòng | Đã nghiệm thu | backend / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Đủ guard capacity/Playing + leave dọn slot; chờ V1-004 done. Test join được serialize qua authority, không diễn giải thành phép đo stress nhiều thread. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-006 | Chủ phòng và kick | Đã nghiệm thu | backend / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Host transfer theo thứ tự join; non-host kick/start bị chặn. Chờ V1-005 done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-007 | Chọn C00/C01 | Đã nghiệm thu | client / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Selection/đồng bộ/hủy ready đã có; không cần đợi RE #149 nâng mapping thành confirmed. Chờ V1-005 done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-008 | Ready/cancel/start | Đã nghiệm thu | backend / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Acceptance 2–4/all-ready, hủy ready khi roster/selection đổi, start lặp một lần đạt trong test. Chờ V1-007 done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c`; [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-009 | Chat trong phòng chờ | Đã nghiệm thu | backend / dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Logic chat UTF-8/phạm vi/rate có bằng chứng; font/wrap sản phẩm theo V1-035. Chờ V1-005 done. | [server.rs](<../../crates/boom-server/src/server.rs>); [room.rs](<../../crates/boom-sync/src/room.rs>); `d0a8571a229c`; `3d34d7fdf99b`; `ae70eaec72bc`; `c8fbd31b7c8a`; `c129a21815b8`; `ee779e43c9bc` |
| V1-010 | Start barrier và spawn | Đã nghiệm thu | net / dsh-a/dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Barrier/15s/cancel có test, snapshot loading đã sửa #153. Chờ V1-008 (V1-030 đã done). | [lib.rs](<../../crates/boom-server/src/lib.rs>); `fb4ed043822a`; [authority.rs](<../../crates/boom-server/src/authority.rs>); [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `ae70eaec72bc`; `ee779e43c9bc` |
| V1-011 | Di chuyển và ưu tiên input | Đã nghiệm thu | core / opencode-4 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Trace movement/rẽ/ưu tiên nhiều hướng và release focus đối chiếu R-107/R-114/R-116; design BnB hợp lệ theo ADR 0013. V1-030/V1-033 đều đã done; không còn blocker acceptance hiện hành. Không gọi đây là hiệu chuẩn KR. | [movement.rs](<../../crates/boom-core/src/systems/movement.rs>); `fefd5c0d810c`; `b26c88c6c852`; [0012-v1-rules-from-bnb.md](<../../docs/decisions/0012-v1-rules-from-bnb.md>); `ee779e43c9bc` |
| V1-012 | Va chạm tường/gạch/bom | Đã nghiệm thu | core / opencode-4 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Không xuyên tường/gạch/bom, ra bom và mất pass sau hết overlap đạt. Chờ V1-011 được ghi done. | [movement.rs](<../../crates/boom-core/src/systems/movement.rs>); [bomb_grace_one_way.toml](<../../fixtures/scenarios/movement/bomb_grace_one_way.toml>); `fefd5c0d810c`; `b26c88c6c852`; `ee779e43c9bc` |
| V1-013 | Đặt bom và hạn mức | Đã nghiệm thu | core / opencode-1 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Bom cùng ô/cap/command lặp đã được kiểm; grace bản mới theo #140. Chờ V1-012 done. | [bomb_place.rs](<../../crates/boom-core/src/systems/bomb_place.rs>); [place_one.toml](<../../fixtures/scenarios/bomb_place/place_one.toml>); `4600810eacb6`; `ee779e43c9bc` |
| V1-014 | Fuse/nổ/lan/chặn | Đã nghiệm thu | core / opencode-2 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Acceptance fuse/ray/block/TTL có nguồn design hiện hành và test; không đợi #148. Chờ V1-013 done; dùng integration #143 thay proof #21 cũ. | [explosion.rs](<../../crates/boom-core/src/systems/explosion.rs>); [rays_stop_at_solid.toml](<../../fixtures/scenarios/explosion/rays_stop_at_solid.toml>); `0c9207c4b67b`; `8bc88e1a38d7`; `ee779e43c9bc` |
| V1-015 | Nổ dây chuyền | Đã nghiệm thu | core / opencode-2 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Chain một lần/thứ tự/double drop đạt theo R-109; không coi test sinh từ implementation là oracle gốc. Chờ V1-014 done; proof phải gồm #143/#160. | [explosion.rs](<../../crates/boom-core/src/systems/explosion.rs>); [chain.toml](<../../fixtures/scenarios/explosion/chain.toml>); `0c9207c4b67b`; `8bc88e1a38d7`; `ee779e43c9bc` |
| V1-016 | Phá gạch và drop giới hạn | Đã nghiệm thu | core / opencode-1 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Đã thay pre-roll #30 bằng roll khi vỡ #160: RNG/ô ổn định, ba item V1, 350‰, trọng số 3650:2800:2000. Chờ V1-015 done và cập nhật evidence mới; không còn để next_action trỏ duy nhất implementation cũ. | [blocks.rs](<../../crates/boom-core/src/systems/blocks.rs>); [destroy_and_reveal.toml](<../../fixtures/scenarios/blocks/destroy_and_reveal.toml>); `c0323b13fb39`; `ee779e43c9bc` |
| V1-017 | Item Bomb tăng capacity | Đã nghiệm thu | core / opencode-2 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Capacity +1, cap hiệu ứng và một người nhận có test; cap vẫn mất item theo R-109 đã đổi #160. Chờ V1-016 done; approval #29 cũ một mình không đủ cho hành vi mới. | [pickup.rs](<../../crates/boom-core/src/systems/pickup.rs>); [bomb.toml](<../../fixtures/scenarios/pickup/bomb.toml>); `5845f1541242`; `ee779e43c9bc` |
| V1-018 | Item Fire tăng tầm | Đã nghiệm thu | core / opencode-2 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Fire +1 và bom đã đặt giữ tầm chụp có code/test. Chờ V1-016 done; dùng proof mới cho cap/pickup. | [pickup.rs](<../../crates/boom-core/src/systems/pickup.rs>); [bomb.toml](<../../fixtures/scenarios/pickup/bomb.toml>); `5845f1541242`; `ee779e43c9bc` |
| V1-019 | Item Roller tăng tốc | Đã nghiệm thu | core / opencode-2 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Theo spec được duyệt: +1 level, 24px/s/level R-114 (oss/design), không phụ thuộc render FPS. Không tuyên bố đã đo KR; ADR 0013 cho phép design này. Chờ V1-011 và V1-016 done. | [pickup.rs](<../../crates/boom-core/src/systems/pickup.rs>); [bomb.toml](<../../fixtures/scenarios/pickup/bomb.toml>); `5845f1541242`; `ee779e43c9bc` |
| V1-020 | Bẫy/bong bóng/chết | Đã nghiệm thu | core / opencode-3 | Claude | Done theo spec V1 hiện hành (rà #161). Ghi chú: Acceptance trap→death/touch/một lần đạt theo R-108/R-115/R-116. Chờ V1-014 done; proof #20 cũ đã khác cơ chế, cần #143. | [life.rs](<../../crates/boom-core/src/systems/life.rs>); [trap_timeout.toml](<../../fixtures/scenarios/life/trap_timeout.toml>); `5b42a6a899a6`; `8bc88e1a38d7`; `ee779e43c9bc` |
| V1-021 | Thắng/hòa/hết giờ | Đã nghiệm thu | core / Claude | Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Winner/draw/trapped còn sống đạt; duration 120s theo R-110, warmup 3s theo R-113. Chờ V1-020 done; không lấy contracts review #2 làm PR approval hiện hành (#2 là issue, không phải PR). | [mode.rs](<../../crates/boom-core/src/mode.rs>); [time_limit_draw.toml](<../../fixtures/scenarios/round/time_limit_draw.toml>); `5b42a6a899a6`; `8bc88e1a38d7`; `ee779e43c9bc` |
| V1-022 | Kết quả và chơi vòng tiếp | Đã nghiệm thu | backend / dsh-a/dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Kết quả/ack hoặc 5s/reset round/không auto-ready có bằng chứng trên transport hoạt động. Chờ V1-021 done. Mất application-frame RoundResult chưa recover là giới hạn đã có regression, cần xử lý ở QA loss V1-027 trước phát | [lib.rs](<../../crates/boom-server/src/lib.rs>); `fb4ed043822a`; [authority.rs](<../../crates/boom-server/src/authority.rs>); [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `ae70eaec72bc`; `ee779e43c9bc` |
| V1-023 | Disconnect/timeout | Đã nghiệm thu | net / dsh-a/dsh-b | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Server leave ngay/neutral→10s/loại một lần/loading cancel/không rejoin/room rỗng hủy có code/test. Chờ V1-022 done. UX báo mất kết nối còn thuộc tích hợp UI, không lấy core server test chứng minh UX. | [lib.rs](<../../crates/boom-server/src/lib.rs>); `fb4ed043822a`; [authority.rs](<../../crates/boom-server/src/authority.rs>); [server.rs](<../../crates/boom-server/src/server.rs>); `ab4eb5a148a1`; `ae70eaec72bc`; `ee779e43c9bc` |
| V1-024 | Input/snapshot authoritative | Đã nghiệm thu | net / dsh-a/opencode-2 | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Authority/bind slot/dedup/window có evidence thực; không còn blocker server-loading #137. Chờ V1-002 done; cập nhật integration/evidence từ #117/#136/#153/#158. | [0009-wire-protocol-input-snapshot-codec.md](<../../docs/decisions/0009-wire-protocol-input-snapshot-codec.md>); `743a38af2a27`; [envelope.rs](<../../crates/boom-proto/src/envelope.rs>); [lib.rs](<../../crates/boom-net/src/lib.rs>); [lib.rs](<../../crates/boom-sync/src/lib.rs>); `2af995d8def9`; `e627112e5fe5`; `fb4ed043822a`; `ab4eb5a148a1`; `c8fbd31b7c8a`; `501f292e8108`; `4ed41a4155fa`; `ee779e43c9bc` |
| V1-025 | Prediction/reconciliation | Đã nghiệm thu | net / opencode | Claude | Xong: một consumer event theo (round_id, EventId) cho offline + mạng, cue gộp theo tick, không phát trùng khi reconcile (PR #170, đóng #163). | `eff7f94c258e` |
| V1-026 | Replay và state hash | Đã nghiệm thu | core / dsh-b/dsh-a | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Determinism/hash/schema và reject rules/content đã có; replay golden là baseline V1 mới, không phải oracle KR. Chờ V1-024 done; ghi proof hash sau #160, không chỉ golden trước đổi luật. | [main.rs](<../../crates/boom-replay/src/main.rs>); [replay_log.rs](<../../crates/boom-server/src/replay_log.rs>); `5bcafa0f3547`; `956d54b37fb4`; `ee779e43c9bc` |
| V1-027 | QA mạng 4 client | Đã nghiệm thu | qa / opencode-1/dsh1 (fix); Codex (QA) | Codex (code); dsh2 (nghiệm thu) | Đạt nghiệm thu V1-027 sau PR #230 và ma trận sáu tổ hợp 2/4 bot × seed 1/2/3. Giữ giới hạn Linux fixture, correction và reject trong QA_NETWORK; không coi đây là SLA hoặc fidelity KR. | `ea7c0070fbb0`; [QA_NETWORK.md](<../../docs/specification/QA_NETWORK.md>); `26435d0cbb23`; `b9c94279db27`; `42bf8be9e5ae`; `e6a1b27ceff2`; `fa1291baa638`; `e0dcdfc31e8e`; `ed16335c889e` |
| V1-028 | Lưu tài khoản và tùy chọn | Đã nghiệm thu | backend / opencode-1/opencode-2 | Claude/Codex | Done theo spec V1 hiện hành (rà #161). Ghi chú: Identity/tùy chọn bền đã có code/test; room/trận không cần phục hồi đúng scope. Chờ V1-001 done; giữ proof options #69/#77 và server identity/auth. | [0011-identity-persistence-log-v1.md](<../../docs/decisions/0011-identity-persistence-log-v1.md>); `06058297ddd9`; [sqlite.rs](<../../crates/boom-server/src/identity/sqlite.rs>); [options.rs](<../../crates/boom-client/src/options.rs>); `53aabbfb9023`; `c8b14a81e1e5`; `bf5ca50b338f`; `a3a78b0ec85b`; `ee779e43c9bc` |
| V1-029 | Log/health và vận hành test | Đã nghiệm thu | backend / dsh-a/opencode-1 | Claude | Xong: log JSON vòng đời round có round_id, lý do disconnect, logger trước load_content (PR #169, đóng #165). | [0011-identity-persistence-log-v1.md](<../../docs/decisions/0011-identity-persistence-log-v1.md>); `06058297ddd9`; [admin.rs](<../../crates/boom-server/src/admin.rs>); [observability.rs](<../../crates/boom-server/src/observability.rs>); `8ffc64ed2421`; `8759e037c647`; `f9c482861e33` |
| V1-030 | Map Village00 + dependency closure | Đã nghiệm thu | assets / opencode-4 | Claude/Codex | Xong: 0 unknown trong bundle Village00, test 4 spawn (PR #126, đóng #121). | [RULES_V1.md](<../../docs/specification/gameplay/RULES_V1.md>); `243d230fd7f0`; [closure.rs](<../../crates/boom-assets/src/closure.rs>); [sprite.rs](<../../crates/boom-assets/src/sprite.rs>); `9372f2074939`; `fd2e5d217251`; `282a051a5ce7`; [scene.rs](<../../crates/boom-client/src/scene.rs>); `abae61bba752`; `b3d4d068a3a7`; `4ec8acf9e133`; `891c7c7d3d76`; [BUNDLES.md](<../../docs/specification/assets/BUNDLES.md>); `2b86a87fb4f4` |
| V1-031 | Animation C00/C01 và bomb | Đã nghiệm thu | client / opencode-3/opencode-4 | Claude | Xong: event mạng (trap/dead/explosion/start) vào journal trình diễn, reset khi sang round (PR #170); frame/pivot theo R-085…R-089 (#100/#122). | [animation.rs](<../../crates/boom-assets/src/animation.rs>); `389b6ce78fc1`; [anim.rs](<../../crates/boom-client/src/anim.rs>); `5242e48f6127`; `40fb031334dd`; `93288f1f300a`; `f545f24d6570`; `eff7f94c258e` |
| V1-032 | UI luồng V1 bằng asset KR | Đã nghiệm thu | client / opencode-3 | Claude | Xong: --connect chạy UiModel<NetBackend>, Playing → ResultsAck → Room → trận 2, test tích hợp hai trận (PR #171, đóng #164). Layout KR (#150) là fidelity, không chặn luồng. | [closure.rs](<../../crates/boom-assets/src/closure.rs>); `4ec8acf9e133`; `c129a21815b8`; `395a7bcc14d6` |
| V1-033 | Input/scale/focus | Đã nghiệm thu | client / dsh-b/opencode-2 | Claude/Codex | Done: mặc định mũi tên+Space (#51), rebind qua options.toml (#69, #77), integer scale + nhả phím khi mất focus (#51); reviewer độc lập Codex; không có dependency. | [input.rs](<../../crates/boom-client/src/input.rs>); `0c5df165ffab`; [options.rs](<../../crates/boom-client/src/options.rs>); `bf5ca50b338f`; `a3a78b0ec85b` |
| V1-034 | Audio/volume | Đã nghiệm thu | client / dsh-b/dsh-a | Claude | Done theo scope V1; cue trapped sửa theo R204 ở PR213. Q015/issue196 còn phần RE khác, không nâng cả audio policy KR. | [audio.rs](<../../crates/boom-client/src/audio.rs>); [AUDIO_V1.md](<../../docs/specification/assets/AUDIO_V1.md>); `6dfdac10040c`; `aace7988256b`; `eff7f94c258e`; `2b86a87fb4f4`; [audio-v1.toml](<../../data/assets/audio-v1.toml>) |
| V1-035 | Việt hóa UI V1 | Đã nghiệm thu | client / Codex (nội dung/code); chủ dự án (duyệt ảnh) | dsh2 (độc lập) | Đã nghiệm thu V1-035. Theo dõi G5/runtime UX riêng; tên gốc C00/C01 tiếp tục RE #150, hiện dùng nhãn DESIGN theo scope. | [vi-v1.toml](<../../data/l10n/vi-v1.toml>); [l10n.rs](<../../crates/boom-content/src/l10n.rs>); `71d36a66cd64`; `82033ce560ab`; [l10n_review_v1.py](<../../tools/l10n_review_v1.py>); [UI_STRINGS_V1.md](<../../docs/specification/localization/UI_STRINGS_V1.md>); `6ad3e062f096`; [ui-vi-v1.toml](<../../data/assets/ui-vi-v1.toml>); [ui_localization.rs](<../../crates/boom-content/src/ui_localization.rs>); `43739e2fa6ab` |
| V1-036 | Build Windows/Linux/macOS | Chờ review | release / integrator/QA | Claude/Codex | Cần nghiệm thu V1-036 và cấu hình máy tối thiểu; QA dùng profile dev release=false, G2/G3/G5 chưa passed. | `942045876f2a`; `d6ca5799c8c9`; `303ab49a07c2`; [package-qa.json](<../../docs/evidence/measurements/rust-client-ux-2026-10-24/package-qa.json>); `796284d422ba` |
| V1-037 | Đối chiếu và báo sai khác | Đã nghiệm thu | qa / opencode-3 | Codex | Xong: oracle có provenance KR/VN tách golden, báo cáo sai khác tự sinh và kiểm trong cargo xtask ci (PR #184, đóng #180). Đo client gốc runtime vẫn hoãn (Q-009); claim RE mới (#146–#150) bổ sung oracle sau. | [VN_ENGINE_DIFF.md](<../../docs/research/findings/VN_ENGINE_DIFF.md>); `440bc8fd5648`; `b83a7cdcfcab`; [ORACLE_REPORT.md](<../../docs/research/ORACLE_REPORT.md>) |

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
