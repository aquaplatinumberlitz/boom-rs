# Bundle nội dung (atlas ảnh + metadata ô + clip animation + bảng UI)

Định dạng **output cục bộ** của `boom-assets bundle`: gom ảnh đã giải mã từ pack KR
thành trang atlas PNG, kèm metadata ánh xạ ô của map → sprite/frame, kích thước, canvas
và pivot, **và clip animation** của V1 (dãi frame, canvas, pivot, thời lượng), **và bảng
UI** của luồng V1 (màn hình, vai trò, trạng thái dùng, ảnh). Client đọc bundle bằng kiểu ở
`crates/boom-content/src/bundle.rs` nên **không cần bộ giải mã pack** (ARCHITECTURE §7).

Bundle **không commit** vào repo (dữ liệu game có bản quyền); đây là artifact dựng từ
pack, giống `manifest` closure của task #34.

Nhãn nguồn/trạng thái theo [AGENTS §4](../../../AGENTS.md#4-bằng-chứng). Định dạng bundle
là quyết định **design** của dự án (ARCHITECTURE §8 "Định dạng bundle content"), không
phải định dạng của client gốc; phần dữ liệu bên trong thì giữ nguồn gốc của pack KR.

**Schema hiện hành: 3** (có `[[clips]]` và `[[ui]]`). Schema 2 (chỉ atlas + ô + clip) và
schema 1 (chỉ atlas + ô) **bị reader từ chối** — xem §5.

## 1. Dựng bundle

```sh
# thư mục pack lấy từ --pack-dir, hoặc biến môi trường BOOM_KR_PACK_DIR
cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/bundle-village00
```

Kết quả in ra số sprite/ảnh/trang atlas/ô/unknown và `content_hash`.

Nguồn của bundle là **manifest closure** (`crates/boom-assets/src/closure.rs`, task #34),
**bộ giải mã sprite** `CDataI16Alpha` v2 (`crates/boom-assets/src/sprite.rs`, task #33),
**bảng clip animation** (`crates/boom-assets/src/animation.rs`, task #52, RULES_V1
R-077…R-079) và **bảng asset UI** của luồng V1 (`crates/boom-assets/src/closure.rs`,
task #84, V1-032):

- Entry nào cần có, vai trò, trạng thái bằng chứng, SHA-256 payload → từ closure.
- Ảnh RGBA + frame 48 byte (12 × `i32`) → từ bộ giải mã sprite; nghĩa field frame theo
  [MAPS §3](MAPS.md).
- Clip nào (state, hướng, lớp) → từ bảng clip animation; ảnh của clip là ảnh của
  sprite tương ứng nên **không** có bản pixel thứ hai trong atlas.
- Phần tử UI nào của màn hình nào dùng được, phần tử nào có chữ chờ duyệt, phần tử nào
  bị loại → từ bảng UI; ảnh của phần tử cũng nằm trong atlas (một bản pixel cho mỗi
  entry, không nhân bản).

Bảng UI **tách khỏi manifest closure** vì hai thứ khác nhau: manifest là closure *nội
dung trận* (ô map, sprite, nhân vật, item), mỗi dòng là một entry pack kèm SHA-256; bảng
UI là closure *luồng hiển thị*, mỗi dòng là một **phần tử** có màn hình, vai trò, trạng
thái duyệt ảnh và lý do thay thế. `[[ui]]` trỏ tới `[[sprites]]` nên ảnh của phần tử
không bị nhân bản trong atlas.

## 2. Cấu trúc thư mục bundle

```text
<bundle>/
  bundle.toml            # metadata (schema 3)
  atlas/page-000.png     # các trang atlas
  atlas/page-001.png     # (chỉ khi cần hơn một trang)
```

## 3. Metadata `bundle.toml` (schema 3)

| Khóa | Kiểu | Ý nghĩa |
|---|---|---|
| `schema` | số | Luôn là `3` |
| `build` | chuỗi | Build của pack nguồn (`kr`) |
| `map` | chuỗi | Tên map (`Village00`) |
| `tileset` | chuỗi | Tile set của map; rỗng nếu map dùng nhiều tile set |
| `width`, `height` | số | Kích thước lưới ô (số cột, số hàng) |
| `spawns` | mảng cặp số | Spawn theo thứ tự file `.map` nguồn |
| `content_hash` | chuỗi | Định danh nội dung, xem §4 |
| `[[atlas]]` | bảng | Một trang atlas: `file`, `width`, `height`, `sha256` (SHA-256 của **file** PNG; nằm trong `content_hash`) |
| `[[sprites]]` | bảng | Một entry ảnh, xem §3.1 |
| `[[assets]]` | bảng | Entry của closure **không phải ảnh** (file map, BGM, bảng chỉ số): `entry`, `role`, `status`, `source_sha256`, `bytes`, `reason` |
| `[[cells]]` | bảng | Ô của map: `x`, `y`, `tile` (ID nền), `object` (ID vật cản, vắng nếu ô trống) |
| `[[unknown]]` | bảng | Phần chưa xác định: `kind`, `id`, `reason`, `candidates` (tuỳ chọn) |
| `[[clips]]` | bảng | Clip animation của V1, xem §3.2 |
| `[[ui]]` | bảng | Phần tử UI của luồng V1, xem §3.3 |

### 3.1 Sprite

```toml
[[sprites]]
id = "FxTile/Tile00/Grass00"     # định danh trong bundle = đường dẫn entry pack
entry = "FxTile/Tile00/Grass00"   # entry pack nguồn
role = "tile"                     # tile/object/theme/preview/character_walk/…
status = "confirmed"              # nhãn bằng chứng (AGENTS §4)
source_sha256 = "e49648e3…"       # SHA-256 của blob sprite gốc trong pack
map_id = 1942                     # ID tile/obj trong file .map (chỉ với vai trò tile/object)
object_name = "Tile00_Ground/Grass00"   # tên trong ObjectTable (chỉ với vai trò tile/object)

[[sprites.images]]
index = 0                         # chỉ số ảnh trong blob gốc
page = 0                          # trang atlas; vắng nếu ảnh rỗng (0×0)
x = 0                             # toạ độ trong trang (px)
y = 0
width = 40
height = 40
pixel_sha256 = "…"                # SHA-256 của buffer RGBA thô (4 byte/pixel, RGBA)

[[sprites.frames]]
image = 0                         # chỉ số ảnh trong images
offset_x = 0                      # f0
offset_y = 39                     # f1
flag = -1                         # f2 — nghĩa chưa chốt (MAPS §3), giữ nguyên số
x0 = 0                            # f4 — hình chữ nhật nguồn, tương đối so với ảnh
y0 = 0
x1 = 40                           # f6
y1 = 40                           # f7
canvas_width = 40                 # f8
canvas_height = 40                # f9
pivot_x = 0                       # f10
pivot_y = 39                      # f11
```

Toạ độ tuyệt đối của frame trong atlas là `image.x + x0`, `image.y + y0`.

**Vai trò của sprite.** Phần lớn vai trò lấy từ closure (`tile`, `object`, `theme`,
`preview`, `character_walk`, `character_idle`, `character_trapped`, `character_dead`,
`bomb_art`, `flame_art`, `explosion_art`, `trapped_art`, `item_icon`). Bundle schema 2 bổ
 sung hai vai trò cho entry **chỉ có trong bảng clip animation** (closure chỉ liệt kê entry
đã fingerprint cho một trận V1, task #34):

| Vai trò | Entry | Vì sao |
|---|---|---|
| `character_start` | `FxCharacter/Character/C0x-Start_ALayer{Lớp}` | clip state `start` của V1; entry không có trong closure |
| `flame_motion_blur` | `fx/MotionBlur/Blur_Fire01_{Hướng}` | clip lửa nhạt theo hướng; entry không có trong closure |

Bundle schema 3 bổ sung vai trò **`ui`** cho ảnh của một phần tử UI (task #84): ảnh nằm
trong atlas như sprite thường, còn trạng thái dùng của phần tử nằm ở `[[ui]]` (§3.3).

**Ảnh rỗng (0×0).** Blob sprite của client gốc có thể khai một ảnh 0×0 làm giữ chỗ cho
frame cuối (`src = (0,0,0,0)`) — ví dụ `FxTile/Tile00/BoxOrange` có 9 frame, ảnh thứ hai
là 0×0. Ảnh rỗng không đặt trong atlas nên `page` vắng mặt; chỉ số ảnh vẫn giữ nguyên số
thứ tự blob để đối chiếu với pack. Clip chết cũng kết thúc bằng một frame như vậy.

### 3.2 Clip

```toml
[[clips]]
id = "c00.walk.down.static"       # ID ổn định của clip
state = { walk = "down" }         # state trình diễn mà clip phục vụ
pack = "FxCharacter"              # pack chứa entry
entry = "Character/C00-WalkDown_ALayerStatic"
layer = "static"                  # lớp sprite; vắng nếu clip không chia lớp
sprite_unk = 1                    # dword thứ tư của header sprite — nghĩa chưa chốt
sprite = "FxCharacter/Character/C00-WalkDown_ALayerStatic"   # sprite chứa ảnh của clip
source = "kr_static"              # nguồn bằng chứng (AGENTS §4)
status = "confirmed"              # nhãn bằng chứng: `confirmed` (R-077) hoặc `candidate` (R-078)
claims = ["R-077"]                # claim RULES_V1; phải có ít nhất một claim
note = "…"                        # lý do, giới hạn, ứng viên

[clips.timing.unknown]
reason = "pack client gốc không mang trường thời lượng frame …"

[[clips.frames]]
image = 0                         # chỉ số ảnh trong ảnh của `sprite`
offset_x = -3                     # f0
offset_y = 26                     # f1
flag = 0                          # f2 — nghĩa chưa chốt (MAPS §3)
x0 = 0                            # f4 — hình chữ nhật nguồn, tương đối so với ảnh
y0 = 0
x1 = 34                           # f6
y1 = 16                           # f7
canvas_width = 56                 # f8
canvas_height = 67                # f9
pivot_x = 8                       # f10
pivot_y = 66                      # f11
```

- **Ảnh của clip không nhân bản.** `sprite` trỏ tới một entry trong `[[sprites]]`; frame
  của clip trỏ tới ảnh (theo chỉ số `image`) của sprite đó. Mỗi entry chỉ có một bản
  pixel trong atlas, nên clip và sprite dùng chung ảnh.
- **`state`** là state trình diễn: `walk` (kèm hướng `up`/`down`/`left`/`right`), `idle`,
  `start`, `trapped`, `dead`, `bomb_fuse`, `flame` (`main` hoặc `motion_blur` kèm hướng),
  `item`.
- **`timing`** giữ đúng trạng thái bằng chứng của R-079:
  - `[clips.timing.unknown]` + `reason`: **không** mang số. V1 chưa chốt thời lượng
    frame nên bundle không tự đặt.
  - `[clips.timing.known]` + `ms`, `source`, `status`, `claims`: khi V1 chốt được nhịp,
    giá trị là `design` **có claim dẫn tới** và đi kèm profile rules — client đọc giá
    trị từ profile/overlay, không hard-code.
  - `known` thiếu claim (kể cả khi nguồn là `design`) thì reader từ chối bundle.

### 3.3 Phần tử UI

```toml
[[ui]]
id = "lobby.create"                  # ID ổn định của phần tử UI
screen = "lobby"                     # màn hình: login/lobby/room/match/result
role = "button"                      # background/button/dialog/result_art/info
entry = "FxStage/Lobby/LB_Bt_T_Create-NM"   # entry pack; vắng nếu V1 không dùng asset
state = "needs_review_text"          # selected / needs_review_text / excluded
reason = "ảnh nút 9 frame có chữ Hàn …"     # lý do/ghi chú — bắt buộc, không rỗng
sprite = "FxStage/Lobby/LB_Bt_T_Create-NM"  # sprite chứa ảnh; vắng khi `excluded`
source = "kr_static"                 # nguồn bằng chứng (AGENTS §4)
status = "confirmed"                 # trạng thái bằng chứng (AGENTS §4)
```

- **Ba trạng thái dùng** (không có trạng thái "đã duyệt"):
  | Trạng thái | Nghĩa | Ảnh trong atlas | Hiển thị được |
  |---|---|---|---|
  | `selected` | Có trong closure V1 và **không có chữ** cần duyệt | có | có |
  | `needs_review_text` | Có trong closure V1 nhưng **có chữ** | có | chỉ khi ảnh được duyệt; chưa duyệt thì client vẽ text từ catalog |
  | `excluded` | Không dùng asset này | không | thay bằng text/màu theo `reason` |
- **Ảnh của phần tử UI không nhân bản**: `sprite` trỏ tới một entry trong `[[sprites]]`
  (vai trò `ui`), nên mỗi entry chỉ có một bản pixel trong atlas — cùng nguyên tắc với
  clip.
- **`reason` bắt buộc** cho mọi phần tử: mọi quyết định dùng phải truy được, và phần tử
  chưa duyệt ảnh phải ghi rõ client vẽ text thay.
- **Nguồn bằng chứng**: phần tử có asset thì `kr_static`/`confirmed` (entry có thật trong
  pack KR 00967, SHA-256 nằm ở `[[sprites]].source_sha256`); phần tử **không** dùng asset
  thì `design`/`design` — đây là quyết định của dự án (vẽ text/màu theo
  [UI_STRINGS_V1](../localization/UI_STRINGS_V1.md)), không phải luật chơi nên không cần
  claim RULES_V1.
- Reader kiểm: ID phần tử là duy nhất; `selected`/`needs_review_text` phải có `entry` và
  trỏ tới một `sprite` có thật; `excluded` không được có `sprite`; `reason` không rỗng.
- **Ảnh không giải được thì phần tử hạ xuống `excluded`**: dựng bundle ghi thêm một mục
  `[[unknown]]` loại `ui` kèm lỗi kỹ thuật, nối lý do đó vào `reason` của phần tử và
  bỏ ảnh khỏi atlas. Client vẽ text thay, còn bundle vẫn dựng được — cùng cách xử lý
  sprite và clip không giải được. Trạng thái bằng chứng của phần tử đó là `candidate`
  (mới biết entry trỏ tới pack KR, chưa xác nhận được payload).


## 4. `content_hash`

`content_hash` = SHA-256 của **encoding chuẩn** của metadata, không phải của file TOML:

```text
"boom-bundle-v3\0"
u32 schema
str build, str map, str tileset
u32 width, u32 height
u32 n_spawns × (u32 x, u32 y)
u32 n_atlas  × (str file, u32 width, u32 height, str sha256)
u32 n_sprites × (
    str id, str entry, str role, str status, str source_sha256,
    opt_u32 map_id, str object_name ("" = không có),
    u32 n_images × (u32 index, opt_u32 page, u32 x, u32 y, u32 width, u32 height, str pixel_sha256),
    u32 n_frames × (u32 image, i32 offset_x, i32 offset_y, i32 flag,
                    i32 x0, i32 y0, i32 x1, i32 y1,
                    i32 canvas_width, i32 canvas_height, i32 pivot_x, i32 pivot_y))
u32 n_cells   × (u32 x, u32 y, opt_u32 tile, opt_u32 object)
u32 n_assets  × (str entry, str role, str status, str source_sha256, u32 bytes, str reason)
u32 n_unknown × (str kind, str id, str reason, u32 n_candidates × str candidate)
u32 n_clips   × (
    str id, str state, str pack, str entry, opt_str layer,
    u32 sprite_unk, str sprite,
    str source, str status,
    u32 n_claims × str claim,
    str note ("" = không có),
    str timing_kind ("unknown" | "known"),
    u32 ms (chỉ khi timing_kind = "known"),
    str timing_source, str timing_status (chỉ khi timing_kind = "known"),
    u32 n_timing_claims × str timing_claim (chỉ khi timing_kind = "known"),
    str timing_reason (chỉ khi timing_kind = "unknown"),
    u32 n_frames × (u32 image, i32 offset_x, i32 offset_y, i32 flag,
                    i32 x0, i32 y0, i32 x1, i32 y1,
                    i32 canvas_width, i32 canvas_height, i32 pivot_x, i32 pivot_y))
u32 n_ui     × (
    str id, str screen, str role, str state,
    opt_str entry, str reason, opt_str sprite,
    str source, str status)
```

- Số là little-endian; chuỗi có tiền tố độ dài `u32` rồi byte UTF-8.
- `opt_u32` = cờ hiện diện (`u32` 0 = không có, 1 = có) rồi mới tới giá trị `u32`.
  **Không** dùng giá trị sentinel (`u32::MAX`): mọi ID `u32` — kể cả `u32::MAX` — phải
  cho encoding khác nhau và khác `None`.
- `opt_str` cũng dùng cờ hiện diện, theo cùng lý do.
- Tiền tố encoding đổi theo schema: schema 1 dùng `"boom-bundle-v1\0"`, schema 2 dùng
  `"boom-bundle-v2\0"`, schema 3 dùng `"boom-bundle-v3\0"`. Hash của hai schema khác nhau
  không thể trùng.
- `content_hash` không nằm trong encoding (vòng lặp), nhưng **SHA-256 của từng file
  atlas thì có**: hash phải ràng buộc đúng byte file mà reader nạp. Nếu bỏ, thay ảnh
  trong atlas vẫn giữ nguyên `content_hash` dù nội dung hiển thị đã khác — reader nhận
  nhầm artifact.
- **Clip nằm trong encoding**: đổi dãi frame, canvas, pivot, nhãn bằng chứng hay thời
  lượng của clip thì `content_hash` phải đổi.
- **Bảng UI nằm trong encoding**: đổi trạng thái dùng, lý do, ảnh hay màn hình của một
  phần tử UI thì `content_hash` phải đổi.
- Đánh đổi của thiết kế này: `content_hash` **phụ thuộc byte file atlas**, nên đổi bộ
  mã PNG cũng đổi hash. Đổi lại reader ràng buộc được nội dung hiển thị mà không cần
  giải ảnh (xem §5).

Tái lập: cùng pack + cùng phiên bản bộ mã PNG ⇒ cùng `content_hash`, vì thứ tự sprite
theo thứ tự manifest closure (rồi entry chỉ có trong bảng clip, rồi ảnh UI theo thứ tự
bảng UI), thứ tự ảnh theo thứ tự blob gốc, thứ tự clip theo thứ tự bảng clip animation,
thứ tự phần tử UI theo thứ tự bảng UI, và thuật toán đóng gói atlas có thứ tự ảnh cố
định (chiều cao giảm dần, rồi chiều rộng, rồi thứ tự ban đầu).

## 5. Reader và kiểm tra

`boom_content::bundle::load(<thư mục>)` đọc `bundle.toml`, kiểm tham chiếu, kiểm
`content_hash` và kiểm SHA-256 từng file atlas. `Bundle::validate()` kiểm:

1. `schema` đúng; lưới ô đủ `width × height`, không ô nào lặp hoặc nằm ngoài lưới.
2. Mọi spawn nằm trong lưới và không trùng toạ độ nhau. Phân loại spawn (va chạm với
   ô, ý nghĩa của vị trí spawn) vẫn thuộc bảng phân loại map, **không** nhân bản vào
   bundle (AGENTS §3 — một thông tin, một nguồn chuẩn).
3. Mỗi ảnh có trang atlas hợp lệ (hoặc là ảnh rỗng 0×0 không có trang) và không tràn trang.
4. Mỗi frame trỏ tới ảnh có thật; hình chữ nhật nguồn nằm trong ảnh; canvas không nhỏ hơn
   hình nguồn; pivot nằm trong canvas.
5. Mỗi ô có sprite nền và (nếu có) sprite vật cản, **hoặc** ID đó nằm trong `unknown` kèm
   lý do — không ô nào bị bỏ qua âm thầm.
6. Mọi mục `unknown` đều có lý do; mọi `id` của sprite là duy nhất.
7. **Clip** (schema 2): ID clip là duy nhất; clip phải có ít nhất một claim; `sprite` phải
   khớp `<pack>/<entry>` và trỏ tới một sprite có thật; clip phải có frame; mỗi frame
   trỏ tới ảnh có thật của sprite đó, rect nằm trong ảnh, canvas không nhỏ hơn hình
   nguồn, pivot nằm trong canvas; thời lượng phải hợp lệ — `unknown` không mang số,
   `known` phải khác 0 và phải có claim (kể cả khi nguồn là `design`).
8. **Bảng UI** (schema 3): ID phần tử là duy nhất; `reason` không rỗng; phần tử
   `selected` hoặc `needs_review_text` phải có `entry` và trỏ tới một `sprite` có thật;
   phần tử `excluded` không được có `sprite` (ảnh của nó không nằm trong atlas).

`load()` kiểm thêm: file atlas tồn tại, SHA-256 khớp `sha256` ghi trong metadata, và
header PNG (chunk IHDR, có CRC) cho kích thước đúng bằng `width`/`height` của trang.

**Schema cũ bị từ chối.** Bundle schema 1 không có clip; đọc nó sẽ im lặng bỏ qua phần
nội dung mà client V1 cần để phát animation. Bundle schema 2 không có bảng UI; đọc nó
sẽ im lặng bỏ qua phần nội dung quyết định phần tử UI nào dùng được. Reader báo lỗi
schema kèm hướng dẫn dựng lại bằng `boom-assets bundle` — không có đường đọc tương
thích.

Đây là kiểm tra *siêu hình*; reader **không** giải ảnh (client giải khi nạp texture) và
không kiểm ngữ nghĩa field frame (`f2` vẫn chưa chốt, xem [MAPS §3](MAPS.md)). Ràng buộc
giữa `content_hash` và **pixel thật** làm theo chuỗi: `content_hash` gồm SHA-256 của
file atlas, còn `pixel_sha256` của từng ảnh là hash buffer RGBA mà builder ghi lúc
dựng (test `atlas_pixels_match_the_recorded_image_hashes` kiểm lại điều đó). Nhờ vậy
thay ảnh trong atlas — kể cả khi người dựng cập nhật luôn `sha256` của file — thì
`content_hash` cũ không còn khớp và `load()` từ chối bundle đó.

## 6. Đo trên pack KR 00967 (2026-10-20)

Nguồn `kr_static` / `confirmed` cho phần dữ liệu; `design` cho định dạng.

| Đại lượng | Giá trị |
|---|---|
| Map | `Village00` (15×13, 4 spawn) |
| Sprite trong bundle | 70 (5 tile, 7 object, theme, preview, 36 sprite nhân vật: 16 walk + 8 idle + 4 trapped + 4 chết + 4 start, 3 icon item, bom, lửa, 4 lửa nhạt, bong bóng, vụ nổ, **9 ảnh UI**) |
| Ảnh | 299 (trong đó 6 ảnh giữ chỗ 0×0) |
| Trang atlas | 1 trang 2048×1430 |
| Ô | 195, mọi ô có sprite nền |
| Clip | 46 (24 clip R-077 `confirmed`, 22 clip R-078 `candidate`), 368 frame |
| Thời lượng frame | 46/46 clip là `unknown` (R-079): không mang số |
| Phần tử UI | 23: 9 `selected`/`needs_review_text` có ảnh trong atlas, 14 `excluded` (không có ảnh) |
| `unknown` | 5 mục: 4 `character_state` (state `idle`/`trapped` của C00/C01 chưa có trace consumer) + 1 `animation_clip` (vụ nổ có ảnh nhưng chưa có clip) |
| `content_hash` | `6938079e9c417fc241b76bc0f6cfc39a0a1c4b2b3aa68e9e3e2720c9da4e12bf` |

Cách kiểm lại:

```sh
BOOM_KR_PACK_DIR=/home/admin/boomkr-full \
  cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/b
# content_hash phải bằng giá trị trên; dựng hai lần cho cùng hash và cùng byte file.
```

## 7. Quy ước đặt ảnh của client (presentation — `design`)

Nguồn `design` / trạng thái `design` (AGENTS §4): đây là **quyết định trình diễn của
client**, không phải luật chơi và **không** phải cách client gốc đặt ảnh — nghĩa
`f0..f11` vẫn là câu hỏi mở **Q-003** (xem [MAPS §3](MAPS.md)). Vì không có consumer
runtime đã xác nhận, quy ước ghi ở đây để client, reviewer và công cụ đo dùng chung
một công thức, và để đổi được khi có trace.

Quy ước (ô rộng `T` px, `T` = `grid.tile_px` trong profile rules, claim R-001):

1. Mốc ô: mép trái `x = cột·T`, mép dưới `y = (hàng+1)·T`.
2. **Canvas** của frame (`f8`, `f9`) đặt sao cho *đáy canvas trùng mép dưới ô* và
   *trục X của pivot (`f10`) trùng mép trái ô*:
   `canvas.x = cột·T − pivot_x`, `canvas.y = (hàng+1)·T − canvas_h`.
3. **Vùng ảnh thật** của frame (rect nguồn `f4`–`f7`) đặt trong canvas: *canh giữa
   ngang*, *canh đáy*.

Vì sao trục Y theo canvas chứ không theo `pivot_y`: ở Village00, `pivot_y` của vật cản
không trùng đáy canvas (`FxTile/Tile00/Fountain00` lệch 31 px, `BoxOrange`/`BoxRed` lệch
2 px), nên neo theo `pivot_y` sẽ đặt vùng ảnh lệch khỏi mặt đất (vòi nước ~30 px dưới
ô). Canh đáy theo canvas đặt đáy vùng ảnh đúng bằng mép dưới ô cho cả 5 vật cản của
Village00 (`Grass00/01`, `Road00/01/02`, `BoxOrange`, `BoxRed`, `Fountain00`,
`HouseBlue/Red/Yellow`, `Tree00`).

Giới hạn đã biết:

- Không có bằng chứng consumer nào cho quy ước này; test của client chỉ là
  **regression** theo đúng công thức đã chốt, không phải bằng chứng fidelity.
- `R-009` (neo hiển thị **nhân vật** `x = cột·40`, `y = hàng·40 + 39`) là quan sát khác
  đối tượng; **không** dùng nó làm bằng chứng cho neo vật cản.
- Bước kiểm lại (đóng ở Q-003): đọc consumer render của client gốc (`CDataI16Alpha`)
  hoặc đối chiếu ảnh chạy VN; nếu cách đặt ảnh gốc khác thì sửa quy ước ở đây và
  cập nhật client theo, không âm thầm lệch.

## 8. Giới hạn đã biết

- **Chỉ map V1.** Closure hiện chỉ dựng được `Village00`; `bundle --map <khác>` báo lỗi
  rõ ràng chứ không đoán.
- **Không nhân bản phân loại ô.** Va chạm/loại ô vẫn ở bảng phân loại `data/maps/*.toml`
  (một thông tin, một nguồn chuẩn — AGENTS §3). Client nạp cả bundle và bảng phân loại.
- **Không có thời lượng frame.** Field `f8..f11` của client gốc **không** phải thời gian
  ([MAPS §3](MAPS.md)); bundle mang `timing = unknown` kèm lý do. Nhịp là việc của
  V1-031 và phải là `design` có claim — khi chốt, giá trị đi kèm profile rules, client
  đọc từ profile/overlay chứ không hard-code.
- **Vụ nổ chưa có clip.** `FxDesktop/Bomb/BombExplosion001` có ảnh trong atlas (vai trò
  `explosion_art`) nhưng bảng clip animation của V1 chưa có clip nào dùng tới: chưa có
  claim state → animation cho hiệu ứng nổ. Bundle ghi rõ ở `[[unknown]]` (kind
  `animation_clip`) thay vì bỏ qua âm thầm.
- **Lớp `Static`/`Adjust` chỉ ghi lại.** Thứ tự vẽ và cách blend của hai lớp chưa có
  consumer xác nhận (Q-003); bundle chỉ ghi lớp nào là lớp nào.
- **`f2` chưa chốt.** Ngữ nghĩa field `flag` vẫn chưa biết; bundle giữ nguyên số.
- **Không có BGM hay bảng chỉ số.** Entry không phải ảnh được ghi ở `[[assets]]` kèm lý do
  để không bị bỏ qua âm thầm.
- **Atlas đóng gói bằng thuật toán kệ (shelf).** Không tối ưu chặt chẽ; một trang 2048×2048
  chứa được toàn bộ ảnh Village00. Ảnh vượt trần 2048 px thì báo lỗi thay vì cắt.
- **Chưa có bước kiểm pixel của client.** Bộ giải mã sprite đã khớp từng pixel với tool
  tham chiếu `boom-re-l10n/tools/i16.py` (task #33), nhưng chưa có trace runtime của client
  KR/VN để chứng minh client tiêu thụ `f0..f11` đúng như ta hiểu.
- **Nền màn hình không vào atlas.** Nền V1 (`FxDesktop/LoginStage/CAStage`,
  `FxStage/Lobby/CAStage`, `FxMap/Prepare/CAStage`) là **JPEG** 800×600: bộ giải mã bundle
  chỉ đọc sprite I16, và ảnh còn có chữ/nhãn ngoài scope V1. Các nền này ở trạng thái
  `excluded` kèm lý do; V1 dựng nền bằng màu, chữ lấy từ catalog. Muốn đưa nền vào
  bundle thì cần bộ giải mã JPEG (và bản ảnh đã che chữ) — quyết định kiến trúc, thuộc
  integrator.
- **Ảnh có chữ chưa được duyệt.** 8/9 ảnh UI trong atlas có chữ, nên ở trạng thái
  `needs_review_text`: client chỉ hiển thị khi có quyết định duyệt ảnh
  ([IMAGE_REVIEW](../localization/IMAGE_REVIEW.md)), tới lúc đó vẽ text từ catalog. Bundle
  **không** có trạng thái "đã duyệt" nên không thể vô tình hiển thị ảnh chưa duyệt.
- **Bảng UI là quyết định của dự án.** Nó ghi trạng thái dùng và cách thay thế, không
  phải luật chơi; nhãn nguồn/trạng thái theo AGENTS §4 (`kr_static`/`confirmed` cho
  asset có thật, `design`/`design` cho phần tử không dùng asset nào).
- **`content_hash` gắn với byte file atlas.** Đổi bộ mã PNG (đổi phiên bản thư viện
  `png`) thì hash đổi, dù pixel không đổi. Đánh đổi có chủ ý: reader ràng buộc được
  nội dung hiển thị mà không cần giải ảnh. Nếu sau này cần hash **theo pixel thuần**
  (đổi bộ mã PNG không đổi hash) thì reader phải giải PNG và đối chiếu từng vùng RGBA
  với `pixel_sha256` — việc đó cần thêm bộ giải ảnh vào `boom-content` (quyết định kiến
  trúc, thuộc integrator).
