# Báo cáo đối chiếu oracle KR/VN ↔ engine (V1-037)

<!-- SINH TỘNG BỞI `cargo xtask oracle` — đừng sửa tay; chạy lại lệnh để cập nhật. -->

Báo cáo đối chiếu **kỳ vọng từ nguồn gốc** (client KR 00967 / VN VBNB_116) với **kết quả
engine V1 đo được**. Mỗi oracle là một file trong [`fixtures/oracle/`](../../fixtures/oracle);
metadata (nguồn, claim, bằng chứng, đại lượng, đơn vị, kỳ vọng, dung sai) là phần bắt buộc
của file đó — thiếu thì validator báo lỗi.

## Cách đọc

- **Nguồn / trạng thái**: nguồn gốc của kỳ vọng và mức bằng chứng theo [AGENTS §4](../../AGENTS.md#4-bằng-chứng).
- **Đại lượng / đơn vị**: tên và đơn vị theo bảng chọn của
[RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013) —
ADR 0013 chỉ so **cùng đại lượng, cùng đơn vị**.
- **Đơn vị là bắt buộc**: mỗi phép đo có một đơn vị cố định (`rule` lấy đơn vị trong
profile, `scenario` là `px/s` hoặc `ms`, `distribution` là `permille`); oracle khai khác là
metadata sai ⇒ validator báo lỗi, không quy đổi âm thầm.
- **Kỳ vọng**: giá trị của nguồn gốc, kèm dung sai.
- **Kết quả engine**: giá trị engine đo được trong lần chạy này.
- **Cỡ mẫu**: với phép đo phân phối, số khối vỡ **do engine đếm** (`block_destroyed`) — đây
là số mẫu thật mà dung sai thống kê bám theo, không phải con số khai trong metadata.
- **Trạng thái**: `khớp` (trong dung sai) · `lệch (đã ghi nhận D-xx/G-xx)` (sai khác đã có trong
[SCOPE_V1 §10](../planning/SCOPE_V1.md#10-sai-khác-có-chủ-ý-so-với-client-gốc)) ·
`lệch (chưa ghi nhận)` (**lỗi** của `cargo xtask ci`) · `chưa có số` (nguồn gốc chưa có giá trị
để so, nên chỉ báo khoảng trống).

Quy tắc CI: **sai khác chưa ghi nhận ở SCOPE_V1 §10 là lỗi**; sai khác đã ghi nhận và
kỳ vọng chưa có số chỉ được báo ở đây. Báo cáo này **không** nâng tầng bằng chứng của
claim: nó chỉ cho biết engine có khớp nguồn gốc hay không.

## Kết quả

| Oracle | Đại lượng | Nguồn | Kỳ vọng | Kết quả engine | Trạng thái | Ghi chú |
|---|---|---|---|---|---|---|
| O-001 | round_duration (ms) | vn_runtime / confirmed (R-010, R-110) | 120000 | 120000 | khớp | profile data/rules/v1-dev.toml |
| O-002 | tile_size (px) | vn_static / confirmed (R-001, R-110) | 40 | 40 | khớp | profile data/rules/v1-dev.toml |
| O-003 | item_bomb_capacity_delta (bomb) | vn_runtime / confirmed (R-025, R-111) | 1 | 1 | khớp | profile data/rules/v1-dev.toml |
| O-004 | item_fire_range_delta (cell) | vn_runtime / confirmed (R-025, R-111) | 1 | 1 | khớp | profile data/rules/v1-dev.toml |
| O-005 | drop_item_weight_bomb (weight) | vn_static / confirmed (R-051, R-112) | 3650 | 3650 | khớp | profile data/rules/v1-dev.toml |
| O-006 | drop_item_weight_fire (weight) | vn_static / confirmed (R-051, R-112) | 2800 | 2800 | khớp | profile data/rules/v1-dev.toml |
| O-007 | drop_item_weight_roller (weight) | vn_static / confirmed (R-051, R-112) | 2000 | 2000 | khớp | profile data/rules/v1-dev.toml |
| O-008 | drop_item_weight_share_bomb (permille) | vn_static / confirmed (R-051, R-112) | 432 ± 40 | 418 | khớp | khối khai 4096 · engine vỡ 4096 khối → 1450 item rơi · seed 20261022 · profile data/rules/v1-dev.toml |
| O-009 | drop_item_weight_share_fire (permille) | vn_static / confirmed (R-051, R-112) | 331 ± 40 | 335 | khớp | khối khai 4096 · engine vỡ 4096 khối → 1450 item rơi · seed 20261022 · profile data/rules/v1-dev.toml |
| O-010 | drop_item_weight_share_roller (permille) | vn_static / confirmed (R-051, R-112) | 237 ± 40 | 246 | khớp | khối khai 4096 · engine vỡ 4096 khối → 1450 item rơi · seed 20261022 · profile data/rules/v1-dev.toml |
| O-011 | trapped_player_speed (px/s) | web / candidate (R-029) | chưa có số | 16 | chưa có số (gap G-08) | kịch bản fixtures/scenarios/movement/trapped_moves_slowly.toml · profile skeleton-test · 60 tick ở 60 Hz |
| O-012 | player_speed (px/s) | vn_runtime / candidate (R-043, R-045, R-114) | 167 ± 5 | 120 | lệch (đã ghi nhận G-06) | kịch bản fixtures/scenarios/vn_engine_diff/speed_5px_per_30ms.toml · profile skeleton-test · 61 tick ở 60 Hz |
| O-013 | trap_to_death (ms) | vn_runtime / confirmed (R-026, R-115) | 6359 ± 5 | 6300 | lệch (đã ghi nhận G-05) | kịch bản fixtures/scenarios/vn_engine_diff/trap_death_6359ms.toml · profile skeleton-test · 378 tick ở 60 Hz |

## Sai khác đã ghi nhận (SCOPE_V1 §10)

- **G-06** — Tốc độ nhân vật. Oracle O-012: kỳ vọng `167 ± 5`, engine `120`.
- **G-05** — Nhốt → chết. Oracle O-013: kỳ vọng `6359 ± 5`, engine `6300`.

## Khoảng trống còn mở

Nguồn gốc của các oracle dưới đây **chưa có giá trị** để so (thường là `candidate`),
nên engine chỉ ghi kết quả đo, không kết luận khớp hay lệch. Khoảng trống đã ghi ở
SCOPE_V1 §10 và vẫn mở ở [QUESTIONS](QUESTIONS.md).

- **G-08** — Người bị nhốt có di chuyển không. Oracle O-011: engine `16`.

## Bằng chứng từng oracle

- **O-001** — Thời hạn vòng 120 s: feature V1-037, nguồn vn_runtime / confirmed, claim R-010, R-110. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: R-011: map có cờ thì byte thời gian từ server ghi đè giá trị này; oracle chỉ kiểm giá trị mặc định của profile.
- **O-002** — Cỡ ô 40 px: feature V1-037, nguồn vn_static / confirmed, claim R-001, R-110. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md), `0x735DA9`.
- **O-003** — ItemBomb +1 bom: feature V1-037, nguồn vn_runtime / confirmed, claim R-025, R-111. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
- **O-004** — ItemFire +1 tầm: feature V1-037, nguồn vn_runtime / confirmed, claim R-025, R-111. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
- **O-005** — Trọng số rơi item Bomb 3650: feature V1-037, nguồn vn_static / confirmed, claim R-051, R-112. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: R-052: vai trò bảng ItemAttr còn unknown — oracle kiểm số bảng engine dùng, không khẳng định đây là xác suất drop gốc.
- **O-006** — Trọng số rơi item Fire 2800: feature V1-037, nguồn vn_static / confirmed, claim R-051, R-112. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: R-052: vai trò bảng ItemAttr còn unknown — oracle kiểm số bảng engine dùng, không khẳng định đây là xác suất drop gốc.
- **O-007** — Trọng số rơi item Roller 2000: feature V1-037, nguồn vn_static / confirmed, claim R-051, R-112. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: R-052: vai trò bảng ItemAttr còn unknown — oracle kiểm số bảng engine dùng, không khẳng định đây là xác suất drop gốc.
- **O-008** — Phân phối rơi item — Bomb ≈ 432‰: feature V1-037, nguồn vn_static / confirmed, claim R-051, R-112. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: Tỷ trọng kỳ vọng tính từ bảng trọng số VN (3650:2800:2000); xác suất rơi gốc vẫn unknown (R-052, R-053). Dung sai ±40‰ theo cỡ mẫu 4096 khối khai (≈1450 item rơi, σ ≈ 13‰ nên ±40‰ ≈ 3σ); số khối vỡ thật do engine báo trong báo cáo.
- **O-009** — Phân phối rơi item — Fire ≈ 331‰: feature V1-037, nguồn vn_static / confirmed, claim R-051, R-112. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: Tỷ trọng kỳ vọng tính từ bảng trọng số VN (3650:2800:2000); xác suất rơi gốc vẫn unknown (R-052, R-053).
- **O-010** — Phân phối rơi item — Roller ≈ 237‰: feature V1-037, nguồn vn_static / confirmed, claim R-051, R-112. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: Tỷ trọng kỳ vọng tính từ bảng trọng số VN (3650:2800:2000); xác suất rơi gốc vẫn unknown (R-052, R-053).
- **O-011** — Người bị nhốt có di chuyển không: feature V1-037, nguồn web / candidate, claim R-029. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: Giữ oracle web R-029 không có scalar để đối chiếu. Native VN R-161 đã xác nhận slow movement; R-162 chọn17px/s DESIGN. Engine đo displacement nguyênpx nên báo16px/s với profile60Hz283mpx/tick; không gọi đây là literalnative17 hoặc tự cho nativeoracle đạt.
- **O-012** — Tốc độ người chơi theo quan sát VN: feature V1-037, nguồn vn_runtime / candidate, claim R-043, R-045, R-114. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md).
  - Ghi chú: R-043 là quan sát thô một run (checksum chưa xác thực) nên dung sai ±5 px/s; chênh lệch đo được hiện nằm ở G-06 và chờ đo client gốc theo Q-009.
- **O-013** — Thời gian nhốt → chết theo khoảng packet VN: feature V1-037, nguồn vn_runtime / confirmed, claim R-026, R-115. Bằng chứng: [`docs/specification/gameplay/RULES_V1.md`](../specification/gameplay/RULES_V1.md), [`docs/evidence/measurements/vn-timing-2026-10-19/trap-packets.csv`](../evidence/measurements/vn-timing-2026-10-19/trap-packets.csv).
  - Ghi chú: R-026 đo khoảng packet, không phải timer nội bộ; V1 chọn timer 4 000 ms theo BnB (R-115), profile test dùng 6 300 ms. Chênh lệch nằm ở G-05.

## Cách sinh lại

- `cargo xtask oracle` — sinh lại file này từ `fixtures/oracle/*.toml`.
- `cargo xtask oracle --check` — kiểm file này còn khớp kết quả đo và không có sai khác
chưa ghi nhận; `cargo xtask ci` chạy lệnh này.
- Oracle là **đối chiếu**, không phải bằng chứng mới: nó không nâng tầng bằng chứng của claim
và không thay thế kịch bản nghiệm thu trong `fixtures/scenarios/`.
