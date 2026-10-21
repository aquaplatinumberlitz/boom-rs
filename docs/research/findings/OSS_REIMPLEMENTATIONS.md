# Bản mô phỏng Crazy Arcade / 泡泡堂 mã nguồn mở — thông số tham khảo

**Ngày truy cập**: 2026-10-19. **Target**: KR build 00967 (các repo dưới đây **không phải** build đó).

Mục đích: tìm implementation độc lập để **đối chiếu hằng số gameplay** khi không có tài liệu chính thức.
Đây là nguồn **third-party**, có thể đã cân bằng lại; bằng chứng OSS vẫn là `candidate`; lựa chọn profile V1 theo ADR 0012 là `design`, không sửa pack.

## 1. Danh sách repo

| Repo | Ngôn ngữ | Quy mô | Có gì dùng được |
|---|---|---|---|
| [tpoisonooo/BnB](https://github.com/tpoisonooo/BnB) | JavaScript (Node + WebSocket) | 194 file, ~37 MB, cập nhật 2026-10-16 | **Server authoritative 30 Hz có đủ hằng số gameplay** (fuse, trap, tick, speed px/s, drop weight, tile 40, collision box). Có test (`test/game.test.js`). |
| [gunheehan/CrazyBoom](https://github.com/gunheehan/CrazyBoom) | C# (Unity) | ~85 MB | Danh sách tính năng (player property, item, water balloon network), **không có số** |
| [ljcduo/CrazyArcade](https://github.com/ljcduo/CrazyArcade) | C++ / DirectX11 | ~20 MB | Demo render + input, **không có số** |
| [developeSHG/GAME-CrazyAcade](https://github.com/developeSHG/GAME-CrazyAcade) | C++ WinAPI | ~128 MB | Map tile tool + collision, **không có số** |
| [weishi-lab/SCNU-426](https://github.com/weishi-lab/SCNU-426) | Java | ~24 MB | Bản một phần (2020), ít dùng |

⇒ Chỉ **BnB** có giá trị định lượng. Các repo còn lại dùng tham khảo kiến trúc.

## 2. Hằng số từ `tpoisonooo/BnB` (`server/game.js`, `server/map.js`, `server/characters.js`)

File: `server/game.js` (26 KB). Nguyên văn comment tiếng Trung, trích số:

| Hằng số | Giá trị | Ghi chú của tác giả |
|---|---|---|
| `TICK_RATE` | **30 Hz** | `DT = 1/30` |
| `BOMB_FUSE_TICKS` | `round(2.5*30)` = **2.5 s** | "水泡引信 2.5s" |
| `EXPLOSION_TICKS` | `round(0.5*30)` = **0.5 s** | "火焰持续 0.5s" |
| `TRAP_TICKS` | `round(4*30)` = **4 s** | "被困 4s 后死亡" ← **lấp gap thời gian bị nhốt** |
| `ROUND_TICKS` | `180*30` = **180 s** | hết giờ → hoà |
| `WARMUP_TICKS` | `3*30` = **3 s** | đếm 3-2-1, đóng băng toàn bộ |
| `ITEM_DROP_RATE` | **0.35** | xác suất block mềm rơi item |
| `SHOE_STEP` | **24 px/s** | mỗi giày |
| `KICK_SPEED` | **360 px/s** | bóng bị đá trượt |
| `PUSH_TICKS` | **10 tick (~0.33 s)** | giữ hướng để đẩy thùng |
| `SLIDE_TICKS` | **6 tick (~0.2 s)** | thời gian trượt của thùng |
| `DEATH_DROP_RATE` | **0.3** | chết → mỗi item đã ăn có 30% rơi lại |
| `MAX_NEEDLE` / `MAX_REMOTE` | **2 / 2** | |
| `HALF` (collision) | **13 px** | hộp va chạm 26×26 |
| `ASSIST_RANGE` | **12 px** | dung sai bám góc khi rẽ |
| `TILE` | **40 px** | khớp `docs/map.md` của dự án |
| `COLS × ROWS` | **15 × 13** | khớp map Baseball của KR |

Bảng rơi item (`rollItem()`, cộng dồn):

| Khoảng random | Item | Tỉ lệ |
|---|---|---|
| 0.000–0.280 | potion (威力 +1) | **28%** |
| 0.280–0.480 | bubble (bóng +1) | **20%** |
| 0.480–0.640 | shoe (speed +24) | **16%** |
| 0.640–0.720 | needle (kim) | 8% |
| 0.720–0.800 | remote (điều khiển) | 8% |
| 0.800–0.880 | kick (giày đá) | 8% |
| 0.880–0.945 | gremlin (tím, power max) | 6.5% |
| 0.945–0.965 | devil (đỏ, speed max) | 2% |
| 0.965–0.9775 | pirateTurtle | 1.25% |
| 0.9775–0.990 | owl | 1.25% |
| 0.990–1.000 | ufo | 1% |

## 3. Quy đổi tốc độ — **candidate quan trọng nhất** (lấp gap Q-005)

`server/characters.js` ghi rõ:

> "游戏内换算：**速度 1 级 = 24 px/s**（与鞋子 +24 一致）；威力=格数、水泡=个数"

⇒ Với `TILE = 40 px` (khớp dự án): **1 cấp tốc độ = 24/40 = 0.6 ô/giây**.

| Cấp tốc độ | px/s | ô/giây |
|---|---|---|
| 1 | 24 | 0.6 |
| 4 | 96 | 2.4 |
| 5 | 120 | 3.0 |
| 6 | 144 | 3.6 |
| 7 | 168 | 4.2 |
| 8 | 192 | 4.8 |
| 9 | 216 | 5.4 |
| 10 | 240 | 6.0 |
| 11.5 (kart nitro) | 276 | 6.9 |

Đối chiếu với thang phương tiện trong register WEB-KR-002 (rùa 1, cú 5, rùa cướp biển 9, UFO 10):
repo này dùng số khác cho phương tiện (rùa 84 px/s = 3.5 cấp; cú 156 = 6.5; rùa cướp biển 204 = 8.5;
UFO 180 = 7.5) ⇒ **repo đã cân bằng lại**, không dùng làm chuẩn phương tiện.

## 4. Đối chiếu chỉ số nhân vật: repo ↔ BomberAttr (pack KR)

Repo dẫn "官方存档值 (百度百科)" — không phải tài liệu Nexon:

| Nhân vật | Repo (official-archive) | `data/kr/bomber_attr.json` | Kết luận |
|---|---|---|---|
| 宝宝 / Bazzi | 泡 1→6, 威 1→7, 速 5→10 | `C06`: bomb 1–6, len 1–7, **speed 5–9** | bomb ✅, len ✅, **speed max lệch 10 vs 9** |
| 小海盗 / Lodumani | 泡 3→9, 威 3→7, 速 6→9 | `C14`, `N05…N25`: bomb 3–9, len 3–7, **speed 6–9** | ✅ **khớp tuyệt đối** |

⇒ Hai nguồn độc lập (pack KR của mình và bản remake dựa trên 百度百科) **trùng nhau** ở 5/6 giá trị,
củng cố mapping field 2-7 trong `docs/kr-data.md` §23. Chỗ lệch duy nhất (Bazzi speed max) cần
đối chiếu thêm — có thể do archive sai hoặc build khác.

## 5. Cơ chế khác lấy được từ code (dùng để thiết kế contract)

- **Server authoritative**: client gửi input, server mô phỏng 30 Hz, gửi snapshot + `gridVer` khi map đổi.
- **Explosion**: `computeFlameCells(grid, tx, ty, range)` lan 4 hướng, **dừng ở block cứng**, phá block mềm.
- **Chain**: bóng nằm trong vùng lửa → kích nổ theo (dự án đã ghi trong `docs/game-spec-web.md` §2.2).
- **Corner assist**: `ASSIST_RANGE = 12 px` — cho phép bám góc khi rẽ (giải thích "걸치기" trong WEB-KR-005).
- **Push**: giữ hướng `PUSH_TICKS` → thùng trượt `SLIDE_TICKS` (không tức thời).
- **Kick**: bóng trượt `KICK_SPEED` tới khi chạm vật cản.
- **Mount**: hấp thụ 1 lần chết (mất mount, người bất tử ngắn).
- **Death drop**: item đã ăn rơi lại 30%, bay theo quỹ đạo `LOOT_FLY_*`.

## 6. Giới hạn (phải nêu khi trích dẫn)

- Là **remake Trung Quốc (盛大/泡泡堂)**, không phải KR 00967; tác giả **tự cân bằng lại** chỉ số.
- Số "official archive" dẫn từ **百度百科**, không phải tài liệu Nexon.
- Không có Magic Market, không có mode KR mới (Alchemist/Baseball).
- Không chứng minh wire protocol của client gốc; chỉ dùng làm **tham chiếu số học**.

## 7. Truy tìm repo có **chỉ số chuẩn từ game gốc** (khảo sát 2026-10-19)

Câu hỏi: có repo nào lấy chỉ số trực tiếp từ Crazy Arcade / Boom / BnB (không tự cân bằng) không?

Đã tìm: GitHub repo search (`crazy arcade`, `泡泡堂`, `크레이지 아케이드`, `물풍선`, `bnbidx`, `crazy arcade assets`,
`popstage`), web search (KR/CN), và đọc README từng ứng viên.

| Repo | Có chỉ số gốc? | Bằng chứng |
|---|---|---|
| `tpoisonooo/BnB` | **Gần nhất** | Dẫn "官方存档值（百度百科角色词条）" cho 宝宝 & 小海盗; **nhưng tác giả nói rõ đã cân bằng lại** cho 2 người đấu (xem §4) |
| `Ark-Point/crazy-arcade` | Không | JS web game + AI agent; item chỉ ghi "+1", không bảng chỉ số |
| `Lor3walker/popo-arcade` | Không | "from-scratch remake", chỉ số tự thiết kế (power-up tiers riêng) |
| `gunheehan/CrazyBoom`, `ljcduo/CrazyArcade`, `developeSHG/GAME-CrazyAcade`, `weishi-lab/SCNU-426`, `SCNU-A225/CrazyArcade`, `dengzx7/BNB_Crazy_Arcade`, `h3llowoori/CrazyArcade` | Không | đều là đồ án/demo; README không nêu nguồn số |

Không tìm thấy repo nào chứa **dữ liệu client gốc** (`.idx/.idd`, `Attr/BomberAttr`, `ItemAttr`) hoặc
server emulator có bảng chỉ số gốc. Các bài "service端 emulator / 抓包" trên CSDN đều bị chặn đọc (2 KB challenge).

### ⇒ Kết luận về nguồn chỉ số

1. **Nguồn chuẩn nhất hiện có của dự án**: pack KR — `data/kr/bomber_attr.json` (67 nhân vật, 3 chỉ số min–max),
   `ItemAttr` (trọng số item), `MonInfo` (drop + physics). Đây là dữ liệu client thật, không phải wiki.
2. **Đối chiếu độc lập**: ja.wikipedia bảng nhân vật (khớp 8/8) và BnB dẫn 百度百科 (khớp 5/6, lệch Bazzi speed max).
3. **Còn thiếu để "chuẩn 100%"**: trang nhân vật chính thức Nexon và 百度百科 gốc — tool bị chặn DNS,
   **cần mở tay**:
   - https://ca.nexon.com/Info/Character/1
   - https://baike.baidu.com/item/泡泡堂 (mục 角色)
4. Không nên dùng repo remake làm oracle cho **giá trị tuyệt đối**; chỉ dùng cho **cơ chế/hằng số thời gian**
   (fuse 2.5 s, trap 4 s, tick 30 Hz, tile 40, hộp va chạm 26 px) — những thứ này độc lập với cân bằng.

### Repo bổ sung đã kiểm (không dùng được)

| Repo | Ngôn ngữ | Lý do loại |
|---|---|---|
| [Ark-Point/crazy-arcade](https://github.com/Ark-Point/crazy-arcade) | JS | web game + AI agent, không có bảng chỉ số |
| [Lor3walker/popo-arcade](https://github.com/Lor3walker/popo-arcade) | JS | remake tự thiết kế, không dẫn nguồn gốc |
| [SCNU-A225/CrazyArcade](https://github.com/SCNU-A225/CrazyArcade) | Java | đồ án, config tự đặt |
| [dengzx7/BNB_Crazy_Arcade](https://github.com/dengzx7/BNB_Crazy_Arcade) | Python | đồ án pygame, không nêu nguồn số |
| [h3llowoori/CrazyArcade](https://github.com/h3llowoori/CrazyArcade) | C++ | mô phỏng 2 tuần, không có bảng số |

## 8. Đối chiếu lại và lựa chọn V1 — 2026-10-21

Đã đọc lại clone chỉ đọc `/home/admin/boom-team/tmp/bnb`, `git rev-parse HEAD` =
`5c108e8ad3ea07684d4666f60e6eec6c2f3e569e`.
Nguồn ghim: [BnB tại SHA này](https://github.com/tpoisonooo/BnB/tree/5c108e8ad3ea07684d4666f60e6eec6c2f3e569e).
Bản khảo sát 2026-10-19 không có SHA, không thể khẳng định repo không đổi giữa hai ngày.

- `server/game.js:12–32`: tick 30 Hz, fuse 75, flame 15, trap 120, round 5400,
  warmup 90 tick; drop 0,35; shoe +24 px/s; half=13, assist=12 px. Khớp số đã trích §2.
- `server/map.js:7–9`: 40 px, 15×13. Layout là 海盗14, **không phải Village00**;
  không nhập layout/spawn/mount vào V1.
- `server/characters.js:17–27`: **giá trị chạy**, khác comment archive §4:
  baobao speed 144→240, range 1→7, bombs 1→6;
  haidao speed 120→216, range 2→7, bombs 1→8. 24 px/s/level ở dòng 15.
- `server/game.js:401–451`: chain queue FIFO, hoãn xóa gạch tới hết queue,
  lửa phá item cũ; roll khi phá gạch, không pre-roll đầu trận; lửa mới giảm TTL cùng tick.
- `server/game.js:433–446`: potion/bubble/shoe = 28/20/16% trên **toàn** bảng.
  V1 chỉ giữ ba loại, trọng số Bomb:Fire:Roller 20:28:16 (31,25/43,75/25% có điều kiện).
- `server/game.js:540–595,616–646`: trap đứng yên, timer giảm cả tick vừa nhốt;
  timeout trước touch strict <28 px mỗi trục; cap vẫn nhặt; warmup đóng băng;
  checkOver trước timeout. Không suy trapped slow movement từ nguồn web.

Chủ dự án 2026-10-21 chọn các giá trị này thành **`oss` / `design`** cho V1, kể cả khi
khác KR/VN confirmed. [ADR 0012](../../decisions/0012-v1-rules-from-bnb.md) là quyết định
hiện hành, thay khuyến nghị không dùng giá trị tuyệt đối ở §7 cho profile V1;
không biến thông số này thành oracle client gốc. Claim R-100…R-109 và task #130–#133
phân biệt thông số đã nạp với cơ chế chưa triển khai. Presentation clip KR và policy vận hành
không có tương đương trực tiếp BnB nên giữ quyết định V1 hiện có.
