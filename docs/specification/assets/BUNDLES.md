# Bundle nội dung (atlas ảnh + metadata ô)

Định dạng **output cục bộ** của `boom-assets bundle`: gom ảnh đã giải mã từ pack KR
thành trang atlas PNG, kèm metadata ánh xạ ô của map → sprite/frame, kích thước, canvas
và pivot. Client đọc bundle bằng kiểu ở `crates/boom-content/src/bundle.rs` nên **không
cần bộ giải mã pack** (ARCHITECTURE §7).

Bundle **không commit** vào repo (dữ liệu game có bản quyền); đây là artifact dựng từ
pack, giống `manifest` closure của task #34.

Nhãn nguồn/trạng thái theo [AGENTS §4](../../../AGENTS.md#4-bằng-chứng). Định dạng bundle
là quyết định **design** của dự án (ARCHITECTURE §8 "Định dạng bundle content"), không
phải định dạng của client gốc; phần dữ liệu bên trong thì giữ nguồn gốc của pack KR.

## 1. Dựng bundle

```sh
# thư mục pack lấy từ --pack-dir, hoặc biến môi trường BOOM_KR_PACK_DIR
cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/bundle-village00
```

Kết quả in ra số sprite/ảnh/trang atlas/ô/unknown và `content_hash`.

Nguồn của bundle là **manifest closure** (`crates/boom-assets/src/closure.rs`, task #34)
và **bộ giải mã sprite** `CDataI16Alpha` v2 (`crates/boom-assets/src/sprite.rs`, task #33):

- Entry nào cần có, vai trò, trạng thái bằng chứng, SHA-256 payload → từ closure.
- Ảnh RGBA + frame 48 byte (12 × `i32`) → từ bộ giải mã sprite; nghĩa field frame theo
  [MAPS §3](MAPS.md).

## 2. Cấu trúc thư mục bundle

```text
<bundle>/
  bundle.toml            # metadata (schema 1)
  atlas/page-000.png     # các trang atlas
  atlas/page-001.png     # (chỉ khi cần hơn một trang)
```

## 3. Metadata `bundle.toml` (schema 1)

| Khóa | Kiểu | Ý nghĩa |
|---|---|---|
| `schema` | số | Luôn là `1` |
| `build` | chuỗi | Build của pack nguồn (`kr`) |
| `map` | chuỗi | Tên map (`Village00`) |
| `tileset` | chuỗi | Tile set của map; rỗng nếu map dùng nhiều tile set |
| `width`, `height` | số | Kích thước lưới ô (số cột, số hàng) |
| `spawns` | mảng cặp số | Spawn theo thứ tự file `.map` nguồn |
| `content_hash` | chuỗi | Định danh nội dung, xem §4 |
| `[[atlas]]` | bảng | Một trang atlas: `file`, `width`, `height`, `sha256` (SHA-256 của **file** PNG) |
| `[[sprites]]` | bảng | Một entry ảnh của closure, xem §3.1 |
| `[[assets]]` | bảng | Entry của closure **không phải ảnh** (file map, BGM, bảng chỉ số): `entry`, `role`, `status`, `source_sha256`, `bytes`, `reason` |
| `[[cells]]` | bảng | Ô của map: `x`, `y`, `tile` (ID nền), `object` (ID vật cản, vắng nếu ô trống) |
| `[[unknown]]` | bảng | Phần chưa xác định: `kind`, `id`, `reason`, `candidates` (tuỳ chọn) |

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

**Ảnh rỗng (0×0).** Blob sprite của client gốc có thể khai một ảnh 0×0 làm giữ chỗ cho
frame cuối (`src = (0,0,0,0)`) — ví dụ `FxTile/Tile00/BoxOrange` có 9 frame, ảnh thứ hai
là 0×0. Ảnh rỗng không đặt trong atlas nên `page` vắng mặt; chỉ số ảnh vẫn giữ nguyên số
thứ tự blob để đối chiếu với pack.

## 4. `content_hash`

`content_hash` = SHA-256 của **encoding chuẩn** của metadata, không phải của file TOML:

```text
"boom-bundle-v1\0"
u32 schema
str build, str map, str tileset
u32 width, u32 height
u32 n_spawns × (u32 x, u32 y)
u32 n_atlas  × (str file, u32 width, u32 height)
u32 n_sprites × (
    str id, str entry, str role, str status, str source_sha256,
    i32 map_id (-1 = không có), str object_name ("" = không có),
    u32 n_images × (u32 index, u32 page, u32 x, u32 y, u32 width, u32 height, str pixel_sha256),
    u32 n_frames × (u32 image, i32 offset_x, i32 offset_y, i32 flag,
                    i32 x0, i32 y0, i32 x1, i32 y1,
                    i32 canvas_width, i32 canvas_height, i32 pivot_x, i32 pivot_y))
u32 n_cells   × (u32 x, u32 y, u32 tile, u32 object)
u32 n_assets  × (str entry, str role, str status, str source_sha256, u32 bytes, str reason)
u32 n_unknown × (str kind, str id, str reason, u32 n_candidates × str candidate)
```

- Số là little-endian; chuỗi có tiền tố độ dài `u32` rồi byte UTF-8.
- Giá trị `u32::MAX` đánh dấu "không có" (`map_id` là `i32` nên dùng `-1`; `page`,
  `tile`, `object` dùng `u32::MAX`).
- `content_hash` và `sha256` của file atlas **không** nằm trong encoding (vòng lặp, và
  phụ thuộc bộ mã PNG).
- Hash tính trên **pixel thô** (`pixel_sha256`), nên **đổi bộ mã PNG không làm đổi
  `content_hash`**; đổi ảnh, bố cục atlas hoặc metadata thì có.

Tái lập: cùng pack ⇒ cùng `content_hash`, vì thứ tự sprite theo thứ tự manifest closure,
thứ tự ảnh theo thứ tự blob gốc, và thuật toán đóng gói atlas có thứ tự ảnh cố định
(chiều cao giảm dần, rồi chiều rộng, rồi thứ tự ban đầu).

## 5. Reader và kiểm tra

`boom_content::bundle::load(<thư mục>)` đọc `bundle.toml`, kiểm tham chiếu, kiểm
`content_hash` và kiểm SHA-256 từng file atlas. `Bundle::validate()` kiểm:

1. `schema` đúng; lưới ô đủ `width × height`, không ô nào lặp hoặc nằm ngoài lưới.
2. Mỗi ảnh có trang atlas hợp lệ (hoặc là ảnh rỗng 0×0 không có trang) và không tràn trang.
3. Mỗi frame trỏ tới ảnh có thật; hình chữ nhật nguồn nằm trong ảnh; canvas không nhỏ hơn
   hình nguồn; pivot nằm trong canvas.
4. Mỗi ô có sprite nền và (nếu có) sprite vật cản, **hoặc** ID đó nằm trong `unknown` kèm
   lý do — không ô nào bị bỏ qua âm thầm.
5. Mọi mục `unknown` đều có lý do; mọi `id` của sprite là duy nhất.

`load()` kiểm thêm: file atlas tồn tại, SHA-256 khớp `sha256` ghi trong metadata, và
header PNG (chunk IHDR, có CRC) cho kích thước đúng bằng `width`/`height` của trang.

Đây là kiểm tra *siêu hình*; reader **không** giải ảnh (client giải khi nạp texture) và
không kiểm ngữ nghĩa field frame (`f2` vẫn chưa chốt, xem [MAPS §3](MAPS.md)).

## 6. Đo trên pack KR 00967 (2026-10-19)

Nguồn `kr_static` / `confirmed` cho phần dữ liệu; `design` cho định dạng.

| Đại lượng | Giá trị |
|---|---|
| Map | `Village00` (15×13, 4 spawn) |
| Sprite trong bundle | 53 (5 tile, 7 object, theme, preview, 32 sprite nhân vật, 8 hình bom/lửa/nổ/bong bóng/item) |
| Ảnh | 224 (trong đó vài ảnh giữ chỗ 0×0) |
| Trang atlas | 1 trang 2048×1241 |
| Ô | 195, mọi ô có sprite nền |
| `unknown` | 4 mục `character_state` (state `idle`/`trapped` của C00/C01 chưa có trace consumer) |
| `content_hash` | `bf12715f52e44d2b0ab48a9dc10748584d011e37b6529d93bc23400c23bddcad` |

Cách kiểm lại:

```sh
BOOM_KR_PACK_DIR=/home/admin/boomkr-full \
  cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/b
# content_hash phải bằng giá trị trên; dựng hai lần cho cùng hash và cùng byte file.
```

## 7. Giới hạn đã biết

- **Chỉ map V1.** Closure hiện chỉ dựng được `Village00`; `bundle --map <khác>` báo lỗi
  rõ ràng chứ không đoán.
- **Không nhân bản phân loại ô.** Va chạm/loại ô vẫn ở bảng phân loại `data/maps/*.toml`
  (một thông tin, một nguồn chuẩn — AGENTS §3). Client nạp cả bundle và bảng phân loại.
- **Không có thời lượng frame.** Field `f8..f11` của client gốc **không** phải thời gian
  ([MAPS §3](MAPS.md)); bundle không mang nhịp animation. Nhịp là việc của V1-031 và phải
  là `design` có claim.
- **`f2` chưa chốt.** Ngữ nghĩa field `flag` vẫn chưa biết; bundle giữ nguyên số.
- **Không có BGM hay bảng chỉ số.** Entry không phải ảnh được ghi ở `[[assets]]` kèm lý do
  để không bị bỏ qua âm thầm.
- **Atlas đóng gói bằng thuật toán kệ (shelf).** Không tối ưu chặt chẽ; một trang 2048×2048
  chứa được toàn bộ ảnh Village00. Ảnh vượt trần 2048 px thì báo lỗi thay vì cắt.
- **Chưa có bước kiểm pixel của client.** Bộ giải mã sprite đã khớp từng pixel với tool
  tham chiếu `boom-re-l10n/tools/i16.py` (task #33), nhưng chưa có trace runtime của client
  KR/VN để chứng minh client tiêu thụ `f0..f11` đúng như ta hiểu.
