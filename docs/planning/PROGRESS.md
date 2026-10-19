# Tiến độ so với catalog KR

> GENERATED — không sửa tay. Chạy `python3 tools/kr_catalog.py`.

Baseline mẫu số: **`KR-00967-V1-scope-1`**.
Nguồn: [features.json](../specification/features.json) và [execution.json](execution.json).
Kế hoạch thực hiện: [REWRITE_PLAN](REWRITE_PLAN.md); danh mục: [FEATURE_CATALOG](../specification/FEATURE_CATALOG.md).

## Số mục đã nghiệm thu

| Phạm vi | Done / tổng | Tỷ lệ số mục |
|---|---:|---:|
| Release 1 | 0 / 37 | 0.00% |
| Toàn catalog cần triển khai | 0 / 112 | 0.00% |

Có **2 mục drop_legacy** hiển thị riêng trong CSV, không cộng vào done hoặc mẫu số.
Đây là tỷ lệ **số mục nghiệm thu**, không phải % công sức, thời gian hay fidelity KR.
Không chia điểm cho in_progress/in_review/blocked. Scope đổi phải đổi revision và giải thích mẫu số.

| Trạng thái | V1 | Toàn catalog cần triển khai |
|---|---:|---:|
| Chưa làm | 19 | 94 |
| Đang làm | 4 | 4 |
| Bị chặn | 0 | 0 |
| Chờ review | 14 | 14 |
| Đã nghiệm thu | 0 | 0 |

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
| V1-001 | Auth tài khoản dự án | Chưa làm | backend / chưa giao | — | — | — |
| V1-002 | Handshake phiên bản/rules/content | Chưa làm | net / chưa giao | — | Issue #14 (đợt 1); nhận việc theo TEAM_WORKFLOW §5. | — |
| V1-003 | Một kênh và sảnh danh sách phòng | Chưa làm | backend / chưa giao | — | — | — |
| V1-004 | Tạo phòng công khai | Chờ review | backend / dsh-3 | Codex | Máy trạng thái phòng thuần đã merge (PR #23); đặt done khi auth/sảnh (V1-001…003) và server bọc nó done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c` |
| V1-005 | Vào/rời phòng | Chờ review | backend / dsh-3 | Codex | Máy trạng thái phòng thuần đã merge (PR #23); đặt done khi auth/sảnh (V1-001…003) và server bọc nó done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c` |
| V1-006 | Chủ phòng và kick | Chờ review | backend / dsh-3 | Codex | Máy trạng thái phòng thuần đã merge (PR #23); đặt done khi auth/sảnh (V1-001…003) và server bọc nó done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c` |
| V1-007 | Chọn C00/C01 | Đang làm | client / dsh-3 | Codex | Phần server (chọn nhân vật, hủy ready) có ở PR #23; còn UI chọn C00/C01 phía client. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c` |
| V1-008 | Ready/cancel/start | Chờ review | backend / dsh-3 | Codex | Máy trạng thái phòng thuần đã merge (PR #23); đặt done khi auth/sảnh (V1-001…003) và server bọc nó done. | [lib.rs](<../../crates/boom-room/src/lib.rs>); `d0a8571a229c` |
| V1-009 | Chat trong phòng chờ | Chưa làm | backend / chưa giao | — | — | — |
| V1-010 | Start barrier và spawn | Chưa làm | net / chưa giao | — | — | — |
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
| V1-022 | Kết quả và chơi vòng tiếp | Chưa làm | backend / chưa giao | — | — | — |
| V1-023 | Disconnect/timeout | Chưa làm | net / chưa giao | — | — | — |
| V1-024 | Input/snapshot authoritative | Đang làm | net / dsh-4 | Codex | Thiết kế ADR 0009 đã merge (PR #24); còn triển khai boom-proto/boom-net và authority (đợt 2). | [0009-wire-protocol-input-snapshot-codec.md](<../../docs/decisions/0009-wire-protocol-input-snapshot-codec.md>); `743a38af2a27` |
| V1-025 | Prediction/reconciliation | Chưa làm | net / chưa giao | — | — | — |
| V1-026 | Replay và state hash | Chưa làm | core / chưa giao | — | — | — |
| V1-027 | QA mạng 4 client | Chưa làm | qa / chưa giao | — | — | — |
| V1-028 | Lưu tài khoản và tùy chọn | Chưa làm | backend / chưa giao | — | — | — |
| V1-029 | Log/health và vận hành test | Chưa làm | backend / chưa giao | — | — | — |
| V1-030 | Map Village00 + dependency closure | Đang làm | assets / dsh-1 | Codex | Phân loại ô + World từ Village00 KR đã merge (PR #25); còn closure render (theme/tile/object/preview, animation state). | [RULES_V1.md](<../../docs/specification/gameplay/RULES_V1.md>); `243d230fd7f0` |
| V1-031 | Animation C00/C01 và bomb | Chưa làm | client / chưa giao | — | — | — |
| V1-032 | UI luồng V1 bằng asset KR | Chưa làm | client / chưa giao | — | Issue #15 (đợt 1); nhận việc theo TEAM_WORKFLOW §5. | — |
| V1-033 | Input/scale/focus | Chưa làm | client / chưa giao | — | — | — |
| V1-034 | Audio/volume | Chưa làm | client / chưa giao | — | — | — |
| V1-035 | Việt hóa UI V1 | Chưa làm | client / chưa giao | — | Issue #15 (đợt 1); nhận việc theo TEAM_WORKFLOW §5. | — |
| V1-036 | Build Windows/Linux/macOS | Chưa làm | release / chưa giao | — | — | — |
| V1-037 | Đối chiếu và báo sai khác | Chưa làm | qa / chưa giao | — | — | — |

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
