# Đối chiếu engine V1 với server giả lập VN

Baseline: `boom-rs` `origin/main` và server VN `/home/admin/boom-re` commit `9ac2075` (chỉ đọc).
Nguồn VN là nguồn tin cậy ngang KR theo [ADR 0002](../../decisions/0002-vn-kr-trusted-sources.md),
nhưng server giả lập **không phải server gốc**: phần client-derived và phần emulator tự đặt được tách
nhãn theo AGENTS §4. Trang này chỉ ghi đối chiếu; không đổi luật, profile hay engine.

## Phương pháp

- Giá trị engine: claim trong [RULES_V1](../../specification/gameplay/RULES_V1.md) và profile
  [`data/rules/v1-dev.toml`](../../../data/rules/v1-dev.toml) (dev). Scenario test dùng
  [`fixtures/rules/skeleton-test.toml`](../../../fixtures/rules/skeleton-test.toml) (60 Hz).
- Hành vi server VN: đọc `boom-re/server/src/*.rs`; mọi dòng là `file:dòng` trong commit trên.
- Nhãn: `vn_static`/`vn_runtime`/`emu_choice` theo AGENTS §4. `emu_choice` **không** phải bằng chứng
  luật gốc và không được dùng để nâng claim lên `confirmed`.
- `không so được` nghĩa server VN không mô phỏng cơ chế đó (client legacy tự làm hoặc server chỉ relay).

## Bảng đối chiếu

| Cơ chế V1 | Engine V1 (claim; profile) | Server VN (file:dòng) | Nguồn / trạng thái VN | Kết luận | Lý do |
|---|---|---|---|---|---|
| Di chuyển, tốc độ, bám góc | R-043/R-045 (raw 5→6 px/tick, candidate), R-075 (`33 px/s` design), R-046 (bám góc 12 px, `oss` candidate), R-047 (client tự mô phỏng, `vn_static` confirmed); profile dev `player.speed_px_per_s_per_level = 33` (`v1-dev.toml:30-36`) | Không có nhánh mô phỏng di chuyển trong dispatch C→S (`session.rs:160-425`); server chỉ nhận peer UDP cho P2P (`session.rs:378-387`, `session.rs:539-552`); `packets.rs:203-218` chỉ là toạ độ record ở sân chờ | `vn_static` + `vn_runtime` / confirmed (client P2P); giá trị px/s của engine là `design` | **Không so được** | Server VN không sở hữu vị trí người chơi; vị trí do client legacy P2P. Không suy px/s engine từ server này. |
| Nhịp tick và timer | R-013 (30 ms/bước, `vn_runtime` confirmed), R-074 (`33 Hz`, `design`); profile `sim.ticks_per_second = 33` (`v1-dev.toml:8-14`); round 120 s R-010 (`vn_runtime` confirmed) | Không có vòng tick mô phỏng; dùng `tokio::time::sleep` và biến môi trường `ROUND_TIME`/`BOMB_FUSE`/`MATCH_END` (`session.rs:36-43`, `session.rs:77-79`, `session.rs:467-568`) | `emu_choice` / candidate (thời lượng do emulator đặt); client VN đo 30 ms/bước là `vn_runtime` confirmed | **Khác/không so được** | Server VN là event-loop theo wall clock, không có tick cố định; 30 ms là đo client, không phải timer server giả lập. |
| Đặt bom và hạn mức bom | R-025 (1 bom khởi đầu, ItemBomb +1), R-032 (sức nổ chụp lúc đặt, `vn_static`+`vn_runtime` confirmed; `v1-dev.toml:69-74`); R-048 (bom chặn ô, design) | Nhận 111, cấp `bomb_id`, echo `power`/record; không kiểm cap hay ô (`session.rs:366-370`, `packets.rs:426-474`) | `vn_static`/`vn_runtime` confirmed cho trường `power` và ID server; policy cap/ô không có trong server | **Không so được** | Server giả lập không mô phỏng inventory/cap/collision; chỉ cấp ID và phát packet. |
| Fuse bom | R-021 gốc unknown; R-022 server giả lập 2,5 s (`emu_choice`/candidate); profile `bomb.fuse_ms = 2500` (`v1-dev.toml:47-53`, design) | `BOMB_FUSE` mặc định 2,5 s (`session.rs:38-39`, `session.rs:77`), sleep rồi gửi 151 (`session.rs:371-375`) | `emu_choice` / candidate | **Khớp giá trị, không phải bằng chứng gốc** | Cùng 2,5 s nhưng server VN tự đặt để client chạy; R-021 vẫn unknown. |
| Nổ, tầm lửa, chặn bởi khối/mép | R-030 (lửa dừng khối cứng, phá mềm, chain; `oss`+`web` candidate), R-033 (lan tối đa range, thứ tự id; design; `v1-dev.toml:55-60`) | Chỉ gửi `bomb_explode(&[id], 2)` khi hết fuse (`session.rs:374`); packet 151 không tính range/ô (`packets.rs:481-495`) | `emu_choice` / candidate | **Không so được** | Server VN không có lưới, không tính tia lửa hay vật cản; client tự vẽ nổ. Không dùng để kiểm R-030/R-033. |
| Dây chuyền nổ | R-033 (bom trong lửa nổ cùng tick, theo id; design) | Không có logic chain; mỗi 111 tạo timer độc lập và mỗi timer chỉ gửi id của nó (`session.rs:366-375`) | `emu_choice` / candidate | **Không so được** | Không có mô phỏng lửa/bom để chain. |
| Phá gạch và item rơi | R-031 (client C→S135, `vn_runtime` confirmed), R-050 (item giấu gửi trước trận, confirmed), R-067 (giấu item theo seed; design), R-068 (lửa không phá item nền; design), R-053 (35% design; `v1-dev.toml:83-100`) | Không có nhánh xử lý 135; chỉ gửi layout item cấu hình trước trận `session.rs:484-489`, `packets.rs:537-557`; item nền/giấu do env `FLOOR_ITEMS`/`ITEMS` (`session.rs:48-51`, `session.rs:85-93`) | `vn_runtime` confirmed cho luồng client báo 135; server giả lập không mô phỏng drop | **Không so được** | Server VN không phá gạch, không quyết drop; layout item là dữ liệu test do emulator đặt. |
| Item Bomb/Fire/Roller | R-025/R-057 (+1 bom, +1 tầm, +1 level; `vn_runtime` confirmed gốc, hiệu ứng V1 design; `v1-dev.toml:62-81`), R-054 (117→158, confirmed) | Nhận 117, broadcast 158 (`session.rs:360-363`), không áp chỉ số; danh sách item do env (`session.rs:484-489`) | `vn_runtime`/`vn_static` confirmed cho giao thức nhặt; server không mô phỏng hiệu ứng | **Không so được** | Client legacy tự áp hiệu ứng; server giả lập chỉ echo xác nhận. |
| Bẫy, cứu, chết | R-026 (packet 120/11→13 = 6,359 s, confirmed), R-028 (chạm địch làm chết, confirmed), R-034/R-035/R-036 (design V1; `v1-dev.toml:38-45`) | Nhận 120 subtype 11/13; chỉ đánh dấu `dead` khi client báo 13 (`session.rs:388-407`); không timer, không cứu/chọc | `vn_runtime` confirmed cho packet client; server giả lập là `emu_choice` | **Không so được** | Server tin client và không mô phỏng trap timer/va chạm; R-026 là khoảng packet quan sát, không phải timer server giả lập. |
| Thắng, hòa, hết giờ | R-010 120 s confirmed; R-061 client không báo hết giờ, server kết thúc (confirmed); SCOPE_V1 §4 FFA; `v1-dev.toml:23-28`; kết quả một lần | Kết thúc khi team còn ≤1 (một team thì all dead), xếp hạng sống=1/chết=2 (`session.rs:392-411`); hết `ROUND_TIME` xếp mọi người sống hạng 1 (`session.rs:557-566`); không có draw theo nghĩa V1 | `emu_choice` / candidate (team, rank, timeout); R-010/R-061 confirmed cho client | **Khác** | Policy team/rank/end của emulator khác FFA V1; không dùng làm bằng chứng cho hòa/winner của engine. |
| Disconnect | SCOPE_V1 §3: quá 10 s loại một lần, input neutral trong chờ; không nhập lại giữa vòng | Socket đóng → `leave_room` ngay (`session.rs:428-430`); không có timeout/neutral | `emu_choice` / candidate | **Khác** | Server giả lập xoá thành viên ngay, không mô phỏng V1-023. |

## Sai khác cần quyết định

- **Tốc độ/tick:** engine chọn 33 Hz và 33 px/s mỗi cấp (design; profile test 24 px/s/cấp) trong khi quan sát VN thô
  là 5 px/tick ở 30 ms (≈166,7 px/s) và còn candidate. Không thể coi hai giá trị là cùng đơn vị nếu chưa
  xác thực checksum/tick; cần Q-005/Q-004 đóng trước khi đổi profile.
- **Trap timer:** R-026 đo 6,359 s; profile V1 chọn 6,300 s (design). Sai lệch ~59 ms (~3,5 tick ở
  60 Hz) là khác biệt có thể kiểm bằng scenario; chưa đủ để sửa profile vì R-026 là khoảng packet,
  không phải timer nội bộ.
- **Fuse:** trùng 2,5 s chỉ là `emu_choice`, không nâng R-021 khỏi `unknown`.

Scenario `pending` tương ứng nằm ở
[`fixtures/scenarios/vn_engine_diff/speed_5px_per_30ms.toml`](../../../fixtures/scenarios/vn_engine_diff/speed_5px_per_30ms.toml)
và [`trap_death_6359ms.toml`](../../../fixtures/scenarios/vn_engine_diff/trap_death_6359ms.toml).
Chúng ghi kỳ vọng theo nguồn VN và **không** phải acceptance của engine hiện tại.

## Giới hạn

- Không so pixel/âm thanh với client (cần runtime; ngoài phạm vi V1-037 phần 1).
- Server VN không có mô phỏng lưới/bom/item/trap; các dòng "không so được" không phải bằng chứng
  engine sai, chỉ là server giả lập thiếu mô hình.
- Không sửa RULES_V1/profile/engine trong task này.
