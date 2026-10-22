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
| R-025 | Tầm nổ khởi đầu 1; mỗi ItemFire +1 tầm; mỗi ItemBomb +1 số bom đặt cùng lúc | — | `vn_runtime` / confirmed | oc_e §5a-5d (hiệu ứng bị động) |
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
| R-036 | Người bị nhốt không di chuyển trong V1 (đứng tại chỗ tới khi chết) | — | `design` / design | SCOPE_V1 §4; R-029 (`web` / candidate) nói "đi rất chậm" nhưng chưa có px/s; quyết định cho V1-012 |

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
| R-059 | V1: người bị nhốt (`Trapped`) hoặc đã chết không nhặt item; người bị nhốt đứng yên nên cũng không tới ô item | — | `design` | Task #7; R-036 (đứng yên), R-028 |
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
| R-082 | Mapping event→entry V1 (quyết định V1, **không phải** suy ra từ nguồn static): `bomb_placed`→`Wave/Cha_BombIgnite`, `bomb_exploded`→`Wave/BombExplosion001`, `item_revealed`→`Wave/Itm_ItemPop`, `item_picked` (Bomb/Fire/Roller)→`Wave/Itm_ItemEaten`, `player_trapped`→`Wave/Itm_UseTrap`, `player_died`→`Wave/Cha_Die`, `round_ended` thắng/thua/hòa→`Wave/Ntc_Win`/`Ntc_Lose`/`Ntc_Draw`, đếm ngược 3/2/1→`Wave/Ntc_Count3`/`Count2`/`Count1`, start→`Wave/Ntc_GameStart`, `ui_click`→`Wave/Bt_p_Default`; mapping là lựa chọn V1 vì chưa có trace consumer gốc, nhưng **cả 15 entry tồn tại trong KR và VN với cùng SHA-256** | — | `design` / design (entry có thật trong static; mapping do dự án chọn) | Issue #64; [AUDIO_V1](../assets/AUDIO_V1.md) §2–3; `crates/boom-assets/src/audio.rs` (`V1_AUDIO_USES`); Q-015 |
| R-083 | Policy chồng tiếng V1: tối đa **8 voice SFX + 1 BGM**; BGM phát loop một bản, không tính voice và không restart khi reconcile; loop/một voice là design, không phải confirmed từ consumer gốc. Cùng một entry phát nhiều lần trong **cùng một tick** gộp thành một lần; event được dedup theo `(round_id, EventId)`, không phát lại khi replay/reconcile. Ưu tiên khi thiếu voice: round-end/start > explosion > trapped/died > bomb/item > UI; hết voice thì bỏ mục ưu tiên thấp nhất, hòa giữ `EventId` nhỏ hơn (cũ hơn). Qua tick vẫn được phát lại | — | `design` / design | Issue #64; [AUDIO_V1](../assets/AUDIO_V1.md) §4; Q-015; ADR 0008 (EventId/dedup); R-082 |

R-081 xác nhận **entry/mapping asset BGM** từ manifest R-070 và hash trùng ở hai build; policy loop
một voice thuộc R-083 design. R-082/R-083 là **DESIGN của dự
án**, không phải claim `confirmed` KR/VN: chưa tìm thấy trace consumer phát SFX trong client gốc
(A10 của `features.md` vẫn là "chưa điều tra") và chưa có consumer audio V1. Q-015 theo dõi việc
đóng hai claim này; khi có trace, sửa bảng/giá trị thay vì tự nhận confirmed.

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
| R-108 | Trapped đứng yên; giảm timer cả tick vừa nhốt; timeout trước touch; touch khi cả hai độ lệch nhỏ hơn 28 px | 28 px (strict); 120 tick tính cả tick bị nhốt | `oss` / design | BnB `server/game.js:454–474,546–567,634–643`; #132; thay R-034/R-035, giữ đứng yên R-036 |
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

| ID | Khóa / lựa chọn hiện hành | Giá trị | Nguồn / trạng thái | Locator và giới hạn |
|---|---|---|---|---|
| R-110 | `grid.tile_px`; `round.duration_ms` khôi phục cùng đại lượng | 40 px; 120000 ms | `vn_static` + `vn_runtime` / confirmed | R-001 (`0x735DA9`); R-010 (`ctx+490518`); ADR 0013 chọn tầng 2; không có KR confirmed cùng khóa |
| R-111 | `item.bomb_capacity_delta`, `item.fire_range_delta` dùng VN; `item.roller_speed_delta` giữ BnB | +1 bomb; +1 cell; +1 level | Bomb/Fire `vn_runtime` / confirmed; Roller `oss` / design | R-025; Roller R-045 còn candidate nên chọn R-104/BnB `server/game.js:21,579–581` tại SHA §10; ADR 0013 |
| R-112 | `drop.weight_bomb/fire/roller` trả số bảng VN; `drop.brick_item_chance` giữ BnB | 3650:2800:2000; 350 permille | Trọng số `vn_static` / confirmed (số bảng); chance `oss` / design | R-051 `sub_89F3DE`; R-052 vai trò bảng vẫn unknown. Dùng bảng làm trọng số ba item V1 là quyết định áp dụng, không xác nhận xác suất drop gốc. Chance R-053 unknown → R-106/BnB `server/game.js:20,425–439` SHA §10 |
| R-113 | `sim.ticks_per_second`; `round.warmup_ms` giữ BnB | 30 Hz; 3000 ms | `oss` / design | R-100/BnB `server/game.js:12–19,616–646` SHA §10. R-013 đo bước client, không xác nhận tick server; R-060 xác nhận pha GO nhưng không xác nhận 3 s |
| R-114 | `player.speed_px_per_s_per_level` giữ BnB | 24 px/s/level | `oss` / design | R-102/BnB `server/characters.js:15`; R-043 checksum chưa xác thực; speed level không phải px/s |
| R-115 | `bomb.fuse_ms`, `bomb.flame_ms`, `player.trap_to_death_ms` giữ BnB | 2500/500/4000 ms | `oss` / design | R-103/BnB `server/game.js:14–17` SHA §10; R-021 unknown, R-022 emu candidate; R-026 là khoảng packet, không phải timer nội bộ; R-079 timing trình diễn không phải TTL damage |
| R-116 | `player.collision_half_px`, `player.corner_assist_px`, `player.trap_touch_distance_px` giữ BnB | 13/12/28 px | `oss` / design | R-107/R-108/BnB `server/game.js:31–32,546–567` SHA §10; chưa có KR/VN confirmed cùng đại lượng; R-028 không xác nhận khoảng cách |
| R-117 | Giữ toàn bộ 12 khóa `characters.c00_*`, `characters.c01_*` theo BomberAttr target; ngoại lệ candidate có chủ ý | C00 speed 5–7, bomb 1–10, range 1–7; C01 speed 4–8, bomb 2–7, range 1–9 | `kr_static` / candidate (nghĩa trường) | R-040/R-041 confirmed dãy số, R-042 mapping candidate; ADR 0013 chọn giữ target thay template BnB R-105. Không nâng lên confirmed khi chưa có consumer |
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
| `item.roller_speed_delta` | R-111 | item_roller_speed_delta | level | `oss` | design | BnB 5c108e8 |
| `drop.weight_bomb` | R-112 | drop_item_weight | weight | `vn_static` | confirmed | VN VBNB_116 |
| `drop.weight_fire` | R-112 | drop_item_weight | weight | `vn_static` | confirmed | VN VBNB_116 |
| `drop.weight_roller` | R-112 | drop_item_weight | weight | `vn_static` | confirmed | VN VBNB_116 |
| `drop.brick_item_chance` | R-112 | drop_brick_item_chance | permille | `oss` | design | BnB 5c108e8 |
| `sim.ticks_per_second` | R-113 | server_tick_rate | Hz | `oss` | design | BnB 5c108e8 |
| `round.warmup_ms` | R-113 | round_warmup | ms | `oss` | design | BnB 5c108e8 |
| `player.speed_px_per_s_per_level` | R-114 | speed_px_per_s_per_level | px/s | `oss` | design | BnB 5c108e8 |
| `bomb.fuse_ms` | R-115 | bomb_fuse | ms | `oss` | design | BnB 5c108e8 |
| `bomb.flame_ms` | R-115 | bomb_flame | ms | `oss` | design | BnB 5c108e8 |
| `player.trap_to_death_ms` | R-115 | trap_to_death_timer | ms | `oss` | design | BnB 5c108e8 |
| `player.collision_half_px` | R-116 | collision_box_half | px | `oss` | design | BnB 5c108e8 |
| `player.corner_assist_px` | R-116 | corner_assist | px | `oss` | design | BnB 5c108e8 |
| `player.trap_touch_distance_px` | R-116 | trap_touch_distance | px | `oss` | design | BnB 5c108e8 |
| `characters.c00_speed` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c00_speed_max` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c00_bombs` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c00_bombs_max` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c00_range` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c00_range_max` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c01_speed` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c01_speed_max` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c01_bombs` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c01_bombs_max` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c01_range` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `characters.c01_range_max` | R-117 | character_stat_level | level | `kr_static` | candidate | KR 00967 |
| `presentation.frame_duration_ms` | R-118 | presentation_frame_duration | ms | `design` | design | V1 design |

Bảng trên là nguồn chuẩn lựa chọn; validator [`tools/rules_precedence.py`](../../../tools/rules_precedence.py)
giữ bảng kỳ vọng máy-đọc-của bảng này (định danh theo **đại lượng**, đơn vị, build/context) và **đối chiếu
ngược với RULES_V1** trước khi kiểm profile [`data/rules/`](../../../data/rules/) không dùng tầng thấp
hơn tầng ghi ở đây; validator chạy trong `cargo xtask ci`, test ở `tools/test_rules_precedence.py`.
Tầng được so **theo từng khóa** từ "Bảng lựa chọn theo khóa" ở đây: bảng kỳ vọng trong tool và ô
"Nguồn / trạng thái" của bảng claim phải khớp từng khóa với bảng đó, lệch thì CI báo lỗi.
Ngoại lệ phải tường minh: 12 khóa `characters.*` (candidate theo R-117, ADR 0013) và `drop.weight_*`
(số bảng `confirmed` nhưng vai trò bảng còn `unknown` R-052). Validator **không** so giá trị số với
claim và không tự kết luận claim đúng; reviewer kiểm nguồn.

## 12. Nền và ảnh UI của client V1

Hai loại asset UI khác nhau về chỗ nằm trong atlas của bundle. **Nền màn hình** là JPEG nên ở
trạng thái `excluded`, **không** vào atlas (bộ giải mã sprite của V1 chỉ đọc blob
`CDataI16Alpha` — I16). **Ảnh UI có chữ** thì **có** trong atlas, nhưng ở trạng thái
`needs_review_text` nên bị chặn hiển thị cho tới khi có quyết định duyệt ảnh; tới lúc đó client
vẽ text từ catalog. Vì vậy V1 dựng nền bằng màu và vẽ chữ từ catalog — đây là quyết định
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
| R-092 | Ảnh UI của luồng V1 **có chữ nằm trong ảnh** (8 ảnh `needs_review_text`): nút tạo phòng `FxStage/Lobby/LB_Bt_T_Create-NM` (9 frame), dialog tạo phòng `FxStage/Lobby/Dlg_LB-NMCreate`, nút ready `FxMap/Prepare/PRE_Bt_Ready` (6 frame), nút start `FxMap/Prepare/PRE_Bt_Start` (6 frame), dialog thông tin phòng `FxMap/Prepare/Dlg_PRE-NMRoominfo`, art kết quả `FxStage/Game/Win`/`Lose`/`Draw`. Chữ đọc **theo từng asset**: 5 ảnh đầu — KR chữ Hàn ("방만들기", "준비", "시작", "방정보변경"), bản VN chữ Việt đọc được ("TẠO PHÒNG", "SẴN SÀNG", "BẮT ĐẦU", "THÔNG TIN"); art kết quả — KR chữ **Latin** "WIN!!"/"LOSE!!"/"DRAW!!", còn **bản VN chưa đọc rõ** (frame 0 chỉ đọc được "THN"/"SC"/"HO", chuỗi frame chưa ghép đủ) ⇒ phần chữ VN của art kết quả **chưa xác nhận** | — | `kr_static` + `vn_static` / confirmed (chỉ phần đã đọc trực tiếp) | Entry có thật trong pack KR 00967, đã đóng gói trong atlas bundle (`[[sprites]].source_sha256` cho cả 8; SHA-256 ở [`kr_v1_content.json`](../../evidence/inventory/kr_v1_content.json) cho 4 ảnh nút/dialog); ảnh giải mã đọc trực tiếp trong cache review ảnh của `boom-re-l10n` — [UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1); `crates/boom-assets/src/closure.rs` (`UI_ASSETS`) |
| R-093 | Quyết định V1 về ảnh có chữ: bảng UI của bundle có **ba** trạng thái (`selected`/`needs_review_text`/`excluded`) và **không** có trạng thái "đã duyệt" ⇒ ảnh `needs_review_text` chỉ hiển thị khi có quyết định duyệt ảnh ([IMAGE_REVIEW](../localization/IMAGE_REVIEW.md)); tới lúc đó client **vẽ text từ catalog**, còn phần tử `excluded` thay bằng text/màu theo lý do ghi trong bảng | 8 ảnh `needs_review_text` | `design` / design | [BUNDLES §3.3](../assets/BUNDLES.md#33-phần-tử-ui); `crates/boom-assets/src/closure.rs` (`UiState`, `UI_ASSETS` — 8 phần tử `needs_review_text` trong 23 phần tử); [UI_STRINGS_V1 §3](../localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1) |

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

## 16. RE KR 00967 — nổ lan, vật cản và dây chuyền (R-138…R-140)

Đọc tĩnh client KR `00967` cho [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148) / Q-019
(mục 5 hàng đợi RE). Định danh build (SHA-256 `cf72da11…e63b`, imagebase `0x400000`,
`RVA = VA − 0x400000`), bảng VA/RVA, `evidence_id` và giới hạn công cụ ở
[evidence KR static flame-chain](../../evidence/measurements/kr-static-flame-chain-2026-10-22/README.md);
suy luận, khác biệt profile và bước tiếp ở
[KR_RE_FLAME_CHAIN](../../research/findings/KR_RE_FLAME_CHAIN.md). Nguồn `kr_static`; `confirmed` chỉ
cho phần đã thấy consumer. Dãy bắt đầu ở R-138 vì R-134…R-137 đã có trên `main` ở §15.

**Ranh giới client/server (đọc kèm mọi claim dưới đây).** Gói nổ của bản VN (`S→C 151`) chỉ mang
id/kind/arg của bom, **không** mang danh sách ô (`vn_runtime`; `oc_e-game` §5a-5b, F5 ở
[VN_FEATURES_REFERENCE](../../research/findings/VN_FEATURES_REFERENCE.md)). Tầm nổ khởi đầu 1 và +1 mỗi
ItemFire là R-025 (`vn_runtime`/confirmed), khớp nghĩa "tầm = số ô" của slot `+20`. KR có code
tính vùng nổ ở client và gọi từ lớp bom (dưới đây), nhưng **handler gói nổ S→C của KR chưa định vị**
trong lượt này — không suy opcode từ VN. Vì vậy: server quyết **bom nào nổ và lúc nào** (chưa chứng
minh cho KR), client quyết **hình học ô cháy** (đã thấy consumer).

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-138 | **Vùng nổ do client tính — `CPlayMap::CalcBombDirectionExplosion`**: lấy từ chính quả bom (slot `+12` = **hàng**, `+16` = **cột**, `+20` = tầm nổ, `+24` = kiểu). Kiểu `1`: 4 hướng × `j = 1..tầm` theo bảng `0x017929B0` = `[-1,0, 0,-1, 1,0, 0,1]` (`[2i]` = delta **cột**, `[2i+1]` = delta **hàng**; ô truy vấn theo thứ tự `(hàng, cột)`) ⇒ `i=0` **trái**, `1` **lên**, `2` **phải**, `3` **xuống**, khớp R-120. Ô ra ngoài map thì dừng. **Mỗi ô trong map được đăng ký vào bom TRƯỚC khi kiểm chặn**, rồi vòng lặp dừng sau ô chặn đầu tiên ⇒ ô chặn (tường/gạch) **thuộc** vùng nổ và tia **không xuyên**. Kiểm chặn theo thứ tự: `sub_EA1230` (object ô thuộc set lớp `map+7460`) → `sub_EA0B10` (set `map+7508`) → object tại ô có `sub_E90A80 != 0` và cờ theo hướng `sub_BA0F80(dir) == 0` (bảng 4 byte `*(context[43])+32..35`; nghĩa "cho xuyên" là **giả thuyết**, chưa xác định); kiểm thêm ô kế với cờ đối diện `sub_BA0EC0(dir) == 0`. Kiểu `2`: **vuông đầy** `cột±tầm × hàng±tầm`, không kiểm chặn. Đơn vị: **ô** (1 ô = 40 px). Consumer: gọi trực tiếp từ lớp bom `0x00B384E0`, `0x00B38A80`, `0x00B393D0`, `0x00B39AF0`, `0x00B3A3B0`, `0x00B3A790`, `0x00B76CD0`, `0x00B994C0`, `0x00B9AFA0`, `0x00E14360`, `0x00F0FD20` | ô, tầm = số ô; ô chặn vẫn thuộc vùng | `kr_static` / confirmed (hình học ô + thứ tự chặn + nhãn hướng khớp R-120); nghĩa nhánh phụ, nội dung 2 set lớp và nghĩa cờ hướng: **unknown** | Hàm `0x00E9C520` (RVA `0x00A9C520`, thân `0x89C`); bảng hướng `0x017929B0`; đăng ký ô `0x00EBD3F0` (message id 27); gạch `0x00E9FCF0` (`CGOTileFixed`/`CGOTileFixedBox`), tường `0x00E9FA90` (`CGOTileFixed`/`CGOTileStatic`); cờ xuyên `sub_BA0E30` đọc `*(context[43]) + 32..35`; evidence `ev_4fd7d146…` |
| R-139 | **Dây chuyền do client tính — `CPlayMap::CalcChainExplosion_Direction`**: tại ô `(hàng,cột)` lấy danh sách object; với từng object có tầm `vf+20` và kiểu `vf+24 == 1`, lấy hướng `sub_149B5E0(&điểm)` và chỉ số bắt đầu `sub_14708A0()+1`, rồi `j = bắt đầu..tầm` tính ô theo **cùng** bảng `0x017929B0` từ điểm của object và **đăng ký ô vào chính object đó** (`sub_EBD3F0`), dừng theo cùng bộ kiểm chặn như R-138. Nghĩa: khi lửa chạm ô có bom khác, client tự tính phần vùng nổ của bom đó và đăng ký — không cần server gửi danh sách ô. Gọi từ `0x00E84650` (2 site), `0x00EA5280`, `0x00EA56C0`, `0x00B92E30`, `0x00B9CEC0`, `0x00B9FDD0`, `0x006D60A0` | ô (cùng đơn vị R-138) | `kr_static` / confirmed (client tính + đăng ký ô dây chuyền); công thức hướng/điểm bắt đầu (`sub_149B5E0`/`sub_14708A0`): **candidate** | Hàm `0x00E9CDC0` (RVA `0x00A9CDC0`, thân `0x33D`); iterator object tại ô `sub_ED8DE0`/`sub_EDA940`; evidence `ev_d667d89c…` |
| R-140 | **Ô nổ là sự kiện, không phải timer damage; thời gian lửa = số frame animation**: mỗi ô được đăng ký thành message **id 27** gửi cho quả bom (`sub_EBD3F0` → `sub_EDB7E0` → `sub_EC2420(object,msg)` push vào list message của object), nên damage áp theo **sự kiện ô**, không theo biến đếm ms. Thời gian sống **hiển thị** của `CGOFlame` (vtable `0x01745EF0`, ctor `0x00B70A80`, object 584 B) là **số frame**: mỗi segment có bộ đếm `flame[i+51]` tăng 1 mỗi update (`0x00B711C0`), xoá khi `>= sub_B333D0(anim)` = `CGameObjectTemplate::GetAnimCount(objNo)` (`0x00BA3FF0`). **Chưa** thấy hằng ms cho vùng damage; **không** thấy fuse trong client. `2500` chỉ xuất hiện ở `sub_E15810`/`sub_E15840` (ngưỡng so `sub_E131A0(stage)`) và nhánh chống hack `sub_E78350`, **không** phải fuse | — (không mang ms); animation = số frame/segment (quy đổi 30 ms/frame theo R-013 **chỉ là quy đổi**, chưa đo nhịp update KR) | `kr_static` / confirmed (message-per-cell; lifetime = frame count); `unknown` (damage window; fuse origin; nghĩa `desc+168`) | `sub_EBD3F0` `0x00EBD3F0`; `sub_B711C0` `0x00B711C0`; `sub_B333D0` `0x00B333D0` → `0x00BA3FF0`; `CGOBomb::Init` `0x00B384E0` đọc `desc+168/172` (u64) → bomb `[+98]/[+99]`, `desc+96` → slot `+104` (nghĩa chưa xác định, **không** phải ô); `CGOBomb` update `0x00B391E0` không có fuse timer; evidence `ev_a86e37b2…`, `ev_bd381d4f…` |

Hệ quả:

- **Không đổi `data/rules` trong PR này.** `bomb.flame_ms = 500` (R-115) là `oss`/design; KR cho thời
  gian sống theo **frame** của template, không phải hằng 500 ms, nên chỉ map sang ms khi có nhịp
  update KR. `bomb.fuse_ms = 2500` và `player.trap_to_death_ms = 4000` giữ design; **không** lấy
  2,5 s của emulator làm fuse gốc (R-021/R-022 vẫn unknown/candidate).
- R-138/R-139 **không** phủ định claim movement (§13): nhãn hướng đọc ở hàm nổ **khớp** R-120
  (`0` trái, `1` lên, `2` phải, `3` xuống) và khớp thứ tự `(hàng, cột)` của `0x00E9F9B0`/`0x00EB7190`.
- Phần còn thiếu có phạm vi: handler gói nổ KR, consumer message id 27, chỗ kiểm bomber trong ô lửa
  (damage window), nội dung 2 set lớp chặn và thân `sub_149B5E0`/`sub_14708A0` — ghi ở
  [KR_RE_FLAME_CHAIN §7](../../research/findings/KR_RE_FLAME_CHAIN.md#7-giới-hạn-và-bước-tiếp).
