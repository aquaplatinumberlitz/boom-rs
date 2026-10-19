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

## 2. Trận và thời gian

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-010 | Giới hạn thời gian mặc định của client: 120 s, đếm từ lúc bắt đầu lượt | 120 000 ms | `vn_static` + `vn_runtime` / confirmed | oc_e "Hết giờ" (`ctx+490518`) |
| R-011 | Map có cờ thì byte thời gian từ server ghi đè R-010 | — | `vn_static` / confirmed | oc_e "Hết giờ" (`sub_71A9EC`) |
| R-012 | Header map có cặp `60000, 22000` (ms?) ở 574 file VN và ở Village00 KR | — | `vn_static` + `kr_static` / unknown | MAPS §1.1; nghĩa chưa rõ |
| R-013 | Tần số tick mô phỏng gốc | — | unknown | Q-004 |
| R-014 | Tick server remake: 30 Hz | 30 Hz | `oss` / candidate | OSS §2 |

## 3. Bom, lửa, nhốt, chết

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-020 | Thời điểm nổ do **server** quyết (S→C 151); client không tự nổ | — | `vn_runtime` / confirmed | oc_e §5a (bom) |
| R-021 | Fuse bom thường gốc | — | unknown | Server gốc thất lạc |
| R-022 | Server giả lập dùng fuse 2,5 s | 2 500 ms | `emu_choice` / candidate | `BOMB_FUSE` trong `boom-re/server` |
| R-023 | Remake dùng fuse 2,5 s, lửa 0,5 s | 2 500 / 500 ms | `oss` / candidate | OSS §2 |
| R-024 | Bom bị ném/đá nổ sau 3 s | 3 000 ms | `web` / candidate | `boomkr-re/docs/game-spec-web.md` §2 |
| R-025 | Tầm nổ khởi đầu 1; mỗi ItemFire +1 tầm; mỗi ItemBomb +1 số bom đặt cùng lúc | — | `vn_runtime` / confirmed | oc_e §5a-5d (hiệu ứng bị động) |
| R-026 | Bị nước trúng → bị nhốt; tự chết sau khoảng 6,3 s nếu không được cứu | ≈6 300 ms ±300 | `vn_runtime` / confirmed | oc_e §5a-5c (120/11 → 120/13) |
| R-027 | Remake: nhốt 4 s rồi chết | 4 000 ms | `oss` / candidate | OSS §2; mâu thuẫn R-026, R-026 thắng |
| R-028 | Địch chạm người bị nhốt → người bị nhốt chết | — | `vn_runtime` / confirmed | VN-B01 |
| R-029 | Bị nhốt: không đặt bom, đi rất chậm | — | `web` / candidate | NAMU §2.1 |
| R-030 | Lửa dừng ở khối cứng, phá khối mềm; bom trong lửa kích nổ dây chuyền | — | `oss` + `web` / candidate | OSS §5; client VN tự vẽ vụ nổ (oc_e) |
| R-031 | Vỡ gạch: client legacy tự tính và báo C→S 135 | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-032 | Sức nổ được chụp lúc đặt bom (client gửi `power` trong 111); ID bom do server cấp (152) | — | `vn_static` + `vn_runtime` / confirmed | [Legacy server](../../research/findings/VN_LEGACY_SERVER_REFERENCE.md) §1 |
| R-033 | Lửa phủ ô đặt bom + tối đa `range` ô mỗi hướng; khối cứng/mép lưới chặn, khối mềm đầu tiên có lửa rồi dừng. Bom đến hạn hoặc bị lửa chạm nổ cùng tick, xử lý theo `id` tăng dần, mỗi bom một lần; nhiều lửa cùng ô giữ `until` lớn nhất, hòa chọn `source` id nhỏ | — | `design` (từ R-030) | Task #5, `systems/explosion.rs`; R-030 |
| R-034 | V1: "chạm" = hai người chơi cùng một ô lưới tại bước life; chỉ người `Alive` làm vỡ bong bóng, hai người `Trapped` chạm nhau không đổi trạng thái | — | `design` / design | SCOPE_V1 §4; issue #8 (cùng ô); `systems/life.rs` |
| R-035 | V1: trong một tick, người `Alive` đứng trên lửa bị nhốt trước; chỉ người đã `Trapped` từ đầu bước life mới xét chết; nếu vừa hết `trap_ticks` vừa bị chạm thì `player_died.by` là người chạm; nhiều người `Alive` cùng ô thì chọn id nhỏ nhất | — | `design` / design | SCOPE_V1 §4; ARCHITECTURE §5.2; issue #8 |

Hai claim `design` R-034/R-035 chỉ định nghĩa cách engine V1 hiện thực hóa "chạm" và thứ tự trong
tick; chưa có bằng chứng client KR/VN cho trường hợp biên. Khi có trace, thay bằng claim `confirmed`
hoặc mở câu hỏi ở [QUESTIONS](../../research/QUESTIONS.md).

## 4. Nhân vật và tốc độ

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-040 | BomberAttr KR C00: f2–f7 = `5,7,1,10,1,7` | — | `kr_static` / confirmed | `boomkr-re/data/kr/bomber_attr.json` |
| R-041 | BomberAttr KR C01: f2–f7 = `4,8,2,7,1,9` | — | `kr_static` / confirmed | Như trên |
| R-042 | f2–f3 = speed min/max, f4–f5 = số bom min/max, f6–f7 = tầm min/max (cấp) | — | `kr_static` + `web` / candidate | `boomkr-re/docs/kr-data.md` §23; OSS §4 khớp 5/6 giá trị |
| R-043 | Quy đổi 1 cấp tốc độ → px/s | — | unknown | Q-005 |
| R-044 | Remake: 1 cấp tốc độ = 24 px/s | 24 px/s | `oss` / candidate | OSS §3 |
| R-045 | Roller: +1 cấp tốc độ | +1 cấp | `web` / candidate | NAMU §1 |
| R-046 | Bám góc khi rẽ trong 12 px | 12 px | `oss` / candidate | OSS §5 |
| R-047 | Di chuyển không có gói C→S; client tự mô phỏng | — | `vn_static` / confirmed | oc_e §5a |

## 5. Item rơi và nhặt

| ID | Claim | Giá trị | Nguồn / trạng thái | Locator |
|---|---|---|---|---|
| R-050 | Item giấu trong gạch được server gửi **trước trận** (553), không quyết lúc vỡ | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-051 | ItemAttr trọng số: Bomb 3650, Fire 2800, Roller 2000; client nạp vào bảng ngẫu nhiên 0x8000 ô | — | `vn_static` / confirmed | items (loader `sub_89F3DE`); thay cách đọc "giá" ở MAPS §2 |
| R-052 | Ai dùng bảng R-051 để rơi item (client hay server) | — | unknown | Q-007 |
| R-053 | Xác suất một gạch có item | — | unknown | Q-007; OSS dùng 0,35 (`oss` / candidate) |
| R-054 | Nhặt: client xin (117), server xác nhận (158), item mất sau xác nhận | — | `vn_runtime` / confirmed | oc_e §5a-5d |
| R-055 | Bảng rơi gốc của Village00 chỉ có 6 loại Lucci | — | `kr_static` / confirmed | `kr_v1_content.json`; V1 thay bằng Bomb/Fire/Roller (DESIGN, SCOPE_V1 §2) |

## 6. Giá trị được chọn cho profile dev

Profile [`v1-dev`](../../../data/rules/v1-dev.toml) không dùng được cho phát hành
(`release = false`). Các giá trị `design` hiện tại và claim dẫn tới:

| Rule | Chọn | Vì sao |
|---|---|---|
| Tick | 60 Hz (`design`) | ADR 0004; đổi khi Q-004 đo được |
| Thời gian lượt | 120 s (`confirmed`, R-010) | Client VN |
| Fuse | 2,5 s (`design`, R-021…R-024) | Hai candidate độc lập trùng nhau; gốc unknown |
| Lửa tồn tại | 0,5 s (`design`, R-023) | Chỉ có OSS |
| Nhốt → chết | 6,3 s (`confirmed` ± 0,3 s, R-026) | Đo trên client VN |
| Tốc độ | 24 px/s mỗi cấp (`design`, R-044) | Chỉ có OSS; Q-005 phải hiệu chuẩn |
| Trọng số rơi | 3650 : 2800 : 2000 (`confirmed`, R-051) | Dữ liệu client; vai trò bảng còn unknown |
| Xác suất gạch có item | 35% (`design`, R-053) | Chỉ có OSS |
| Căn trục khi rẽ | Căn về tâm hàng/cột, tối đa bằng tốc độ mỗi tick (`design`) | Thay thế tạm cho R-046 tới khi Q-005 xong |

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
