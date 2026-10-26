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

**Schema hiện hành: 4** (có `[[clips]]`, `[[ui]]` và `[[audio]]`). Bundle schema 3 (không audio)
vẫn đọc được. Schema 2 (chỉ atlas + ô + clip) và schema 1 (chỉ atlas + ô) **bị reader từ chối** — xem §5.

## 1. Dựng bundle

```sh
# thư mục pack lấy từ --pack-dir, hoặc biến môi trường BOOM_KR_PACK_DIR
cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/bundle-village00
```

Kết quả in ra số sprite/ảnh/trang atlas/ô/unknown và `content_hash`.

Nguồn của bundle là **manifest closure** (`crates/boom-assets/src/closure.rs`, task #34),
**bộ giải mã sprite** `CDataI16Alpha` v2 (`crates/boom-assets/src/sprite.rs`, task #33) và
**bốn bảng mapping** ở `data/assets/` (issue #123):

| File data | Bảng | Nguồn trước đây |
|---|---|---|
| `data/assets/animation-v1.toml` | clip animation: state → entry pack | `boom-assets/src/animation.rs` |
| `data/assets/ui-v1.toml` | asset UI: phần tử → entry pack | `boom-assets/src/closure.rs` |
| `data/assets/audio-v1.toml` | âm thanh: event → entry `FxSound` | `boom-assets/src/audio.rs` |
| `data/assets/closure-v1.toml` | closure: entry cố định, họ entry nhân vật, mapping state V1 → state client | `boom-assets/src/closure.rs` |

Các bảng này là **data có nhãn bằng chứng** (AGENTS §4), giống `data/rules/*.toml`: mỗi dòng
mang `source`, `status`, `claims` và ghi chú. Loader chung nằm ở
`crates/boom-content/src/assets.rs` (`boom_content::assets`); `boom-assets bundle` dùng
loader đó, nên **đổi mapping = sửa file data, không sửa code** (xem §1.1).

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

### 1.1 Sửa mapping: quy trình và quy tắc loader

Bốn bảng ở `data/assets/` là nguồn chuẩn duy nhất (một thông tin — một nguồn, AGENTS §3).
Muốn đổi mapping (đổi entry của một state, đổi nhãn bằng chứng, thêm cue âm thanh…):

1. Sửa file `.toml` tương ứng trong `data/assets/`.
2. Chạy `cargo xtask ci` — loader từ chối dữ liệu sai (xem bảng lỗi bên dưới).
3. Nếu đổi nội dung hiển thị thì `content_hash` **phải** đổi: test
   `changing_one_mapping_changes_the_content_hash` (boom-assets) chặn trường hợp mapping
   đổi mà hash không đổi.

| Lỗi loader | Nghĩa |
|---|---|
| ID trùng trong một bảng | hai dòng cùng ID (clip/UI/cue/entry/họ/state) |
| Thiếu trường bắt buộc | dòng thiếu `entry`, `pack`, `state`, `reason`… |
| State/hướng không hợp lệ | `state` không thuộc state V1, hoặc hướng không hợp lệ |
| `timing.unknown` mang giá trị | thời lượng `unknown` không được mang số (R-079) |
| `timing.known` thiếu claim | thời lượng đã chốt phải có claim, kể cả nguồn `design` |
| Nhãn sai | `design` không được mang `confirmed`/`candidate`; `confirmed` cần nguồn `kr_*`/`vn_*`; `web`/`oss`/`emu_choice` chỉ được là `candidate`/`design`; dòng mapping không được mang `unknown` |
| Claim sai dạng | claim phải có dạng `R-<số>` |
| Khoá lạ trong dòng | gõ sai tên trường bị từ chối (không bỏ qua âm thầm) |

Ý nghĩa cột `source`/`status` **khác nhau theo bảng** và được ghi ở đầu mỗi file data:
bảng clip nói về *mapping* state → entry, bảng UI nói về *sự tồn tại của asset*, bảng audio
nói về *mapping* event → entry. Nhãn cũ được giữ nguyên khi chuyển sang data — mapping
consumer **không** được nâng lên `confirmed` chỉ vì entry có trong pack (AGENTS §4).


## 2. Cấu trúc thư mục bundle

```text
<bundle>/
  bundle.toml            # metadata (schema 4)
  audio/<id>.<format>    # payload âm thanh thô (schema 4; không commit asset vào repo)
  atlas/page-000.png     # các trang atlas
  atlas/page-001.png     # (chỉ khi cần hơn một trang)
```

## 3. Metadata `bundle.toml` (schema 4)

| Khóa | Kiểu | Ý nghĩa |
|---|---|---|
| `schema` | số | `4` cho bundle có audio; `3` (không audio) vẫn đọc được |
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
| `[[audio]]` | bảng | Cue âm thanh của V1 (BGM + SFX), xem §3.4; **chỉ schema 4** |

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

Nguồn: `data/assets/animation-v1.toml` (bảng state → entry của V1, có nhãn bằng chứng).

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
  `explosion`, `item`.
- **`timing`** giữ đúng trạng thái bằng chứng của R-079:
  - `[clips.timing.unknown]` + `reason`: **không** mang số. V1 chưa chốt thời lượng
    frame nên bundle không tự đặt.
  - `[clips.timing.known]` + `ms`, `source`, `status`, `claims`: khi V1 chốt được nhịp,
    giá trị là `design` **có claim dẫn tới** và đi kèm profile rules — client đọc giá
    trị từ profile/overlay, không hard-code.
  - `known` thiếu claim (kể cả khi nguồn là `design`) thì reader từ chối bundle.

### 3.3 Phần tử UI

Nguồn: `data/assets/ui-v1.toml` (bảng phần tử → entry của luồng V1, có nhãn bằng chứng).

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


### 3.4 Bảng audio (schema 4)

Nguồn: `data/assets/audio-v1.toml` (bảng event → entry của V1, có nhãn bằng chứng).

Mỗi entry là một payload thô trong `audio/`, không decode:

| Trường | Kiểu | Ý nghĩa |
|---|---|---|
| `id` | chuỗi | ID cue theo AUDIO_V1, ví dụ `bgm_loop`, `bomb_exploded` |
| `entry` | chuỗi | Entry nguồn `FxSound`, ví dụ `MP3/Village` |
| `kind` | `bgm`/`sfx` | Loại voice |
| `format` | `ogg`/`wav` | Định dạng macroquad đọc được; builder bỏ qua mp3/unknown có log |
| `file` | chuỗi | Đường dẫn tương đối `audio/<id>.<format>` |
| `bytes` | số | Kích thước payload |
| `sha256` | chuỗi | SHA-256 payload thô, nằm trong `content_hash` |
| `status` | nhãn | Nhãn bằng chứng của mapping (AUDIO_V1) |

Reader kiểm `id`/`file` duy nhất, `file` nằm trong `audio/`, `bytes`/SHA-256 khớp file thật.
Schema 3 không được có bảng này.

### Chuỗi phát ODT của clip (`[[clips.sequences]]`, ADR 0014)

Clip có `odt` trong `data/assets/animation-v1.toml` mang thêm:

| Trường | Kiểu | Ý nghĩa |
|---|---|---|
| `odt` | chuỗi | Khối bảng animation trong `fx/ObjectTable/ObjectTable`, ví dụ `Character/C00_A`, `Bomb/Bubble` |
| `sequences[].state` | chuỗi | Tên state ODT (nhãn bỏ số thứ tự đầu), ví dụ `Walk Left`, `Lock`, `Help` |
| `sequences[].steps[]` | `{frame, units}` | Chỉ số frame trong `frames` của clip và thời lượng thô theo đơn vị ODT (R-251) |

Reader kiểm mỗi chuỗi có ít nhất một bước và mọi `frame` nằm trong clip. Chuỗi vào `content_hash`
sau thẻ `\0odt` chỉ khi clip có chuỗi, nên bundle cũ không có chuỗi giữ nguyên hash.
Bộ dựng báo lỗi nếu mapping khai khối/state mà ODT của pack không có.

## 4. `content_hash`

`content_hash` = SHA-256 của **encoding chuẩn** của metadata, không phải của file TOML:

```text
"boom-bundle-v4\0"          # schema 3 dùng "boom-bundle-v3\0"
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
u32 n_audio  × (
    str id, str entry, str kind, str format, str file,
    u32 bytes, str sha256, str status)          # chỉ schema 4
```

- Số là little-endian; chuỗi có tiền tố độ dài `u32` rồi byte UTF-8.
- `opt_u32` = cờ hiện diện (`u32` 0 = không có, 1 = có) rồi mới tới giá trị `u32`.
  **Không** dùng giá trị sentinel (`u32::MAX`): mọi ID `u32` — kể cả `u32::MAX` — phải
  cho encoding khác nhau và khác `None`.
- `opt_str` cũng dùng cờ hiện diện, theo cùng lý do.
- Tiền tố encoding đổi theo schema: schema 1 dùng `"boom-bundle-v1\0"`, schema 2 dùng
  `"boom-bundle-v2\0"`, schema 3 dùng `"boom-bundle-v3\0"`, schema 4 dùng `"boom-bundle-v4\0"`.
  Hash của hai schema khác nhau
  không thể trùng.
- `content_hash` không nằm trong encoding (vòng lặp), nhưng **SHA-256 của từng file
  atlas thì có**: hash phải ràng buộc đúng byte file mà reader nạp. Nếu bỏ, thay ảnh
  trong atlas vẫn giữ nguyên `content_hash` dù nội dung hiển thị đã khác — reader nhận
  nhầm artifact.
- **Clip nằm trong encoding**: đổi dãi frame, canvas, pivot, nhãn bằng chứng hay thời
  lượng của clip thì `content_hash` phải đổi.
- **Bảng UI nằm trong encoding**: đổi trạng thái dùng, lý do, ảnh hay màn hình của một
  phần tử UI thì `content_hash` phải đổi.
- Đánh đổi của thiết kế này: `content_hash` **phụ thuộc hash byte file atlas/audio**, nên đổi bộ
  mã PNG cũng đổi hash. Đổi lại reader ràng buộc được nội dung hiển thị mà không cần
  giải ảnh (xem §5).

Tái lập: cùng pack + cùng phiên bản bộ mã PNG ⇒ cùng `content_hash`, vì thứ tự sprite
theo thứ tự manifest closure (rồi entry chỉ có trong bảng clip, rồi ảnh UI theo thứ tự
bảng UI), thứ tự ảnh theo thứ tự blob gốc, thứ tự clip theo thứ tự bảng clip animation,
thứ tự phần tử UI theo thứ tự bảng UI, và thuật toán đóng gói atlas có thứ tự ảnh cố
định (chiều cao giảm dần, rồi chiều rộng, rồi thứ tự ban đầu).

## 5. Reader và kiểm tra

`boom_content::bundle::load(<thư mục>)` đọc `bundle.toml`, kiểm tham chiếu, kiểm
`content_hash` và kiểm SHA-256 từng file atlas/audio. `Bundle::validate()` kiểm:

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
9. **Bảng audio** (schema 4): ID/file duy nhất; `kind` là `bgm`/`sfx`; `format` là `ogg`/`wav`;
   `file` nằm trong `audio/`; `bytes` khác 0 và SHA-256 hợp lệ; schema 3 không có audio.

`load()` kiểm thêm: file atlas tồn tại, SHA-256 khớp `sha256` ghi trong metadata, header PNG
(chunk IHDR, có CRC) cho kích thước đúng bằng `width`/`height` của trang; file audio tồn tại,
đúng `bytes` và SHA-256.

**Schema cũ bị từ chối.** Bundle schema 1 không có clip; schema 2 không có bảng UI; reader báo lỗi
kèm hướng dẫn dựng lại bằng `boom-assets bundle`. Schema 3 (không audio) vẫn đọc được và
`content_hash` giữ encoding v3; client không phát audio nếu bảng vắng.

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
| Sprite trong bundle | 71 (5 tile, 7 object, theme, preview, 36 sprite nhân vật: 16 walk + 8 idle + 4 trapped + 4 chết + 4 start, 3 icon item, bom, lửa, 4 lửa nhạt, bong bóng, chữ HELP!, vụ nổ, **9 ảnh UI**) |
| Ảnh | 303 (trong đó 6 ảnh giữ chỗ 0×0) |
| Trang atlas | 1 trang 2048×1430 |
| Audio V1 | 15 entry (1 BGM + 14 SFX), 468.234 byte (457,3 KiB) payload thô trong `audio/` (không commit) |
| Ô | 195, mọi ô có sprite nền |
| Entry không phải ảnh | 4: BGM `FxSound/MP3/Village`, file map, 2 bảng chỉ số |
| Clip | 48 (25 clip `confirmed` — R-077/R-084, 22 clip R-078 `candidate`, `trapped.help` R-220 `candidate`), 382 frame |
| Thời lượng frame | 48/48 clip là `unknown` (R-079): không mang số |
| Phần tử UI | 23: 9 `selected`/`needs_review_text` có ảnh trong atlas, 14 `excluded` (không có ảnh) |
| `unknown` | **0 mục** — bốn state nhân vật `idle`/`trapped` đã giải bằng quyết định của V1 (R-086 `design`) đối chiếu bảng state của client trong ODT (§6.1) |
| `content_hash` | `cd9578328a0730fec8056e1eb6c479c81d7242c7cdb2b39887344a3775da0ad7` (schema 4, **có** audio, có chuỗi ODT R-251 và chữ HELP! R-220/R-250; dựng không kèm `--ui-localization`). Lịch sử: `dea731aa…` → `74903a59…` sau `64b5620`/#237 (quên cập nhật) → `b410bbbd…` (#351) → hash này (#346) |

Cách kiểm lại:

```sh
BOOM_KR_PACK_DIR=/home/admin/boomkr-full \
  cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/b
# content_hash phải bằng giá trị trên; dựng hai lần cho cùng hash và cùng byte file.
```

#206 đổi `player_trapped` sang `Wave/Cha_LockedIn` (6810 B thay 5639 B) nên tổng audio và
`content_hash` ở bảng trên đã đo lại; logic bundle không đổi.

`content_hash` **băm cả byte file atlas**, nên đổi bộ mã PNG cũng đổi hash: giá trị ở
trên gắn với bộ mã trong `Cargo.lock` hiện hành. Trước issue #121, giá trị ghi trong
bảng này đã **stale** (đo trước khi bảng audio được thêm vào hash) nên không khớp với
bundle dựng được; bảng được cập nhật lại theo lần đo mới.

Giá trị hash của **schema 3** (bundle chưa có audio) là
`27464dc28530563e1db4fba63a54062f9a75944ca8f5dab099dd29ff587609d2`; **không** dùng lại giá trị
đó cho schema 4. Lần đo ngày 2026-10-20 (đã có bảng `[[audio]]`, schema 4) cho giá trị
`6d74ad4f0462b87bf458d65d369bfb24d8a9923ad164f98c9008b7063ce87dc0` — dựng **hai lần** cho cùng
hash và cùng byte file (atlas `page-000.png` 2048×1430, 665.372 byte) — nay đã cũ sau #121
(manifest/closure đổi), nên bảng ở trên lấy giá trị đo mới.

**Issue #123 (mapping → data):** giá trị `content_hash` ở trên **không đổi** khi bốn bảng
mapping chuyển từ khai báo Rust sang `data/assets/*.toml` — bundle dựng từ data cho
**cùng** hash và **cùng** byte file. Test `bundle_from_the_data_tables_matches_the_recorded_content_hash`
chốt điều đó; test `changing_one_mapping_changes_the_content_hash` chặn trường hợp đổi
mapping mà hash không đổi.

### 6.1. Bốn state `unknown` của bundle đã giải bằng bảng state của client (issue #121)

Trước issue #121, bundle có **4 mục `unknown`** loại `character_state`:
`C00:idle`, `C01:idle`, `C00:trapped`, `C01:trapped` — "chưa có trace consumer xác nhận
state nào dùng clip nào".

**Nguồn trong chính pack KR** (claim R-094): ODT `fx/ObjectTable/ObjectTable` không chỉ
có bảng tên ô của map. Sau bảng tên là các **khối dữ liệu**, mỗi khối là bảng state của
một đối tượng. Khối `Character/<Id>_A` là **bảng state trình diễn của nhân vật**:

| nhãn | state | entry (tiền tố) | số frame khác nhau |
|---|---|---|---|
| `00 Blink` | `Blink` | `Character/C00-Blink_A` | 3 |
| `01 Die` | `Die` | `Character/C00-Die` | 14 |
| `02 Faint` | `Faint` | `Character/C00-Faint` | 3 |
| `03 Hurrah` | `Hurrah` | `Character/C00-Hurrah_A` | 4 |
| `12 Stop Left` … `15 Stop Down` | đứng yên theo hướng | `Character/C00-Walk<Dir>_A` (frame 0) | 1 |
| `20 Walk Left` … `23 Walk Down` | đi theo hướng | `Character/C00-Walk<Dir>_A` | 5 |
| `24 Start` | `Start` | `Character/C00-Start_A` | 10 |
| `25 Locked In` (+ `52`…`56`) | bị khoá (bị nhốt) | `Character/C00-Die` (frame 0…1, giữ vô hạn) | 2 |

Cột "số frame khác nhau" là số chỉ số frame **khác nhau** dùng trong state (state lặp
frame, ví dụ `Blink` có 7 cặp `(frame, thời lượng)` nhưng chỉ 3 frame ảnh khác nhau).
Con số này khớp **đúng** số frame của sprite trong pack (`C00-Blink_ALayerStatic` 3 frame,
`C00-DieLayerStatic` 14 frame, `C00-FaintLayerStatic` 3 frame,
`C00-Hurrah_ALayerStatic` 4 frame, `C00-WalkDown_ALayerStatic` 5 frame) ⇒ bảng ODT là bảng
state của chính build KR 00967, không phải dữ liệu của build khác. Client ghép tên entry
trong ODT với lớp `Layer` + `Static`/`Adjust` (đối chiếu client VN:
`CGameObjectTemplateLib::LoadNameTable` tại `0x5C9741` nối `LayerStatic`/`LayerAdjust` và
tách riêng nhánh `Die`/`Faint`).

#### Hai tầng nhãn — đừng gộp

Bảng ODT xác nhận **nhãn → entry** (dữ liệu của client). Nó **không** xác nhận state core
của V1 dùng nhãn nào — việc đó là **quyết định của V1** (R-086 `design`). Nên bundle gắn
nhãn theo đúng tầng:

| Mục | Trước | Sau | Căn cứ |
|---|---|---|---|
| `unknown` | 4 mục `character_state` | **0** | bốn state được **chốt bằng quyết định của V1** (R-086 `design`) và đối chiếu bảng ODT (R-094); xem "Giới hạn" bên dưới |
| sprite `character_walk` `C0x-Walk<Dir>_ALayer*` | `confirmed` | `confirmed` | tên entry + dãi frame đọc được trong pack (R-077) |
| sprite `character_dead` `C0x-DieLayer*` | `confirmed` | `confirmed` | tên entry + dãi frame đọc được trong pack (R-077) |
| sprite `character_idle` `C0x-Blink_ALayer*` | `candidate` | **`design`** | mapping `idle` → `Blink` là **quyết định của V1** (R-086); ODT chỉ xác nhận nhãn `Blink` → entry (R-094), **không** xác nhận state core V1 dùng nhãn đó |
| sprite `character_trapped` `C0x-FaintLayer*` | `candidate` | **`design`** | mapping `trapped` → `Faint` là **quyết định của V1** (R-086); ODT chỉ xác nhận nhãn `Faint` → entry (R-094) |
| sprite `character_idle` `C0x-BlinkSmall_ALayer*` | `candidate` | `candidate` | V1 **không chọn** họ này: bảng state `Character/<Id>_A` của C00/C01 **không** có state `BlinkSmall`. Họ sprite này **có** trong ODT ở khối `Character/<Id>_Display_A` ⇒ **không** kết luận là client không dùng |

Nhãn `design` ở bảng `[[sprites]]` nói "**V1 chọn** họ entry này cho vai trò đó"; bảng
`[[clips]]` vẫn giữ `idle`/`trapped` là `candidate` (R-078) vì claim của bảng clip là
"client gốc có dùng clip này cho state nào", mà điều đó **chưa** có consumer nào xác nhận
(Q-003). Hai bảng trả lời hai câu khác nhau — cùng một entry, hai claim khác nhau.

**Sai khác còn lại, đã ghi nhận (R-095, `candidate`):** client gốc dùng `Stop <hướng>`
(frame 0 của `Walk<hướng>_A`) cho **tư thế đứng yên** và `Blink` làm overlay nháy mắt;
state **bị nhốt** của client (`Locked In`) dùng frame 0…1 của `Die` giữ vô hạn, kèm bong
bóng. V1 chọn khác (`idle` → clip `Blink`, `trapped` → clip `Faint`, R-086 `design`).
Chưa có trace runtime nối state core của V1 với state của client, nên mức bằng chứng
của *sai khác này* vẫn là `candidate`; việc đổi lựa chọn của V1 thuộc task client
(V1-031), không thuộc issue này.

**Giới hạn đã biết** (AGENTS §4: không suy diễn quá bằng chứng):

- Bảng ODT là bằng chứng cho **nhãn → entry**. Bốn state đã giải là **quyết định của
  V1**, không phải consumer nào đã xác nhận; nhãn `design` trong bundle là để nói đúng
  điều đó.
- Phạm vi khảo sát: ODT có 189 khối `Character/*_A`; trong đó **63** khối
  `Character/<Id>_A` là bảng state trình diễn (4 state đầu giống nhau ở mọi nhân vật),
  `C00`/`C01` có **90** state. Bảng state của C00/C01 **không** có state `BlinkSmall`;
  state `BlinkSmall` chỉ thấy ở khối `Character/<Id>_Display_A` (**25** nhân vật: C00…C13,
  C17…C22, C25…C29) ⇒ họ sprite `BlinkSmall` **có** mặt trong ODT. Không thấy ở bảng
  state **không** loại trừ consumer khác hay tạo tên entry trực tiếp.
- Vì `BlinkSmall` **có** trong ODT, nhãn `candidate` của nó là "V1 giữ trong bảng clip
  làm ứng viên nhưng **không chọn**", không phải "client không dùng".

Cơ chế "không bỏ qua âm thầm": closure **đọc bảng ODT lúc dựng manifest** và kiểm
(1) mọi họ entry mà V1 dùng đều có trong bảng state của nhân vật, (2) state mà V1 chọn
(`V1_CHARACTER_STATES`) có đúng nhãn và đúng entry trong bảng đó. Lệch thì manifest ghi
mục `unknown` kèm lý do và ứng viên — xem `crates/boom-assets/src/closure.rs`.

Bảng ODT **không** nằm trong closure: nó là dữ liệu lúc dựng (client đọc lúc chạy),
không phải nội dung lúc chơi, nên không vào `[[assets]]` của bundle. SHA-256 của nó
(2.391.729 B) là `a68fd5ffcc815b24b5c5199e9910f10d14f1ac6f30aba4c237a23f4e279608dc`
— ghi ở claim R-094 để đối chiếu khi pack đổi.

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

**Actor (nhân vật) — khác quy ước tile (#305, 2026-10-26):** nhân vật đặt theo **toạ độ thật**
của core, không làm tròn về ô. Neo `(ax, ay)` = `(x_px − T/2, y_px + T/2 − 1)` (tâm hộp → mép
trái ô và chân `hàng·T + T − 1` khi đứng đúng ô, cùng mốc R-210/R-009). Mỗi frame đặt
**góc trên-trái vùng ảnh = neo − (`f0`, `f1`)** (`offset_x/offset_y`; R-209 xác nhận `f0/f1` là
trường đăng ký theo frame). Vẽ **hai lớp** `…static` rồi `…adjust` của cùng clip. Z-order của mọi
đối tượng theo **y chân** (px), cùng chân thì cột rồi thứ tự trong ô. Căn cứ: ghép lớp C00 theo
công thức này khớp ảnh client gốc VN (`battle_village01`: nhân vật đứng giữa ô, đầu nhô lên ô
trên); quy tắc canvas của tile làm nhân vật lệch xuống, chỉ vẽ lớp `static` (34×32) và bị vật
cản hàng dưới che gần hết. Vẫn là `design` (chưa trace consumer vẽ native); tile/object giữ quy
ước dưới.

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

### 7.1 Chọn clip và frame (V1)

Ngoài quy ước đặt ảnh, client V1 còn phải chọn **clip nào** cho state core nào và
**frame nào** trong clip đó. Đây là quyết định **design** của V1 (RULES_V1
[R-085…R-089](../gameplay/RULES_V1.md#81-quyết-định-trình-diễn-của-client-v1-r-085r-089)),
ghi ở đây để client, reviewer và công cụ đo dùng chung một công thức.

1. **Chọn clip theo state core** (R-086):

   | State core | Clip trong bundle | Nhãn bằng chứng |
   |---|---|---|
   | Nhân vật **đang đi** (vị trí đổi so với tick trước) | `c0x.walk.{up,down,left,right}.static` | R-077 `confirmed` |
   | Nhân vật **đứng yên** | `c0x.idle.blink.static` | R-078 `candidate` |
   | Nhân vật **bị nhốt** | `c0x.trapped.static` | R-078 `candidate` |
   | Nhân vật **chết** | `c0x.dead.static` (chuỗi chết phát trọn clip rồi giữ khung cuối **có ảnh** — §7.2) | R-077 `confirmed` |
   | Bom đang chờ nổ | `bomb.fuse` | R-077 `confirmed` |
   | Lửa trên ô | `flame.main`, **dải frame lõi 28..=59** | R-078 `candidate` (R-087) |
   | Vụ nổ (core báo event `bomb_exploded`) | `explosion.main` tại ô bom | R-084 `confirmed` (R-088) |

   Nhân vật V1 ghép với `C00`/`C01` theo `id` người chơi vì state core chưa mang trường
   nhân vật. "Đang đi" suy ra từ **vị trí đổi** giữa hai tick: hướng trong state core
   được giữ qua lệnh `Dir::None` nên không dùng làm tín hiệu đi/đứng.

2. **Nhịp frame** (R-085): **100 ms** một frame cho mọi clip, đọc từ rules profile
   (`presentation.frame_duration_ms`) — không hard-code trong client. Frame tiến theo
   **tick mô phỏng**, không theo đồng hồ wall clock (ADR 0004):
   `frame = (tick · 1000 / ticks_per_second) / frame_ms mod số frame`.

3. **Hiệu ứng nổ** (R-088): core báo event `bomb_exploded` (ô của bom) ⇒ client phát
   clip `explosion.main` tại ô đó. Hiệu ứng **phát một lần**, dài bằng số frame của
   clip × `presentation.frame_duration_ms` (10 frame × 100 ms = 1.000 ms với clip
   `BombExplosion001` của pack KR 00967); frame chọn theo **tick mô phỏng** tính từ
   tick của event, không theo đồng hồ wall clock. Hết khoảng clip thì ô đó **về lại**
   `flame.main` (lửa trong state core). Clip nổ **vẽ đè lên lửa** ở cùng ô.
   Bundle không có clip nổ ⇒ không có gì để phát, client báo cảnh báo lúc khởi động.

4. **Z-order trong cùng ô**: vật cản → lửa → nổ → bom → nhân vật. Lửa là hiệu ứng nằm trên
   nền, nổ vẽ đè lên lửa ở cùng ô, bom là vật thể đứng trên nền, nhân vật đứng trên
   bom khi mới đặt.

5. **Thiếu clip**: bundle không có clip cần cho trạng thái này → client vẽ **ô màu**
   đúng kích thước ô tại ô đó và ghi cảnh báo; không bỏ qua âm thầm (AGENTS §4) và
   không panic.

Giới hạn đã biết:

- `idle` và `trapped` là clip **candidate** (R-078): chưa có consumer nào xác nhận state
  nào dùng clip đó (Q-003). Test của client chỉ là **regression** theo quy ước đã
  chốt, không phải bằng chứng fidelity.
- Clip `flame.main` là **một** blob 100 frame gộp thân/đuôi/nhánh theo hướng; state
  core V1 không mang hướng lửa nên V1 chỉ dùng dải frame lõi 28..=59 và **mọi ô lửa
  vẽ cùng một frame** (R-087, Q-003).
- Thứ tự vẽ **nổ đè lên lửa** và cách client gốc vẽ nổ là `design` của V1 (R-088):
  chưa có consumer gốc xác nhận (Q-003); test của client chỉ là regression theo quy
  ước đã chốt.
- Chuỗi chết phát theo **sổ trình diễn** (§7.2, R-089) — quyết định `design` của V1:
  chưa có consumer gốc xác nhận (Q-003).

### 7.2 Sổ trình diễn (V1)

Sổ trình diễn (presentation journal) của client gom mọi hiệu ứng đang chạy: **nổ**,
**chết**, **bị nhốt** và **mở đầu lượt** (R-089). Đây là quyết định `design` của V1:
chưa có consumer gốc xác nhận (Q-003).

1. **Mỗi đối tượng một mục**: đối tượng là ô bản đồ (vụ nổ) hoặc nhân vật (chết, nhốt,
   mở đầu lượt). Mục mới của cùng đối tượng **thay** mục cũ (một ô chỉ có một vụ nổ;
   nhân vật chết thay chuỗi nhốt).
2. **Khoá event**: mục từ event server đã xác nhận gắn `(round_id, EventId)`. Event
   **lặp** (reconcile/snapshot gửi lại event cũ) **không** tạo mục mới ⇒ hiệu ứng không
   phát lại (ADR 0005, ARCHITECTURE §6).
3. **Mục client tự sinh**: hiệu ứng chưa có event server (mở đầu lượt) hoặc dự đoán thì
   mục **không** có khoá; khi event xác nhận của cùng đối tượng tới thì mục dự đoán bị
   **thay** và chuỗi phát lại từ tick của event.
4. **Phát một lần**: hiệu ứng chạy trong khoảng clip (số frame của clip ×
   `presentation.frame_duration_ms`), frame chọn theo **đồng hồ trình diễn** tính từ
   tick của event; hết khoảng clip thì mục bị **dọn** khỏi sổ (khoá event **vẫn giữ** ⇒
   event lặp sau đó không tạo mục mới; khoá chỉ bỏ khi đổi lượt).
5. **Nhân vật chết**: chuỗi chết phát trọn clip rồi nhân vật giữ **khung cuối có ảnh**
   của clip chết (khung cuối trong pack là ảnh rỗng 0×0). Nếu clip chết không có khung
   nào có ảnh thì nhân vật chết **không được vẽ** — không vẽ ô màu. Ô màu chỉ dùng khi
   bundle **thiếu clip** (§7.1 §5).
6. **Sổ không biết bundle**: client đưa vào sổ bảng số frame theo clip ID, nên đổi bundle
   không phải sửa sổ.
7. **Đồng hồ trình diễn**: client chọn frame theo **đồng hồ trình diễn** của nó, không
   theo đồng hồ wall clock (ADR 0004). Đồng hồ này **bám** tick mô phỏng khi lượt còn
   chạy và **tiến nốt** (cùng nhịp `sim.ticks_per_second`) sau khi lượt **kết thúc** và
   mô phỏng dừng — nếu không, chuỗi chết của lượt cuối (một người chết là lượt hai
   người kết thúc) sẽ kẹt ở frame đầu, không bao giờ tới khung cuối có ảnh (R-089).
   Quy tắc này áp dụng cho **cả hai** chế độ (offline và mạng: khi lượt kết thúc thì
   đồng hồ **tiến nốt** thay vì bám tick của state). Ở chế độ mạng client chưa nối
   event server vào sổ trình diễn (#114) nên **chưa có** chuỗi chết để phát.

## 8. Giới hạn đã biết

- **Chỉ map V1.** Closure hiện chỉ dựng được `Village00`; `bundle --map <khác>` báo lỗi
  rõ ràng chứ không đoán.
- **Không nhân bản phân loại ô.** Va chạm/loại ô vẫn ở bảng phân loại `data/maps/*.toml`
  (một thông tin, một nguồn chuẩn — AGENTS §3). Client nạp cả bundle và bảng phân loại.
- **Không có thời lượng frame.** Field `f8..f11` của client gốc **không** phải thời gian
  ([MAPS §3](MAPS.md)); bundle mang `timing = unknown` kèm lý do. Nhịp là việc của
  V1-031 và phải là `design` có claim — khi chốt, giá trị đi kèm profile rules, client
  đọc từ profile/overlay chứ không hard-code. V1 đã chốt nhịp 100 ms (R-085) trong
  profile `presentation.frame_duration_ms`; **bundle vẫn mang `unknown`** vì pack
  không có trường thời gian.
- **Lửa không có hướng trong state.** Clip `flame.main` gộp thân/đuôi/nhánh; state core
  V1 không mang hướng lửa nên V1 chỉ dùng dải frame lõi 28..=59 cho mọi ô lửa
  (BUNDLES §7.1, R-087, Q-003).
- **Clip nổ chỉ có một chuỗi.** `FxDesktop/Bomb/BombExplosion001` (vai trò `explosion_art`)
  là **một** clip 10 frame liên tục: pack không tách tâm/nhánh/đuôi nên không có clip
  riêng theo phần (R-084). Client V1 phát clip này tại **ô bom** khi core báo
  `bomb_exploded`, trong khoảng clip, rồi ô đó về lại `flame.main` (R-088).
- **Lớp `Static`/`Adjust` chỉ ghi lại.** Thứ tự vẽ và cách blend của hai lớp chưa có
  consumer xác nhận (Q-003); bundle chỉ ghi lớp nào là lớp nào.
- **`f2` chưa chốt.** Ngữ nghĩa field `flag` vẫn chưa biết; bundle giữ nguyên số.
- **BGM và SFX đã có trong bundle (schema 4).** 15 entry V1 nằm ở bảng `[[audio]]` (§3.4) với
  payload thô trong `audio/<id>.<format>`: 1 BGM (`MP3/Village`) + 14 SFX — xem
  [AUDIO_V1](AUDIO_V1.md). Entry **không phải ảnh còn lại** (file map, bảng chỉ số `Attr/*`) nằm ở
  `[[assets]]` kèm lý do để không bị bỏ qua âm thầm.
  Lưu ý: entry BGM của closure (`FxSound/MP3/Village`, vai trò `music`) **vẫn** được liệt kê ở
  `[[assets]]` với lý do ghi sẵn của builder ("âm thanh nền: không phải ảnh, chưa đưa vào bundle") —
  câu chữ đó **không còn đúng** vì payload BGM đã nằm ở `[[audio]]`/`audio/bgm_loop.ogg`
  (đo 2026-10-20 trên `main` @ `32a6d81`, tái lập sau khi gộp `main` @ `f545f24`). Sửa câu chữ
  trong builder sẽ **đổi `content_hash`**, nên tách task code riêng; ở đây chỉ ghi nhận là giới
  hạn đã biết.
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
  integrator. Claim: R-090 (nền là JPEG 800×600, `confirmed`) và R-091 (quyết định V1,
  `design`) ở [RULES_V1 §12](../gameplay/RULES_V1.md#12-nền-và-ảnh-ui-của-client-v1).
- **Ảnh có chữ chưa được duyệt.** 8/9 ảnh UI trong atlas có chữ, nên ở trạng thái
  `needs_review_text`: client chỉ hiển thị khi có quyết định duyệt ảnh
  ([IMAGE_REVIEW](../localization/IMAGE_REVIEW.md)), tới lúc đó vẽ text từ catalog. Bundle
  **không** có trạng thái "đã duyệt" nên không thể vô tình hiển thị ảnh chưa duyệt. Claim:
  R-092 (ảnh có chữ trong ảnh, `confirmed`) và R-093 (quyết định V1, `design`) ở
  [RULES_V1 §12](../gameplay/RULES_V1.md#12-nền-và-ảnh-ui-của-client-v1).
- **Bảng UI là quyết định của dự án.** Nó ghi trạng thái dùng và cách thay thế, không
  phải luật chơi; nhãn nguồn/trạng thái theo AGENTS §4 (`kr_static`/`confirmed` cho
  asset có thật, `design`/`design` cho phần tử không dùng asset nào).
- **`content_hash` gắn với byte file atlas/audio.** Đổi bộ mã PNG (đổi phiên bản thư viện
  `png`) thì hash đổi, dù pixel không đổi. Đánh đổi có chủ ý: reader ràng buộc được
  nội dung hiển thị mà không cần giải ảnh. Nếu sau này cần hash **theo pixel thuần**
  (đổi bộ mã PNG không đổi hash) thì reader phải giải PNG và đối chiếu từng vùng RGBA
  với `pixel_sha256` — việc đó cần thêm bộ giải ảnh vào `boom-content` (quyết định kiến
  trúc, thuộc integrator).


## Overlay ảnh UI Việt hóa đã duyệt

[ui-vi-v1.toml](../../../data/assets/ui-vi-v1.toml) chứa quyết định chủ dự án ngày 2026-10-24.
CLI bundle nhận --ui-localization, --vn-pack-dir và --l10n-assets-dir; cách chạy, hash PNG Hòa và
phạm vi nguồn ở [UI_STRINGS_V1 §3.2](../localization/UI_STRINGS_V1.md#32-bundle-ui-việt-hóa-đã-duyệt-2026-10-24).

BuiltBundle::apply_ui_localization kiểm manifest và SHA trước khi thay bundle; lỗi không sửa bản
đang có. VN I16 dùng frame VN, PNG đã biên tập tạo một frame toàn canvas. Mỗi ảnh Việt hóa nằm ở
trang atlas bổ sung có hash file/pixel; source_sha256 của sprite, nguồn/lý do UI và content_hash
được cập nhật rồi validate. Không sửa pack/ảnh KR và không bật ảnh thiếu approval. Layout chỉ chọn
result art tương ứng kết quả người chơi; renderer thu nhỏ đồng tỷ lệ khi ảnh rộng hơn cột UI.
Đây là overlay DESIGN của bản Việt hóa Rust, không phải khẳng định cách engine KR ghép ảnh.
