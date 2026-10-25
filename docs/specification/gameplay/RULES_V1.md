# Luật trận V1 — claim, bằng chứng và giá trị được chọn

Cập nhật 2026-10-21 (ADR 0013 — thứ tự ưu tiên nguồn luật: KR/VN `confirmed` → BnB `design`;
lựa chọn hiện hành ở §11).
Cập nhật 2026-10-20 (thêm §12 — nền và ảnh UI của client V1: R-090…R-093).
Cập nhật 2026-10-19. Nguồn chuẩn cho **claim luật chơi V1**; giá trị engine dùng nằm ở
[`data/rules/v1-dev.toml`](../../../data/rules/v1-dev.toml), mỗi giá trị trỏ claim ID ở đây.
Nhãn nguồn/trạng thái theo [AGENTS §4](../../../AGENTS.md#4-bằng-chứng). VN được dùng trực tiếp
([ADR 0002](../../decisions/0002-vn-kr-trusted-sources.md)); thứ tự giá trị theo
[ADR 0013](../../decisions/0013-rule-source-precedence.md).

Locator viết tắt: `oc_e` = `/home/admin/boom-re/docs/oc_e-game.md` (main `4ccc890`),
`items` = `/home/admin/boom-re/docs/items.md`, `MAPS` = [MAPS](../assets/MAPS.md),
`OSS` = [OSS_REIMPLEMENTATIONS](../../research/findings/OSS_REIMPLEMENTATIONS.md),
`NAMU` = [WEB_RULES_NAMU](../../research/findings/WEB_RULES_NAMU.md).

Lựa chọn hiện hành: **§11 / ADR 0013**. Các mục §1–§9 giữ bằng chứng và lịch sử triển khai;
claim design bị thay thế có ghi superseded. Chưa coi cơ chế mới đã triển khai trước #130–#133.

## 1. Lưới và map

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-001 | Một ô = 40 px | 40 px | `vn_static` / confirmed | MAPS §1.1 (`idiv 0x28` ở `0x735DA9`); lựa chọn hiện hành xem §11 (ADR 0013) |
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
| R-010 | Giới hạn thời gian mặc định của client: 120 s, đếm từ lúc bắt đầu lượt | 120 000 ms | `vn_static` + `vn_runtime` / confirmed | oc_e "Hết giờ" (`ctx+490518`); lựa chọn hiện hành xem §11 (ADR 0013) |
| R-011 | Map có cờ thì byte thời gian từ server ghi đè R-010 | — | `vn_static` / confirmed | oc_e "Hết giờ" (`sub_71A9EC`) |
| R-012 | Header map có cặp `60000, 22000` (ms?) ở 574 file VN và ở Village00 KR | — | `vn_static` + `kr_static` / unknown | MAPS §1.1; nghĩa chưa rõ |
| R-013 | Tần số bước mô phỏng gốc: **30 ms/bước** (đo 33,377/33,436 bước/s, tức 29,96/29,91 ms/bước) | 30 ms (±0,1 ms) | `vn_runtime` / confirmed | Bộ đếm `ctx+0x1DF190` tăng tại 0x6ED316, `ctx = *(stage+0x547C)`; 2 run, 24+12 mẫu, cửa sổ 11,505/5,503 s; [tick-runs.csv](../../evidence/measurements/vn-timing-2026-10-19/tick-runs.csv); cửa sổ 2,5 s 32,98–34,00 do lượng tử mẫu 0,5 s; Q-004; lựa chọn hiện hành xem §11 (ADR 0013) |
| R-014 | Tick server remake: 30 Hz | 30 Hz | `oss` / candidate | OSS §2 |
| R-074 | V1 chọn `sim.ticks_per_second = 33` (xấp xỉ nguyên 1000/30 ≈ 33,3 Hz) cho profile dev | 33 Hz | `design` / design | R-013; sai lệch ≈1%; Q-004; `data/rules/v1-dev.toml`; **superseded by R-100** (ADR 0012; lịch sử implementation) |

## 3. Bom, lửa, nhốt, chết

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-020 | Thời điểm nổ do **server** quyết (S→C 151); client không tự nổ | — | `vn_runtime` / confirmed | oc_e §5a (bom) |
| R-021 | Fuse bom thường gốc | — | unknown | Server gốc thất lạc |
| R-022 | Server giả lập dùng fuse 2,5 s | 2 500 ms | `emu_choice` / candidate | `BOMB_FUSE` trong `boom-re/server` |
| R-023 | Remake dùng fuse 2,5 s, lửa 0,5 s | 2 500 / 500 ms | `oss` / candidate | OSS §2 |
| R-024 | Bom bị ném/đá nổ sau 3 s | 3 000 ms | `web` / candidate | `boomkr-re/docs/game-spec-web.md` §2 |
| R-025 | Tầm nổ khởi đầu 1; mỗi ItemFire +1 tầm; mỗi ItemBomb +1 số bom đặt cùng lúc | — | `vn_runtime` / confirmed | oc_e §5a-5d (hiệu ứng bị động); [control VN24/10](../../evidence/measurements/vn-item-effects-2026-10-24/README.md): Bomb1→2/no3, Fire r18 1→2, chưa maxcap/nativewriter |
| R-026 | Bị nước trúng → bị nhốt; khoảng packet `120/11 → 120/13` là **6,359 s** (ba lần: 6,355/6,364/6,358; biên độ 9 ms) | 6 359 ms ± 5 ms (biên độ) | `vn_runtime` / confirmed | [trap-packets.csv](../../evidence/measurements/vn-timing-2026-10-19/trap-packets.csv), [trap-packets.txt](../../evidence/measurements/vn-timing-2026-10-19/trap-packets.txt); run3 chỉ còn log worker, run4/5 còn raw `server.log`; đây là khoảng packet gồm xử lý/gửi, chưa phải timer nội bộ; `210 tick`/literal 6300 chưa chứng minh, Q-006; lựa chọn hiện hành xem §11 (ADR 0013) |
| R-076 | V1 chọn `player.trap_to_death_ms = 6300` từ khoảng packet đo được; `210 tick` chỉ là giả thuyết, không phải hằng số đã đọc | 6 300 ms | `design` / design | R-026; Q-006; [trap-packets.csv](../../evidence/measurements/vn-timing-2026-10-19/trap-packets.csv); `data/rules/v1-dev.toml`; **superseded by R-103** (ADR 0012; lịch sử implementation) |
| R-027 | Remake: nhốt 4 s rồi chết | 4 000 ms | `oss` / candidate | OSS §2; khác đại lượng R-026 (packet); lựa chọn timer hiện hành R-115 |
| R-028 | Địch chạm người bị nhốt → người bị nhốt chết | — | `vn_runtime` / confirmed | VN-B01 |
| R-029 | Bị nhốt: không đặt bom, đi rất chậm | — | `web` / candidate | NAMU §2.1 |
| R-030 | Lửa dừng ở khối cứng, phá khối mềm; bom trong lửa kích nổ dây chuyền | — | `oss` + `web` / candidate | OSS §5; client VN tự vẽ vụ nổ (oc_e) |
| R-031 | Vỡ gạch: client legacy tự tính và báo C→S 135 | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-032 | Sức nổ được chụp lúc đặt bom (client gửi `power` trong 111); ID bom do server cấp (152) | — | `vn_static` + `vn_runtime` / confirmed | [Legacy server](../../research/findings/VN_LEGACY_SERVER_REFERENCE.md) §1 |
| R-033 | Lửa phủ ô đặt bom + tối đa `range` ô mỗi hướng; khối cứng/mép lưới chặn, khối mềm đầu tiên có lửa rồi dừng. Bom đến hạn hoặc bị lửa chạm nổ cùng tick, xử lý theo `id` tăng dần, mỗi bom một lần; nhiều lửa cùng ô giữ `until` lớn nhất, hòa chọn `source` id nhỏ | — | `design` (từ R-030) | Task #5, `systems/explosion.rs`; R-030; **superseded by R-109** (ADR 0012; lịch sử implementation) |
| R-034 | V1 trước ADR 0012: "chạm" = hai người chơi cùng một ô lưới tại bước life; chỉ người `Alive` làm vỡ bong bóng, hai người `Trapped` chạm nhau không đổi trạng thái | — | `design` / design | SCOPE_V1 §4; issue #8 (cùng ô); `systems/life.rs`; **superseded by R-108** (ADR 0012; lịch sử implementation) |
| R-035 | V1 trước ADR 0012: trong một tick, người `Alive` đứng trên lửa bị nhốt trước; chỉ người đã `Trapped` từ đầu bước life mới xét chết; nếu vừa hết `trap_ticks` vừa bị chạm thì `player_died.by` là người chạm; nhiều người `Alive` cùng ô thì chọn id nhỏ nhất | — | `design` / design | SCOPE_V1 §4; ARCHITECTURE §5.2; issue #8; **superseded by R-108** (ADR 0012; lịch sử implementation) |
| R-036 | Lựa chọn lịch sử: người bị nhốt đứngyên; **superseded by R-161/R-162** (VN runtime24/10) | — | `design` / design | SCOPE_V1 §4; R-029 (`web` / candidate) nói "đi rất chậm" nhưng chưa có px/s; quyết định cho V1-012 |

Hai claim lịch sử `design` R-034/R-035 (nay superseded by R-108) chỉ định nghĩa cách engine V1 hiện thực hóa "chạm" và thứ tự trong
tick; chưa có bằng chứng client KR/VN cho trường hợp biên. Khi có trace, thay bằng claim `confirmed`
hoặc mở câu hỏi ở [QUESTIONS](../../research/QUESTIONS.md).

## 4. Nhân vật và tốc độ

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-040 | BomberAttr KR C00: f2–f7 = `5,7,1,10,1,7` | — | `kr_static` / confirmed | `boomkr-re/data/kr/bomber_attr.json`; lựa chọn hiện hành xem §11 (ADR 0013) |
| R-041 | BomberAttr KR C01: f2–f7 = `4,8,2,7,1,9` | — | `kr_static` / confirmed | Như trên; lựa chọn hiện hành xem §11 (ADR 0013) |
| R-042 | f2–f3 = speed min/max, f4–f5 = số bom min/max, f6–f7 = tầm min/max (cấp) | — | `kr_static` + `web` / candidate | `boomkr-re/docs/kr-data.md` §23; OSS §4 khớp 5/6 giá trị; chưa đối chiếu VN vì R-043 còn candidate |
| R-043 | Quan sát thô tốc độ C00: 5 px/tick base; sau Roller 6 px/tick (một run, checksum x/y chưa xác thực) | 5 → 6 px/tick (raw) | `vn_runtime` / candidate | [speed-observations.csv](../../evidence/measurements/vn-timing-2026-10-19/speed-observations.csv); item3 117/158 22:31:20; `xok/yok=false`, decoder cần xác minh; Q-005 |
| R-044 | Remake: 1 cấp tốc độ = 24 px/s | 24 px/s | `oss` / candidate | OSS §3; chưa đối chiếu được VN vì R-043 còn candidate |
| R-045 | Roller: +1 cấp tốc độ | +1 cấp (raw 5→6 px/tick, một run) | `web` + `vn_runtime` / candidate | NAMU §1; [speed-item3-changes.txt](../../evidence/measurements/vn-timing-2026-10-19/speed-item3-changes.txt); một run, checksum chưa xác thực; Q-005 |
| R-075 | V1 chọn `player.speed_px_per_s_per_level = 33` (xấp xỉ nguyên 1000/30 ≈ 33,3 px/s) cho profile dev | 33 px/s | `design` / design | R-043/R-045 (candidate); sai lệch ≈1%; Q-005; `data/rules/v1-dev.toml`; **superseded by R-102** (ADR 0012; lịch sử implementation) |
| R-046 | Bám góc khi rẽ trong 12 px | 12 px | `oss` / candidate | OSS §5 |
| R-047 | Di chuyển không có gói C→S; client tự mô phỏng | — | `vn_static` / confirmed | oc_e §5a |
| R-048 | Bom đang tồn tại chặn mọi người đi vào ô của nó | — | `design` / design | SCOPE_V1 §4 ("bom chặn/đi ra"); chưa đối chiếu collision client gốc |
| R-049 | Grace một chiều (lịch sử trước ADR 0012, phần rời ô superseded by R-107): người đang đứng trên bom được đi ra; rời ô thì mất quyền, không được quay lại | — | `design` / design | SCOPE_V1 §4; V1-012 |

## 5. Item rơi và nhặt

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-050 | Item giấu trong gạch được server gửi **trước trận** (553), không quyết lúc vỡ | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-051 | ItemAttr trọng số: Bomb 3650, Fire 2800, Roller 2000; client nạp vào bảng ngẫu nhiên 0x8000 ô | — | `vn_static` / confirmed | items (loader `sub_89F3DE`); thay cách đọc "giá" ở MAPS §2; lựa chọn hiện hành xem §11 (ADR 0013) |
| R-052 | Ai dùng bảng R-051 để rơi item (client hay server) | — | unknown | Q-007 |
| R-053 | Xác suất một gạch có item | — | unknown | Q-007; OSS dùng 0,35 (`oss` / candidate) |
| R-054 | Nhặt: client xin (117), server xác nhận (158), item mất sau xác nhận | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-055 | Bảng rơi gốc của Village00 chỉ có 6 loại Lucci | — | `kr_static` / confirmed | `kr_v1_content.json`; V1 thay bằng Bomb/Fire/Roller (DESIGN, SCOPE_V1 §2) |
| R-056 | V1: người còn sống (`Alive`) có tâm trong ô có item trên nền thì nhặt; duyệt người chơi theo id tăng dần nên hai người cùng ô trong cùng tick chỉ người id nhỏ nhận; item mất khỏi nền sau khi nhặt và phát `item_picked` | — | `design` (từ R-054) | Task #7, `systems/pickup.rs`; R-054; SCOPE_V1 §4 |
| R-057 | V1: ItemBomb +1 số bom, ItemFire +1 tầm nổ, ItemRoller +1 cấp tốc độ; lượng lấy từ profile (`item.*_delta`), kẹp theo trần chỉ số; bom đã đặt giữ tầm chụp lúc đặt | — | `design` (từ R-025, R-032, R-045) | Task #7, `systems/pickup.rs`; R-025; R-032; R-045 |
| R-058 | V1: chỉ số đã ở trần thì không nhặt — item vẫn nằm trên nền, không áp hiệu ứng, không phát event; nhặt là atomic (item chỉ mất khi hiệu ứng áp được) | — | `design` | Task #7; R-054 (item mất sau khi server xác nhận); SCOPE_V1 §4; Q-007; **superseded by R-109** (ADR 0012; lịch sử implementation) |
| R-059 | V1: người bị nhốt (`Trapped`) hoặc đã chết không nhặt item; slow movement theo R-161/R-162 không cho phép pickup; lý do đứngyên R-036 là lịch sử | — | `design` | Task #7; R-036 (đứng yên), R-028 |
| R-067 | V1 giấu item khi bắt đầu lượt: duyệt khối mềm theo hàng rồi cột, mỗi khối rút một số phần nghìn so với `drop.brick_item_chance`; trúng thì chọn loại theo trọng số `Rules::items` (bảng trọng số cộng dồn, lấy mẫu không lệch bằng `Rng::below`) | — | `design` / design | Issue #6, `systems/blocks.rs`; R-050, R-051, R-053; **superseded by R-106** (ADR 0012; lịch sử implementation) |
| R-068 | V1: lửa **không** phá item đang nằm trên nền; item chỉ rời nền khi người chơi nhặt (R-054) | — | `design` / design | Issue #6, `systems/blocks.rs`; R-030, R-054; **superseded by R-109** (ADR 0012; lịch sử implementation) |

Bốn claim `design` R-056…R-059 là quyết định V1 trước ADR 0012 (R-058 nay superseded by R-109) cho luồng nhặt, chưa có bằng chứng client KR/VN cho
các ca biên: thứ tự khi hai người cùng tick (Q-007), và nhặt khi chỉ số đã ở trần (Q-007, `cap`).
Luồng chung đã có bằng chứng: client xin nhặt (117), server xác nhận (158) rồi item mới mất
(R-054), nên V1 cho server quyết cả trường hợp "đã chạm trần". ADR 0012 đã đổi ca biên đó theo
BnB (`server/game.js:577–581`): item rời nền **trước** rồi mới áp hiệu ứng, nên **ở trần item
vẫn mất** (R-109, `systems/pickup.rs`). Khi có trace, thay các claim này bằng `confirmed` hoặc
mở câu hỏi ở
[QUESTIONS](../../research/QUESTIONS.md).

Hai claim lịch sử `design` R-067/R-068 (nay superseded by R-106/R-109) định nghĩa cách V1 giấu item và cách lửa đối xử với item trên nền;
chưa có bằng chứng client KR/VN cho hai điểm này. R-068 chọn "lửa không phá item" vì item lộ ra
nằm đúng ô mà lửa vừa phá khối: nếu lửa phá item trên nền thì item vừa lộ ra sẽ mất ngay, và
protocol cũ không có gói thu hồi item ngoài xác nhận nhặt (R-054). ADR 0012 đã đổi cả hai điểm
theo BnB: **không giấu item trước lượt nữa** mà **rơi item khi gạch vỡ** (35%, ba loại V1), và
**lửa đốt item cũ trước khi sinh item mới** ở ô vừa vỡ — nên item vừa rơi không bị đốt bởi
chính vụ nổ đó (R-106, R-109; `systems/blocks.rs`). Khi có trace, thay bằng claim `confirmed`
hoặc mở câu hỏi ở [QUESTIONS](../../research/QUESTIONS.md).

## 6. Giá trị được chọn cho profile dev

Profile [`v1-dev`](../../../data/rules/v1-dev.toml) dùng các claim lựa chọn R-110…R-118 theo
[ADR 0013](../../decisions/0013-rule-source-precedence.md), quyết định chủ dự án 2026-10-21.
`release = false`: profile có candidate/design; #130 đã tích hợp, #131/#132 còn backlog.
Không tiếp tục thay Attr KR bằng template BnB của #133.
Các claim trước đó mô tả bằng chứng hoặc implementation cũ; bảng dưới §11 là lựa chọn hiện hành.
Presentation (R-085) giữ riêng vì BnB không cung cấp timing cho clip asset KR.
Không coi profile đã đổi là engine đã hoàn thành hoặc đã đạt fidelity KR/VN.

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
| R-082 | Mapping event→entry V1 (quyết định V1, **không phải** suy ra từ nguồn static): `bomb_placed`→`Wave/Cha_BombIgnite`, `bomb_exploded`→`Wave/BombExplosion001`, `item_revealed`→`Wave/Itm_ItemPop`, `item_picked` (Bomb/Fire/Roller)→`Wave/Itm_ItemEaten`, `player_trapped`→`Wave/Cha_LockedIn` (**phần này superseded bởi R-203/R-204**), `player_died`→`Wave/Cha_Die`, `round_ended` thắng/thua/hòa→`Wave/Ntc_Win`/`Ntc_Lose`/`Ntc_Draw`, đếm ngược 3/2/1→`Wave/Ntc_Count3`/`Count2`/`Count1`, start→`Wave/Ntc_GameStart`, `ui_click`→`Wave/Bt_p_Default`; phần lớn mapping là lựa chọn V1 chưa nâng nhãn/áp đầy đủ đối chiếu consumer (R-204 đã có consumer cho một số event), nhưng **cả 15 entry tồn tại trong KR và VN với cùng SHA-256** | — | `design` / design (entry có thật trong static; mapping do dự án chọn; **riêng `player_trapped` đã confirmed bởi R-203/R-204, không nâng cả R-082**) | Issue #64; [AUDIO_V1](../assets/AUDIO_V1.md) §2–3; R-203; R-204; `crates/boom-assets/src/audio.rs` (`V1_AUDIO_USES`); Q-015 |
| R-083 | Policy chồng tiếng V1: tối đa **8 voice SFX + 1 BGM**; BGM phát loop một bản, không tính voice và không restart khi reconcile; policy mộtvoice/reconcile giữdesign; riêng yêu cầu gọi lại sample từEOS native được nối R-213, không nâng toàn policy. Cùng một entry phát nhiều lần trong **cùng một tick** gộp thành một lần; event được dedup theo `(round_id, EventId)`, không phát lại khi replay/reconcile. Ưu tiên khi thiếu voice: round-end/start > explosion > trapped/died > bomb/item > UI; hết voice thì bỏ mục ưu tiên thấp nhất, hòa giữ `EventId` nhỏ hơn (cũ hơn). Qua tick vẫn được phát lại | — | `design` / design | Issue #64; [AUDIO_V1](../assets/AUDIO_V1.md) §4; Q-015; ADR 0008 (EventId/dedup); R-082 |

R-081 xác nhận **entry/mapping asset BGM** từ manifest R-070 và hash trùng ở hai build; policy loop
một voice thuộc R-083 design. R-082/R-083 vẫn là **DESIGN của dự án** cho phần chưa nâng nhãn/áp đầy
đủ đối chiếu consumer; điều này **không** có nghĩa client gốc không có consumer. Lượt RE #196
(R-203/R-204) đã tìm consumer client KR cho nổ/đặt/nhốt/chết/item; #206 chỉ supersede riêng mapping
`player_trapped` sang `Wave/Cha_LockedIn` (id 77 solo) và nâng **mapping đó** thành confirmed. Các
mapping khác và policy R-083 giữ design; A10 của `features.md` ghi "chưa điều tra" là trạng thái
trước #196. Q-015 theo dõi phần còn lại; không suy toàn R-082 thành confirmed.

## 10. Lịch sử chọn luật từ BnB — ADR 0012

Phần này giữ quyết định cũ; phần bị thay và lựa chọn hiện hành xem §11 / ADR 0013.

Quyết định chủ dự án 2026-10-21: chọn có chủ ý OSS thành `design`, kể cả khi khác
bằng chứng KR/VN `confirmed`. Không nâng OSS thành bằng chứng luật gốc, không hạ nhãn claim gốc.
Locator `BnB` dưới đây là SHA `5c108e8ad3ea07684d4666f60e6eec6c2f3e569e`, repo
[tpoisonooo/BnB](https://github.com/tpoisonooo/BnB/tree/5c108e8ad3ea07684d4666f60e6eec6c2f3e569e),
đọc lại clone `/home/admin/boom-team/tmp/bnb` ngày 2026-10-21. Mọi dòng là lựa chọn V1 theo
quyết định trên; bảng sai khác và issue triển khai nằm trong ADR 0012.

| ID | Claim V1 chọn theo BnB | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-100 | Fixed step, thời hạn và warmup đóng băng không chạy đồng hồ lượt | 30 Hz; 180000 ms; 3000 ms | `oss` / design | BnB `server/game.js:12–19,616–646` |
| R-101 | Cỡ ô V1 giữ giá trị BnB (trùng R-001); map vẫn Village00 KR | 40 px | `oss` / design | BnB `server/map.js:7–9`; ADR 0012 giữ scope content |
| R-102 | Một cấp tốc độ = 24 px/s | 24 px/s/level | `oss` / design | BnB `server/characters.js:15,20–26`, `server/game.js:21,229` |
| R-103 | Fuse/lửa/nhốt dùng timer BnB | 2500/500/4000 ms = 75/15/120 tick | `oss` / design | BnB `server/game.js:14–17`; biên tick theo R-108/R-109, không phải đo packet gốc |
| R-104 | Bomb/Fire +1; Roller +24 px/s tức +1 cấp R-102 | 1 bomb / 1 cell / 1 level | `oss` / design | BnB `server/game.js:21,579–581` |
| R-105 | Ánh xạ cân bằng design C00→baobao, C01→haidao (không phải identity KR) | C00 speed 6–10/bomb 1–6/range 1–7; C01 speed 5–9/bomb 1–8/range 2–7 | `oss` / design | BnB `server/characters.js:17–27`; #133 sau #117 |
| R-106 | Roll khi vỡ gạch; V1 chỉ giữ ba loại, chuẩn hóa tương đối BnB bubble/potion/shoe | 350 permille; Bomb:Fire:Roller = 20:28:16 | `oss` / design | BnB `server/game.js:20,425–439`; bỏ loại ngoài scope, không giữ xác suất tuyệt đối 20/28/16% |
| R-107 | Hộp va chạm và assist khi bị chặn; pass bom tới khi hộp rời hoàn toàn | half 13 px (26×26), assist ≤12 px | `oss` / design | BnB `server/game.js:31–32,150–153,173–275,392–399`; #130 |
| R-108 | Trapped đứngyên là lịch sử (thay bởi R-161/R-162); giảm timer cả tick vừa nhốt; timeout trước touch; touch khi cả hai độ lệch nhỏ hơn 28 px | 28 px (strict); 120 tick tính cả tick bị nhốt | `oss` / design | BnB `server/game.js:454–474,546–567,634–643`; #132; thay R-034/R-035; phần đứngyên R-036 đã thay bằng R-161/R-162 |
| R-109 | Chain FIFO, gạch chỉ mất sau queue; lửa phá item cũ, TTL mới giảm ngay; cap vẫn nhặt; gameplay trước kết quả rồi timeout. Biên TTL: ttl 1 → 0 ⇒ vụ nổ bị xóa **trước** khi áp sát thương (không có ô lửa, không ai bị nhốt) nhưng gạch trong blast vẫn vỡ, vì BnB phá gạch lúc nổ | — | `oss` / design | BnB `server/game.js:401–451,573–652`; #131/#132; V1 duyệt id ổn định thay insertion Map, xem ADR 0012 |

R-014/R-023/R-027/R-044/R-046 giữ `oss`/candidate như bằng chứng lịch sử; không phải claim lựa chọn.
R-056/R-057/R-059 còn áp dụng phần alive/tâm ô/cap hiệu ứng; phần giữ item tại cap (R-058),
giấu trước trận (R-067), lửa không phá item (R-068) và tie-break life cũ đã superseded như ghi trên.


## 11. Ưu tiên nguồn hiện hành — ADR 0013

Quyết định chủ dự án 2026-10-21, [ADR 0013](../../decisions/0013-rule-source-precedence.md).
Các claim lựa chọn là quyết định áp dụng; nhãn của bằng chứng gốc giữ nguyên.
R-100 **superseded by R-110/R-113**; R-101 **superseded by R-110**;
R-102 **superseded by R-114**; R-103 **superseded by R-115**;
R-104 **superseded by R-111**; R-105 **superseded by R-117**;
R-106 phần trọng số/chance **superseded by R-112**; phần roll khi vỡ do #131 triển khai
(`systems/blocks.rs`: rơi item lúc gạch vỡ, 35%, ba loại V1).
R-107/R-108 thông số **superseded by R-116**, cơ chế vẫn áp dụng;
R-109 giữ backlog #132; phần #131 (roll khi vỡ, lửa đốt item cũ, nhặt ở cap) do #131 triển khai
trong `systems/{blocks,pickup}.rs`, không biến thành bằng chứng gốc.
Biên TTL giảm ngay lúc tạo của R-109 được thay riêng bởi R-158 (§24); các phần khác giữ nguyên.

| ID | Khóa / lựa chọn hiện hành | Giá trị | Nguồn / trạng thái | Locator và giới hạn |
|---|---|---|---|---|
| R-110 | `grid.tile_px`; `round.duration_ms` khôi phục cùng đại lượng | 40 px; 120000 ms | `vn_static` + `vn_runtime` / confirmed | R-001 (`0x735DA9`); R-010 (`ctx+490518`); ADR 0013 chọn tầng 2; không có KR confirmed cùng khóa |
| R-111 | `item.bomb_capacity_delta`, `item.fire_range_delta` dùng VN | +1 bomb; +1 cell | `vn_runtime` / confirmed | R-025; ADR0013. Phần Roller BnB lịch sử được R-185 thay riêng; không supersede Bomb/Fire |
| R-112 | `drop.weight_bomb/fire/roller` trả số bảng VN; `drop.brick_item_chance` giữ BnB | 3650:2800:2000; 350 permille | Trọng số `vn_static` / confirmed (số bảng); chance `oss` / design | R-051 `sub_89F3DE`; R-052 vai trò bảng vẫn unknown. Dùng bảng làm trọng số ba item V1 là quyết định áp dụng, không xác nhận xác suất drop gốc. Chance R-053 unknown → R-106/BnB `server/game.js:20,425–439` SHA §10 |
| R-113 | `sim.ticks_per_second`; `round.warmup_ms` giữ BnB | 30 Hz; 3000 ms | `oss` / design | R-100/BnB `server/game.js:12–19,616–646` SHA §10. R-013 đo bước client, không xác nhận tick server; R-060 xác nhận pha GO nhưng không xác nhận 3 s |
| R-114 | Lịch sử `player.speed_px_per_s_per_level` theo BnB | 24 px/s/level | `oss` / design | R-102/BnB; **superseded by R-184** từ curve native có đối chứng R-183. Không nâng R-043 hoặc R-075 lịch sử |
| R-115 | `bomb.fuse_ms` giữ BnB; lựa chọn flame500ms/trap4000ms là lịch sử, hiện dùng R-158/R-160 | 2500ms (flame hiện540ms; trap hiện6350ms) | `oss` / design | R-103/BnB `server/game.js:14–17` SHA §10; R-021 unknown, R-022 emu candidate; R-026 là khoảng packet, không phải timer nội bộ; R-079 timing trình diễn không phải TTL damage |
| R-116 | `player.collision_half_px`, `player.corner_assist_px`, `player.trap_touch_distance_px` giữ BnB | 13/12/28 px | `oss` / design | R-107/R-108/BnB `server/game.js:31–32,546–567` SHA §10; chưa có KR/VN confirmed cùng đại lượng; R-028 không xác nhận khoảng cách |
| R-117 | **Lịch sử; superseded by R-182 cho profile hiện hành.** Giữ toàn bộ 12 khóa `characters.c00_*`, `characters.c01_*` theo BomberAttr target; ngoại lệ candidate có chủ ý | C00 speed 5–7, bomb 1–10, range 1–7; C01 speed 4–8, bomb 2–7, range 1–9 | `kr_static` / candidate (nghĩa trường) | R-040/R-041 confirmed dãy số, R-042 mapping candidate; ADR 0013 chọn giữ target thay template BnB R-105. Không nâng lên confirmed khi chưa có consumer |
| R-118 | `presentation.frame_duration_ms` giữ design riêng | 100 ms | `design` / design | R-085; R-079 unknown; BnB không cung cấp nhịp clip KR. RE consumer frame/timing có thể thay sau |

### Bảng lựa chọn theo khóa (máy-đọc)

Bảng dưới ràng buộc metadata **theo từng khóa profile**: claim lựa chọn, đại lượng, đơn vị,
nguồn, trạng thái và ngữ cảnh/build áp dụng. Đây là nguồn chuẩn để validator so tầng ADR 0013
**theo từng khóa** — ô "Nguồn / trạng thái" của bảng claim ở trên chỉ là tóm tắt và phải khớp
đúng tập nguồn/trạng thái của các khóa thuộc claim đó; lệch thì validator báo lỗi. Dòng gộp
chưa tách được theo từng khóa thì validator **fail closed** (báo lỗi), không đoán theo ô claim.

| Khóa profile | Claim lựa chọn | Đại lượng | Đơn vị | Nguồn | Trạng thái | Ngữ cảnh / build |
|---|---|---|---|---|---|---|
| `grid.tile_px` | R-110 | tile_size | px | `vn_static` | confirmed | VN VBNB_116 |
| `round.duration_ms` | R-110 | round_duration | ms | `vn_runtime` | confirmed | VN VBNB_116 |
| `item.bomb_capacity_delta` | R-111 | item_bomb_capacity_delta | bomb | `vn_runtime` | confirmed | VN VBNB_116 |
| `item.fire_range_delta` | R-111 | item_fire_range_delta | cell | `vn_runtime` | confirmed | VN VBNB_116 |
| `item.roller_speed_delta` | R-185 | item_roller_speed_delta | level | `vn_runtime` | confirmed | VN VBNB_116 |
| `drop.weight_bomb` | R-112 | drop_item_weight | weight | `vn_static` | confirmed | VN VBNB_116 |
| `drop.weight_fire` | R-112 | drop_item_weight | weight | `vn_static` | confirmed | VN VBNB_116 |
| `drop.weight_roller` | R-112 | drop_item_weight | weight | `vn_static` | confirmed | VN VBNB_116 |
| `drop.brick_item_chance` | R-112 | drop_brick_item_chance | permille | `oss` | design | BnB 5c108e8 |
| `sim.ticks_per_second` | R-113 | server_tick_rate | Hz | `oss` | design | BnB 5c108e8 |
| `round.warmup_ms` | R-113 | round_warmup | ms | `oss` | design | BnB 5c108e8 |
| `player.speed_px_per_s_per_level` | R-184 | speed_px_per_s_per_level | px/s | `design` | design | V1 design; calibration VN R-183 |
| `bomb.fuse_ms` | R-115 | bomb_fuse | ms | `oss` | design | BnB 5c108e8 |
| `bomb.flame_ms` | R-158 | bomb_flame | ms | `design` | design | V1 design |
| `player.trapped_speed_px_per_s` | R-162 | trapped_speed | px/s | `design` | design | V1 design |
| `player.trap_to_death_ms` | R-160 | trap_to_death_timer | ms | `design` | design | V1 design |
| `player.collision_half_px` | R-116 | collision_box_half | px | `oss` | design | BnB 5c108e8 |
| `player.corner_assist_px` | R-116 | corner_assist | px | `oss` | design | BnB 5c108e8 |
| `player.trap_touch_distance_px` | R-116 | trap_touch_distance | px | `oss` | design | BnB 5c108e8 |
| `characters.c00_speed` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c00_speed_max` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c00_bombs` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c00_bombs_max` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c00_range` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c00_range_max` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c01_speed` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c01_speed_max` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c01_bombs` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c01_bombs_max` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c01_range` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `characters.c01_range_max` | R-182 | character_stat_level | level | `vn_runtime` | confirmed | VN VBNB_116 logical statpoint |
| `presentation.frame_duration_ms` | R-118 | presentation_frame_duration | ms | `design` | design | V1 design |

Bảng trên là nguồn chuẩn lựa chọn; validator [`tools/rules_precedence.py`](../../../tools/rules_precedence.py)
giữ bảng kỳ vọng máy-đọc-của bảng này (định danh theo **đại lượng**, đơn vị, build/context) và **đối chiếu
ngược với RULES_V1** trước khi kiểm profile [`data/rules/`](../../../data/rules/) không dùng tầng thấp
hơn tầng ghi ở đây; validator chạy trong `cargo xtask ci`, test ở `tools/test_rules_precedence.py`.
Tầng được so **theo từng khóa** từ "Bảng lựa chọn theo khóa" ở đây: bảng kỳ vọng trong tool và ô
"Nguồn / trạng thái" của bảng claim phải khớp từng khóa với bảng đó, lệch thì CI báo lỗi.
Lựa chọn12khóa `characters.*` hiện theo R182/VNconsumer, thay ngoại lệ candidate R117 lịch sử.
Ngoại lệ còn tường minh: `drop.weight_*` (số bảng `confirmed` nhưng vai trò bảng còn `unknown` R-052). Validator **không** so giá trị số với
claim và không tự kết luận claim đúng; reviewer kiểm nguồn.

## 12. Nền và ảnh UI của client V1

Hai loại asset UI khác nhau về chỗ nằm trong atlas của bundle. **Nền màn hình** là JPEG nên ở
trạng thái `excluded`, **không** vào atlas (bộ giải mã sprite của V1 chỉ đọc blob
`CDataI16Alpha` — I16). **Ảnh UI có chữ** được phân loại theo quyết định duyệt: phần tử còn
`needs_review_text` bị chặn, client vẽ text catalog thay; manifest Việt hóa đã duyệt chuyển sáu
phần tử V1 sang `selected` khi nạp nguồn VN/PNG (PR #241). Vì vậy V1 dựng nền bằng màu và vẽ chữ từ catalog — đây là quyết định
**design** của dự án, không phải luật chơi.

Bằng chứng phần client gốc là **đọc trực tiếp từ pack**: ba nền là JPEG 800×600 (đo trên blob
đã giải mã của pack KR 00967 và của bản VN), và ảnh UI của luồng V1 **có chữ nằm trong ảnh**
(đọc trên PNG giải mã). Chữ được ghi **theo từng asset đã đọc**; phần chưa đọc rõ thì ghi "chưa
xác nhận" — **không** suy ngôn ngữ chữ từ việc hai build khác byte. Chừa của nền và ảnh:
[UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1),
[IMAGE_REVIEW](../localization/IMAGE_REVIEW.md) và [BUNDLES §8](../assets/BUNDLES.md#8-giới-hạn-đã-biết).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-090 | Nền màn hình của client gốc là **ảnh JPEG 800×600 có chữ nằm trong ảnh**, **không phải** sprite I16: `FxDesktop/LoginStage/CAStage` (556.273 B), `FxStage/Lobby/CAStage` (225.527 B), `FxMap/Prepare/CAStage` (231.612 B). Chữ đọc **theo từng asset**: KR login = lời thoại tiếng Hàn + biểu tượng "ALL"/"전체이용가" (góc trên phải); KR lobby = hai tab "대전 모드"/"몬스터 모드" + nhãn "레벨"; KR prepare = nhãn "캐릭터선택"/"팀선택"/"맵" + nút "자동 준비"; bản VN login = logo Boom + dòng bản quyền tiếng Anh ở góc dưới phải (ghi "NexonKorea Corporation"; **không** có chữ Việt), bản VN lobby/prepare = nhãn tiếng Việt ("ĐẠI CHIẾN"/"NHIỆM VỤ"; "NHÂN VẬT"/"CHỌN ĐỔI"/"BẢN ĐỒ"/"TỰ ĐỒNG") | 800×600 JPEG | `kr_static` + `vn_static` / confirmed | Đo blob đã giải mã (`boom-assets pack-extract /home/admin/boomkr-full <stem> <entry> <out>`): magic `FFD8FFE0` (Lobby, Prepare) / `FFD8FFE1` (Login), marker **SOF0 (`FFC0`)**, 800×600, 8-bit, 3 thành phần; bản VN (`/home/admin/boom-re/assets/fx/{LoginStage,Lobby,Prepare}/CAStage.bin`) cũng SOF0 800×600 8-bit 3 thành phần, **khác byte** (488.361/227.530/232.346 B) — khác byte **không** kéo theo kết luận về chữ; chữ đọc trên ảnh giải mã — [UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1); ảnh trong cache review ảnh mà §3 dùng để đọc chữ có **pixel giống hệt** blob trích từ pack (đối chiếu RGB 800×600, cả KR lẫn VN) nên đọc chữ trên ảnh đó là bằng chứng cho blob trong pack. Locator tái lập: `/home/admin/boom-re-l10n/l10n/.cache/image-review/manifest.json` (entry `FxDesktop/LoginStage/CAStage`, `FxStage/Lobby/CAStage`, `FxMap/Prepare/CAStage`) → PNG `images/0a129afd5e751013c7878d83-0.png` (login KR), `images/b279b18e1df7ec638dc24663-0.png` (lobby KR), `images/4225a1799b64a164d0c5f31d-0.png` (prepare KR), `images/e5004c4fb85c5be40af35e82-0.png` (login VN), `images/ad1683ba3b1c388a7a9894af-0.png` (lobby VN), `images/1fc4e2555d111efe70cb4d9f-0.png` (prepare VN) |
| R-091 | Quyết định V1 về nền: **không** đưa nền màn hình vào atlas (bộ giải mã bundle chỉ đọc sprite I16, và nền còn chữ/nhãn ngoài scope FFA) ⇒ client **dựng nền bằng màu**, chữ lấy từ catalog. Đưa nền vào bundle cần bộ giải mã JPEG + bản ảnh đã che chữ — quyết định kiến trúc, thuộc integrator | — | `design` / design | `crates/boom-assets/src/closure.rs` (`UI_ASSETS`: `login.background`, `lobby.background`, `room.background` → `excluded` kèm lý do); [BUNDLES §8](../assets/BUNDLES.md#8-giới-hạn-đã-biết); [UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1) |
| R-092 | Ảnh UI của luồng V1 **có chữ nằm trong ảnh** (8 ảnh được kiểm ở closure KR ban đầu, trước override Việt hóa PR #241): nút tạo phòng `FxStage/Lobby/LB_Bt_T_Create-NM` (9 frame), dialog tạo phòng `FxStage/Lobby/Dlg_LB-NMCreate`, nút ready `FxMap/Prepare/PRE_Bt_Ready` (6 frame), nút start `FxMap/Prepare/PRE_Bt_Start` (6 frame), dialog thông tin phòng `FxMap/Prepare/Dlg_PRE-NMRoominfo`, art kết quả `FxStage/Game/Win`/`Lose`/`Draw`. Chữ đọc **theo từng asset**: 5 ảnh đầu — KR chữ Hàn ("방만들기", "준비", "시작", "방정보변경"), bản VN chữ Việt đọc được ("TẠO PHÒNG", "SẴN SÀNG", "BẮT ĐẦU", "THÔNG TIN"); art kết quả — KR chữ **Latin** "WIN!!"/"LOSE!!"/"DRAW!!", còn **bản VN chưa đọc rõ** (frame 0 chỉ đọc được "THN"/"SC"/"HO", chuỗi frame chưa ghép đủ) ⇒ phần chữ VN của art kết quả **chưa xác nhận** | — | `kr_static` + `vn_static` / confirmed (chỉ phần đã đọc trực tiếp) | Entry có thật trong pack KR 00967, đã đóng gói trong atlas bundle (`[[sprites]].source_sha256` cho cả 8; SHA-256 ở [`kr_v1_content.json`](../../evidence/inventory/kr_v1_content.json) cho 4 ảnh nút/dialog); ảnh giải mã đọc trực tiếp trong cache review ảnh của `boom-re-l10n` — [UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1); `crates/boom-assets/src/closure.rs` (`UI_ASSETS`) |
| R-093 | Quyết định V1 về ảnh có chữ: ba trạng thái bundle là `selected`/`needs_review_text`/`excluded`. Phần tử chưa duyệt dùng text catalog; manifest Việt hóa đã duyệt chọn nguồn VN/PNG và chuyển sáu phần tử sang `selected` khi áp dụng. `Win`/`Lose` vẫn `excluded`, dùng text “Thắng”/“Thua”; PNG Draw chỉ dùng cho kết quả hòa. Nền JPEG vẫn theo R-091 | 6 sprite được áp manifest, không còn ảnh pending được render trong luồng V1 | `design` / design | [UI_STRINGS_V1 §3.2](../localization/UI_STRINGS_V1.md#32-bundle-ui-việt-hóa-đã-duyệt-2026-10-24); [IMAGE_REVIEW](../localization/IMAGE_REVIEW.md); `data/assets/ui-vi-v1.toml`; `crates/boom-content/src/ui_localization.rs`; PR #241, integration `43739e2fa6abe09f28457f218648141113cd2fb4` |

R-090/R-092 là **bằng chứng `confirmed` đọc từ static**: nền là JPEG 800×600 (đo trực tiếp trên
blob đã giải mã của pack KR 00967 và của bản VN) và ảnh UI có chữ nằm trong ảnh (đọc trên PNG
giải mã). Phần ngôn ngữ chỉ khẳng định **ở asset đã đọc**; art kết quả của bản VN **chưa xác
nhận**. Việc hai build khác byte **không** phải bằng chứng về chữ. R-091/R-093 là **quyết định
`design` của V1**: không đưa nền vào atlas, và vẽ text từ catalog tới khi ảnh được duyệt.

Trạng thái duyệt ảnh là **quy trình của dự án** (`images_review.csv` ở `boom-re-l10n`: 3.038
ứng viên, tất cả `pending`), **không** phải thuộc tính client gốc; theo dõi ở
[Q-012](../../research/QUESTIONS.md). Không claim **nội dung** chữ trong ảnh: phần đọc chữ chỉ là
**bằng chứng mô tả** của [UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1),
không phải bản dịch — bản dịch chuẩn là `catalog.csv` của `boom-re-l10n`.

## 13. RE KR 00967 — movement và va chạm (R-119…R-125)

Đọc tĩnh client KR `00967` cho [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) / Q-017.
Định danh build (SHA-256 `cf72da11…e63b`, imagebase `0x400000`, `RVA = VA − 0x400000`) và bảng VA/RVA ở
[evidence KR static 2026-10-21](../../evidence/measurements/kr-static-2026-10-21/README.md);
cách đối chiếu VN↔KR, dẫn xuất trợ góc và giới hạn ở
[KR_RE_MOVEMENT](../../research/findings/KR_RE_MOVEMENT.md). Nguồn `kr_static`; `confirmed` chỉ khi đã
thấy consumer trong logic game.

**Phạm vi lớp (đọc kèm mọi claim dưới đây).** Chuỗi consumer của `CMovable::MoveSimple` `0x01310200`
đã trace — `0x00B95180 → 0x00B93830`, rồi `0x00B93830` gọi cổng `0x00B95970` và mover `0x00B93890` —
thuộc `CGOPet`/AI theo R-134/R-135 của PR #185 (`re-kr-trapped-speed`, **đang mở, chưa có trên
main**). Vì vậy R-119…R-122 `confirmed` **chỉ trong phạm vi `CMovable`/consumer đã trace**;
áp dụng hộp 40×40 và trợ góc cho **bomber người chơi là `candidate`/chưa chứng minh**. Đây **không**
phải khẳng định bomber không dùng hay không tồn tại đường gián tiếp/wrapper từ bomber — chỉ là
**chưa chứng minh cho bomber**. Không dùng các claim này làm luật người chơi `confirmed`.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-119 | Trong `CMovable::MoveSimple` (consumer đã trace: `CGOPet`/AI — xem phạm vi lớp), hộp va chạm là **hộp bàn chân đúng một ô**: `rect(x, y−39, 40, 40)` px. Neo `y` là mép **dưới** hộp nên `y = hàng·40 + 39`, `x = cột·40` khi đã thẳng ô. Các truy vấn ô trong bước đi suy từ hộp này: `x/40`, `(x+39)/40`, `(y−39)/40`, `y/40`. Áp dụng cho bomber người chơi: **candidate**/chưa chứng minh | 40×40 px, neo `y−39` / `x` | `kr_static` / confirmed (chỉ `CMovable`, consumer đã trace; bomber: `candidate`) | Consumer `CMovable::MoveSimple` KR `0x01310200` (RVA `0x00F10200`): dựng `rect(x, y−39, 40, 40)` rồi lấy bốn chỉ số ô; hằng số đoạn đường `w+h−80` (80 = 40+40). Đối chiếu VN cùng hàm `0x0095199B`; VN đã có R-009 cho neo `hàng·40+39`. Đường gọi đã trace: `0x00B95180 → 0x00B93830` → cổng `0x00B95970` → mover `0x00B93890` (`CGOPet`/AI theo R-134/R-135 PR #185 pending) |
| R-120 | Hướng di chuyển trong bảng vector của `CMovable` mã hoá `0..3` = **trái, lên, phải, xuống** với `(dx,dy)` = `(-1,0) (0,-1) (1,0) (0,1)` chỉ số `2·dir`. Bảng/hướng dùng trong `CMovable` (consumer đã trace: `CGOPet`/AI); việc bomber dùng cùng thứ tự khi dịch chuyển **chưa chứng minh** (dispatch dir của bomber ghi ở R-123) | 4 hướng, thứ tự 0/1/2/3 | `kr_static` / confirmed (chỉ bảng hướng trong `CMovable`/consumer đã trace; bomber movement: `candidate`/chưa chứng minh) | Consumer: `CMovable::MoveSimple` `0x01310200` (đẩy lùi theo `2·dir`), mover `0x00B93890` (tính ô đích; `CGOPet`/AI, R-134/R-135 pending), bảng `0x017929B0` |
| R-121 | Bước đi không vượt vật cản: hộp bàn chân được kéo dài theo hướng đi thêm `len` px (rect quét), cắt theo hình học với vật cản, rồi `nLen = w+h−80` là quãng đường **thực sự đi được**; vị trí cập nhật theo `nLen·(dx,dy)`, `len` bị trừ `nLen`, phần dư trả về cho vòng lặp ngoài. Cơ chế đọc trong `CMovable` (consumer đã trace: `CGOPet`/AI); chưa chứng minh cho bomber | px, không vượt | `kr_static` / confirmed (chỉ `CMovable`, consumer đã trace; bomber: `candidate`) | Consumer `CMovable::MoveSimple` `0x01310200` (nhánh `nLen ≥ 0` cập nhật vị trí; nhánh âm là lỗi nội bộ) + mover `0x00B93890` (`CGOPet`/AI, R-134/R-135 pending) |
| R-122 | Trợ góc: chỉ chạy khi hộp **đã thẳng hàng ô trên trục di chuyển** (`x%40 == 0` khi đi ngang, `(y−39)%40 == 0` khi đi dọc); khi đó trượt **vuông góc** để căn hộp 40 px vào một làn trống. Với `r = x%40`: `r < 6` → không trượt (trừ khi đang ở làn trái trống thì trượt trái `r`); `6 ≤ r < 34` → trượt trái `r` px nếu làn hiện tại trống, trượt phải `40−r` nếu làn hiện tại bị chặn; `r ≥ 34` → trượt phải `40−r`. Không trượt nếu làn đích bị chặn, nên trợ góc **không** cho xuyên ô. Đọc trong `CMovable` (consumer đã trace: `CGOPet`/AI); áp dụng cho bomber: **candidate**/chưa chứng minh | tối đa 34 px/bước; ngưỡng `r ≥ 34`, `r < 6` (viết theo tâm: `14`/`26` quanh `x+20`) | `kr_static` / confirmed (chỉ `CMovable`, consumer đã trace; bomber: `candidate`) | Consumer mover `0x00B93890` (`CGOPet`/AI, R-134/R-135 pending) gọi `0x01310C50` (kiểm thẳng hàng) rồi slot vtable `+4`/`+8` = `0x01310D30` (trượt X) / `0x01310EE0` (trượt Y); ngưỡng đọc trong `0x01310D30` |
| R-123 | Input di chuyển đi vào **controller của bomber** (`controller->vf3(dir)`, slot 3 / byte +12); đây là phần **duy nhất** `confirmed` cho bomber trong phạm vi đã trace. Chuỗi `0x00B95180 → 0x00B93830` (rồi cổng `0x00B95970`, mover `0x00B93890`) **không** phải mô phỏng bomber: R-134/R-135 của PR #185 (pending) cho thấy nó thuộc `CGOPet`/AI, nên không gộp thành quỹ đạo bomber. Claim **không** khẳng định KR không gửi gói C→S mang vị trí | — | `kr_static` / confirmed (chỉ dispatch dir vào controller bomber); mô phỏng bomber và phủ định mạng: **chưa chứng minh**/`candidate`, phạm vi ghi ở Q-017 | `CGameContext` input dispatch KR `0x00E13D00` (lấy bomber theo slot → controller là sub-object nhúng tại `(byte*)bomber + 1412` → `vf3(dir)`, slot 3 / byte +12, case 4 đặt bom); VN tương ứng `0x006E5E05`. Chuỗi `0x00B95180 → 0x00B93830` → `0x00B95970`/`0x00B93890` → `0x01310200` là của `CGOPet`/AI (R-134/R-135, PR #185 pending) — **không** dùng làm bằng chứng mô phỏng bomber. **Không** dùng R-047 (`vn_static`, build VN) làm bằng chứng phủ định cho KR |
| R-124 | Trong `CMovable::MoveSimple` (consumer đã trace: `CGOPet`/AI), tầng va chạm quyết "ô đi được" bằng truy vấn lưới + **lớp tile** (RTTI `CGOTileFixed` / `CGOTilePassable`), không theo ID tile cứng; truy vấn chặn gộp nhiều vị từ (biên map, cờ ngữ cảnh, object trên ô). Áp dụng cho bomber: **candidate**/chưa chứng minh | — | `kr_static` / candidate | Consumer `CMovable::MoveSimple` `0x01310200` gọi `0x00EA0FD0(row, col, ctx)`; `0x00E9F9B0(row, col)` lấy object ô rồi kiểm lớp tile. **Giới hạn**: polarity "Fixed = chặn" chưa chốt; và consumer đã trace thuộc `CGOPet`/AI (R-134/R-135 pending) |
| R-125 | Chưa xác định consumer quy đổi **cấp tốc độ → px mỗi bước**. `CMovable::MoveSimple` chỉ nhận `len` từ caller. Chuỗi `0x00B95180 → 0x00B93830` (cổng `0x00B95970`, mover `0x00B93890`) chuyển `len` nguyên vẹn nhưng **thuộc `CGOPet`/AI** (R-134/R-135, PR #185 pending); phép quy đổi nằm sau `vf[+8]` (byte +8 = slot 2) của controller sub-object `+0xE0` **của `CGOPet`**, không phải bomber. Với bomber, mover/`len` **chưa định vị** | — (không mang giá trị) | `kr_static` / unknown | `0x00B95180` nhận `(dir, len)` từ `vf[+8]` (byte +8 = slot 2) của sub-object `+0xE0` (`CGOPet`); ctor `CGOBomber`/`CGOGameBomber` không gán `+0xE0` theo R-134. Bomber: dispatch `0x00E13D00` → controller nhúng `(byte*)bomber + 1412` → `vf3(dir)` (slot 3 / byte +12) chỉ chứng minh nhận hướng. Bước tiếp: đọc slot 24 bomber `0x00B44000`/`0x00B77B10` tìm mover/setter vị trí; nếu thân hàm ở `.themida` thì dừng và ghi entry VM |

Trong phạm vi `CMovable`/consumer đã trace (pet/AI), R-119 và R-122 khác thông số va chạm đang dùng ở
R-116 (`13` px nửa hộp = 26×26, assist `12` px): hộp đúng một ô 40×40 và trợ góc là phép căn làn tối đa
34 px, không phải nới hộp. **Không** dùng để kết luận cho bomber người chơi: áp dụng cho bomber chỉ là
`candidate`/chưa chứng minh, nên R-116 vẫn là giá trị profile hiện hành; nếu muốn thử nghiệm thì để PR
riêng theo ADR 0013, không phải luật `confirmed`. Bảng khác biệt ở
[KR_RE_MOVEMENT §6](../../research/findings/KR_RE_MOVEMENT.md#6-khác-biệt-với-profile-hiện-tại-và-với-bnb).
R-125 giữ nguyên R-114 (`24 px/s/level`, `oss`/design) vì chưa có consumer KR.

R-134/R-135 nêu ở mục này là **tham chiếu PR #185 đang mở**, chưa có trên `main`; chỉ khi #185 merge
mới trở thành claim trong nguồn chuẩn. Không dùng chúng để thay R-119…R-125 trong PR này.

## 14. RE KR 00967 — bị nhốt (R-126…R-131)

Đọc tĩnh client KR `00967` cho [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / Q-018.
Định danh build (SHA-256 `cf72da11…e63b`, imagebase `0x400000`, `RVA = VA − 0x400000`) và bảng VA/RVA ở
[evidence KR static trapped](../../evidence/measurements/kr-static-trapped-2026-10-21/README.md);
suy luận, giới hạn và cách đối chiếu ở [KR_RE_TRAPPED](../../research/findings/KR_RE_TRAPPED.md).
Dãy số bắt đầu ở R-126 vì R-119…R-125 (movement) đã có trên `main` ở §13; mục này không sửa các claim
movement. R-134/R-135 (phạm vi lớp `CGOPet`) ở §15 (PR #185).

**Phạm vi lớp (bổ sung sau review #157):** chuỗi `0xB95180 → 0xB93830 → (0xB95970, 0xB93890)` và state
machine `0xB95xxx` có consumer đã trace là **`CGOPet`** (`0xB93A60` = slot 24 `??_7CGOPet@@6B@`). Đây
**không** phải bằng chứng cho bomber người chơi, và cũng **không** loại trừ đường gián tiếp/wrapper/VM
từ bomber — chỉ giới hạn phạm vi đường đã trace. R-134/R-135 ở §15 (PR #185):
tham chiếu pending, chưa có trên `main`.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-126 | **Trong cụm `0xB95xxx` (consumer đã trace: `CGOPet`):** các số state `{0,1,2,3,4,8,9..12,13,14,15..18,19..22}` và điều kiện/chuyển state đã đọc: `sub_B952E0` đặt `3` khi `sub_B95AC0(this) != 0`, `state != 3`, `sub_B956C0(this,-1) == 0` và state không thuộc `{13,14,15..18,19..22}`; khi `sub_B95AC0(this) == 0` và `state == 3` thì đặt `2`; `13→14→2`; `sub_B95840` chọn `dir+9`/`dir+19`; `sub_B95BD0` đặt bộ đếm `20` — **confirmed trong phạm vi pet**. **Nhãn ngữ nghĩa** `3` = "bị nhốt", và các nhãn gameplay khác, là **candidate**: chưa có enum/consumer chung định nghĩa `sub_B95AC0`/`sub_B956C0`/virtual `+56`/`+60`; trùng số `3` với packet VN không chứng minh cùng nghĩa. **Claim riêng của bomber:** packet C→S VN 120 subtype 11 = state 3 là `vn_runtime`/confirmed (R-026), **không** ghép sang pet | — | `kr_static` / confirmed (số state, điều kiện, chuyển state trong cụm `CGOPet`); `kr_static` / **candidate** (nhãn "3 = bị nhốt" của cụm pet — chưa có enum/consumer chung); `vn_runtime` / confirmed (packet bomber state 3, claim riêng R-026, không ghép sang pet) | Consumer `sub_B952E0` KR `0x00B952E0`; `sub_B95840` `0x00B95840`, `sub_B95BD0` `0x00B95BD0`, hàm `sub_B95AC0` `0x00B95AC0`, `sub_B956C0` `0x00B956C0`. Nhãn "bị nhốt" **không** có trong binary; packet VN 120/11 ở [`VN_FEATURES_REFERENCE`](../../../docs/research/findings/VN_FEATURES_REFERENCE.md) §5b là ngữ cảnh bomber, claim riêng R-026. **Phạm vi lớp:** `sub_B952E0` được gọi từ `0xB93A60` (`CGOPet` slot 24); chưa có consumer thuộc lớp bomber người chơi |
| R-127 | Cổng input/di chuyển cho state `3` **chưa chứng minh cho bomber người chơi**: chuỗi đã trace `sub_B95180` → `0xB93830` (rồi `0xB93830` gọi cổng `sub_B95970` và mover `0xB93890`) nằm trong đường cập nhật `CGOPet` (`0xB93A60` = slot 24 `??_7CGOPet@@6B@`); **trong phạm vi `CGOPet`**, `sub_B95180` chỉ bỏ qua state `13`,`14`,`1` và `sub_B95970` chỉ loại `{0,1,4,13,14}`, nên state `3` qua cổng. Với bomber người chơi **chưa** có consumer thuộc lớp bomber; `len`/mover **unknown** — không suy ra "bị nhốt đi được" | — | `kr_static` / **candidate** (bomber); quan sát `CGOPet` chỉ trong phạm vi lớp, **không** nâng thành luật người chơi | `sub_B95180` `0x00B95180`, `0xB93830`, `sub_B95970` `0x00B95970`, `0xB93890`. `0xB93830` gọi cổng **trước** rồi mover — không phải `0xB95180 → 0xB95970 → 0xB93830`. `(dir,len)` là out-param của virtual call tại **byte +8** (slot 2 trên x86) của controller pet `*(pet+0xE0)` (getter `0x00B96310`/`0x00B971C0`); **không** gán `+0xE0` cho bomber — input dispatch bomber dùng sub-object **nhúng** tại byte `+1412`. Thân virtual byte +8 chưa định vị nên **không** suy ra `len > 0` cho state `3`. Đối chiếu `web`/NAMU R-029 "đi rất chậm" cũng chỉ là candidate |
| R-128 | Họ state đi được chọn theo nhóm: đang ở nhóm khoá (`15..18` hoặc `19..22`) thì đặt `dir+19`, ngược lại `dir+9`; đổi nhóm xong thì state là `22` (mode khoá) hoặc `8` (mode thường) | 4 hướng × 2 họ | `kr_static` / confirmed | `sub_B95840` `0x00B95840` (chọn `a2+9`/`a2+19`), `sub_B95E00` `0x00B95E00` (nhận diện nhóm), `sub_B95BD0` `0x00B95BD0`. Phạm vi lớp: cùng cụm `0xB95xxx`, consumer đã trace là `CGOPet`; chưa chứng minh cho bomber người chơi |
| R-129 | Đổi nhóm khoá đặt **bộ đếm giữ 20 bước cập nhật** (`this[235]` hoặc `this[236] = 20`), giảm 1 mỗi bước; trong lúc `> 0` cổng di chuyển trả `false` ⇒ đóng di chuyển 20 bước. Ở 30 ms/bước (R-013) tương đương ≈ 600 ms — **quy đổi, chưa đo trên KR**, và **không** phải thời gian bị nhốt | 20 bước (≈600 ms quy đổi) | `kr_static` / confirmed (cơ chế); đơn vị ms là quy đổi | `sub_B95BD0` `0x00B95BD0` (đặt `20`), `sub_B952E0` `0x00B952E0` (giảm), `sub_B95DA0`/`sub_B95DD0` (đọc), `sub_B95970` (dùng làm cổng). Phạm vi lớp: cùng cụm `0xB95xxx`; chưa có consumer thuộc lớp bomber người chơi |
| R-130 | Chuỗi chết: state `13` (đang chết) → gọi `vf128` + `sub_B936F0(this[59], this[58])` → state `14`; state `14` → (nếu `this[234] == 1` thì đổi nhóm khoá về mode thường) → state `2`. `this[58]`/`this[59]` chưa xác định nghĩa | — | `kr_static` / confirmed (chuỗi state); nghĩa hai trường con `unknown` | `sub_B952E0` `0x00B952E0` case `13`/`14`. Phạm vi lớp: cùng cụm `0xB95xxx`; chưa chứng minh là chuỗi chết của bomber người chơi |
| R-131 | **Chưa định vị** timer nhốt → chết. Client có tên hàng đợi defer `kCGameStage_DeferBomberLockedIn` (`0x017953C8`) và `kCGameStage_DeferBomberReleaseOrDie` (`0x01795074`) nhưng bảng tên chỉ đăng ký id, không trỏ hàm; khoảng packet 6359 ms (R-026) **không** phải timer nội bộ | — (không mang giá trị) | `kr_static` / unknown | Bảng tên tại `0x01795074`, `0x017953C8`; xref duy nhất là bảng đăng ký. Bước tiếp: tìm bộ thực thi defer theo id rồi lấy case tương ứng; nếu thân hàm ở `.themida` thì dừng và ghi entry VM theo DEVIRT_METHOD |

R-127 **không** còn là bằng chứng cho bomber người chơi: chuỗi cổng `0xB95180 → 0xB93830 →
(0xB95970, 0xB93890)` thuộc đường cập nhật `CGOPet` (R-134/R-135, §15), và
`len`/mover của bomber vẫn `unknown`. Vì vậy mô hình `trapped` đứng yên của R-036/R-108 **không bị phủ
định** bởi lượt đọc này; chỉ ghi nhận quan sát trong phạm vi `CGOPet`. Giá trị profile hiện hành
(`player.trap_to_death_ms`, `player.trap_touch_distance_px`) **không đổi** trong PR này — và R-129
**không** được dùng làm `trap_to_death_ms`: 20 bước là bộ đếm chuyển nhóm trình diễn, không phải thời
gian bị nhốt. Bảng khác biệt và đề xuất ở
[KR_RE_TRAPPED §7](../../research/findings/KR_RE_TRAPPED.md#7-khác-biệt-với-profile-hiện-tại-và-với-bnb).

## 15. RE KR 00967 — phạm vi lớp của chuỗi movement (R-134…R-137)

Truy vết bổ sung cho R-125/R-127, Refs [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) /
[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / #176.
Định danh build và bảng VA/RVA ở
[evidence KR trapped-speed](../../evidence/measurements/kr-static-trapped-speed-2026-10-22/README.md);
suy luận, giới hạn và bước tiếp ở
[KR_RE_TRAPPED_SPEED](../../research/findings/KR_RE_TRAPPED_SPEED.md).
R-119…R-125 đã vào `main` qua PR #152; R-126…R-131 đã vào `main` qua PR #157 (§14); R-132/R-133 thuộc PR #177
đã đóng (không merge) nên không dùng. R-136 là lượt nối tiếp R-135 (dispatch dir vào sub-object `+1412`);
R-137 truy ngược caller setter vị trí và writer clamp `[+0x930]/[+0x934]`.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-134 | **Đường đã trace** `0x00B95180 → 0x00B93830` (trong `0x00B93830`: nhánh kiểm cổng gọi `0x00B95970`, rồi mover `0x00B93890 → 0x01310200` = `CMovable::MoveSimple`) là đường của `CGOPet`: `0x00B93A60` = slot 24 vtable `??_7CGOPet@@6B@` là caller trực tiếp của `0x00B95180` theo xref; slot 24 của bomber là `0x00B77B10` (`CGOGameBomber`) / `0x00B44000` (`CGOBomber`), và trong **thân hai hàm đã đọc** không thấy gọi chuỗi trên. Kết luận này **chỉ** cho đường đã trace: **không** loại trừ đường gián tiếp/wrapper/con trỏ tính toán/VM từ bomber và **không** kết luận bomber không dùng chuỗi này. Quan hệ bomber với chuỗi: **unknown** | — | `kr_static` / confirmed (đường đã trace thuộc `CGOPet`); quan hệ bomber: `unknown`/chưa chứng minh | Vtable `??_7CGOPet@@6B@` `0x01746E40` slot 24 = `0x00B93A60`; `??_7CGOGameBomber@@6B@` `0x01745FE8` slot 24 = `0x00B77B10`; `??_7CGOBomber@@6B@` `0x01744964` slot 24 = `0x00B44000`; `??_R2CGOGameBomber@@8` `0x017CB498` liệt kê cơ sở `CGOBomber→CGOBomberBase→CGOAvatar→CGameObject→CObject` |
| R-135 | Đường **đã trace** đưa hướng vào bomber người chơi là input dispatch `CGameContext` `0x00E13D00`: lấy bomber theo slot rồi gọi `vf3(dir)` của controller là sub-object **nhúng tại byte +1412** (`bomber[353]`, vtable pointer nằm ngay tại đó, ctor `0x00B73410`/`0x00B79DC0` gán vtable cho offset này). Hàm đọc lại `dir` **và** tạo/giữ `len`/mover của bomber **chưa định vị** — nên chưa có cơ sở cho "bị nhốt đi được" hay cho `cấp tốc độ → px/bước` của bomber | — | `kr_static` / confirmed (dispatch); `unknown` (len/mover của bomber) | `0x00E13D00` (dispatch), controller nhúng tại `(byte*)bomber + 1412`; `vf[+8]` (byte +8 = slot 2 x86) trả `(dir,len)` là của controller `+0xE0` **của `CGOPet`** (getter `0x00B96310`→`0x00B971C0`, `return *(this)`), không dùng cho bomber; bước tiếp: đọc slot 24 `0x00B44000`/`0x00B77B10` tìm setter vị trí |
| R-136 | **Dispatch dir → sub-object nhúng +1412 → reader**: `CGameContext` `0x00E13D00` (case dir `0..3`/`0x11..0x14`) gọi virtual slot **+12 (slot 3 x86)** của sub-object tại `bomber+1412` (`bomber[353]`), `this = bomber+1412`, arg = dir; vtable `CGOGameBomber::vftable` `0x01745FE8` +12 = `0x00B32390` (cùng slot ở `CGOBomber::vftable` `0x01744964`). `0x00B32390` ghi dir vào **protected field +0x1C** của sub-object (= `bomber+1440` = `0x5A0`) qua `0x0055FD30`, rồi ghi `0x80000000` vào `+0x70`/`+0x7C` (= `bomber+1524`/`+1536`). Reader `0x00B40D60` trả field `+1440`; giá trị được dùng làm điều kiện trong `0x00B40030`/`0x00B40290`/`0x00B405E0`/`0x00B407E0`/`0x00B40A50`/`0x00B422F0`/`0x00B42880`/`0x00B45570`. **Chưa nối** được từ reader/predicate tới setter vị trí `0x00B334C0`/`0x00B43750` hay `CMovable::MoveSimple`; đường cập nhật vị trí/`len` của bomber **chưa định vị** | — | `kr_static` / confirmed (đoạn dispatch→field→reader); `unknown` (position/mover/`len`) | `0x00E13D00` (call site `0x00E13DFD`), vtable `0x01745FE8` slot +12 = `0x00B32390`; ctor `0x00B79DC0` qua factory `0x00AECE00` (`0xAECEB6`) gán `+0x584`; ctor thay thế `0x00B73410` gọi `0x00B3BE30` overwrite `+0x584`; field `+0x1C` = `bomber+1440` (`0x5A0`), reader `0x00B40D60`; position getters `0x00B335C0`/`0x00B335E0` (`+0x4C`/`+0x58`), setters `0x00B334C0`/`0x00B33520`/`0x00B33540`/`0x00B43750`; update slot 24 `0x00B77B10`→`0x00B44000` không đọc `+0x5A0` |
| R-137 | **Setter vị trí bomber (`0x00B43750` → `0x00B334C0`) — cạnh đã đọc và lớp sử dụng**: `0xB43750` (vtable slot **+116**) gọi `0xB334C0(a2,a3)` tại `0x00B43764`; `0xB334C0` ghi protected x/y tại `+0x4C`/`+0x58`. `0xB479E0` (slot **95**, `+0x17C`, của `CGOGameBomber::vftable` `0x01745FE8`; cùng slot `CGOBomber` `0x01744964`) gọi `0xB43750(this,a3,a4)` tại `0x00B47A7B` và ghi `[this+0x930]=a3`/`[this+0x934]=a4` tại `0x00B47AD8`/`0x00B47AE4`. Direct caller của `0xB479E0` chỉ có `0x00F087E0` (`CGOHedgehogBomber` slot 95) tại `0x00F087FD`; các wrapper `0x69BD60`/`0x69FE80`/`0xF06770`/`0x11260C0` gọi trực tiếp `0xB43750` và thuộc vtable `CGONpcCaptain2Bomber`/`CGONpcCaptainBomber`/`CGOCaptainBomber`/`CGOLobbySquareBomber`/`CGOSquareBomber`/`CGOSquareNPCGuideBomber` — **chỉ các wrapper này là NPC/AI**. **Lớp sử dụng của `0xB479E0` = `unknown`**: là virtual của `CGOGameBomber` nhưng chưa truy được caller ảo; **không** gán "NPC/AI correction" cho `0xB479E0`. Giả thuyết **candidate** (không phải claim): `B479E0(a2,x,y)` đặt vị trí rồi lưu `[+0x930]/[+0x934]` để `B44000` clamp — khớp mẫu **position sync từ server cho người chơi**. **Blocker có phạm vi**: quét toàn bộ `call [vtable+0x17C]` (5 site `0xF2BDC5`/`0xF34EB7`/`0xF3EAD1`/`0x12C2596`/`0x12C2B5C`) — không thấy site nào gọi `0xB479E0` (3 site bomber-family có slot 95 tương ứng `0xF3E3C0`/`0xF34ED0`/`0xF2D7C0`; 2 site còn lại là object UI không có x/y bomber); `0xB57340`/`0xB77EA0` không gọi. Gói S→C vị trí tuyệt đối **VN opcode 176 / KR opcode 183** (`sub_706C42` / `sub_E65350`): `u8 id, u8 flags` (bit7=absolute, low7=slot), absolute thêm `u8 y, u8 x`, đặt `(40*x, 40*y+39)`; KR gọi `sub_B88800` → virtual `+116` (= `0xB43750` ở họ vtable bomber) — **không** qua slot 95/`B479E0` | — | `kr_static` / confirmed (cạnh setter/writer, wrapper NPC); `vn_static` (layout packet); `unknown` (lớp sử dụng `0xB479E0`; player input→position) | `0xB43750`→`0xB334C0`; call `0xB47A7B`, writes `0xB47AD8`/`0xB47AE4`; `B479E0` slot 95 vtable `0x1745FE8`; wrapper `0x69BD60`/`0x69FE80`/`0xF06770`/`0x11260C0`; scan slot95 `0xF2BDC5`/`0xF34EB7`/`0xF3EAD1`/`0x12C2596`/`0x12C2B5C`; packet VN `0x706C42`, KR `0xE65350` → `0xE654C2` → `0xB88800` → `0xB8898A` slot `+116`; units 40 px, `y=40*row+39`; `0xB44000` clamp `0xB44975`/`0xB44A08` |

**Blocker/evidence bổ sung R-137 (2026-10-22).** Evidence REA: `ev_ea00a69a…` (decompile setter/caller), `ev_977db7f0…` (xrefs `0xB43750`), `ev_dd433bf5…` (xrefs `0xB479E0`), `ev_ef51aa30…` (5 hàm chứa site slot 95), `ev_b68075d5…` (ghost reset), `ev_d94e551b…` (handler opcode 183). Chi tiết phạm vi quét và `evidence_id` đầy đủ ở [evidence KR trapped-speed](../../evidence/measurements/kr-static-trapped-speed-2026-10-22/README.md) và [KR_RE_TRAPPED_SPEED §8](../../research/findings/KR_RE_TRAPPED_SPEED.md).

Hệ quả cho các claim liên quan:

- **R-125** (cấp tốc độ → px/bước) vẫn `unknown`. `vf[+8]` (byte +8 = slot 2 x86) của sub-object
  `+0xE0` là của `CGOPet`, **không dùng được** làm bằng chứng cho bomber; controller bomber nhúng tại
  byte `+1412` mới chỉ chứng minh nhận `dir`.
- **R-127** (bị nhốt đi được): phần "cổng" dựa trên `0x00B95970` là của đường đã trace `CGOPet`; với
  bomber chỉ còn R-123 (input vào controller). "Bị nhốt đi được" phải giữ `candidate`.
- **R-119…R-122** (hộp 40×40, trợ góc): vẫn đúng cho `CMovable` nhưng consumer đã thấy là AI/pet;
  chưa chứng minh cho bomber người chơi.
- **R-136**: chỉ confirmed đoạn dispatch→field `+0x5A0`→reader `0xB40D60`; **chưa nối** tới setter vị
  trí `0xB334C0`/`0xB43750`, nên R-125/R-127 vẫn `unknown`/`candidate`; không suy ra mover/`len`.
- **R-137**: direct caller của setter là wrapper NPC captain/square; `B479E0` là slot 95 của
  `CGOGameBomber` nhưng **lớp sử dụng `unknown`** (chưa có caller ảo). `[+0x930]/[+0x934]` do chính
  `B479E0` ghi từ x,y của caller, không phải dispatch dir. Quét 5 site slot 95 không site nào trỏ
  `B479E0`; packet VN 176/KR 183 dùng slot `+116` (= `B43750`), không slot 95. Candidate: position
  sync server cho player. Player input→position vẫn **unknown**, không đổi R-125/R-127.
- **PR #177** đã đóng, không merge: R-132/R-133 không có trong nguồn chuẩn nên không còn việc hạ nhãn.

## 16. RE KR 00967 — packet vị trí S→C và setter `CGOGhost` (R-138)

Tiếp nối R-137, Refs [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) /
[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147). Build KR `00967`; đối chiếu VN
bằng docs/IDB bản sao (không mở DB gốc). Chi tiết và phạm vi quét ở
[KR_RE_TRAPPED_SPEED §9](../../research/findings/KR_RE_TRAPPED_SPEED.md) và
[evidence KR trapped-speed](../../evidence/measurements/kr-static-trapped-speed-2026-10-22/README.md).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-138 | **Packet S→C vị trí KR opcode 183 → `CGOGhost` → setter `0xB334C0`**: `sub_E5D3A0` (dispatch kiểu `CGameStage__OnPacket`, case = opcode−20) đưa opcode 183 tới `0xE5D65B` → `sub_E65350`. Layout: `u8 id` (0..15), `u8 flags` (bit7 = absolute; low7 = `slot`); nếu absolute đọc thêm `u8 y`, `u8 x`. `id` tra `stage + 0x20 + 122628*id` = **`CGOGhost`** (ctor `0xB886B0`, vtable `0x174698C`); `slot` lưu `CGOGhost+0x9C` và dùng để khớp controller slot `[bomber+1412]` (getter `+40`). Nhánh absolute gọi `sub_B88800(ghost, id, slot, 40*x, 40*y+39)` tại `0xE654C2` → virtual `+116` = **`0xB334C0`** tại `0xB8898A` (vtable `0x174698C` slot 29 = `0x1746A00`), ghi protected `+0x4C`/`+0x58`; **không** gọi `0xB43750`/`0xB479E0`. Nhánh bit7=0 chỉ ghi `slot` vào `CGOGhost+0x9C` qua `sub_E92110` (`+0x9C` = `slot`), không đổi vị trí. Handler/`sub_B88800` **không** đọc dir field `+1440`/reader `0xB40D60` và không gọi `0xB32390`; chỉ dùng `+1412` để lấy slot. C→S ứng viên: opcode **149** (`sub_F158C0`: `u16, u32 x, u32 y, u8, u8`) và **148** (`sub_E733D0`: `u32, u8 n, u16, u16, u8, u32, u16, n×(u8,u8)`); chưa chứng minh cặp S→C 183. Ngữ nghĩa **candidate**: server position sync/teleport cho ghost người chơi, không phải input dir | — | `kr_static` / confirmed (layout, cạnh packet→`0xB334C0`, hai nhánh); `vn_static` (VN 176 layout/docs, server giả lập không gửi 176); `candidate` (ngữ nghĩa sync, cặp C→S); `unknown` (runtime cadence/authority, quan hệ với input/di chuyển) | `sub_E5D3A0` case 183 → `0xE5D65B` → `sub_E65350` (`0xE654C2`); `sub_E15D60` (`stage+0x20+122628*id`, id<16); `sub_B886B0` ctor/vtable `0x174698C`; `sub_B88800` call slot `+116` `0xB8898A`; `0xB334C0` `+0x4C`/`+0x58`; `sub_E92110` `+0x9C`; C→S `sub_F158C0` (149) / `sub_E733D0` (148); evidence `ev_...` ở findings §9.6 |

Hệ quả:

- **Đã nối**: opcode 183 → `CGOGhost` theo `id` → đặt vị trí tuyệt đối qua `0xB334C0`; đơn vị từ
  handler là `x = 40*byte`, `y = 40*byte + 39` (ô 40 px). Đây là **packet→setter**, không phải
  packet→input dir.
- **Không nối**: packet → field `+0x5A0`/reader `0xB40D60`/`0xB32390` → input dir. R-136/R-137 giữ
  nguyên; không suy trapped/px/s/nhịp server.
- **C→S**: có sender 148/149 trong client, layout ghi ở claim; **chưa** chứng minh chúng là cặp của
  S→C 183. VN 141/142 có layout khác; không suy opcode/slot giữa hai build.
- **VN server giả lập**: không gửi 176 (không thấy trong `server/src`), docs ghi không có handler
  S→C 141/142 và di chuyển đồng bộ qua UDP P2P — chỉ `vn_static`, không phải `vn_runtime`.

## 17. RE KR 00967 — nổ lan, vật cản và dây chuyền (R-139…R-141)

Đọc tĩnh client KR `00967` cho [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148) / Q-019
(mục 5 hàng đợi RE). Định danh build (SHA-256 `cf72da11…e63b`, imagebase `0x400000`,
`RVA = VA − 0x400000`), bảng VA/RVA, `evidence_id` và giới hạn công cụ ở
[evidence KR static flame-chain](../../evidence/measurements/kr-static-flame-chain-2026-10-22/README.md);
suy luận, khác biệt profile và bước tiếp ở
[KR_RE_FLAME_CHAIN](../../research/findings/KR_RE_FLAME_CHAIN.md). Nguồn `kr_static`; `confirmed` chỉ
cho phần đã thấy consumer. Dãy bắt đầu ở R-139 vì R-134…R-137 ở §15, còn §16/R-138 đã vào `main` qua PR #192
(phiên dsh 1) — không dùng trùng mục/ID.

**Ranh giới client/server (đọc kèm mọi claim dưới đây).** Gói nổ của bản VN (`S→C 151`) chỉ mang
id/kind/arg của bom, **không** mang danh sách ô (`vn_runtime`; `oc_e-game` §5a-5b, F5 ở
[VN_FEATURES_REFERENCE](../../research/findings/VN_FEATURES_REFERENCE.md)). Tầm nổ khởi đầu 1 và +1 mỗi
ItemFire là R-025 (`vn_runtime`/confirmed), khớp nghĩa "tầm = số ô" của slot `+20`. KR có code
tính vùng nổ ở client và gọi từ lớp bom (dưới đây), nhưng **handler gói nổ S→C của KR chưa định vị**
trong lượt này — không suy opcode từ VN. Vì vậy: server quyết **bom nào nổ và lúc nào** (chưa chứng
minh cho KR), client quyết **hình học ô cháy** (đã thấy consumer).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-139 | **Vùng nổ do client tính — `CPlayMap::CalcBombDirectionExplosion`**: lấy từ chính quả bom (slot `+12` = **hàng**, `+16` = **cột**, `+20` = tầm nổ, `+24` = kiểu). Kiểu `1`: 4 hướng × `j = 1..tầm` theo bảng `0x017929B0` = `[-1,0, 0,-1, 1,0, 0,1]` (`[2i]` = delta **cột**, `[2i+1]` = delta **hàng**; ô truy vấn theo thứ tự `(hàng, cột)`) ⇒ `i=0` **trái**, `1` **lên**, `2` **phải**, `3` **xuống**, khớp R-120. Ô ra ngoài map thì dừng. **Mỗi ô trong map được đăng ký vào bom TRƯỚC khi kiểm chặn**, rồi vòng lặp dừng sau ô chặn đầu tiên ⇒ ô chặn (tường/gạch) **thuộc** vùng nổ và tia **không xuyên**. Kiểm chặn theo thứ tự: `sub_EA1230` (object ô thuộc set lớp `map+7460`) → `sub_EA0B10` (set `map+7508`) → object tại ô có `sub_E90A80 != 0` và cờ theo hướng `sub_BA0F80(dir) == 0` (bảng 4 byte `*(context[43])+32..35`; nghĩa "cho xuyên" là **giả thuyết**, chưa xác định); kiểm thêm ô kế với cờ đối diện `sub_BA0EC0(dir) == 0`. Kiểu `2`: **vuông đầy** `cột±tầm × hàng±tầm`, không kiểm chặn. Đơn vị: **ô** (1 ô = 40 px). Consumer: gọi trực tiếp từ lớp bom `0x00B384E0`, `0x00B38A80`, `0x00B393D0`, `0x00B39AF0`, `0x00B3A3B0`, `0x00B3A790`, `0x00B76CD0`, `0x00B994C0`, `0x00B9AFA0`, `0x00E14360`, `0x00F0FD20` | ô, tầm = số ô; ô chặn vẫn thuộc vùng | `kr_static` / confirmed (hình học ô + thứ tự chặn + nhãn hướng khớp R-120); nghĩa nhánh phụ, nội dung 2 set lớp và nghĩa cờ hướng: **unknown** | Hàm `0x00E9C520` (RVA `0x00A9C520`, thân `0x89C`); bảng hướng `0x017929B0`; đăng ký ô `0x00EBD3F0` (message id 27); gạch `0x00E9FCF0` (`CGOTileFixed`/`CGOTileFixedBox`), tường `0x00E9FA90` (`CGOTileFixed`/`CGOTileStatic`); cờ xuyên `sub_BA0E30` đọc `*(context[43]) + 32..35`; evidence `ev_4fd7d146…` |
| R-140 | **Dây chuyền do client tính — `CPlayMap::CalcChainExplosion_Direction`**: tại ô `(hàng,cột)` lấy danh sách object; với từng object có tầm `vf+20` và kiểu `vf+24 == 1`, lấy hướng `sub_149B5E0(&điểm)` và chỉ số bắt đầu `sub_14708A0()+1`, rồi `j = bắt đầu..tầm` tính ô theo **cùng** bảng `0x017929B0` từ điểm của object và **đăng ký ô vào chính object đó** (`sub_EBD3F0`), dừng theo cùng bộ kiểm chặn như R-139. Nghĩa: khi lửa chạm ô có bom khác, client tự tính phần vùng nổ của bom đó và đăng ký — không cần server gửi danh sách ô. Gọi từ `0x00E84650` (2 site), `0x00EA5280`, `0x00EA56C0`, `0x00B92E30`, `0x00B9CEC0`, `0x00B9FDD0`, `0x006D60A0` | ô (cùng đơn vị R-139) | `kr_static` / confirmed (client tính + đăng ký ô dây chuyền); công thức hướng/điểm bắt đầu (`sub_149B5E0`/`sub_14708A0`): **candidate** | Hàm `0x00E9CDC0` (RVA `0x00A9CDC0`, thân `0x33D`); iterator object tại ô `sub_ED8DE0`/`sub_EDA940`; evidence `ev_d667d89c…` |
| R-141 | **Ô nổ đăng ký message id 27; thời gian lửa = số frame animation; `2500` không phải fuse**: mỗi ô được đăng ký thành message **id 27** gửi cho quả bom (`sub_EBD3F0` → `sub_EDB7E0` → `sub_EC2420(object,msg)` push vào list message của object). Việc **tiêu thụ** message (áp damage) nằm ở consumer **chưa trace**, nên **không** suy mô hình damage (một lần hay cửa sổ thời gian) từ đoạn này. Thời gian sống **hiển thị** của `CGOFlame` (vtable `0x01745EF0`, ctor `0x00B70A80`, object 584 B) là **số frame**: mỗi segment có bộ đếm `flame[i+51]` tăng 1 mỗi update (`0x00B711C0`), xoá khi `>= sub_B333D0(anim)` = `CGameObjectTemplate::GetAnimCount(objNo)` (`0x00BA3FF0`). **Chưa** thấy hằng ms cho vùng damage; **không** thấy fuse trong client. `2500` chỉ xuất hiện ở `sub_E15810`/`sub_E15840` (ngưỡng so `sub_E131A0(stage)`) và nhánh chống hack `sub_E78350`, **không** phải fuse | — (không mang ms); animation = số frame/segment (quy đổi 30 ms/frame theo R-013 **chỉ là quy đổi**, chưa đo nhịp update KR) | `kr_static` / confirmed (mỗi ô → message id 27 vào list message của object; thời gian hiển thị = số frame `GetAnimCount`; `2500` không phải fuse); **candidate/unknown** (mô hình damage: sự kiện vs cửa sổ thời gian; consumer msg 27 chưa trace; fuse origin; nghĩa `desc+168`) | `sub_EBD3F0` `0x00EBD3F0`; `sub_B711C0` `0x00B711C0`; `sub_B333D0` `0x00B333D0` → `0x00BA3FF0`; `CGOBomb::Init` `0x00B384E0` đọc `desc+168/172` (u64) → bomb `[+98]/[+99]`, `desc+96` → slot `+104` (nghĩa chưa xác định, **không** phải ô); `CGOBomb` update `0x00B391E0` không có fuse timer; evidence `ev_a86e37b2…`, `ev_bd381d4f…` |

Hệ quả:

- **Không đổi `data/rules` trong PR này.** `bomb.flame_ms = 500` (R-115) là `oss`/design; KR cho thời
  gian sống theo **frame** của template, không phải hằng 500 ms, nên chỉ map sang ms khi có nhịp
  update KR. `bomb.fuse_ms = 2500` và `player.trap_to_death_ms = 4000` giữ design; **không** lấy
  2,5 s của emulator làm fuse gốc (R-021/R-022 vẫn unknown/candidate).
- R-139/R-140 **không** phủ định claim movement (§13): nhãn hướng đọc ở hàm nổ **khớp** R-120
  (`0` trái, `1` lên, `2` phải, `3` xuống) và khớp thứ tự `(hàng, cột)` của `0x00E9F9B0`/`0x00EB7190`.
- Phần còn thiếu có phạm vi: handler gói nổ KR, consumer message id 27, chỗ kiểm bomber trong ô lửa
  (damage window), nội dung 2 set lớp chặn và thân `sub_149B5E0`/`sub_14708A0` — ghi ở
  [KR_RE_FLAME_CHAIN §7](../../research/findings/KR_RE_FLAME_CHAIN.md#7-giới-hạn-và-bước-tiếp).

## 18. RE KR 00967 — BomberAttr → consumer Bomb/Fire/Roller/speed (R-150, blocker)

Tiếp nối #149/Q-020. Claim **R-150** thuộc dải phiên 1 **R-150…R-199** (phiên 2 dùng R-200…R-249;
R-139…R-141 thuộc #193); mục này dùng §18. Build KR `00967`; REA MCP bản sao
`kr-full.exe`, đối chiếu VN bằng IDB bản sao. Chi tiết/phạm vi ở
[KR_RE_BOMBERATTR](../../research/findings/KR_RE_BOMBERATTR.md) và
[evidence KR item](../../evidence/measurements/kr-static-item-2026-10-21/README.md).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-150 | **Blocker `CBomberAttr` → consumer Bomb/Fire/Roller/speed**: loader `0x13500D0` đọc row theo thứ tự `name; f0,f1,f2..f10`; theo thứ tự đó + `bomber_attr.json` field_map, f2/f3 = speed min/max, f4/f5 = bomb min/max, f6/f7 = range min/max; f5 bị kẹp **≤ 10** tại `0x1350438`/`0x1350448`. Nhưng **không có consumer** cho f2–f7: singleton `dword_19D8A90` chỉ có 4 xref (getter/lifecycle); row accessor duy nhất `0x1351D10` đọc field `row+408` và được dùng như availability/khởi tạo (`0xB349A0`, `0x6E27B0`), không phải bomb/range/speed. KR opcode 158 `0xE5D55E` → `sub_E5F450` → `sub_E14360` → `sub_E9E660`: đã đọc **hết** thân `E9E660` (207 dòng, 0x644 B) — chỉ ghi `this+176=sub_B37E50(a2)`, xử lý effect/vị trí/loop và gọi virtual `+104`/`+80`; **không có** `cur+1`/`min(cur+1,max)`/setter stat. VN 158 `sub_706E0A` chỉ xoá pending. Không có consumer cho **Roller (kind 2)** hay **Shoe (kind 3)**; không đánh đồng. Speed level→px/bước vẫn **unknown** (R-125) | — | `kr_static` / **unknown (blocker)**; thứ tự loader + kẹp f5 `confirmed`, nhưng nghĩa f2–f7 vẫn theo R-042 `candidate`, và chưa có consumer nên **không** dùng làm base/max cho gameplay | `0x13500D0` loader; `0x1350438`/`0x1350448` kẹp 10; `dword_19D8A90` singleton; `0x1351D10` `row+408`; `0xB349A0`/`0x6E27B0`; `0xE5D55E`→`sub_E5F450`→`sub_E14360`→`sub_E9E660`; VN `sub_706E0A`; `ev_f8c6effc…` (đọc hết E9E660); evidence §10.6 |

Phạm vi blocker:

- Đã đọc loader và toàn bộ xref của singleton/row accessor; **không** thấy đường đọc f2–f7 trong
  bomber/item. Các hàm item đã đọc ở trên chỉ xử lý vị trí/pending/effect object, không cộng stat.
- **Đã đọc hết** thân `sub_E9E660` (207 dòng, 0x644 B): không có phép `cur+1`/`min(cur+1,max)`/
  setter stat; chỉ ghi `+176`, effect/vị trí, virtual `+104`/`+80`. Các **callee effect** (`EB8000`,
  `EB7A60`, virtual `+104`) chưa mở hết; chưa có runtime.
- **Không tạo claim riêng cho `E9E660`**: đã đọc hết nhưng không thấy consumer/`cur+1`/`min`; R-150 giữ blocker.
- Chưa thấy `.themida` trong các đường này; nếu mở rộng sang hàm VM thì ghi entry theo
  [DEVIRT_METHOD](../../research/findings/DEVIRT_METHOD.md).
- Đề xuất profile: **không đổi** `characters.*`/`R-117`; giữ cấp (level), không đổi sang px. Nếu
  sau này có consumer base/max thì PR profile riêng theo ADR 0013.

## 19. RE KR 00967 — bộ đếm bomber → chết (R-200…R-202)

Đọc tĩnh client KR `00967` cho [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / Q-018
(mục 4 hàng đợi RE). Số mục để trống theo yêu cầu của phiên (phiên 1 dùng dải R-150…R-199; phiên này
dùng **R-200…R-249**) — integrator đánh số §19 khi merge (#195). Định danh build (SHA-256 `cf72da11…e63b`,
imagebase `0x400000`, `RVA = VA − 0x400000`), bảng VA/RVA và `evidence_id` ở
[evidence KR trap timer](../../evidence/measurements/kr-static-trap-timer-2026-10-23/README.md); suy
luận, giới hạn và bước tiếp ở [KR_RE_TRAP_TIMER](../../research/findings/KR_RE_TRAP_TIMER.md).
Đây là lượt nối tiếp R-131 (§14): R-131 vẫn đúng — tên defer `kCGameStage_DeferBomberLockedIn` /
`kCGameStage_DeferBomberReleaseOrDie` chỉ là nhãn đăng ký, không trỏ hàm. Lượt này tìm được **cạnh
native** của một bộ đếm trên bomber, nhưng **chưa** chứng minh đó là timer nhốt.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-200 | **Bộ đếm native trên bomber: `+0x2748`, giảm 1 mỗi update, `≤ 0` → gửi packet 167 (nghĩa *candidate*: báo chết).** Trường dword tại byte `+0x2748` (dword index `2514`) của object bomber; vị từ `0x006833D0` = `*(this+2514) > 0`. Trong `CGOBomber::Update` (slot 24 `0x00B44000`, vtable `0x01744964`; được `CGOGameBomber::Update` slot 24 `0x00B77B10` gọi ở `0x00B77E22`): nếu `vị từ \|\| sub_B534D0(this)` thì `sub_621A20(this+0x272C)`, `--*(this+2514)`, gọi `*(this+0x0D74)+0x60` (với `this+0x0D74`), rồi nếu `<= 0` gọi virtual `[vtable+0x328]` (slot 202). `CGOGameBomber` slot 202 = `0x00B7A760`: đặt trường `= 0`, và nếu object con `+356` qua `vf+56` thì mở `CSendPacket` type **167** (`0x01164C10(167)`), ghi 2 giá trị (`sub_14EBCF0`: `vf+40` của object con `+353`, và hằng `6`), gửi (`0x01164D50`/`0x01164C80`); `CGOBomber` slot 202 = `0x00B5D630` (rỗng). Consumer thứ hai: `0x00E84460` serialize từng bomber (byte `predicate(bomber)` = timer đang chạy) cùng `+0x5E8`/`+0x24DC`/…, `0x00E67E70` đóng gói `CSendPacket` type **184** (`push 0B8h` tại `0x00E68201`) | **1 đơn vị / 1 lần update bomber**; chết khi trường `≤ 0`; quy đổi ms cần nhịp update KR (30 ms của R-013 là đo trên VN) | `kr_static` / confirmed (cơ chế đếm–ngưỡng–packet); nhãn **"timer nhốt"**: `candidate` (xem R-201) | `0x006833D0` (predicate), `0x00B44000` (update, call site `0x00B4409A`–`0x00B440DE`), `0x00B7A760` (slot 202), `0x01164C10` (`CSendPacket`), `0x00E84460`→`0x00E67E70` (status 184); zero-init: `0x00B3DCB0`, `0x00B3D1D0`, `0x00B58B60`, `0x00B7A760`, `0x01036700`; evidence `ev_6035bfe5…`, `ev_fb801a51…`, `ev_3420dbcb…`, `ev_c8ea4fa9…` |
| R-201 | **Setter/seed của bộ đếm không nằm trong code native; biên VM (blocker có phạm vi).** Quét toàn image mọi encoding của displacement `0x2748` (`[reg+2748h]`, `lea`, `add`): **10 site** — đọc trong `0x006833D0`, RMW giảm trong `0x00B44000`, zero-init trong `0x00B3DCB0`/`0x00B3D1D0`/`0x00B58B60`/`0x00B7A760`/`0x01036700`, và 1 dword trong bảng `0x017A60CC` (không phải code). **Không** có store giá trị dương, **không** có `lea`/`add` tạo địa chỉ ⇒ seed không được ghi bởi code native đã thấy. Vị từ thứ hai của update `sub_B534D0` `0x00B534D0` là thunk Themida: prologue native rồi `jmp near ptr 0x02012376` (segment `________`, `0x01D7B000–0x02076000`; byte đích `0xFF`) — cùng thunk được gọi từ `0x00B53330`, `0x00B46760`, `0x00B46830`, `0x00B46940`, `0x00B78780`, `0x00682790`. **Điểm vào nhốt + seed + state 3** chưa định vị | — (không mang giá trị) | `kr_static` / **unknown** (blocker có phạm vi; entry VM `0x00B534D0 → 0x02012376`) | `sub_B534D0` `0x00B534D0`; danh sách 10 site ghi ở [evidence](../../evidence/measurements/kr-static-trap-timer-2026-10-23/README.md#quét-displacement-0x2748); `kCGameStage_DeferBomberLockedIn` `0x017953C8` / `DeferBomberReleaseOrDie` `0x01795074` chỉ là nhãn đăng ký (R-131) |
| R-202 | **Chạm/chọc và thứ tự trong một bước: chưa định lượng cho KR.** R-028 (`vn_runtime`/confirmed) "chạm địch → chết" giữ nguyên cho VN; KR **chưa** đọc được khoảng cách/hộp/ngưỡng chạm, cũng chưa nối nhánh chạm vào `0x00B7A760`. Thứ tự **hết hạn đếm vs chạm vs lửa trong cùng một bước** chưa chứng minh: đếm nằm trong `CGOBomber::Update` (`0x00B44000`, được gọi ở **cuối** `CGOGameBomber::Update` `0x00B77B10`), còn áp vụ nổ/lửa nằm ở đường map (`0x00E9E660` → `0x00E9EEE0`, xem R-139…R-141) — thứ tự giữa hai đường phụ thuộc vòng tick của stage, **chưa trace** | — | `kr_static` / **unknown**; `vn_runtime` (R-028) chỉ là tham chiếu | `0x00B44000`/`0x00B77B10`; R-139/R-141; [KR_RE_TRAPPED §5](../../research/findings/KR_RE_TRAPPED.md#5-timer-nhốt--chết-chưa-định-vị-r-131) |

Hệ quả:

- **Không đổi `data/rules`.** `player.trap_to_death_ms = 4000` (R-115) giữ `design`; KR chỉ cho **đơn vị
  update** và ngưỡng, **chưa có** seed/nhịp KR nên không suy ra ms. Không lấy 6300/6359 ms (R-026,
  khoảng packet) hay 2,5 s emulator làm timer.
- Nhãn "timer nhốt" cho trường `+0x2748` là **candidate**: trường này chưa có consumer nào nối tới
  state bị nhốt; nó có thể là bộ đếm trạng thái khác. Chỉ cạnh **đếm → gửi packet 167** là chắc.
- Packet **167** (báo chết) và **184** (status bomber có byte timer) là phát hiện mới ở phía client;
  chưa đối chiếu server, không dùng để đổi contract V1 ở PR này.
- Nếu cần đóng Q-018: bước tiếp là devirt/đọc VM tại `0x00B534D0 → 0x02012376`, hoặc đo runtime
  (entry/seed, state 3, thời điểm packet 167) theo Q-009 — không suy từ VN.

## § (đánh số khi merge) — RE KR 00967 — state→animation, duration ODT và nhịp update (R-151, blocker)

Tiếp nối mục 6 hàng đợi RE; Refs #148/V1-031. Build KR `00967`; REA MCP bản sao
`kr-full.exe`, không mở DB gốc. Chi tiết/phạm vi ở
[KR_RE_ANIM_STATE](../../research/findings/KR_RE_ANIM_STATE.md) và
[evidence flame](../../evidence/measurements/kr-static-flame-2026-10-21/README.md).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-151 | **Consumer `BA3530` đã thấy nhánh; blocker phần đơn vị/cadence/getter**: với `record = BADF80(animIdx)`, `BA3530` so `B33170()` với `[record+4]`: `[record+4] == -1` → gọi `B33170()`, return 1; `B33170() < [record+4]` → return 5 (giữ frame); đủ ngưỡng → `B33190(0)` reset, `B331E0()` lấy giá trị; `B331E0() < GetAnimCount` → return 4 (nhánh tiến/không giữ); cuối dãy + `B33F40()!=0` → `B33200(count-1)`, return 3 (giữ cuối); ngược lại `B33200(0)`, return 2 (quay đầu). Đây là **consumer so bộ đếm/chuyển/giữ cuối/quay đầu** đã thấy; **chưa chứng minh** đơn vị/cadence của `B33170`, tên/nghĩa getter, và runtime state. `BA40F0`/`BA4160` vẫn đọc field per-anim; `B33B30` dùng `sub_15625C2` = **RNG**. R-141 giữ confirmed lửa frame (`B711C0`/GetAnimCount); `B44000` mã `24`/`27–30`/`39–42` chưa nối nhãn; hướng/tâm/thân/đuôi lửa chưa có consumer; `B711C0` xref duy nhất vtable. | — | `kr_static` / **confirmed** (nhánh `BA3530`, mã trả 1/2/3/4/5); **unknown/blocker** (đơn vị/cadence/getter semantics, state runtime, fire layer); R-141 giữ confirmed frame | `BA3530`; `B33170`/`B33190`/`B331E0`/`B33200`/`B33230`; `B33F40`; `BA40F0`/`BA4160`; `B338F0`; `B9CEC0`/`B9FDD0`; `B33B30`; `B44000`; evidence §11.6 |

Hệ quả:

- **R-141 (đã có)**: hiển thị lửa tính bằng **frame**, không phải ms; `B711C0` chỉ `++`, không nhận `dt`.
  Không đổi `bomb.flame_ms=500` hoặc bất kỳ giá trị profile nào khi chưa có nhịp update.
- **ODT duration**: `BA40F0`/`BA4160` là consumer của field per-anim, nhưng chưa chứng minh field là
  duration/ms; `B33B30` dùng RNG nên không phải elapsed. Không suy “duration ODT = ms”.
- **State**: nhãn idle/walk/trapped/dead vẫn theo ODT (R-095 candidate); chưa có trace nối state core
  với nhãn, và chưa thấy consumer chọn tâm/thân/đuôi/hướng lửa.
- **Nhịp update**: chưa định vị call site/loop gọi vf24; do đó `frame → ms` vẫn `unknown`.
- Đề xuất presentation: giữ frame-based, không đặt ms; nếu cần ms phải có PR riêng sau khi đo nhịp.

## § (đánh số khi merge) — RE KR 00967 — UI V1: rect/hitbox và layout phòng (R-152)

Tiếp nối #150/Q-012/Q-021, V1-032/033/035. Build KR `00967`; REA MCP bản sao `kr-full.exe`.
Chi tiết/phạm vi ở [KR_RE_UI](../../research/findings/KR_RE_UI.md) và
[evidence UI](../../evidence/measurements/kr-static-ui-2026-10-23/README.md).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-152 | **UI V1 — hitbox danh sách phòng + layout Ready**: `sub_549180` (lobby room list) lặp 6 ô, dùng bảng `0x19D8A58` = `(20,90)`, `(194,90)`, `(20,160)`, `(194,160)`, `(20,230)`, `(194,230)`; `width/height` lấy từ control vtable `+36`/`+40`; click `sub_1504A5C` đọc fields `+136`/`+140`, trừ gốc container `sub_149BBA0`/`sub_149BEF0`; hit khi `left ≤ x < left+width` và `top ≤ y < top+height` (đơn vị **UI/input px**, không scale trong hàm). `sub_6F47C0` (màn PrepareStage) tạo Ready bằng `sub_6F8790(..., "PRE_Bt_Ready")` + `sub_54BEF0(512,494,0x10000000,0x10000000)`; room info `(344,61,…)`, hai nút nhân vật `(541,231)`/`(690,231)`, nền `(800,600,40,48)`; caller `sub_6E3DF0` tại `0x6E3E64`. Key resource = ghép `base + "/" + key` (`6F8790`, `%s/%s`), base lấy từ object `+267088`; key pack tương ứng `Prepare/PRE_Bt_Ready`. **Chưa chứng minh**: diễn giải 4-int của `54BEF0` thành rect/scale, native framebuffer (800×600 chỉ là literal layout, không phải JPEG/FrameBuffer), focus path; `C00/C01→tên` vẫn `unknown` (`room.character_c00/c01` chưa có locator) | — | `kr_static` / **confirmed** (hit test 6 ô + công thức); `candidate` (Ready placement/key, rect interpretation); `unknown` (native size/scale/focus, C00/C01 name, các màn login/result) | `549180` (RVA `149180`); bảng `19D8A58`; `1504A5C`; `149BBA0`/`149BEF0`; `6F47C0` (RVA `2F47C0`); `6F8790`; `54BEF0`; `6E3DF0`; evidence §12 |

## § (đánh số khi merge) — RE KR 00967 — âm thanh theo event (R-203…R-208)

Đọc tĩnh client KR `00967` cho [#196](https://github.com/aquaplatinumberlitz/boom-rs/issues/196) /
Q-015, V1-034 (mục 8 hàng đợi RE). Số mục để trống theo yêu cầu phiên (dải **R-203…R-249**); integrator
đánh số khi merge. Định danh build (SHA-256 `cf72da11…e63b`, imagebase `0x400000`,
`RVA = VA − 0x400000`), bảng VA/RVA và `evidence_id` ở
[evidence KR audio](../../evidence/measurements/kr-static-audio-2026-10-23/README.md); suy luận, giới
hạn và đề xuất AUDIO_V1 ở [KR_RE_AUDIO](../../research/findings/KR_RE_AUDIO.md).

Bối cảnh: R-081 chỉ confirmed **entry/mapping** BGM; R-082 (event→entry) và R-083 (chồng tiếng) là
**design** của V1 cho phần chưa nâng nhãn/áp đầy đủ đối chiếu consumer — **không** phải vì client
không có consumer. Lượt này tìm được consumer thật (bảng id tĩnh + API phát + các điểm gọi theo event),
nên một phần R-082/R-083 có thể nâng bằng chứng — #206 chỉ nâng riêng mapping `player_trapped`; các
mapping khác và policy R-083 giữ design (đề xuất đầy đủ ở findings §7).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-203 | **Bảng âm thanh tĩnh id→entry + API phát.** Mảng entry 0x20 B tại `0x19E40D8` (649 entry, id `0..651`): `+0` id, `+4` con trỏ tên `Wave/...`, `+0x1C` type; khởi tạo bởi `0x41C980`. Manager `CSoundManRenew` là singleton `0x19D8A88` (getter `0x54EAF0`); phát bằng `0xD1CD10(manager, id, a3, a4)`. Id V1: `7 Cha_BombExplode`, `8 Cha_BombExplodeMulti`, `9 Cha_BombIgnite`, `10 Cha_DominoBombIgnite`, `37 Cha_CoupleLockedIn`, `42 Cha_Die`, `43 Ntc_Draw`, `47 Ntc_GameEnd`, `68 Itm_ItemEaten`, `69 Itm_ItemPop`, `77 Cha_LockedIn`, `78 Ntc_Lose`, `164 Ntc_TimebombExplode`, `174 Itm_UseTrap`, `175 Ntc_Win`, `343 Cha_BombExplode_SnowBall`, `344 Cha_BombIgnite_SnowBall`, `444 Ntc_Help2`, `471 BombExplosion001` | id (số nguyên); tên entry dạng `Wave/<tên>` | `kr_static` / confirmed (bảng + dispatcher) | `0x19E40D8` (base), `0x41C980` (init), `0x19D8A88`/`0x54EAF0` (manager), `0xD1CD10`; evidence `ev_79413114…` |
| R-204 | **Event → entry của client (có consumer).** Nổ bom: `0xEB3720` phát **7** (`a3=1`: dedup theo id — R-207, **không** phải cờ loop) hoặc **343** khi `sub_B38390(bomber) == 9` (bom tuyết); `0xEB9C30` phát **7** cho object type `31`. Đặt bom: `0x71F490`/`0xAF1140` phát **9**, tuyết type 9 → **344**; domino **10** (`0x8326F0`). Bị nhốt: `0xB42D60` phát **37** (`Cha_CoupleLockedIn`) nếu cờ couple `[ebp-0x34] != 0`, ngược lại **77** (`Cha_LockedIn`), cả hai `a3=1`; nếu `[this+0x2788] != 0` phát thêm **444** (`Ntc_Help2`). Chết: `0xB53C80` (slot 146 vtable `CGOBomber` `0x1744964`) gọi `vf+320(this,-1)` rồi phát **42**. Item: ăn/nhặt **68** (`0xB4B410`), item lộ **69** (`0xB8B680`), dùng trap **174** (`0xB9DEF0` case `a2==2`) | id | `kr_static` / confirmed (consumer từng event) | `0xEB3720`, `0xEB9C30`, `0x71F490`, `0xAF1140`, `0x8326F0`, `0xB42D60`, `0xB53C80`, `0xB4B410`, `0xB8B680`, `0xB9DEF0`; evidence `ev_6ac4a4d1…`, `ev_8cb8b610…` |
| R-205 | **Chọn biến thể: remap modulo khi guard bật; guard tắt giữ id gốc; nổ không remap.** Trong `0xD1CD10`, **chỉ khi** `sub_76FBF0(&v24) != 0` mới lấy `v20 = sub_15625C2()` (`0x15625C2` = LCG MSVC `214013/2531011`, trả `(v>>16)&0x7FFF`) và remap; guard false thì **giữ nguyên `a2`**. Công thức `rand()%N + base` (evidence chỉ chứng minh công thức modulo; `rand()` 15-bit nên `%5`/`%6` có **modulo bias** — không claim phân bố đều): `18`/`128`→`436..439`, `21`→`440..443`, `43`→`428..429`, `47`→`430..435`, `64`→`445..446`, `78`→`447..452`, `175`→`453..457`. Nổ id `7`: nếu **7** đang phát (< 200 ms) thì dừng 7+8 và phát **8**; nếu **8** đang phát (< 200 ms) thì phát **7**; còn lại phát **8** rồi return. Tuyết: `343`/`344` theo type 9. Không thấy chọn theo tầm/hướng | `%N + base`, N = 2/4/5/6; cửa sổ 200 ms | `kr_static` / confirmed | `0xD1CD10` (`0xD1D012`–`0xD1D0F7`), `0x15625C2`, `0x14BEA70`; evidence `ev_79413114…` |
| R-206 | **BGM do dữ liệu map quyết định, không hard-code; Village = id 48.** Bảng BGM 0x20 B tại `0x19E9258` (117 entry): id `48` = `Mp3/Village` (chuỗi `0x1761FE8`). `0xE71D70`: lấy map id (`0x123FC20`/`0x544520`), tra record map (`0xE04F00` quét danh sách, khớp `sub_E01930(i)`), lấy **tên BGM tại record+68** (`0xE02110`), ghép `"Mp3/%s"` rồi phát qua `0xD1D710`. Nếu bật `Xmas` thì thay bằng `Carol%02d` với `rand()%4` (Carol00..03). Loop/một voice **không** chứng minh được ở đây | tên entry; id 48 | `kr_static` / confirmed (chuỗi consumer); mapping map→"Village" cụ thể: `candidate` (record map chưa đọc) | `0x19E9258`, `0x1761FE8`, `0xE71D70`, `0xE04F00`, `0xE02110`, `0xD1D710`; evidence `ev_e53b55a5…`; R-081 |
| R-207 | **Voice: pool cố định, dedup theo id, không ưu tiên/không đuổi.** `0x14BF4D0(engine, id, a3, a4)`: engine=`[manager+0x68]` (R-212), block voice tại `engine+16`, số voice `engine+12`; nếu `a3 != 0` mà đã có voice cùng id → **trả handle cũ, không phát thêm**; ngược lại tìm voice trạng thái `2` (rảnh); **không có voice rảnh → bỏ** (`return 0`), không dừng voice cũ/không ưu tiên. Volume nhân `*(float*)(engine+48)`. Cửa sổ dedup: 200 ms cho cặp `7/8`; 90 ms cho nhóm id `337..342` (`0xD1CD10`) | voice; 90/200 ms | `kr_static` / confirmed (cơ chế); **khởi tạo pool đã giải ở R-212: enginecount tốiđa16 thành công; runtime count unknown** | `0x14BF4D0` (`0x14BF52F`–`0x14BF5DA`), `0x14BEA70`, `0xD1CD10`; init chain `0x1231BD0`→`0xD1C0A0`→`0xD20E10` (singleton `0x19D8A88`; evidence `ev_378e543a…`, `ev_425ff988…`), thân ctor `0xD1C0A0` (evidence `ev_d48e48f7…`); evidence `ev_003bb7d0…` |
| R-208 | **Bounded negative: không thấy direct call/literal-push cho brick-break hoặc `BombExplosion001` trong phạm vi quét.** Bảng 649 entry **không có tên** brick/break (chỉ `Npc_Pen_NpcEggBreak`, `Ui_BasketEggBreaking`, …) và `CPlayMap::ExplodeObjectByFlame` `0xEB9840` **không gọi trực tiếp** `0xD1CD10` — nhưng nó phá tile qua **virtual `vf+148`** (chưa đọc thân) nên **chưa loại trừ**. Id `471 Wave/BombExplosion001` không có `push 471` trong 597 site gọi trực tiếp ⇒ **không thấy consumer native trực tiếp**; id động/wrapper/đường script `PlayWaveEffect` (theo tên, `0xD16410`→`0xD1CD10`) **chưa bị loại trừ**. Vì vậy **không** kết luận "gạch vỡ không có tiếng" hay "tiếng chỉ gắn với bom" | — | `kr_static` / `confirmed` (quan sát âm tính **trong phạm vi quét direct call + literal push**; "bounded negative" là giới hạn phương pháp, **không** phải nhãn trạng thái) | `0xEB9840` (+`vf+148` chưa đọc), `0x41C980` (bảng), `0xD16410` (script by-name); evidence bảng ở [evidence](../../evidence/measurements/kr-static-audio-2026-10-23/README.md) |

Hệ quả:

- **Không đổi `data/assets/audio-v1.toml`** trong PR này. Đề xuất (cần task riêng, có review):
  `player_trapped` nên là **77/37** (client) thay vì `Itm_UseTrap` (174 = tiếng dùng item trap);
  `bomb_exploded` nên là **7/8** (+343 tuyết); `BombExplosion001` (471) **không thấy consumer native trực tiếp**
  (chưa loại trừ id động/wrapper/script); `bomb_placed` **9** (+344 tuyết); round-end/start giữ entry nhưng biết
  client **remap** id theo `rand()%N+base` **khi guard `sub_76FBF0` bật** (guard tắt giữ id gốc; modulo bias),
  đích có thể là `Ntc_Win1..5`, `Ntc_Lose1..6`, `Ntc_Draw1..2`; `item_revealed` 69, `item_picked` 68, `player_died` 42
  **khớp** V1.
- BGM: consumer theo record map (tên tại `+68`) là bằng chứng mới cho R-081 phần **consumer**, không
  phải loop policy; R-083 vẫn `design`.
- Voice: bằng chứng mới cho R-083 phần **dedup theo id + không ưu tiên**: client **bỏ** khi hết voice
  rảnh, không đuổi mức thấp nhất; V1 đang mô hình ngược lại (ưu tiên + bỏ mức thấp nhất). Đề xuất giữ
  design V1 (8 voice) nhưng ghi khác biệt, không tự đổi.

## § (đánh số khi merge) — RE KR 00967 — ItemAttr reader và bảng trọng số (R-153)

Tiếp nối #149/Q-007/Q-020, V1-016. Build KR `00967`; REA MCP bản sao `kr-full.exe`.
R-150 (BomberAttr/E9E660) giữ nguyên, không lặp; chi tiết ở
[KR_RE_BOMBERATTR](../../research/findings/KR_RE_BOMBERATTR.md) và
[evidence item](../../evidence/measurements/kr-static-item-2026-10-21/README.md).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-153 | **ItemAttr: loader + reader id; bảng trọng số 0x8000, reader chưa thấy trong phạm vi literal** — `CItemResourceMan` loader `0x1196A70` (RVA `0xD96A70`, segment `.priv`) đọc bảng item `+244048` stride **192** (`0xC0`), count `+244036`; row `+84` = item id, `+20` tên; `sub_1197900` tìm row theo `+84` (xref **7 hàm**; **đã đọc 4 thân**: `E84CD0`, `6B3FA0`, `B978B0`, `B98250` — dùng id cho model/UI; **3 thân còn lại đã đọc**: `FFFA70` map `a2`→id 53/54 rồi **bỏ lookup**; `1191DD0` **bỏ lookup**; `1192700` dispatch item-use/ability theo id, gán lookup vào `v47`; **không thấy dùng lại trong toàn bộ 227 dòng pseudocode đã lưu**; cả 3 gọi `1191D20` gắn model; **không thấy đọc trực tiếp `+96..+180`/weight/placement** trong 3 thân. Helper/virtual chưa loại trừ). Bảng group `+28` stride **244** (`0xF4`), count `+16`; group `+148` (`0x94`) = số slot; loader tạo mảng `0x8000` byte tại `+436048` (`0x6A750`): khởi tạo `0xFF`, mỗi group ghi **group index** vào `[offset, offset+count)`, offset cộng dồn, cap `≤0x7FFF`; **không chia tổng** ⇒ không phải phần trăm. Quét **literal displacement** `+0x6A750` trên disassembly: chỉ **2** hit, cả hai trong loader (write) ⇒ **chưa thấy reader trong phạm vi quét literal này**. Không loại trừ alias/base điều chỉnh, địa chỉ tính toán, helper, indirect hoặc VM; không chứng minh client chọn item/drop. Placement: `B479E0`/`B4C9D0` tạo item model trên bomber (`this+513`) từ id; `E84CD0` đọc byte user-info/character, không phải map drop/brick. Weight ≠ %; placement ≠ xác suất gạch. | — | `kr_static` / **confirmed** (loader, kích thước/stride, id `+84`, lookup `1197900`, fill bảng 0x8000); `candidate` (vai trò group/6 tên đặc biệt); `unknown` (reader ngoài phạm vi literal, client drop vs server placement, nghĩa `+96..+180`, cap, 35%) | `1196A70` (RVA `D96A70`); `+244036`/`+244048` (0x3B944/0x3B950); `+16`/`+28`; `+148` (0x94); `+436048` (0x6A750); `1197900`; `E84CD0`; `6B3FA0`; `B978B0`; `B479E0`/`B4C9D0`; evidence §R-153 |

## § (đánh số khi merge) — RE KR 00967 — render: accessor frame, neo tile và lớp Static/Adjust (R-209…R-211)

Đọc tĩnh cho [#201](https://github.com/aquaplatinumberlitz/boom-rs/issues/201) / Q-002/Q-003, V1-030/031
(mục 7 hàng đợi RE). Số mục để trống theo yêu cầu phiên (dải **R-209…R-249**); integrator đánh số khi
merge. Điểm đầu VN `CDataI16Alpha__Load` `0x00A3D850` (client VN `unpacked_ca.exe`, SHA-256
`13a0a33cccc8f5f5124cdd8dac863b68118e6dda83340649b897fc41faed364a`, imagebase `0x400000`), đối chiếu
KR `00967` (`cf72da11…e63b`). Chi tiết/giới hạn ở [KR_RE_RENDER](../../research/findings/KR_RE_RENDER.md)
và [evidence](../../evidence/measurements/kr-static-render-2026-10-23/README.md).

Bối cảnh: [MAPS §3](../assets/MAPS.md) đã đo 12×`i32` mỗi frame (f0/f1 đăng ký, f2 flag, f3 ảnh,
f4–f7 rect, f8/f9 canvas, f10/f11 pivot) nhưng **chưa có consumer render**. Lượt này tìm được
**accessor** của lớp dữ liệu ở VN, **neo tile** và **quy ước **tên lớp** `LayerStatic`/`LayerAdjust`**;
**chưa** tìm được hàm vẽ dùng f8–f11, nên câu hỏi công thức đặt ảnh + thứ tự lớp + clip/alpha vẫn mở
(blocker có phạm vi §4 findings).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-209 | **Accessor frame của `CDataI16Alpha` (VN) — f4..f11 là một khối liên tiếp, f0/f1/f2 đọc/ghi riêng.** `vf8(frame)` trả `frame + 16` = con trỏ tới **f4..f11** (`0x8D50DF`); `vf9` = `f6−f4` (rộng rect), `vf10` = `f7−f5` (cao rect) (`0x8D50F3`, `0x8D510B`); `vf11` = `f0`, `vf12` = `f1`, `vf13` = `f2` (`0x8D5123`, `0x8D5136`, `0x8D514A`); `vf14(frame,a,b)` **ghi f0/f1** (`0x8D515E`) ⇒ đăng ký **thay đổi được** theo frame; `vf15(frame)` lấy **index từ `f3`** rồi gọi `vf16`; `vf16(index)` = `*(this[16] + 4·index)` — **getter phần tử bảng theo index**; ý nghĩa kiểu/handle/kích thước ảnh **chưa chứng minh** (`0x8D5180`, `0x8D519B`). Frame record 48 B, index `f3` | 12×`i32`/frame; rect/canvas/pivot: px | `vn_static` / confirmed (cấu trúc accessor); **nghĩa vf15/vf16 và consumer của f8–f11: unknown** | VN `0x8D50DF`, `0x8D50F3`, `0x8D510B`, `0x8D5123`, `0x8D5136`, `0x8D514A`, `0x8D515E`, `0x8D5180`, `0x8D519B` (RVA `0x4D50DF`…`0x4D519B`); [MAPS §3](../assets/MAPS.md) |
| R-210 | **Neo thế giới của tile/object (VN): `x = cột·40`, `y = hàng·40 + 39`.** `CGOTile::vf33` `0x5BED20` ghi `this[19] = 40·a5` và `this[22] = 40·a4 + 39` (a5 = cột, a4 = hàng), rồi gọi `vf+20(this, a3, -1)` và `vf+12(this, 1)`. Đơn vị **px**, ô 40 px; mốc là **1 px trên mép dưới ô** (`row·40+39`), không phải tâm ô | px; ô 40 | `vn_static` / confirmed (công thức + vị trí ghi); caller map/loader: `candidate` | VN `0x5BED20` (RVA `0x1BED20`), `CGOTileFixed__vf33` `0x5BEE9E` |
| R-211 | **Tên lớp `LayerStatic`/`LayerAdjust` và loader ghép `LayerStatic`; cặp sprite theo object chưa chứng minh.** Loader VN ghép tên `<tên>LayerStatic` bằng `strcat` (`0x5C8107` trong `sub_5C7F8E`, `0x5C9F66` trong `sub_5C9741`); bảng tên lớp có cả `LayerStatic` (`0xC3797C`) và `LayerAdjust` (`0xC79760`). KR cũng ghép `<tên>LayerStatic` (`0xBA3384` trong `sub_BA3100`, `0xBB34C5`/`0xBB3545` trong `sub_BB2620`); chuỗi `LayerAdjust` KR ở `0x17902D8` **chỉ thấy trong bảng dữ liệu** `0x19A64B4`, **không** có code xref trực tiếp. **Chưa** chứng minh mỗi object nạp/giữ/vẽ đủ hai sprite, chưa chứng minh thứ tự vẽ Static→Adjust, clip và alpha | tên entry | `vn_static` + `kr_static` / confirmed (**tên + loader đã thấy**); **cặp theo object, consumer `LayerAdjust`, thứ tự/clip/alpha: candidate/unknown** | VN `0x5C7F8E`/`0x5C9741`; KR `sub_BA3100`/`sub_BB2620`, `0x19A64B4` |

Hệ quả:

- **Không đổi BUNDLES §7** hay code/profile: quy ước đặt ảnh hiện tại vẫn là `design`. Lượt này **không**
  chứng minh công thức `canvas.x = cột·T − pivot_x`, `canvas.y = (hàng+1)·T − canvas_h`; cũng **không**
  chứng minh thứ tự `Static` trước `Adjust`. Đề xuất: giữ nguyên, ghi rõ Q-003 vẫn mở; nếu cần đổi thì
  task riêng có review.
- Neo tile `x=cột·40`, `y=hàng·40+39` khớp R-009 (neo hiển thị nhân vật) và củng cố BUNDLES §7 mục 1
  (mép dưới ô) — nhưng **không** chứng minh vùng ảnh canh đáy canvas.
- f0/f1 **thay đổi được** (setter vf14): mọi mô hình coi f0/f1 là hằng số của sprite cần kiểm lại; frame
  record vẫn 48 B, 12 field.

## § (đánh số khi merge) — RE KR 00967 — bomb pass từ phía object bom (R-154)

Tiếp nối #146/Q-017, V1-012/013. Build KR `00967`; REA MCP bản sao `kr-full.exe`.
Chi tiết/phạm vi ở [KR_RE_MOVEMENT](../../research/findings/KR_RE_MOVEMENT.md) và
[evidence movement](../../evidence/measurements/kr-static-2026-10-21/README.md).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-154 | **Predicate passability của `CGOBomb` (chỉ thân hàm) + consumer `MoveSimple` đã đọc**: `CGOBomb` vtable `0x174426C` slot **+156** = `0xB37DB0` (RVA `0x737DB0`) trả `0` nếu `+328` (`0x148`) thuộc `{1,2}`; nếu không, trả flag `+152` của `CSensorTrigger` tại `+144` (`B38250`/`B38290`; `11982F0`), nên flag `+152 == 0` cũng cho predicate `0`. `B37150` ctor gọi `1198240(bomb+144,1)` (flag mặc định `1`); `B384E0` Init ghi `+328` từ **u8 descriptor+108** (`529C20`). **Trong thân `B37DB0` không đọc owner/player/overlap**; owner getter `B38370` (`+268`) có **xref 14 hàm**; đã đọc thêm **4 thân** `ACC340`/`AEF700`/`EB73D0`/`EB8000`: cả 4 chỉ thấy dataflow descriptor/event, effect/flame hoặc **bỏ kết quả**; **chưa thấy kiểm collision/grace trực tiếp** trong 4 thân (không thấy gọi `B37DB0`/`EA0FD0`/`+152`/`+328` hay `MoveSimple`); helper/callee và **10 thân còn lại chưa loại trừ**. Lượt tĩnh KR này chưa tạo claim player grace vì chưa thấy kết nối trực tiếp collision/grace; R-155 ở §24 là quan sát VN riêng. `CMovable::MoveSimple` `0x1310200` — consumer đã trace thuộc **pet/AI** (R-134/R-135), **chưa** nối mover bomber — gọi virtual `+156` trên object ô đích, nhưng đường va chạm còn **OR `EA0FD0`** và các gate virtual **`+164`** phía trước; predicate khác `0` **không đủ** kết luận đi qua. Vì vậy **không** suy player/bomber grace, đi ra/vào, hay “không owner toàn hệ thống”; thời điểm mất pass/timer **unknown**. | — | `kr_static` / **confirmed** (predicate `B37DB0`, ctor/flag, nhánh `MoveSimple` có phạm vi pet/AI); **unknown** (đường bomber, nghĩa/giá trị `+328`, setter runtime, owner role/10 caller còn lại, grace/timer, VN counterpart) | `B37DB0` (RVA `737DB0`); vtable `174426C` slot `+156`; `B38250`/`B38290`; `11982F0`; `1198240`; `B37150`; `B384E0`; `1310200`; `B38370`; evidence §R-154 |

## 24. Đo động VN — grace và cửa sổ lửa (2026-10-24)

Quyết định trực tiếp của chủ dự án2026-10-24: dùng kết quả đo động client VN làm nguồn luật.
Build VBNB_116, fixture SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`;
server chỉ gửi đặt/nổ, không tính damage/collision. Biết các patch khởi chạy; không gọi fixture là vanilla.

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-155 | Owner đang chồng ô bom đi ra được; sau khi rời, thử quay lại bị chặn. Lặp lại khi đứng chờ5s trước khi rời. Chưa đo non-owner, chưa xác định timer grace | owner-only, không mang ms | `vn_runtime` / confirmed | Hai capture host-run2/3; [evidence grace](../../evidence/measurements/vn-player-grace-2026-10-24/README.md). Không thay phần non-owner của lựa chọn R-107 |
| R-156 | Lửa là vùng còn gây sát thương sau khi routine nổ đã trả về. Cùng tọa độ native(50,39), owner vào khi segment counter18/N1 bị nhốt; vào khi counter19/N0 vẫn sống. Counter nằm tại flame+0xCC, N tại+0x234, vtable0xC35594. Đây không phải timer ms cố định hoặc bằng chứng damage lặp sau trapped | counter18 còn hiệu lực, counter19 đã hết; owner-only | `vn_runtime` / confirmed | `winedbg --gdb` i386; routine0x7324C0/return0x6E9AEE; getter0x58D56D/0x58D575; callback0x5AEEA0. [Evidence và samples](../../evidence/measurements/vn-flame-window-2026-10-24/README.md); phase12/13, clock/XY/N thực |
| R-158 | `bomb.flame_ms`: ánh xạ cửa sổ VN sang V1: chọn540ms, ceil17tick tại30Hz; tính đủ số tick kể cả tick tạo, `until = created + duration_ticks - 1`. Một tick lửa vẫn gây sát thương tick tạo. Thay riêng flame500ms của R-115 và boundary giảm tuổi lúc tạo của R-109; giữ các phần khác | 540ms; last tick16≈533ms, clear tick17≈567ms | `design` / design | R-156; ba capture N0 sau return531.084/535.177/561.943ms. Creation trong entry..return; độ lượng tử một native update và mẫu5ms, không suy hằng native540ms. [ADR0013 bổ sung24/10](../../decisions/0013-rule-source-precedence.md#bổ-sung-2026-10-24--dùng-phép-đo-động-vn-cho-cửa-sổ-lửa); `systems/explosion.rs`, `data/rules/v1-dev.toml` |

R-141 và các báo cáo tĩnh KR giữ nguyên phạm vi: counter animation KR không tự chứng minh damage.
R-156 là bằng chứng chức năng VN riêng; R-158 là hiệu chuẩn cho fixed step V1, không khẳng định19tick
native bằng17tick Rust. Không thay fuse, timer nhốt, speed, bản đồ hoặc chế độ ngoài scope.

## 25. Nhốt → hết hạn VN và hiệu chuẩn V1 (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-159 | Trong fixture VN owner đứng yên bị bom nhốt: cùng hero/bubble native, nhánh `716FE1` so age từ bubble+0x40 với stage+0x56C0=200 rồi đi tới `71734E`→sender`5AF165` subtype13; ngưỡng phụ stage+0x56B4=16. Ba lần baseline có serialize11→13 khoảng6332,232–6355,413ms | 200 bước counter native; khoảng native-event ms là phép đo riêng | `vn_static` + `vn_runtime` / confirmed (đúng build/case) | [Evidence](../../evidence/measurements/vn-trap-timer-2026-10-24/README.md), SHA bc875…; controller+5F4/member378→bubble vtableC2E21C slot88→57450F→58D507→boxed+40; cùngptr trong từngrun. Sampler có3reset sớm3→0, không có epoch đơn điệu toàn kỳ; không gọi200 làms/tickRust hoặc hằngtimer server |
| R-160 | `player.trap_to_death_ms` chọn6350ms hiệu chuẩn endpoint native R-159 thay fallback BnB4000ms. 30Hz làm tròn lên191tick; `life` tính tick bị nhốt nên chết T+190 (~6333,333ms) | 6350ms (lựa chọn V1),191tick ở30Hz | `design` / design | R-159; ba repeat baseline, HELP vẫn ở+5,2s, negative ngoài lửa không có11/13. Ánh xạ native serialize11→serialize13 thành Rust Trapped→Dead; chưa tách death-animation tail native. Nếu endpoint13 được chứng minh nằm sau animation/phụ thuộc mode thì phải hiệu chuẩn lại. Không đổi200counter thành200tickRust/không gọi6350làliteralnative |

R-115 chỉ còn chọn fuse2500ms; trap4000ms của R-103/R-115 là lịch sử cho khóa này.
Cơ chế tính tick tạo, timeout trước touch, warmup đóng băng giữ R-108; giá trị R-160 đổi thời hạn,
không tự xác nhận thứ tự touch/timeout, input khi trapped hoặc non-owner của client gốc.
R-026 vẫn confirmed **khoảng packet** riêng; R-160 không dùng clock packet làm nguồn timer.
Fixture VN động được dùng theo quyết định chủ dự án/ADR0013; không đòi server gốc hoặc RE lại KR.
Không nâng root147/G3 thành done chỉ vì claim này đã có. Increment writer, ý nghĩa reset, native clock
unit, mode/character khác và animation-death phase còn trong Q-018.

## 26. Di chuyển chậm khi bị nhốt — runtime VN (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-161 | Sau native sender11 và trước13, cùnghero C664C4 dịch chuyển theo phím ngang: r1Left35→26/Right26→35, r3Right40→61/Left61→40; bước1px so alivecontrol5px, idle không đổiXY | Slow movement quan sát; first/last fit16,318–17,007px/s trong4đoạn | `vn_runtime` / confirmed (đúngbuild/case) | [Evidence](../../evidence/measurements/vn-trapped-input-2026-10-24/README.md); VNfixtureSHA bc875…; X/Yboxed+4C/+58, getter58D56D, sender5AF165. Chỉphímngang; khôngliteralvelocity/cấp tốcđộ/collision/crosscharacter |
| R-162 | `player.trapped_speed_px_per_s` chọn17px/s hiệu chuẩn R-161, thay đứngyên R-036/phần movement R-108. V1 dùng fixed slow speed không nhânlevel, cảhai trục; hộp/bámgóc giữ DESIGN hiệnhành | 17px/s DESIGN;30Hz566mpx/tick=16,98px/s | `design` / design | R-161; chưa nối mover/cấp native; trụcdọc/hộp bubble/char khác là DESIGN chưa đo. Dead đứngyên; trapped không đặtbom/nhặtitem giữ policy hiệnhành, không nâng no111 thành gateconfirmed |

Thứtự movement trước life giữ nguyên: timeout/touch xét ở vịtrí sau movement của tick; đây vẫn
là DESIGN R-108, không phải quan sát nativeorder. Profilefixture cũng thêm khóa explicit để
kiểm cùngcontract; identity thay đổi có chủý. Không fallback ngầm về đứngyên khi thiếukey.

## 27. Trần bom hữu hiệu trong fixture VN (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-163 | Ba lượt Factory01, cùng room19/heroC664C4: một bom và12 pickup Bombkind0 qua117/158 cho phép tối đa10 bom còn tồn tại; ba pickup cuối (index9/10/11 trong tổng12) vẫn có S158, nhưng Space sau từng lần không C111; thêm thử ô trống khác cũng không C111. Lượt cuối sau151 đầu lại có111 trước151 thứhai | Trần hữu hiệu10 trong fixture/actor đã đo | vn_runtime / confirmed (đúngbuild/case) | [Evidence/210rawhash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md); VNfixturebc875…/mapFactory01index193; ba ca này chưa tách stat cap/pool allocation (R-168 bổ sung phía dưới), không phải C00/C01KR/writer hay cap mọi nhân vật. Lịch nổ0/45/25s là emu_choice, không fuse gốc |

Bổ sung cơ chế tăng Bomb/từ chối tại sức chứa; không sửa khóa nhân vật R-117 khi chưa nối receiver/
identity KR. Firefieldr18 1→2 củng cố R-025/R-111; Roller còn thiếu.

## 28. Consumer state lửa VN và bảng dùng chung VN/KR (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-164 | Với bom power1 trong hai ca Ocean01 VN, CGOFlameC35594 chọn motion theo vị trí thực: tâm0, trái15, trên30, phải45, dưới60. Selector+90/+CC đi qua draw5AD00C; trước baseDraw58DA94, boxedmotion+28 khớp ordinal và giá trịcounter tại+34=1 khớp counter output ở tâm/L/U/R (không phải chỉ số ảnh I16/ODT first_frame); Down chỉ ghi preselector | 0/15/30/45/60 là ordinal state, không phải duration | vn_static + vn_runtime / confirmed (đúngcase) | [Evidence/61rawhash](../../evidence/measurements/vn-flame-render-2026-10-24/README.md); ctor5ACBD7 truyền keyFlame/Flame→58D9BA, variant0 trong run; selector5ACC58, gates5AD042/5AD0F9. Không xác nhận mọi power/mode, pixel/layer hay cadence |
| R-165 | Member Flame/Flame trong ObjectTable có76 state:75 state dùng Bomb/Fire00 và stateTimebomb dùng Bomb/TimebombFrame. Block13.604byte VN/KR00967 giống hệt, gồm toàn bộ cặp frame-duration; rawduration đều1 | 76 state; duration1 chưa rõ đơn vị | kr_static + vn_static / confirmed (dữ liệu) | [ODT bảng/hash](../../evidence/measurements/vn-flame-render-2026-10-24/odt-states.csv); blockSHA70ff67a0…; VNmember328/KR584. R-164 xác nhận tiêu thụ một số ordinal trênVN, không nativeKR runtime/offsets hoặc1ms/tickRust |

R-078/R-079 không bị đổi thành full mapping/timing confirmed: chỉ phạm vi R-164/R-165 nêu trên được bổ sung.
Profile và luật damage giữ nguyên; không lấy group C/L/U/R/D hoặc rawduration1 làm lifetime vùng sát thương.


## 29. Getter tầm thực sự dùng khi đặt bom VN (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-166 | Hai lần đặt bom cùng actorVN: virtualcall5B1F92/+0x7C nhận receiver=hero+0x510, vtableC663C4→5AE518; EAX1 trước Fire và2 sau117/158 khớp111r18 tương ứng. Wrapper gate+2C14 khác0 trả1, bằng0 tail5A021B; basegetter578963 đọc boxed receiver+1C qua414543 sau virtual+BC(arg1), nhưng5A021B còn override có điều kiện | Getter/wire scalar1→2, không phải cap | vn_runtime + vn_static / confirmed (đúng callsite, receiver, return/wire và cấu trúc getter) | [Evidence/CSV/50hash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#receiver-thật-của-getter-tầm--r-166); fixturebc875…;2call/3returnrows có duplicate. Gate semantics, writer/clamp, override runtime, Roller và C00/C01KR unknown. Boxhero+52C chỉ là số học;5B1FB5 là directcall6EFD24, không phải virtualgetter |

Bổ sung R-025/R-111/R-163, không thay profile hoặc nâng các trường BomberAttrKR candidate thành
confirmed. Runtime dùng giá trị getter cuối; chưa đo box nền hoặc chứng minh mọi nhánh override.


| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-167 | Nhánh chọn bom cùng facet: virtual5B1EC5/+48→59F493→579DC0, rồi container=facet−8=hero+508→578A16. Duyệt list+4C/count+50; chỉ chọn entry có boxed+1C==0 vàboxed+E4==0; hết list trả0, chọn được thì điền descriptor/trả pointer | Điều kiện chọn phần tử; không có cap số học đã chứng minh | vn_static / confirmed (cạnh hàm, offset, nhánh và kết quả trả về) | [Evidence/hash và giới hạn](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#nhánh-chọn-phần-tử-bom--r-167); predicate58D4DA/59111A→414543. Chưa đo runtime count/predicate, writer count, nghĩa field hoặc KR. Không đồng nhất count với trần stat10 |


## 30. Giới hạn sức chứa và số phần tử pool VN (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-168 | Cùng actorVN/Factory01, probe so trước cấp phát5B1E58: virtual+88/+84 cho0/1 rồi1/2…9/10 sau9pickup; ba pickup còn lại và một ôtrống khác đều10/10, không111. List luôn25 phần tử; tại giới hạn còn15 entry có cảhai predicate0. Điều kiệnJGE bỏ qua allocator5B1EC5 dù pool chưa hết | Getter limit dừng10 trong ca; pool25, còn15 phần tử đạt điều kiện chọn | vn_runtime + vn_static / confirmed (fixture/actor/path đã đo) | [14mẫu/35hash/patchscope](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#so-sức-chứa-trước-cấp-phát--r-168); vtableC663C4 +88→5AEC77, +84→5A0274.12pickup117/158,10placement111/152,không151/120. Không phải cap mọi nhân vật/KR hoặc native literal/clampwriter10 |

Thay phần chưa phân biệt plateau của R-163 bằng phép so getter và list thực trong đúng ca. Base5788B8
truyềnfacet+4 qua stub.devirt16B62F0→414543 (patch5788C9); không đo box nền hoặc các override5A0274.
Giới hạn patchedfixture giữ rõ ở evidence; các khóa KR candidate/profile không đổi khi identity chưa nối.


## 31. Khởi tạo count pool SFX trên đúng engine KR (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-212 | DispatcherD1CD10 nhận singletonmanager nhưng Play14BF4D0 dùng engine `[manager+0x68]`. Ctor/helper14BE290 khởi tạoengine+0C/count và+10/block; loop tạo tốiđa16voice, dừnghandle0, lưu sốiteration thành công tại+0C. Block0x48byte códriver+0, voicehandles+4+4i, handle riêng+44 | Init voice count≤16; không runtime count cố định | kr_static / confirmed (receiver, loop bound, writer/countpointer) | [7hash/PEanchors/ctorcache](../../evidence/measurements/kr-static-audio-2026-10-23/pool-count.json); KRcf72…; D1D12A..142 ECXfrom+68,14BE3F0..418loop,14BE423writer; helper463Bfull. Runtime count/devicefailure unknown; không thay policy8DESIGN R-083 hoặc BGMloop |

D1C3301428B không directstoremanager+0C/+10; điều này không phủ định helper, nhưng count đã được nối
trên engine khácreceiver. Không dùng1000/649/0x48 làm voicecount. R-207 giữ cơ chế dedup/drop,
R-212 bổ sung nguồn count; chưa có phép nghe/runtime16voice và chưa nâng gateG5.


## 32. Roller làm tăng vận tốc native trong cùng actor VN (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-169 | Một pickup117/158kind2 trên cùngactorVN/C664C4: hai cặpUp/Down trước/sau cho vận tốc cửa sổ trungtâm165.149–170.311 trước và197.861–203.787px/s sau; mẫu đổiY chủyếu5→6px. Không111/151/120, X560/actor448364544 giữ nguyên | px/s ước lượng theo nativeXY/monotonic window; khônglevel | vn_runtime / confirmed (hiệu ứng cùngactor/case) | [9windows/35hash/sai số](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#roller-và-vận-tốc-thực--r-169); fixturebc875…/Factory01index193/room19;755validrows. Cadence/level/normalization vàC00/C01KR unknown; +550probefailgiữrõ; không đổi speed24/Roller+1levelDESIGN |

Tĩnh đã truy input/state/dir/effect nhưng chưa thấy getter cấp dùng choXY. R-045 giữcandidate;
R-104/R-111phầnRoller và R-114speed normalization giữ lựa chọn hiện hành, không nâng nguồn chỉ vì
mẫu dịch chuyển có5px/6px. Hoàn thành câu hỏi hiệu ứng trong ca không tự đóng root149/146.

## 33. Yêu cầu lặp BGM qua callback EOS KR (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-213 | Đường BGM mapE71D70→D1D710→D1DA60→14BF390→14BEB10 đặt sampleuserdata5=1 vàđăngký EOScallback14BEC60. Callback nếuuserdata5!=0 gọi lạiAIL_start_sample trêncùnghandle; nhánh0mớicleanup. Bảy snapshotpointer khớp export của Mss32.dll đi kèm cùngpreferredbase21100000 | Static conditional restart request từ EOS | kr_static / confirmed (đường gọi, giá trịflag, pointer/export mapping của snapshot) | [11hash/5body/7exports](../../evidence/measurements/kr-static-audio-2026-10-23/bgm-eos-loop.json); callback14BEC6F/73/76, register14BEC06, startptr16EA868=211149B0→DLLRVA149B0; DLLSHA5cafd1f6…. Runtimeaudibleloop/device/stopflags/livebinding chưađo; không sửa toànR-083 hoặcaudioengine |

Chỉ phần yêu cầu lặp có consumer mới được bổ sung; mộtBGMvoice/mapping/priority/reconcile vẫn giữ
quyết định V1 riêng. Không lấy literal0 trong wrapper làm no-loop hoặc một nhánhcleanup làm cảcallback.


## 34. Getter movement VN trước/sau Roller (2026-10-24)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-170 | Cùng actorVN/C664C4 và controllerhero+5F4/C6630C: getter59EC93 trả tại595CCE và hero595CB6 trả tại5A64E6 đều5 trước,6 sau đúngmột Roller117/158; native158 return71B717 nằm giữa hai phase. Protectedcontroller+64 đổi5→6; các box+58/+2F8/+304/+20FC giữ nguyên | Native return scalar5→6 đưa vào local movement amount; không xác nhận level hoặc đơn vị thời gian | vn_runtime + vn_static / confirmed (đúng receiver, return, field và ca đã đo) | [4rows/52hash/staticPE/source/limits](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#giá-trị-getter-đưa-vào-movement-trướcsau-roller--r-170); fixturebc875…/Factory01index193/room19; actor449413520. Timingexcluded; branch+64/writer/clamp, normalization/max/KRidentity unknown. Consumervirtual+128→5A0F75 chưa đọc; không nâng5→6 thành level hoặc24px/s/level |

Bổ sung R-169 bằng getter thực, không đo lại vận tốc và không đổi các khóa profile. Giá trị EAX được
chụp sau return trước local store; field bằng nhau không chứng minh cùng nhánh. R-045/R-114 còn giới hạn
về level/đơn vị; không tự nâng source chỉ vì return và bước pixel có cùng số.


## 35. Amount và bước XY trên movement call VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-171 | CùngactorVN/C664C4: actualvirtual+244call59D902→59DA6F/return59D908 nhậnamount5→0 vàY519→514 trước Roller; sau native158 nhận6→0 vàY284→278, X560 giữnguyên. Haiouterreturn131E9DF, target/frame/thread/XYchecksum hợp lệ | Native argument5/6 và bướcY−5/−6pixel tronghai cặpđọc; timingexcluded | vn_runtime / confirmed (hai lời gọi trongđúng actor/ca) | [4rows/44hash/guardfail/actualcaller](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#amount-và-xy-tại-virtual-movement-thực--r-171); fixturebc875…/Factory01/room19, one117/158kind2/no111151120. Không suy level/FPS/px/s/collision/max/KRidentity hoặc nốiR170producerkhác capture; cache5A0F75conditional không làactualcallerđãđo |

Hai EAX5/6 là giá trị quan sát, chưa gán nghĩa toànhàm. R-170 getter vàR-169velocity là các capture khác;
không ghép đồnghồ để tạo công thức cấp tốc độ. Phần arg→XY trong hai lời gọi này đã có kếtquả, còn
sourceproducer/cadence/writer/normalization là câu hỏi tiếp, không đổi profile từ dữ liệu chưa cùng đại lượng.

## 36. Plateau của trường/getter movement sau nhiều Roller VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-172 | CùngactorVN/C664C4 và controllerhero+5F4/C6630C: protectedcontroller+64 banđầu5; native158pickup1 trả6, pickup2 trả7, pickup3..12 giữ7 dù 12ACKkind2 đều xửlý. Saupickup12, actualhero595CB6 return5A64E6 cóEAX7/C647; nativeXY vẫnđiDown/Up trong đối chứng cuối | Trường/getter scalar5→6→7 rồi plateau7 qua10ACK thêm; khônglevel/đơnvị thời gian | vn_runtime / confirmed (trường/getter trongđúng actor và12pickup đã đo) | [CSV/manifest/guard/failure](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#plateau-c64-sau-12-roller--r-172); fixturebc875…/Factory01index193/room19, actor449222352;24nativeACKrows+2getterrows/27traps. Chọn actor từ bootstrap, không chứng minh ECXreceiver của handler715E90; không suy capmọiKR/C00/C01/level/FPS/writer hoặc every-frame amount |

C64 tại mỗi entry/return native được đọc có kiểm cookie/checksum; EAX7 cuối là giá trị register độc lập.
Payload mãhóa không được lưu đủ để giải mã lại offline, giới hạn giống R-170/R-171. Mười ACK sau khi
đạt7 không làm C64 tăng trong các mốc đã chụp; đây là plateau quan sát, chưa xác định maxfield/clampwriter.
Đối chứng nativeXY đi xuống46→193 và lên193→53, không suy vận tốc từ keydown hoặc đồnghồ debugger.

Lượt capr1 không đăngký bootstrap nên không có phépđo. Capr2 có ba cặp actualv244 amount5/6/7 với
Ydelta−5/−6/−7, sauđó bỏsót cặp trước pickup4 và failclosed; không lấy12wireACK thành12nativecapture.
Capr3 chuyển điểm quan sát sang native158entry/return và getter cuối; giữ toàn bộ raw các lượt.
Không sửa profile, nhãn C00/C01 candidate hoặc gate; cadence/normalization/identity và writer vẫn theo Q-017/Q-020.

## 37. Roller VN: receiver thật và phép tăng/kẹp C64 (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-173 | Nativekind2 đi qua89C202 vớiARG1index2/ARG2selector3/ARG3=hero+5FC, receiverVT C66284/COLoffset5FC/CGOZzolmanBomber. Selector3 chọn nhánh89C2D8 gọislot+14; PEslot trỏ599499. Method dùngC64 tạireceiver+5C=controller+64, gọi445516 đọcold→old+1→ghi413F32, sauđó so updatedC64 với maxfieldreceiver+50=controller+58 và kẹp nếu vượt. Native trước/sau selecteddispatch là5→6 | Phép tăng1 và kẹp theo maxfield trên lvalueC64; khônglevel/đơnvị thời gian | vn_runtime + vn_static / confirmed (typedreceiver, dataflow và ca đã đo) | [CSV/payload/hash và scope](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#receiver-và-phép-tăngkẹp-c64--r-173); fixturebc875…/Factory01/room19; actualdispatch89C202→89CDCB trên cùngthread/frame. Virtualtarget599499 là liên kếtPE+nhánh đã chọn, không breakpoint trực tiếp lên target. Maxfield có địa chỉ, chưa đo số max trong actor này; chưa ánh xạ KR/C00/C01/f2–f3 |

Method599499 có63byte và child445516 có25byte đã kiểm gồmRET. Childcall445525→413F32,
return44552A xuất hiện trong hardwarewatch stack của capture khác. Capturewatch actor449290536
và capturedispatch actor449155584 được giữ riêng, không ghép đồnghồ hoặc identity. Bốn watchstop
cùngPC413FDA là trạng thái ghi payload mãhóa dở/checksumunknown, không phải bốn writer khác nhau.
Payload đầy đủ ở mốc trước/sau cho phép giải mã lại offline5/6 và kiểm cookie/checksum.

Công thức kẹp mô tả đường khôngoverflow của ca này; không gọi mọi phép ghi mãhóa là tăng chỉ số.
R-172 đã xác nhận plateau7 trong actor khác; không ghép nó thành literalmax7 của method này.
Reader414543 ở đường bảng item không tự chứng minh nó đọcC64; alias được chốt từ receiver/lvalue
thật. Còn thiếu nguồn khởi tạo current/maxfield, identity/Attr KR và normalization; không thay profile,
12khóa candidate hoặc trạng thái G2/G6.

## 38. Khởi tạo base/max/current của actor VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-174 | Trên một actor VN C664C4, base/max/current tại RET7BC41E là0/0/0, tới updater5B1472 là5/7/5. Hardware watch nối ghi base/max tới597432; setter59BD36 sao chép baseactor+640 vào currentactor+658, max ởactor+64C. Capture riêng cho current/max0/0→5/7 trên cùng actor của capture đó | Scalar protected fields; lifecycle và pipeline reset/ghi, chưa là level/px/s | vn_runtime + vn_static / confirmed (hai capture và lvalue đã nối) | [CSV/payload/hash, getter và giới hạn](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#khởi-tạo-base-max-và-current-trên-actor-vn--r-174); fixturebc875…/Factory01/room19;6+10stop,12ciphertransient unknown giữ raw, không ghép actor/clock |

`597432` nhận sáu output qua typed virtual `+0xA0→595FD8`, có nhánh modifier chưa xác nhận,
kẹp ba cặp local rồi ghi base/max; current reset riêng từ base. Getter có nhánh override/fallback,
chưa chứng minh sáu cột của một row, literal5/7 hoặc f2–f7 KR. Binding virtual theo PE không thay
breakpoint trực tiếp lên mọi target. Không đổi profile hay 12 khóa candidate; key/identity và
nguồn fallback còn ở Q-020.

## 39. Sáu output khởi tạo và key quan sát trên actor VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-175 | Virtual+A0→595FD8 cung cấp ba cặp đầu vào: movementbase/max actor+640/+64C, capacitybase/counterpart facet510+68/+74 và Firebase/counterpart+80/+8C. Setter5787AE kẹp hai cặp cuối; reset578979 sao chép+68→+4 và+80→+1C tới consumer capacity/Fire đã nối. Hai ca byte lựa chọn0/1 đều có key0 và output `[5,7,1,10,1,7]` trong lần khởi tạo đầu quan sát | Native scalar, vai trò field khởi tạo; không level/FPS hoặc identity KR | vn_static + vn_runtime / confirmed (dataflow và hai cửa sổ đã đo) | [8mốc/77hash/5block và wire checksum](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#sáu-output-tới-field-khởi-tạo-và-key-đầu-vào--r-175); fixturebc875…/Factory01/room19. Counterpart làm trần sau pickup còn candidate; không nâng f2–f7/C00/C01 |

Byte lựa chọn khác nhau đã kiểm trong cả S103/S106, nhưng hai key đầu vẫn0; đây là kết quả
không phân biệt được identity, không chứng minh lựa chọn bị bỏ qua hoặc các lần khởi tạo sau.
Marker+A4 không chạm; không kết luận hiệu ứng modifier. Getter base/max có override/fallback
theo key; không phải literal5/7 hoặc sáu cột row liền nhau. Giữ profile/12candidate/G2/G6.

## 40. Argument và phản ứng rẽ trên mover player VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-176 | Trên actorVN C664C4, actualmover59DA6F nhánh59DABE gọi594B68; rare stop59DAD9 cho out16=`[560,480,40,40]` khi nativeXY560/519. Hai ca trước Down có XY540/39 và535/39, sau điều khiển chỉDown đạtX560/Y109 hoặc104; có Up/Right stop và Left/Down positive controls | Bốn scalar argument và trajectory native trong ca; không level/thời gian/hurtbox | vn_runtime + vn_static / confirmed (argument và phản ứng quan sát); corner assistance là candidate | [Payload/13endpoint/60hash/source/giới hạn](../../evidence/measurements/vn-player-movement-2026-10-25/README.md); fixturebc875…/Factory01/room19, sameactor/thread. Output ứng với `[X,Y−39,40,40]` tại mốc; downstream collision/damage semantics, maxassist và mọi mode chưa xác nhận |

Đây là đường actor người chơi được điều khiển trong ca VN, độc lập PET/AI MoveSimple KR.
Không đổi radius13/assist12 hoặc gán số40 làm toàn bộ hitbox. CaB có mốc trướcX535 nhưng mẫu
đầu sau markerDown đãX540; không gọi25px là toàn bộ dịch chuyển sau native nhận input. Giữ
overshoot từ requested550/540 tới actual540/535, không loại mẫu để ép ngưỡng.

## 41. Hai offset ngang có phản ứng Down khác nhau trong một actor VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-177 | Cùng actorVN C664C4/base5/max7/current5: trước Down ởX530/Y39, sau điều khiển chỉDown sửa ngang vềX560 rồiY139; trước DownX525/Y39 thì giữ525/39. AlignedX560 có Down→Y164/Up→39, các reset/opposite controls hợp lệ | Offset30 có phản ứng sửa ngang;35 không di chuyển trong ca. Native pixel, không ngưỡng chính xác | vn_runtime / confirmed (coordinate response); cơ chế corner assistance candidate | [Trajectory/9endpoint/42hash/giới hạn](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#đối-chứng-lệch30-và35-trên-cùng-actor--r-177); fixturebc875…/Factory01/room19/actor449280160. Không max34, mọi level/mode/hướng hoặc hurtbox; không dùng timing để suy velocity |

R-176 vàR-177 là hai actor khác nhau, không nối đồnghồ/identity. Giữ mọi overshoot giữa requested
và actual; current5 là scalar theo field đã nối, chưa chứng minh speed level KR. Profile không đổi.

## 42. Field S501 tới key tra chỉ số trên context VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-178 | S501 per-slot byte thứ hai được zero-extend rồi8E76A3 ghi DWORD vào contexttable+slot244+60. Native slot0 ghiFFFFFFFF→1; descriptor C20934/CBomberExternalContextFC có pointer+4 alias đúngcontext, virtual+34→4B82C2/8E76CF đọc field này. Arg3 qua call6EB89D/7BC250→5AED95 và key597471 đều1; output6 `[4,8,2,7,1,9]`, reset4/8/4 và capacity2/7/2, Fire1/9/1 | Key và ba cặp scalar khởi tạo trong ca, không level/FPS hoặc identity KR | vn_runtime + vn_static / confirmed (nativefield, alias và dataflow) | [Payload/contrasts/source/failures](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#field-bắt-đầu-trận-tới-key-và-sáu-output--r-178); fixturebc875…/Factory01/room19/actor395728984,7stop. Counterpart cap sau pickup/C00C01 asset binding chưa đo; không auto nâng12candidate |

Control trước có roombyte1 nhưng field S5010→key0/output `[5,7,1,10,1,7]`; actor khác giữ riêng.
Nativecurrent4 có hai cửa sổ XY sau hooks tắt, ước lượng136,799/134,336px/s với giả định nhạy ghi ở
evidence; không dùng scalar4 làm speedlevel/FPS hoặc thay speed24DESIGN. Input S501 có native
setter argument và generator provenance, không log framehex để tuyên bố wireCRC. Profile/gate giữ nguyên.

## 43. Nhánh tile passable theo hướng trong player mover VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-179 | Player mover59DA6F có nhánh lấy object theo ô qua7293B4, guard/cast CGOTileFixed→CGOTilePassable rồi5C1027→5C0E98 đọc byte theo dir. Return0 đặt cờ chặn và đi nhánh kẹp rectangle | `BYTE[DWORD[tile+AC]+4+dir]` với dir0..3; invaliddir trả0 | vn_static / confirmed (code path) | [Caller/RTTI/case/hash](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#predicate-ô-passable-của-player--r-179); fixturebc875…, anchor playerR176. Chưa có runtime branch witness/property producer; không phải toàn bộ map collision/hurtbox/max34 |

Object lookup dùng `context+B328+(row*101+column)*4`, sau các guard ghi ở metadata; return là
cast result hoặc0. Byte nonzero chỉ bỏ qua restriction này, không bảo đảm được đi qua mọi vật cản.
Giữ native property values, actor.vAC geometry, nghĩa tên bốn hướng và profile hiện hành chưa đổi.

## 44. Logical Attr row và trần loaded của Bomb/Fire VN (2026-10-25)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-180 | Reader971383/post971517 nạp inputname/f0 Character/C00=0, Character/C01=1 vào cùng manager10596C50/C83AEC, rowstride19C. Consumer595FD8 dùng globalE60844 alias chính manager, key1; sáu actual414543return từ rowC01+1D4/1E0/1EC/1F8/204/210 lần lượt4/8/2/7/1/9, out6 giống tại597477 | Logical row/f0 và native scalar; C00stored5/7/1/10/1/7, C01stored4/8/2/7/1/9 | vn_static + vn_runtime / confirmed (named rows, pointeralias và actual reader chain) | [18CSV/63rawhash/source/cleanup](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#tên-hàng-attr-tới-sáu-reader-native--r-180); fixturebc875…, actor393578184. Không avatar/tên hiển thị/nativeKRconsumer/FPS. Inventoryfinal là pre-disable snapshot, không proof enabled=false; wrapperexit/namespacecleanup kiểm riêng. Captioncandidate ca khác chỉ ra tester, nhánh đó dừng |
| R-181 | Selector1/2 của dispatcher89C202 trên kiểu receiverC66284→594EDB/594EE9 rebase hero+5FC−EC→facet510/C663C4 vA4/vA0→578842/5788E1. Mỗi method dùng445516 tăng current1 rồi kẹp theo max loaded; R175 nối max này tới output3/5 | Capacity hero+514/max+584; Fire hero+52C/max+59C; current=min(old+1,max) trong miền signed32 bình thường | vn_static / confirmed (typed source và semantics method) | [Source/slot/fieldrole](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#tăng-và-kẹp-capacityfire-theo-trần-đã-nạp--r-181);7block/761retained,25Bchildreuse. Selector1/2 chưa là runtimeactivation mới, overflow/override/globalKRcap/ItemAttrgroup unknown. Counterpart capacity/Fire của R175 đã có max-consumer; không gọi cap mọi mode |

R180 giải logical Attr name/key bằng native source, không chỉ matching số R040/R041. R181 bổ sung
max-role còn candidate trong R175; capture cũ giữ scope riêng. Không tự nâng R042/R117 hoặc12khóa
profile: quan hệ statpoint, đại lượng engine và quy đổi thời gian cần review tường minh theo ADR0013.
G2 không đòi avatar cho logicalrow đã xác nhận; nativeKRconsumer/ItemAttrdrop và toàn bộ146/149 còn
fidelity riêng. Profile/releaseflag/gate không đổi trong hai claim này.


## 45. Lựa chọn logical statpoint C00/C01 cho profile V1 — R-182

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-182 | Chọn12khóa `characters.c00_*`, `characters.c01_*` theo namedrow/consumer VN cho logical base/max statpoint: movement, capacity, Fire. `unit=level` là nhãn họ scalar CharacterSpec/Stat; không phải tốc độ vật lý | C00:5/7,1/10,1/7; C01:4/8,2/7,1/9; logical statpoint | `vn_runtime` / confirmed (logical row và scalar role; có source static hỗ trợ) | R173/175/178/180/181; [namedrow/reader/max](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#tên-hàng-attr-tới-sáu-reader-native--r-180). R171 nối movementamount; R168 capacitycount; R166 Firegetter/r18 và R025 tăng tầm. Không nativeKRconsumer/FPS/pxs hoặc effectivecap mọi mode. DESIGN mapping vật lý/engine bên dưới không thành nativeconfirmed |

R-117 **superseded by R-182** cho12profilekeys; R040/R041 sốKR vẫn confirmed và R042 mappingKR
candidate lịch sử giữ nguyên. Nguồn chọn hiện `vn_runtime/confirmed` theo ADR0002/0013, không gọi
R042 là nativeKRconfirmed. Chỉ thay metadata nguồn/claim, không đổi12số hoặc corealgorithm.

Đại lượng engine được khai báo tường minh:
- Movementpoint là scalar base/current/max, đã có native amount5/6 và loadedcounterpart clamp.
  Engine dùng `speed.value` nhân **R18433px/s/point DESIGN**, chia server30Hz DESIGN. R183 đo
  cùngactor point4..8; ví dụ point4 engine132px/s so với native khoảng133,70px/s. Lựa chọn24 của
  R114 trước hiệu chuẩn là lịch sử, không phải vận tốc hiện hành.
  Không suy nativeFPS/linearcurve hoặc gán30Hzserver thành nhịp client.
- Capacitypoint được engine dùng làm số bom còn tồn tại tối đa; native R168 có actualactivecount
  so với capacitygetter. Giá trị khởi tạo/max đến R175/180/181; không gọi pool25 là statmax.
- Firepoint là scalar của getter tầm và base/max. Engine chọn một point tương ứng một ô ray theo
  contract R057/R111; R025/R166 hỗ trợ tăng tầm, nhưng pixel/vùng damage/all-mode vẫn không từ đó
  thành confirmed. Kích thước tile40px là khóa riêng R110, không đơn vị của field scalar.

Việc dùng logicalstatpoint cùng nhãn `level` là contract schema hiện hành, không chứng minh native
có enum tên level. Source scalar và DESIGN ánh xạ được tách, không nâng hệ số33/shape13/assist12 thành nativeconfirmed.
`release=false` giữ nguyên; loader vẫn chặn candidate nếu bật release. Profile đủ nguồn logical
không tự là nghiệm thu G2/G6/V1-036; cần reviewer độc lập tiêu chí G2, gói/version/hash và phần G6.

## 46. Curve tốc độ cùng actor và hiệu chuẩn V1 — R-183/R-184/R-185

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-183 | Cùng actor C01/key1/base4/max8, đo nativeXY ngoài process ở movementpoint4..8; bốn native Roller ACK đưa4→5→6→7→8 | Mean OLS133,698888 /166,077857 /200,698866 /233,232185 /267,052717 px/s; normalized33,2156–33,4498 px/s/point trong ca | `vn_runtime` / confirmed (quan sát rate cùngactor trong ca) | [12window/40rawhash, R1 riêng và phương pháp](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#curve-tốc-độ-cùng-actor--r-183r-184r-185). Fixturebc875…, actor449284224/C664C4, Factory01/room19. HOST CLOCK_MONOTONIC bracket đọc nativeXY, không nativeclock/FPS, literal coefficient hoặc mọi mode/character |
| R-184 | `player.speed_px_per_s_per_level` chọn hiệu chuẩn DESIGN theo R183, thay fallback24 của R114 | 33 px/s/point; engine132/165/198/231/264px/s ở4..8; server30Hz→1100mpx/tick/point | `design` / design | Trung tâm normalized33,358116; chọn nguyên gần nhất33. Không chứng minh native chứa33 hoặc phân biệt33/34 theo thống kê; point1..3/actor/mode khác là ngoại suy V1. Không khôi phục confidence R075/R043 |
| R-185 | `item.roller_speed_delta`: một Roller tăng một logical movementpoint, bị kẹp bởi max loaded | +1 logical point; R2 bốn ACK4→5→6→7→8, số profile1 không đổi | `vn_runtime` / confirmed | R173 actualselector3/increment/clamp, R182 logicalunit và R183 bốn actual native715E90→71B717 pair. Thay riêng fallback Roller của R111; không cap mọi mode/+1pxs/ItemAttrgroup |

R183 có900rawrow/0invalid,9observerstop/3hook;12windowUp/Down không chồng stop. Phương pháp lấy
first/last nativeYchange trong scope điều khiển, trim150ms mỗi đầu, bracket đọc nằm trọn bên trong;
được chọn sau R1 rồi áp không đổi cho R2. Không lấy keyhold/server/packet timestamp làm vận tốc.
OLS theo midpoint bracket; giữ first-last/Theil, read-bracket variation và mọi sampled delta kể cả21.
Sensitivity mô hình bảo thủ khoảng±31–39px/s dùng2×maxΔY21/span cộng full read-bracket difference;
đây không95%CI/nativeupdate. Riêng full read-bracket OLS difference khoảng0,001–0,078px/s.
R1 actor khác, fullcurve incomplete do controller/lastACK, giữ34rawhash và validpoint4/5 riêng,
không gộp actor hoặc bỏ lỗi để thành phép đo hoàn chỉnh. Không client mới để tìm lượt xanh.

Coefficient24 cũ làm engine chậm khoảng28% so với ca;33 còn lệch mean khoảng−0,65…−1,34%.
Đây là calibration có chủ ý, không nâng literal33 thànhconfirmed. Trapped17, fuse2500, flame540,
trap6350, halfbox13/assist12, server30Hz,12charvalues và releasefalse giữ nguyên. Actual movement
regression chạy30tick: profilecũ24 đỏ96000≠132000mpx ởpoint4; profile33 xanh4..8 và trapped16980mpx
độc lập point. Scenario/replay dùng profile riêng không được bless chỉ để che đổi tốc độ.

## 47. CItemResourceMan — hai bảng và startup global alias (2026-10-26)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-186 | Trong `CItemResourceMan` (RTTI `.?AVCItemResourceMan@@`, vtable C77D70) có hai bảng: bảng1 count `[this+0x3B94C]`, base inline `this+0x3B9AC`, stride192; bảng2 count `[this+0x10]`, base inline `this+0x1C+idx*0xF4`, stride244 | Hai table descriptor; stride192/244 byte; row/group semantics chưa gán | `vn_static` / confirmed (cấu trúc prefix) | Block 89F436/89F536/89F5A1, metadata/hash và terminal trong [evidence](../../evidence/measurements/vn-itemattr-manager-2026-10-25/README.md). Prefix-only, `complete_function=false`; không whole parser/function boundary. Nghĩa row, quan hệ R153 group244, caller89D113 `+0xBC`, field/ItemAttr group/drop vẫn unknown |
| R-187 | Startup loader object `CItemResourceMan` tại `0x109CEAE8`/C77D70 mở literal `Attr/ItemAttr`; tại Open89F436 `E60934` bằng đúng object/vtable và actual EAX1 | Object/C77D70; literal C77D7C; `E60934=0x109CEAE8`; EAX1 | `vn_runtime` / confirmed (startup scoped) | R1 `host-item-manager-alias-149-r1` probe9afec/harnessd178/serveref688; [metadata](../../evidence/measurements/vn-itemattr-manager-2026-10-25/README.md). S191 thiếu, không lateBoot/Boot, EXIT2 không rerun. Chỉ startup loader/global alias; không persistence/whole parse/field/group/drop |

| R-188 | Bảng2 stride244 của `CItemResourceMan` (count `[this+0x10]`, base inline `this+0x1C`) có cấu trúc tương ứng bảng group ItemAttr KR R-153 (stride244, count `+16`, base `+28`, field `+0x94` slot); VN row ghi `+0x94/+0xA0/+0xAC`, nhưng offset count/base lệch `0xC` | stride244; VN count+10/base+1C/row+94; KR count+16/base+28/group+148 | `vn_static` / candidate (tương ứng cấu trúc, chưa identity) | [Crosswalk/limit](../../evidence/measurements/vn-itemattr-manager-2026-10-25/README.md#r-188r-189-bảng-244-và-caller-89d113). Không kết luận cùng class/table; không `item.*`, weight/drop/cap |
| R-189 | Caller `89D093/89D113` đọc row `manager+idx*0xF4+0xBC`; giá trị nonzero được truyền vào `receiver.vtable+0x18`, zero đi `89CAC8` theo tọa độ. Không có phép cộng/ghi `item.*` hay cap trong prefix đã đọc | row+BC; dispatch vtable+18 / fallback 89CAC8 | `vn_static` / unknown (role field) | Block89F5A1/89D093/89CAC8; `+0xBC` role chưa gán ItemAttr group/effect/cap; không +1 arithmetic, không đổi profile V1. Không đọc body mới/helper |

Hai bảng nằm cùng typed manager nhưng **không** đủ để gọi bảng192 là item hay bảng244 là group/ItemAttr.
R153 (192/244 native KR) và caller89D113 `manager+idx*0xF4+0xBC` giữ là cạnh riêng; `+0xBC` role
unknown, không phải +1 arithmetic. Legacy `items.md`/export chỉ là parser/locator, không xác nhận
weight/drop/pack identity. Không đổi profile, G2, G6, release hoặc 146/149 khác.

R188 chỉ là **crosswalk có giới hạn** giữa bảng244 VN và group244 KR R-153; chênh offset count/base `0xC` và thiếu consumer item
nên **không khóa `item.*`** và không chứng minh cùng một bảng. R189 cho thấy `+0xBC` là giá trị dispatch/opaque, không phải
delta/cap item. V1 giữ `item.*` hiện hành; nếu một consumer thực xuất hiện, cần PR profile riêng. Không đổi gate/G2/G3/release.

## 48. Caller player của predicate R179 — R-214

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-214 | Predicate R179 nằm trong **player mover `59DA6F`**, không phải nhánh `CMovable`/pet: R171 đã thấy call player virtual+244 `59D902→59DA6F`; trong cùng thân hàm, `59DD49` nạp tile qua `7293B4`, rồi `59DD5E` gọi `5C1027→5C0E98`. Đây là caller-chain static của player, không phải suy từ tên hay từ pet | Caller chain `59DA6F→59DD49→7293B4`, `59DD5E→5C1027→5C0E98`; RVA `19DA6F`, `19DD49`, `19DD5E`, `3293B4`, `1C1027`, `1C0E98` | `vn_static` + `vn_runtime` / confirmed (static caller chain; receiver player anchored bởi R171 actual call) | Fixturebc875…/base400000; cached `59DA60..59E210` SHA `04a198a7…`, clips7293B4/5C1027/5C0E98; [R171](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#amount-và-xy-tại-virtual-movement-thực--r-171), R176/R179. Không runtime branch witness mới, không chứng minh map-block/hurtbox/max34/toàn bộ59DA6F, không dùng CMovable/pet. REA IDB adapter không trả evidence_id trong lượt này; locator cache/VA là nguồn static đã đối chiếu, không tạo ID giả |

R-214 chỉ nối caller R179 vào player mover đã có R171/R176/R179; không đổi giá trị
`player.collision_half_px`/`corner_assist_px`, rules, profile, source precedence hoặc gate. G2 vẫn
được nghiệm thu bằng DESIGN/VN có giới hạn; #146 vẫn OPEN PARTIAL, chưa có runtime branch/property/
hurtbox/max34/all-map. Fixture/namespace/read-only giữ nguyên; không runtime/PR khác.

## 49. Nhịp frame nhân vật/lửa: counter cập nhật, chưa có ms (2026-10-26)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-215 | Consumer animation dùng **counter cập nhật** so với duration per-record, không thấy đổi frame bằng `dt`/ms: `BA3530` so `B33170()` với `[record+4]`; `B33170`/`B331E0` chỉ delegate `sub_636620(this+64/+52)`. Flame dùng nhánh riêng `B711C0`, tăng counter mỗi update và xoá theo `GetAnimCount` (`BA3FF0`). ODT duration là ngưỡng counter; flame raw duration đều1. Loop/hold nằm trong các nhánh trả của `BA3530` (giữ/tiến/quay đầu) | Counter/threshold raw; **không** chứng minh ms, tick hay tần số update; flame raw duration1 | `kr_static` / confirmed (consumer/threshold shape) — **unknown** unit/cadence/runtime state | `BA3530`/RVA`7A3530`, `BA40F0`/`7A40F0`, `BA4160`/`7A4160`, `B711C0`/`7711C0`, `B33170`/`733170`, `B331E0`/`7331E0`, `sub_636620`/`236620`, `BA3FF0`/`7A3FF0`; evidence `ev_ec5aea3b…`, `ev_719cc83b…`, `ev_7ff15ec0…`. R-141/R-151/R-164/R-165. Không đọc body `sub_636620`; không chứng minh nhịp update, state walk/idle/blink, loop/hold runtime hoặc mọi nhân vật/mode |

R-215 chỉ chốt **cơ chế consumer dùng counter/ODT threshold**, không chốt giá trị frame duration
tính bằng ms. `presentation.frame_duration_ms` hiện100ms (R-118) và `anim::character_loop_frame`
không đổi bởi claim này; chưa có bằng chứng để thay33/100 hay bất kỳ số nào. Đề xuất kế tiếp nếu cần
nhịp thật: một runtime counter observation trên `B33170`/`B331E0`/`B711C0` trong cùng actor, ghi
số lần update trên cửa sổ thời gian; không suy từ screenshot/30Hz server.

R-164/R-165 vẫn chỉ xác nhận flame state/raw duration; R-141 vẫn xác nhận flame display = frame
counter. Không đổi profile, G2/G6, release, feature count hoặc #149.

## 50. Định vị VN cho counter animation — blocker (2026-10-26)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-216 | **Chưa định vị được consumer/counter animation tương đương trên fixture VN**; không dùng VA KR cho VN. Các accessor `CDataI16Alpha` (vtable C79714) chỉ là candidate frame-record API; chưa chứng minh chúng là `BA3530`/`B33170`/`sub_636620` hay counter update | Không có giá trị ms/tick; không đổi profile | `vn_static` / **unknown** (candidate locator only) | Fixture `bc875…`; candidate `CDataI16Alpha` accessors `0x8D50DF/0x8D50F3/0x8D510B/0x8D5123/0x8D5136/0x8D514A` (record base/+0/+4/+8, stride0x30), vtable C79714; `CGameObjectTemplate` vtable C378D4/methods `5C…`; `Animation` constructor `412A1F` is not a counter. KR evidence `ev_ec5…`/`ev_719…` belongs to `kr-full.exe`, not this VN fixture. REA IDB VN returned unreadable; no counter/consumer confidence |

Không có PR/thay đổi profile từ R-216: `presentation.frame_duration_ms` giữ100ms DESIGN và
`anim::character_loop_frame` giữ nguyên. Đây là **blocker phạm vi VN locator**: cần root/IDA names
hoặc pattern body xác nhận trước khi đặt breakpoint; không chạy counter observation bằng địa chỉ KR.
Không suy ms từ30Hz/screenshot. R-215 vẫn chỉ static mechanism; #311 runtime chưa có số.

## 51. Nhịp idle/blink C00 trên video VN — R-217 (2026-10-26)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-217 | C00 đứng yên trên bản sao VN đổi trạng thái idle/blink theo chu kỳ khoảng **1,52 s** (hai khoảng nền 1,517/1,533 s); trong cửa sổ active, khoảng đổi hình median **83 ms** (16–117 ms). Đây là **quan sát video**, không phải frame duration native: capture thực tế 431 frame/11,983 s, PTS median33 ms (≈35,9 fps), không phải60 fps | Chu kỳ idle/blink≈1,52 s; active change median83 ms, sampling ±33 ms; không có ms/frame native | `vn_runtime` / **partial** (visual change, không locator) | Raw local `/home/admin/boom-rs-dsh/re-kr/.re-work/anim311-video-r2/capture.mkv` (không commit), `actions.jsonl`, `analysis.json`; C00 đứng5 s rồi đi3 s. Walk không tách được frame khỏi dịch chuyển vị trí; fire không đặt. Không dùng PTS60/30Hz giả; không có native counter/ODT mapping |

R-217 không đủ để đổi `presentation.frame_duration_ms`: giá trị hiện100 ms nằm trong sai số/quan sát
83±33 ms; **đề xuất giữ100 ms** và không đổi `anim::character_loop_frame` chỉ từ video này. Nếu cần
per-clip walk/blink, phải đo với anchor sprite tách vị trí hoặc locator native; không suy walk từ video
dịch chuyển. Không đổi profile/G2/G6/release; #311 runtime vẫn partial.

## 50. Walk C00: pose và bước dịch trên video VN — R-218 (2026-10-26)

| ID | Claim | Giá trị / đơn vị | Nguồn / trạng thái | Locator / giới hạn |
|---|---|---|---|---|
| R-218 | Trên video VN đã quay (C00 giữ trái 3 s), bám **red-mask bbox** theo vị trí: 105/105 frame walk tracked; trong đoạn 5,15–6,30 s có 41 mẫu. Bước dịch median **6,24 px/frame video**, PTS median **33 ms** (≈30,3 fps); linear x ≈**214 px/s** trong đoạn (có đoạn né/kẹp nên chỉ là video-scope). Pose đổi theo cụm median **3 frame video** (~99 ms ở33 ms); transition median **83 ms** (17–433 ms) | Step≈6,24 px/video frame; walk pose≈3 frame/~83–100 ms; speed video-scope≈214 px/s | `vn_runtime` / **partial** (visual tracking, không locator native) | Raw local `/home/admin/boom-rs-dsh/re-kr/.re-work/anim311-video-r2/capture.mkv` (không commit), `walk_track.json`, `actions.jsonl`. Capture thực tế 30,3 fps, **không phải60 fps**; sai số±33 ms, không phải±16.7. Không có native counter/ODT, không chứng minh mọi đoạn/mode; fire không đặt |

R-218 **không đổi** `presentation.frame_duration_ms`: pose walk median3 frame video ≈99 ms (transition
median83 ms) nằm trong/quanh100 ms DESIGN; **đề xuất giữ100 ms** và không đổi
`anim::character_loop_frame`. Bước6,24 px/frame video cho thấy client gốc cập nhật vị trí theo bước
rời (không nội suy tại capture), khác mô tả Rust30 Hz+nội suy client; đây là quan sát để so cảm giác,
không phải bằng chứng FPS/native tick hay yêu cầu đổi profile. Không đổi G2/G6/release; #311 partial.
