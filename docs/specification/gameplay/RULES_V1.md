# Luật trận V1 — claim, bằng chứng và giá trị được chọn

Cập nhật 2026-10-19. Nguồn chuẩn cho **claim luật chơi V1**; giá trị engine dùng nằm ở
[`data/rules/v1-dev.toml`](../../../data/rules/v1-dev.toml), mỗi giá trị trỏ claim ID ở đây.
Nhãn nguồn/trạng thái theo [AGENTS §4](../../../AGENTS.md#4-bằng-chứng). VN và KR ngang nhau
([ADR 0002](../../decisions/0002-vn-kr-trusted-sources.md)).

Locator viết tắt: `oc_e` = `/home/admin/boom-re/docs/oc_e-game.md` (main `4ccc890`),
`items` = `/home/admin/boom-re/docs/items.md`, `MAPS` = [MAPS](../assets/MAPS.md),
`OSS` = [OSS_REIMPLEMENTATIONS](../../research/findings/OSS_REIMPLEMENTATIONS.md),
`NAMU` = [WEB_RULES_NAMU](../../research/findings/WEB_RULES_NAMU.md).

## 1. Lưới và map

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-001 | Một ô = 40 px | 40 px | `vn_static` / confirmed | MAPS §1.1 (`idiv 0x28` ở `0x735DA9`) |
| R-002 | Village00 KR: 15×13 ô, version 1000, 2 dword/ô `(tileId, objId)`, `-1` = trống | — | `kr_static` / confirmed | Probe 2026-10-19 trên payload `bd34c418…` |
| R-003 | Header map KR dài hơn VN 4 byte: spawn bắt đầu ở +80 (VN +76); count spawn ở +20 cả hai | +80 / +76 | `kr_static` + `vn_static` / confirmed | So Village00 KR với `boom-re/assets/fx/Map/Village00.map` |
| R-004 | Village00 KR và VN cùng bố cục ô; ID KR = ID VN + 937 (tile) hoặc +937/+938 (obj) | — | `kr_static` + `vn_static` / confirmed | Cùng phép so như R-003; ID là khóa tra bảng runtime theo build |
| R-005 | Bốn spawn KR `(13,11) (1,11) (1,1) (13,1)` đều là ô objId `-1` | — | `kr_static` / confirmed | `kr_v1_content.json` + probe lưới |
| R-006 | Viền ngoài Village00 là obj `1958` (KR) / `1020` (VN) = `Tile00_Static/Tree00` ở mọi ô viền; nghĩa "không đi qua" | — | `kr_static` + `vn_static` / confirmed | ObjectTable KR index 1958 và VN index 1020 cùng tên `Tile00_Static/Tree00`; TileInfoList `Data[0]=1` (Static); client VN không đi được vì viền/cây chặn spawn (`oc_e` §5a-3); bảng `data/maps/village00-kr.toml` |
| R-007 | Mỗi spawn có ít nhất hai ô `objId -1` liền kề theo hình chữ L; obj `-1` để lộ tile `Tile00_Ground/*` và là ô đi được | — | `kr_static` + `vn_static` / confirmed | Lưới KR/VN + ObjectTable: bốn spawn có tile `Tile00_Ground/*` và obj `-1`; TileInfoList `Data[0]=0` (Ground); test `every_spawn_has_two_adjacent_empty_cells_and_an_escape` (Village00 KR) |
| R-008 | Item giấu trong gạch chỉ đặt được ở ô loại `CGOTileFixedBox` | — | `vn_runtime` / confirmed | oc_e §5a-5d (S→C 553) |
| R-009 | Neo hiển thị nhân vật: x = cột·40, y = hàng·40 + 39 | px | `vn_runtime` / confirmed | oc_e §5a-5c (C→S 120) |
| R-069 | ID tile và ID obj trong file `.map` là **chỉ số entry** của `fx/ObjectTable/ObjectTable`; tên entry `<TileSet>_<Lớp>/<Tên>` ⇒ sprite của ô nằm ở entry `<TileSet>/<Tên>` (pack KR `FxTile`, pack VN `fx`); lớp trong tên chỉ quyết định va chạm | — | `kr_static` + `vn_static` / confirmed | ObjectTable KR index 1958 = `Tile00_Static/Tree00` (entry `FxTile/Tile00/Tree00`), index 1942 = `Tile00_Ground/Grass00`; tile ID 1942 (KR) và 1005 (VN) cùng ra `Tile00_Ground/Grass00`, obj 1958 (KR) và 1020 (VN) cùng ra `Tile00_Static/Tree00`; `crates/boom-assets/src/closure.rs` (`resolve_sprite`) |
| R-070 | Map V1 `Village00` của pack KR: bộ entry theme/preview/BGM là `FxMap/Theme/Village`, `FxMap/Etc/Village00_mm`, `FxSound/MP3/Village`; tile set của map là tiền tố `<TileSet>` trong tên ObjectTable (`Tile00` cho Village00) | — | `kr_static` / confirmed | `kr_v1_content.json` (role `theme`/`preview`/`music`, SHA-256 `d2226fdd…`, `950b4576…`, `7a34054f…`); pack KR 00967 `FxMap`: 142 thư mục `Theme/*`, trong đó chỉ `Village` là tiền tố của `Village00`; `Etc/*` có đúng 1.151 entry `<Map>_mm`, khớp 1.151 map trong `Map/*`; ObjectTable KR `Tile00_*`; manifest `crates/boom-assets/src/closure.rs` |
| R-071 | Quy tắc chọn theme tổng quát: theme của map là thư mục `Theme/*` **dài nhất** làm tiền tố tên map sau khi bỏ `NXCL_`/`WS_`/`WinningStreak_`; entry theme là `FxMap/Theme/<Theme>`, preview `FxMap/Etc/<Map>_mm`, BGM `FxSound/MP3/<Theme>`; khớp không thì legacy server dùng `ThemeDefault` (pack KR có `Theme/ThemeDefault`) | — | `vn_static` + `vn_runtime` / candidate | [VN_LEGACY_SERVER_REFERENCE](../../research/findings/VN_LEGACY_SERVER_REFERENCE.md) §1 hàng "Map/theme" (`boom-re/server/src/maps.rs` `all_battle_maps`); 9 map VN chạy được trong client ([VN-B04](../../research/findings/VN_BATTLE_RUNTIME_REFERENCE.md)) chỉ là bằng chứng gián tiếp; **giới hạn đo trên pack KR 00967**: rule khớp 914/1.151 map, 237 map không có thư mục theme nào làm tiền tố, và `MP3/<theme>` chỉ tồn tại cho 32/80 theme dùng được ⇒ phần BGM chỉ chắc ở `Village`; chưa có trace client xác nhận chọn theme ngoài `Village00` (Q-002) |

## 2. Trận và thời gian

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-010 | Giới hạn thời gian mặc định của client: 120 s, đếm từ lúc bắt đầu lượt | 120 000 ms | `vn_static` + `vn_runtime` / confirmed | oc_e "Hết giờ" (`ctx+490518`) |
| R-011 | Map có cờ thì byte thời gian từ server ghi đè R-010 | — | `vn_static` / confirmed | oc_e "Hết giờ" (`sub_71A9EC`) |
| R-012 | Header map có cặp `60000, 22000` (ms?) ở 574 file VN và ở Village00 KR | — | `vn_static` + `kr_static` / unknown | MAPS §1.1; nghĩa chưa rõ |
| R-013 | Tần số bước mô phỏng gốc: **30 ms/bước** (đo 33,377/33,436 bước/s, tức 29,96/29,91 ms/bước) | 30 ms (±0,1 ms) | `vn_runtime` / confirmed | Bộ đếm `ctx+0x1DF190` tăng tại 0x6ED316, `ctx = *(stage+0x547C)`; 2 run, 24+12 mẫu, cửa sổ 11,505/5,503 s; [tick-runs.csv](../../evidence/measurements/vn-timing-2026-10-19/tick-runs.csv); cửa sổ 2,5 s 32,98–34,00 do lượng tử mẫu 0,5 s; Q-004 |
| R-014 | Tick server remake: 30 Hz | 30 Hz | `oss` / candidate | OSS §2 |
| R-074 | V1 chọn `sim.ticks_per_second = 33` (xấp xỉ nguyên 1000/30 ≈ 33,3 Hz) cho profile dev | 33 Hz | `design` / design | R-013; sai lệch ≈1%; Q-004; `data/rules/v1-dev.toml` |

## 3. Bom, lửa, nhốt, chết

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-020 | Thời điểm nổ do **server** quyết (S→C 151); client không tự nổ | — | `vn_runtime` / confirmed | oc_e §5a (bom) |
| R-021 | Fuse bom thường gốc | — | unknown | Server gốc thất lạc |
| R-022 | Server giả lập dùng fuse 2,5 s | 2 500 ms | `emu_choice` / candidate | `BOMB_FUSE` trong `boom-re/server` |
| R-023 | Remake dùng fuse 2,5 s, lửa 0,5 s | 2 500 / 500 ms | `oss` / candidate | OSS §2 |
| R-024 | Bom bị ném/đá nổ sau 3 s | 3 000 ms | `web` / candidate | `boomkr-re/docs/game-spec-web.md` §2 |
| R-025 | Tầm nổ khởi đầu 1; mỗi ItemFire +1 tầm; mỗi ItemBomb +1 số bom đặt cùng lúc | — | `vn_runtime` / confirmed | oc_e §5a-5d (hiệu ứng bị động) |
| R-026 | Bị nước trúng → bị nhốt; khoảng packet `120/11 → 120/13` là **6,359 s** (ba lần: 6,355/6,364/6,358; biên độ 9 ms) | 6 359 ms ± 5 ms (biên độ) | `vn_runtime` / confirmed | [trap-packets.csv](../../evidence/measurements/vn-timing-2026-10-19/trap-packets.csv), [trap-packets.txt](../../evidence/measurements/vn-timing-2026-10-19/trap-packets.txt); run3 chỉ còn log worker, run4/5 còn raw `server.log`; đây là khoảng packet gồm xử lý/gửi, chưa phải timer nội bộ; `210 tick`/literal 6300 chưa chứng minh, Q-006 |
| R-076 | V1 chọn `player.trap_to_death_ms = 6300` từ khoảng packet đo được; `210 tick` chỉ là giả thuyết, không phải hằng số đã đọc | 6 300 ms | `design` / design | R-026; Q-006; [trap-packets.csv](../../evidence/measurements/vn-timing-2026-10-19/trap-packets.csv); `data/rules/v1-dev.toml` |
| R-027 | Remake: nhốt 4 s rồi chết | 4 000 ms | `oss` / candidate | OSS §2; mâu thuẫn R-026, R-026 thắng |
| R-028 | Địch chạm người bị nhốt → người bị nhốt chết | — | `vn_runtime` / confirmed | VN-B01 |
| R-029 | Bị nhốt: không đặt bom, đi rất chậm | — | `web` / candidate | NAMU §2.1 |
| R-030 | Lửa dừng ở khối cứng, phá khối mềm; bom trong lửa kích nổ dây chuyền | — | `oss` + `web` / candidate | OSS §5; client VN tự vẽ vụ nổ (oc_e) |
| R-031 | Vỡ gạch: client legacy tự tính và báo C→S 135 | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-032 | Sức nổ được chụp lúc đặt bom (client gửi `power` trong 111); ID bom do server cấp (152) | — | `vn_static` + `vn_runtime` / confirmed | [Legacy server](../../research/findings/VN_LEGACY_SERVER_REFERENCE.md) §1 |
| R-033 | Lửa phủ ô đặt bom + tối đa `range` ô mỗi hướng; khối cứng/mép lưới chặn, khối mềm đầu tiên có lửa rồi dừng. Bom đến hạn hoặc bị lửa chạm nổ cùng tick, xử lý theo `id` tăng dần, mỗi bom một lần; nhiều lửa cùng ô giữ `until` lớn nhất, hòa chọn `source` id nhỏ | — | `design` (từ R-030) | Task #5, `systems/explosion.rs`; R-030 |
| R-034 | V1: "chạm" = hai người chơi cùng một ô lưới tại bước life; chỉ người `Alive` làm vỡ bong bóng, hai người `Trapped` chạm nhau không đổi trạng thái | — | `design` / design | SCOPE_V1 §4; issue #8 (cùng ô); `systems/life.rs` |
| R-035 | V1: trong một tick, người `Alive` đứng trên lửa bị nhốt trước; chỉ người đã `Trapped` từ đầu bước life mới xét chết; nếu vừa hết `trap_ticks` vừa bị chạm thì `player_died.by` là người chạm; nhiều người `Alive` cùng ô thì chọn id nhỏ nhất | — | `design` / design | SCOPE_V1 §4; ARCHITECTURE §5.2; issue #8 |
| R-036 | Người bị nhốt không di chuyển trong V1 (đứng tại chỗ tới khi chết) | — | `design` / design | SCOPE_V1 §4; R-029 (`web` / candidate) nói "đi rất chậm" nhưng chưa có px/s; quyết định cho V1-012 |

Hai claim `design` R-034/R-035 chỉ định nghĩa cách engine V1 hiện thực hóa "chạm" và thứ tự trong
tick; chưa có bằng chứng client KR/VN cho trường hợp biên. Khi có trace, thay bằng claim `confirmed`
hoặc mở câu hỏi ở [QUESTIONS](../../research/QUESTIONS.md).

## 4. Nhân vật và tốc độ

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-040 | BomberAttr KR C00: f2–f7 = `5,7,1,10,1,7` | — | `kr_static` / confirmed | `boomkr-re/data/kr/bomber_attr.json` |
| R-041 | BomberAttr KR C01: f2–f7 = `4,8,2,7,1,9` | — | `kr_static` / confirmed | Như trên |
| R-042 | f2–f3 = speed min/max, f4–f5 = số bom min/max, f6–f7 = tầm min/max (cấp) | — | `kr_static` + `web` / candidate | `boomkr-re/docs/kr-data.md` §23; OSS §4 khớp 5/6 giá trị; chưa đối chiếu VN vì R-043 còn candidate |
| R-043 | Quan sát thô tốc độ C00: 5 px/tick base; sau Roller 6 px/tick (một run, checksum x/y chưa xác thực) | 5 → 6 px/tick (raw) | `vn_runtime` / candidate | [speed-observations.csv](../../evidence/measurements/vn-timing-2026-10-19/speed-observations.csv); item3 117/158 22:31:20; `xok/yok=false`, decoder cần xác minh; Q-005 |
| R-044 | Remake: 1 cấp tốc độ = 24 px/s | 24 px/s | `oss` / candidate | OSS §3; chưa đối chiếu được VN vì R-043 còn candidate |
| R-045 | Roller: +1 cấp tốc độ | +1 cấp (raw 5→6 px/tick, một run) | `web` + `vn_runtime` / candidate | NAMU §1; [speed-item3-changes.txt](../../evidence/measurements/vn-timing-2026-10-19/speed-item3-changes.txt); một run, checksum chưa xác thực; Q-005 |
| R-075 | V1 chọn `player.speed_px_per_s_per_level = 33` (xấp xỉ nguyên 1000/30 ≈ 33,3 px/s) cho profile dev | 33 px/s | `design` / design | R-043/R-045 (candidate); sai lệch ≈1%; Q-005; `data/rules/v1-dev.toml` |
| R-046 | Bám góc khi rẽ trong 12 px | 12 px | `oss` / candidate | OSS §5 |
| R-047 | Di chuyển không có gói C→S; client tự mô phỏng | — | `vn_static` / confirmed | oc_e §5a |
| R-048 | Bom đang tồn tại chặn mọi người đi vào ô của nó | — | `design` / design | SCOPE_V1 §4 ("bom chặn/đi ra"); chưa đối chiếu collision client gốc |
| R-049 | Grace một chiều: người đang đứng trên bom được đi ra; rời ô thì mất quyền, không được quay lại | — | `design` / design | SCOPE_V1 §4; V1-012 |

## 5. Item rơi và nhặt

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-050 | Item giấu trong gạch được server gửi **trước trận** (553), không quyết lúc vỡ | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-051 | ItemAttr trọng số: Bomb 3650, Fire 2800, Roller 2000; client nạp vào bảng ngẫu nhiên 0x8000 ô | — | `vn_static` / confirmed | items (loader `sub_89F3DE`); thay cách đọc "giá" ở MAPS §2 |
| R-052 | Ai dùng bảng R-051 để rơi item (client hay server) | — | unknown | Q-007 |
| R-053 | Xác suất một gạch có item | — | unknown | Q-007; OSS dùng 0,35 (`oss` / candidate) |
| R-054 | Nhặt: client xin (117), server xác nhận (158), item mất sau xác nhận | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-055 | Bảng rơi gốc của Village00 chỉ có 6 loại Lucci | — | `kr_static` / confirmed | `kr_v1_content.json`; V1 thay bằng Bomb/Fire/Roller (DESIGN, SCOPE_V1 §2) |
| R-056 | V1: người còn sống (`Alive`) có tâm trong ô có item trên nền thì nhặt; duyệt người chơi theo id tăng dần nên hai người cùng ô trong cùng tick chỉ người id nhỏ nhận; item mất khỏi nền sau khi nhặt và phát `item_picked` | — | `design` (từ R-054) | Task #7, `systems/pickup.rs`; R-054; SCOPE_V1 §4 |
| R-057 | V1: ItemBomb +1 số bom, ItemFire +1 tầm nổ, ItemRoller +1 cấp tốc độ; lượng lấy từ profile (`item.*_delta`), kẹp theo trần chỉ số; bom đã đặt giữ tầm chụp lúc đặt | — | `design` (từ R-025, R-032, R-045) | Task #7, `systems/pickup.rs`; R-025; R-032; R-045 |
| R-058 | V1: chỉ số đã ở trần thì không nhặt — item vẫn nằm trên nền, không áp hiệu ứng, không phát event; nhặt là atomic (item chỉ mất khi hiệu ứng áp được) | — | `design` | Task #7; R-054 (item mất sau khi server xác nhận); SCOPE_V1 §4; Q-007 |
| R-059 | V1: người bị nhốt (`Trapped`) hoặc đã chết không nhặt item; người bị nhốt đứng yên nên cũng không tới ô item | — | `design` | Task #7; R-036 (đứng yên), R-028 |
| R-067 | V1 giấu item khi bắt đầu lượt: duyệt khối mềm theo hàng rồi cột, mỗi khối rút một số phần nghìn so với `drop.brick_item_chance`; trúng thì chọn loại theo trọng số `Rules::items` (bảng trọng số cộng dồn, lấy mẫu không lệch bằng `Rng::below`) | — | `design` / design | Issue #6, `systems/blocks.rs`; R-050, R-051, R-053 |
| R-068 | V1: lửa **không** phá item đang nằm trên nền; item chỉ rời nền khi người chơi nhặt (R-054) | — | `design` / design | Issue #6, `systems/blocks.rs`; R-030, R-054 |

Bốn claim `design` R-056…R-059 là quyết định V1 cho luồng nhặt, chưa có bằng chứng client KR/VN cho
các ca biên: thứ tự khi hai người cùng tick (Q-007), và nhặt khi chỉ số đã ở trần (Q-007, `cap`).
Luồng chung đã có bằng chứng: client xin nhặt (117), server xác nhận (158) rồi item mới mất
(R-054), nên V1 cho server quyết cả trường hợp "đã chạm trần" — giữ item trên nền thay vì xóa
đi. Khi có trace, thay các claim này bằng `confirmed` hoặc mở câu hỏi ở
[QUESTIONS](../../research/QUESTIONS.md).

Hai claim `design` R-067/R-068 định nghĩa cách V1 giấu item và cách lửa đối xử với item trên nền;
chưa có bằng chứng client KR/VN cho hai điểm này. R-068 chọn "lửa không phá item" vì item lộ ra
nằm đúng ô mà lửa vừa phá khối: nếu lửa phá item trên nền thì item vừa lộ ra sẽ mất ngay, và
protocol cũ không có gói thu hồi item ngoài xác nhận nhặt (R-054). Khi có trace, thay bằng claim
`confirmed` hoặc mở câu hỏi ở [QUESTIONS](../../research/QUESTIONS.md).

## 6. Giá trị được chọn cho profile dev

Profile [`v1-dev`](../../../data/rules/v1-dev.toml) không dùng được cho phát hành
(`release = false`). Các giá trị `design` hiện tại và claim dẫn tới:

| Rule | Chọn | Vì sao |
|---|---|---|
| Tick | 30 ms/bước (`confirmed`, R-013); profile 33 Hz (`design`, R-074) | Đo 2 run + chọn nguyên 33 Hz |
| Thời gian lượt | 120 s (`confirmed`, R-010) | Client VN |
| Fuse | 2,5 s (`design`, R-021…R-024) | Hai candidate độc lập trùng nhau; gốc unknown |
| Lửa tồn tại | 0,5 s (`design`, R-023) | Chỉ có OSS |
| Nhốt → chết | Packet 6,359 s (`confirmed`, R-026); profile 6,30 s (`design`, R-076) | 3 run, biên độ 9 ms; timer nội bộ unknown |
| Tốc độ | Raw 5→6 px/tick (`candidate`, R-043/R-045); profile 33 px/s (`design`, R-075) | Checksum chưa xác thực; một run cấp 6 |
| Trọng số rơi | 3650 : 2800 : 2000 (`confirmed`, R-051) | Dữ liệu client; vai trò bảng còn unknown |
| Xác suất gạch có item | 35% (`design`, R-053) | Chỉ có OSS |
| Căn trục khi rẽ | Căn về tâm hàng/cột, tối đa bằng tốc độ mỗi tick (`design`) | Thay thế tạm cho R-046 tới khi Q-005 xong |
| Bom chặn đường | Ô có bom không đi vào được (`design`, R-048) | Chưa có phép đo collision client gốc |
| Grace | Một chiều, xóa khi rời ô (`design`, R-049) | SCOPE_V1 §4; V1-012 |
| Người bị nhốt | Đứng yên (`design`, R-036) | R-029 candidate web nói "đi rất chậm" nhưng chưa có px/s; V1 chọn đứng yên |
| Nhặt item | Chỉ người `Alive`, duyệt theo id; đã chạm trần thì không nhặt (`design`, R-056…R-059) | Luồng client xin 117 → server xác nhận 158 (R-054); ca biên chưa có bằng chứng client gốc, V1 tự quyết (Q-007) |

Profile phát hành phải thay mọi `candidate` bằng `confirmed` hoặc `design` đã được review;
`unknown` không được có giá trị. `boom-content` kiểm điều này khi nạp.

## 7. Vòng đời trận và phòng (legacy VN)

Nguồn: [VN_LEGACY_SERVER_REFERENCE](../../research/findings/VN_LEGACY_SERVER_REFERENCE.md). Policy V1
nằm ở SCOPE_V1 §3; bảng này ghi client gốc làm gì để thiết kế không đi ngược nó vô cớ.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-060 | Lượt đi qua: nạp lượt → "GO" (đồng hồ dừng) → bắt đầu (đồng hồ chạy) → chơi → kết quả → về phòng | — | `vn_runtime` / confirmed | 189/190/191/146/168/173 |
| R-061 | Client không báo hết giờ; server kết thúc lượt | — | `vn_runtime` / confirmed | oc_e "Hết giờ" |
| R-062 | Slot phòng có trạng thái đóng/mở, chủ phòng, ready, nhân vật (6 bit), đội (5 bit); client có 16 slot | — | `vn_static` / confirmed | 106 |
| R-063 | Chủ phòng không bấm ready; nút START chỉ bật khi client thấy mọi người ready | — | `vn_static` / confirmed | `sub_6C869F`; V1 khác có chủ ý (SCOPE_V1 §3) |
| R-064 | Bị chủ phòng kick: hiện "BannedByLeader" và về sảnh | — | `vn_runtime` / confirmed | `CPrepareStage__vf143` |
| R-065 | Kết quả lượt có hạng mỗi slot và cột bắt/cứu/EXP/tiền | — | `vn_runtime` / confirmed | 168 |
| R-066 | Di chuyển người chơi khác được đồng bộ P2P qua UDP (bảng peer trong 146) | — | `vn_runtime` / confirmed | op 92/146; engine mới dùng snapshot server (ADR 0005) |

## 8. Animation và trình diễn V1

Nguồn: pack KR 00967 đọc bằng `boom-assets`; định dạng frame 48 byte ở
[MAPS](../assets/MAPS.md) §3. Đây là **dữ liệu content** (state nào dùng clip nào),
không phải luật chơi; giá trị luật của V1 vẫn nằm ở `data/rules/`.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-077 | V1 có entry animation **đã xác nhận** cho: đi 4 hướng `FxCharacter/Character/C0x-Walk{Up,Down,Left,Right}_ALayer{Static,Adjust}` (5 frame), chết `C0x-DieLayer{Static,Adjust}` (14 frame), bom đếm fuse `FxDesktop/Bomb/Bomb00` (4 frame), item rơi `fx/Item/{Bomb,Fire,Roller}` (1 frame); mọi frame đọc được có chỉ số ảnh trong miền và rect nằm trong ảnh | — | `kr_static` / confirmed | `crates/boom-assets/src/animation.rs` (`v1_clip_sources`, `extract_clips`); pack KR 00967 (`FxCharacter`, `FxDesktop`, `fx`); test `every_v1_state_has_entry_and_valid_frames` |
| R-078 | Ứng viên state → animation (**chưa có consumer xác nhận**): idle `C0x-Blink_ALayer{…}`/`C0x-BlinkSmall_ALayer{…}` (3 frame), start `C0x-Start_ALayer{…}` (10 frame), trapped `C0x-FaintLayer{…}` (3 frame) + bong bóng `FxDesktop/Bomb/Bubble00` (9 frame), lửa `FxDesktop/Bomb/Fire00` (100 frame — **một** blob gộp nhiều phần, cách chia tâm/thân/đuôi và thứ tự hướng chưa rõ) và lửa nhạt theo hướng `fx/MotionBlur/Blur_Fire01_{Up,Down,Left,Right}` (10 frame) | — | `kr_static` / candidate | Cùng nguồn R-077; cùng mức với manifest closure (issue #34); Q-003 |
| R-079 | Thời lượng frame: pack client gốc **không** mang trường thời gian (blob `CDataI16Alpha` v2 chỉ có 12 số mỗi frame; `f8`–`f11` là canvas/pivot) và consumer phía client chưa định vị được ⇒ V1 **chưa chốt** thời lượng frame, ghi `unknown` chứ không tự đặt | — | `kr_static` / unknown | MAPS §3 (đo trên 474.120 frame của 4 pack KR); `boom-content::animation::FrameTiming`; Q-003 |
| R-084 | Clip nổ của pack KR 00967: `FxDesktop/Bomb/BombExplosion001` là **một** clip 10 frame / 8 ảnh, canvas 84×103, pivot (20,95); pack **không** tách tâm/nhánh/đuôi (dãi frame liên tục, có frame dùng chung ảnh) ⇒ hiệu ứng nổ **không** có clip riêng theo từng phần; bảng clip V1 dùng **một** clip cho cả vụ nổ | — | `kr_static` / confirmed | `boom-assets sprite-info FxDesktop Bomb/BombExplosion001 --build kr`; pack KR 00967 (`FxDesktop`); `crates/boom-assets/src/animation.rs` (`v1_clip_sources`, clip `explosion.main`); bundle `FxDesktop/Bomb/BombExplosion001` (vai trò `explosion_art`) |
| R-085 | Nhịp phát frame của V1: **100 ms** cho mọi clip (10 frame/s). Nguồn gốc vẫn `unknown` (R-079) nên đây là **lựa chọn** của dự án; client đọc giá trị từ rules profile (`presentation.frame_duration_ms`) và **không** hard-code trong code | 100 ms | `design` / design | Issue #92; `data/rules/v1-dev.toml` (`presentation.frame_duration_ms`); `crates/boom-client/src/anim.rs` (`FrameClock`); BUNDLES §7 |
| R-086 | Chọn clip theo state core (quyết định trình diễn của V1): nhân vật **đang đi** (vị trí đổi so với tick trước) → clip `walk` theo hướng đang quay; **đứng yên** → clip `idle`; **bị nhốt** → clip `trapped`; **chết** → clip `dead`; bom đang chờ → clip `bomb.fuse`; lửa trên ô → clip `flame.main`. Clip `idle`/`trapped` là R-078 **candidate** (chưa có consumer gốc xác nhận), còn `walk`/`dead`/`bomb.fuse` là R-077 `confirmed`. State core V1 chưa mang trường nhân vật nên V1 ghép `id` người chơi với `C00`/`C01` theo thứ tự | — | `design` / design | Issue #92; `crates/boom-client/src/anim.rs` (`character_clip`, `is_moving`); `crates/boom-client/src/scene.rs` (`presentation_plan`); BUNDLES §7 |
| R-087 | Hình lửa trên một ô: V1 dùng **dải frame lõi** của clip `flame.main` (index 28..=59 trên pack KR 00967 — 32 frame ảnh đơn 40×40, hình lửa đứng giữa ô). Phần thân/đuôi/nhánh theo hướng **không** dùng: state core V1 không mang hướng lửa (`Flame` chỉ có ô và hạn), nên mọi ô lửa vẽ cùng một frame của dải này | — | `design` / design (quan sát dải frame: `kr_static`) | Issue #92; `crates/boom-client/src/anim.rs` (`FLAME_FRAMES`); BUNDLES §7; Q-003 |
| R-088 | Hiệu ứng nổ của client V1: khi core báo event `bomb_exploded` (ô của bom) thì client phát clip `explosion.main` tại **ô bom** — pack chỉ có **một** clip nổ (R-084) nên không có clip riêng theo tâm/nhánh/đuôi. Hiệu ứng **phát một lần**: dài bằng số frame của clip × `presentation.frame_duration_ms` (R-085), frame chọn theo **tick mô phỏng** tính từ tick của event (không theo đồng hồ wall clock, ADR 0004); hết khoảng clip thì ô đó **về lại** `flame.main` (lửa trong state core). Clip nổ **vẽ đè lên lửa** ở cùng ô (z-order: vật cản → lửa → nổ → bom → nhân vật) | — | `design` / design (quyết định trình diễn của V1; chưa có consumer gốc xác nhận thứ tự vẽ nổ với lửa — Q-003) | Issue #101; `crates/boom-client/src/anim.rs` (`ExplosionJournal`, `FrameClock::once_frame`); `crates/boom-client/src/scene.rs` (`presentation_plan`); `crates/boom-client/src/main.rs` (`absorb_events`); BUNDLES §7.1; R-084 |
| R-089 | **Sổ trình diễn** (presentation journal) của client V1: mỗi hiệu ứng (nổ, chết, bị nhốt, mở đầu lượt) là **một mục** trong sổ, mỗi đối tượng (ô bom / nhân vật) chỉ có một mục đang chạy. Mục **đã xác nhận** gắn khoá `(round_id, EventId)` ⇒ event **lặp** (reconcile/snapshot gửi lại event cũ) **không** tạo mục mới; mục **client tự sinh** (mở đầu lượt, dự đoán) **không** có khoá và bị **thay** khi event xác nhận của cùng đối tượng tới (ADR 0005). Clip **chết** phát **trọn** một lần (số frame của clip × `presentation.frame_duration_ms`, frame theo **đồng hồ trình diễn** tính từ tick của event), hết khoảng clip thì nhân vật chết giữ **khung cuối có ảnh** của clip chết (khung cuối trong pack là ảnh rỗng 0×0 nên **không** vẽ ô màu) — còn `trapped`/`start` là clip **candidate** (R-078). **Đồng hồ trình diễn** của client bằng tick mô phỏng khi lượt còn chạy và **tiến nốt** (cùng nhịp `sim.ticks_per_second`) sau khi lượt **kết thúc** và mô phỏng dừng, để chuỗi chết vẫn phát trọn clip thay vì kẹt ở frame đầu | — | `design` / design (quyết định trình diễn của V1; chưa có consumer gốc xác nhận — Q-003) | Issue #118; `crates/boom-client/src/anim.rs` (`PresentationJournal`, `PresentationKey`, `presentation_of`); `crates/boom-client/src/scene.rs` (`presentation_plan`, `last_visible_frame`); `crates/boom-client/src/main.rs` (`absorb_events`, `Game::tick`); BUNDLES §7.2; ADR 0005 |

R-079 là `unknown` nên **không** mang giá trị (AGENTS §4). Schema
`boom-content::animation::FrameTiming` đã có sẵn hai nhánh: `Known` (giá trị + nguồn +
trạng thái + claim) và `Unknown` (lý do, không có giá trị); bộ trích
`boom-assets::animation` chỉ dựng `Unknown` cho tới khi có nguồn. Thời lượng chốt
được sẽ vào profile rules cùng claim `design` có nguồn — **không** đặt hằng số trong
code.

Nhánh `Known` của thời lượng frame **bắt buộc** có ít nhất một claim, kể cả khi nguồn
là `design`: ngoại lệ `design` thuần của rule profile (rule kiến trúc không cần claim)
không áp cho thời lượng frame, vì theo AGENTS §4 `design` là "dự án chọn, **có claim
dẫn tới**". `FrameTiming::validate` từ chối mọi `Known` thiếu claim nên không thể lọt
thời lượng tự đặt không truy được quyết định.

Lớp sprite `Static`/`Adjust` của nhân vật được ghi lại theo entry, nhưng **thứ tự vẽ và
cách blend chưa xác nhận** (Q-003); quyết định của V1 cho phần này thuộc task client P6
và cần claim riêng — chưa có ở đây.

### 8.1 Quyết định trình diễn của client V1 (R-085…R-089)

Năm claim cuối §8 là quyết định **design** của dự án cho client V1, không phải luật
chơi và không phải suy ra từ client gốc:

- **Nhịp (R-085)**: 100 ms mỗi frame cho mọi clip. Frame tiến theo **tick mô phỏng**
  (`frame = (tick · 1000 / ticks_per_second) / 100 ms mod số frame`), không theo đồng
  hồ wall clock (ADR 0004) ⇒ cùng tick cho cùng frame. Giá trị nằm ở profile rules
  (`presentation.frame_duration_ms`) để đổi được mà không sửa code.
- **Chọn clip (R-086)**: state core → clip như bảng trên. Hai điểm cần nói rõ vì
  state core chưa có:
  - *Đang đi* được suy ra từ việc **vị trí đổi** giữa hai tick, vì hướng trong state
    core được giữ qua lệnh `Dir::None` (movement bỏ qua người chơi không giữ phím nên
    `facing` không về `None`).
  - *Chết*: khung cuối của clip chết là ảnh rỗng 0×0 (dữ liệu thật của client gốc).
    Chuỗi chết **phát trọn clip** theo sổ trình diễn (R-089) rồi nhân vật chết giữ
    **khung cuối có ảnh**; nếu không có khung nào có ảnh thì nhân vật chết **không
    được vẽ** (không vẽ ô màu).
- **Hình lửa (R-087)**: clip `flame.main` là một blob 100 frame gộp thân/đuôi/nhánh
  (R-078) và state core không mang hướng lửa, nên V1 chỉ dùng dải frame lõi
  (28..=59) cho mọi ô lửa. Đây là **giới hạn đã biết**, ghi ở
  [BUNDLES §8](../assets/BUNDLES.md); Q-003 theo dõi việc đóng.
- **Hiệu ứng nổ (R-088)**: client phát clip `explosion.main` tại ô bom khi core báo
  `bomb_exploded`; hiệu ứng phát **một lần** trong khoảng clip (số frame của clip ×
  `presentation.frame_duration_ms`), frame chọn theo tick mô phỏng tính từ tick của
  event, rồi ô đó **về lại** `flame.main`. Clip nổ vẽ **đè lên lửa** ở cùng ô; thứ tự
  vẽ này là quyết định của V1, chưa có consumer gốc xác nhận (Q-003).
- **Sổ trình diễn (R-089)**: sổ gom mọi hiệu ứng (nổ, chết, nhốt, mở đầu lượt) và
  phân biệt **event đã xác nhận** với **event client tự sinh** (ADR 0005):
  - mục đã xác nhận gắn khoá `(round_id, EventId)` ⇒ event lặp **không** phát lại;
  - mục client tự sinh (mở đầu lượt) bị **thay** khi event xác nhận của cùng đối
    tượng tới;
  - chuỗi chết phát **trọn** clip một lần rồi nhân vật giữ khung cuối **có ảnh**.
  - **Đồng hồ trình diễn** bám tick mô phỏng khi lượt còn chạy và **tiến nốt** sau khi
    lượt kết thúc (core dừng): nếu không, chuỗi chết của lượt cuối (một người chết là
    lượt hai người kết thúc) sẽ kẹt ở frame đầu và không bao giờ tới khung cuối có
    ảnh.
  - **Giới hạn đã biết**: quy tắc đồng hồ áp dụng cho **cả hai** chế độ — offline và
    mạng đều **tiến nốt** khi lượt kết thúc (mạng: đồng hồ không bám tick của state
    server nữa). Nhưng ở chế độ mạng client **chưa** nối event server vào sổ trình
    diễn (việc của #114) nên **chưa có** chuỗi chết để phát; sẽ phát khi #114 nối
    event.

Cả năm giá trị đều nằm ở nguồn chuẩn: quy ước đặt ảnh và chọn frame ở
[BUNDLES §7](../assets/BUNDLES.md), giá trị nhịp ở `data/rules/v1-dev.toml`.

### 8.2 Bảng state của client trong ODT (R-094, R-095)

Hai claim này là **bằng chứng đọc từ pack** (không phải quyết định design của §8.1).
Điểm quan trọng: chúng tách hai tầng khác nhau — **nhãn ODT → entry** (đọc được, xác nhận
được) và **state core V1 → nhãn của client** (chưa có trace runtime nào nối).

ID đánh số sau `R-089` (đã có ở `main`) và `R-090…R-093` (claim của #124) để không trùng
ID đang mở.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-094 | Bảng state của client nằm trong ODT `fx/ObjectTable/ObjectTable` của pack KR 00967 (2.391.729 B, SHA-256 `a68fd5ffcc815b24b5c5199e9910f10d14f1ac6f30aba4c237a23f4e279608dc`): sau bảng tên ô là các khối dữ liệu, mỗi khối là bảng state của một đối tượng. ODT có 189 khối `Character/*_A`; trong đó **63** khối `Character/<Id>_A` là **bảng state trình diễn của nhân vật** (`C00`/`C01` có **90** state), phần còn lại là khối `Character/<Id>_Display_A` và `Character/<Id>_Portrait_A`. Bảng state liệt kê state theo thứ tự, mỗi state gồm nhãn `<số> <tên>`, entry và danh sách `(chỉ số frame, thời lượng)`; bốn state đầu giống nhau ở mọi nhân vật: `Blink` → `Character/<Id>-Blink_A`, `Die` → `Character/<Id>-Die`, `Faint` → `Character/<Id>-Faint`, `Hurrah` → `Character/<Id>-Hurrah_A`; client ghép thêm lớp nên entry pack là `Character/<Id>-<họ>Layer{Static,Adjust}`. **Số frame khác nhau** dùng trong state khớp **đúng** số frame của sprite trong pack (`Blink` 3, `Die` 14, `Faint` 3, `Hurrah` 4, `Walk<Dir>` 5) ⇒ đây là bảng của chính build KR 00967, không phải build khác. Claim này xác nhận **nhãn ODT → entry**; nó **không** xác nhận state core của V1 dùng nhãn nào (xem R-095). `BlinkSmall` **không có** trong bảng state `Character/<Id>_A` của C00/C01 (state `BlinkSmall` chỉ thấy ở khối `Character/<Id>_Display_A` của **25** nhân vật: C00…C13, C17…C22, C25…C29) ⇒ **V1 không chọn** họ sprite này cho `idle`; họ sprite đó **có** mặt trong ODT nên **không** kết luận là client không dùng | — | `kr_static` + `vn_static` / confirmed (chỉ phần **nhãn → entry**) | Issue #121; pack KR 00967 `fx/ObjectTable/ObjectTable`; `crates/boom-assets/src/closure.rs` (`parse_object_table`, `parse_animation_block`, `character_animations`); client VN: `CGameObjectTemplateLib::LoadNameTable` (0x5C9741) ghép `LayerStatic`/`LayerAdjust` và tách riêng nhánh `Die`/`Faint`, bảng state client `sub_C0440D` (`Blink` = state 44); BUNDLES §6.1; MAPS §3 |
| R-095 | State trình diễn của client, đọc từ bảng ODT (R-094): **đứng yên** = `Stop <hướng>` → frame 0 của `Walk<hướng>_A` (overlay nháy mắt `Blink`); **bị khoá (bị nhốt)** = `Locked In` và `Locked In Move/Stop` → frame 0…1 của `Die` giữ vô hạn (thời lượng 32767), kèm bong bóng; **chết** = `Die` (14 frame); **đi** = `Walk<Dir>` (5 frame); **bắt đầu** = `Start` (10 frame). V1 **chọn khác** (R-086, `design`): `idle` → clip `Blink`, `trapped` → clip `Faint` ⇒ sai khác với client gốc đã ghi nhận, chưa có trace runtime nối state core của V1 với state của client | — | `kr_static` + `vn_static` / candidate (đọc nhãn của bảng client; chưa có trace runtime nối state core ↔ state client) | R-094; `crates/boom-assets/src/closure.rs` (`V1_CHARACTER_STATES`); `crates/boom-client/src/anim.rs` (`character_clip`); BUNDLES §6.1; Q-003 |

**Giới hạn đã biết** (AGENTS §4: không suy diễn quá bằng chứng):

- Bảng ODT xác nhận **nhãn → entry**; nó **không** xác nhận state core của V1 dùng nhãn
  nào. Mapping `idle` → `Blink`, `trapped` → `Faint` là **quyết định của V1**
  (R-086 `design`), nên bundle gắn nhãn `design` cho hai sprite đó — **không** phải
  `confirmed`.
- `BlinkSmall` **không có** trong bảng state `Character/<Id>_A` của C00/C01 ⇒ **V1 không
  chọn** họ sprite này cho `idle`. Nhưng họ sprite đó **có** trong ODT (state
  `BlinkSmall` ở khối `Character/<Id>_Display_A` của **25** nhân vật) ⇒ **không** kết luận
  là client không dùng họ sprite đó; không thấy ở bảng state cũng không loại trừ
  consumer khác hay tạo tên entry trực tiếp.
- Chưa có trace runtime nối state core của V1 với state của client gốc (Q-003), nên
  mapping client ↔ V1 vẫn `candidate` (R-095).

## 9. Input

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-080 | Policy input V1-033: hướng dùng phím hướng **được nhấn gần nhất theo thứ tự event** trong các phím còn giữ (last-pressed wins); bỏ autorepeat; release rồi press lại giữa hai frame vẫn tạo cạnh bom mới; mất focus xóa toàn bộ phím đang giữ và cạnh bom chưa tiêu thụ | — | `design` / design | Issue #47; `crates/boom-client/src/input.rs` (`InputAdapter`, `InputTracker::apply`); test `input::tests::event_order_wins_across_same_frame_reverse_bindings`, `adapter_repress_between_frames_keeps_bomb_edge`, `adapter_ignores_autorepeat`, `adapter_focus_event_clears_held_and_pending_bomb`; R-047 (client tự mô phỏng di chuyển) |

R-080 là **DESIGN của dự án**, không phải claim `confirmed` từ client KR/VN: chưa có trace client gốc
cho thứ tự ưu tiên khi giữ nhiều phím. Claim chỉ chốt policy input của V1-033 để code/test có nguồn
chuẩn; nếu sau này có trace, cập nhật giá trị/trạng thái theo trace thay vì suy diễn. R-080 giữ riêng
cho task #47; không thay đổi R-047 và không sửa các claim khác.

## 10. Âm thanh

Nguồn dữ liệu: pack KR 00967 `FxSound` (797 entry: 157 `MP3/` + 640 `Wave/`) đọc bằng
`boom-assets`; đối chiếu file đã giải nén của VN trong `/home/admin/boom-re/assets/fx` (124 MP3 +
492 Wave). Đây là **mapping trình diễn của V1**, không phải luật mô phỏng; asset tồn tại không
chứng minh consumer gốc dùng entry cho event nào.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-081 | Entry/mapping BGM V1 của map Village là `FxSound/MP3/Village` (270.601 B, SHA-256 `7a34054f23a66a1f19cd1cd47b70705bda0c774b6d2afd094c4bd5b6d709038f`), theo mapping map/theme R-070; **không** claim việc consumer gốc loop hay chỉ phát một voice | — | `kr_static` + `vn_static` / confirmed (chỉ entry/mapping) | R-070; [AUDIO_V1](../assets/AUDIO_V1.md) §2; `crates/boom-assets/src/audio.rs`; `boom-assets audio <pack> --list` |
| R-082 | Mapping event→entry V1 (quyết định V1, **không phải** suy ra từ nguồn static): `bomb_placed`→`Wave/Cha_BombIgnite`, `bomb_exploded`→`Wave/BombExplosion001`, `item_revealed`→`Wave/Itm_ItemPop`, `item_picked` (Bomb/Fire/Roller)→`Wave/Itm_ItemEaten`, `player_trapped`→`Wave/Itm_UseTrap`, `player_died`→`Wave/Cha_Die`, `round_ended` thắng/thua/hòa→`Wave/Ntc_Win`/`Ntc_Lose`/`Ntc_Draw`, đếm ngược 3/2/1→`Wave/Ntc_Count3`/`Count2`/`Count1`, start→`Wave/Ntc_GameStart`, `ui_click`→`Wave/Bt_p_Default`; mapping là lựa chọn V1 vì chưa có trace consumer gốc, nhưng **cả 15 entry tồn tại trong KR và VN với cùng SHA-256** | — | `design` / design (entry có thật trong static; mapping do dự án chọn) | Issue #64; [AUDIO_V1](../assets/AUDIO_V1.md) §2–3; `crates/boom-assets/src/audio.rs` (`V1_AUDIO_USES`); Q-015 |
| R-083 | Policy chồng tiếng V1: tối đa **8 voice SFX + 1 BGM**; BGM phát loop một bản, không tính voice và không restart khi reconcile; loop/một voice là design, không phải confirmed từ consumer gốc. Cùng một entry phát nhiều lần trong **cùng một tick** gộp thành một lần; event được dedup theo `(round_id, EventId)`, không phát lại khi replay/reconcile. Ưu tiên khi thiếu voice: round-end/start > explosion > trapped/died > bomb/item > UI; hết voice thì bỏ mục ưu tiên thấp nhất, hòa giữ `EventId` nhỏ hơn (cũ hơn). Qua tick vẫn được phát lại | — | `design` / design | Issue #64; [AUDIO_V1](../assets/AUDIO_V1.md) §4; Q-015; ADR 0008 (EventId/dedup); R-082 |

R-081 xác nhận **entry/mapping asset BGM** từ manifest R-070 và hash trùng ở hai build; policy loop
một voice thuộc R-083 design. R-082/R-083 là **DESIGN của dự
án**, không phải claim `confirmed` KR/VN: chưa tìm thấy trace consumer phát SFX trong client gốc
(A10 của `features.md` vẫn là "chưa điều tra") và chưa có consumer audio V1. Q-015 theo dõi việc
đóng hai claim này; khi có trace, sửa bảng/giá trị thay vì tự nhận confirmed.
