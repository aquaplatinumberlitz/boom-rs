# Boom Online pack files (.idx / .idd)

Tool: `tools/idf.py` (đọc pack), `tools/i16.py` (giải mã sprite → PNG).

```sh
python3 tools/idf.py ~/Downloads/Boom                        # liệt kê
python3 tools/idf.py extract ~/Downloads/Boom assets [fx ...] # giải nén (PNG + file thô)
```

Toàn bộ đã kiểm chứng trên dữ liệu thật:
- Mọi `.idx` parse vừa khít.
- Tổng offset/size phủ đúng 100% `.idd`.
- Mọi entry mã hoá qua CRC32.
- Mọi entry `_mm` giải nén zlib được.
- Sprite render ra đúng hình.

## Pack

Code: `PackFS__AttachIdf` 0xA388C0, gọi từ 0x8E4A03.

| Pack | Khoá idx | Entry | Ghi chú |
|---|---|---|---|
| `fx` | 1906092848 | 13447 | UI, hiệu ứng, map, âm thanh, bảng thuộc tính |
| `charte` | 95551996 | 3695 | trang phục nhân vật |
| `emote` | 95551996 | 5459 | biểu cảm nhân vật |
| `fxasset` | 95551996 | 11 | |

### `.idx`

Toàn bộ file được giải mã bằng `PackFS__DecryptIdx` 0xA355B0:
```
p[0] = c[0] ^ key;  p[i] = p[i-1] ^ c[i]        (dword LE)
byte lẻ cuối: c ^ (p[last] >> 8*j)
```
Sau khi giải mã:
```
u32 0, u32 version(=15), u32 count
count × { u32 name_len, char name[name_len] (cp949), u32 offset, u32 size, u32 key, u32 crc }
```
(Nếu dword đầu ≠ 0 thì đó là count, và entry không có 2 trường cuối — định dạng cũ.)

### Entry trong `.idd`

`CData__LoadFromPack` 0xA37880:
- `key != 0` → giải mã bằng `PackFS__DecryptEntry` 0xA35640:
  ```
  p[0] = c[0] ^ key ^ 0x9C6C95CE
  p[i] = p[i-1] ^ c[i] ^ (i*i*(i+966)*(i+2020)) ^ 0x9C6C95CE
  ```
  rồi kiểm `CRC32(init=key, plaintext) == crc` (CRC32 giống protocol mạng).
- Tên chứa `_mm` → `u32 comp_len, u32 raw_len, zlib data` (zlib 1.1.4).
- Còn lại: dữ liệu thô.

Entry mã hoá: `Map/*`, `Attr/*` (chỉ số nhân vật, vật phẩm, nhóm map…), khoảng 1441 entry.

## Sprite `CDataI16Alpha` (version 2)

`CDataI16Alpha__Load` 0xA3D850:
```
u32 nFrames, u32 version=2, u32 nImages, u32 ?
nImages × (u32 height, u32 width)
nFrames × 48 byte          frame record (12 × `i32` — nghĩa field ở [MAPS §3](MAPS.md))
nImages × (u32 len, RLE data)
```
RLE (`CWriteRLE` 0xA2AC00, color 0xA29F90, alpha 0xA2C970):
- Byte đầu `&0xC1 == 0xC0` → pixel 24-bit BGR, không alpha.
- Byte đầu `&0xC1 == 0xC1` → pixel 24-bit BGR, alpha stream ở `offset = u32 @+1`, màu bắt đầu ở +5.
- Còn lại → pixel 16-bit RGB565, màu trong suốt `0xF81F`.
- Byte điều khiển: `run = b & 0x3F`; bit7 = trong suốt, bit6 = lặp 1 pixel, 00 = pixel liên tiếp.
- Alpha stream cùng kiểu; giá trị là độ trong suốt (alpha thật = 255 − v).

### Bộ giải mã Rust

`crates/boom-assets/src/sprite.rs` giải blob này (CLI `boom-assets sprite-info` /
`sprite-png`). Viết lại từ mô tả format và tool tham chiếu `boom-re-l10n/tools/i16.py`,
không copy code legacy. Nhãn nguồn/trạng thái theo [AGENTS §4](../../../AGENTS.md#4-bằng-chứng).

Mô tả format ở mục trên (`CDataI16Alpha__Load` 0xA3D850, `CWriteRLE` 0xA2AC00, `CDataI16Alpha`
0xA29F90, alpha 0xA2C970) là phân tích tĩnh **client VN** → `vn_static` / `confirmed`; bộ giải
mã Rust không kết luận mới về format. Dưới đây chỉ liệt kê **quan sát trên dữ liệu KR** và
**giới hạn kỹ thuật tự đặt**, tách riêng.

#### Quan sát trên dữ liệu KR (`kr_static` / `confirmed`)

Phạm vi đã quét (2026-10-19, build `KR-00967`, `/home/admin/boomkr-full`): 13 pack có sprite —
`fx`, `FxCharacter`, `FxTile`, `CharTE`, `EmoTE`, `FxAsset`, `FxContents`, `FxDesktop`,
`FxHiddenCatch`, `FxMap`, `FxMonster`, `FxStage` (`FxSound` không chứa sprite) — **48.898 sprite,
444.181 ảnh** (409.539 ảnh có dữ liệu RLE, 34.642 ảnh rỗng `len == 0`), 563.285 frame.

Cách đo, hai mức khác nhau:

- **Mức header** (toàn bộ 444.181 ảnh): parse header + danh sách ảnh, khối RLE phải khớp đúng
  cuối blob (giống `crates/boom-assets/src/sprite.rs`); đếm chế độ màu theo byte đầu, đo cạnh
  ảnh.
- **Mức RLE** (mẫu **37.646 ảnh**: mỗi entry thứ 10 của 4 pack `fx`, `FxCharacter`, `FxTile`,
  `CharTE`): đọc bằng `boom-re-l10n/tools/i16.py`, kiểm vị trí con trỏ sau luồng màu.

| Claim | Nguồn / trạng thái | Locator / cách đo |
|---|---|---|
| Frame 48 byte là **12 × `i32` có dấu**: `f0`/`f1`/`f2` có giá trị âm trong dữ liệu thật; `f8`,`f9` cũng đọc có dấu (chưa quan sát thấy giá trị âm, chưa có bằng chứng là bất hợp lệ) | `kr_static` / confirmed | 474.120 frame của 4 pack KR (`fx`, `FxCharacter`, `FxTile`, `CharTE`): `f0` ∈ [−165, 560] (254.624 âm), `f1` ∈ [−163, 599] (48.671 âm), `f2` ∈ [−1, 102.722.344] (443.493 âm), `f8` ∈ [1, 800], `f9` ∈ [1, 600]; bảng field ở [MAPS §3](MAPS.md) |
| Ảnh 24-bit BGR không alpha: byte đầu chỉ là marker, luồng màu bắt đầu ở byte 1 | `kr_static` / confirmed | 41.615 ảnh có RLE; byte đầu `& 0xC1 == 0xC0` (mức header) |
| Ảnh 24-bit BGR có alpha: byte đầu marker + `u32` offset alpha; luồng màu bắt đầu ở byte 5, luồng alpha bắt đầu tại offset đó | `kr_static` / confirmed | 6.812 ảnh; byte đầu `& 0xC1 == 0xC1` (mức header) |
| Ảnh 16-bit RGB565: luồng màu bắt đầu ngay byte 0; màu `0xF81F` là pixel trong suốt | `kr_static` / confirmed | 361.112 ảnh; byte đầu còn lại (mức header) |
| Mỗi pixel của ảnh tiêu **một** giá trị alpha, kể cả pixel thuộc run trong suốt của luồng màu | `kr_static` / confirmed | Mức RLE: số byte luồng alpha khớp số pixel; đối chiếu `i16.py` (mục dưới) |
| Control byte có `run == 0` trong luồng alpha trả giá trị 0 (tức alpha 255) và đọc control mới ở pixel kế tiếp | `kr_static` / confirmed | Mức RLE: hành vi run rỗng ở luồng alpha theo `i16.py` |
| Luồng màu trong mẫu ảnh KR đã quét không có run rỗng | `kr_static` / confirmed | Mức RLE: run rỗng ở luồng màu sẽ lệch vị trí con trỏ — mẫu 37.646 ảnh không có ca nào lệch |
| Sau khi giải hết ảnh, phần màu kết thúc đúng tại cuối ảnh (24-bit không alpha) hoặc đúng tại offset alpha (24-bit có alpha); không có byte thừa | `kr_static` / confirmed | Mức RLE: vị trí con trỏ sau luồng màu khớp đúng cuối ảnh / offset alpha (37.646 ảnh, 0 sai) |
| Cạnh ảnh lớn nhất đã quét: rộng **1198** px (`FxDesktop/LobbySquare/MapObject_Img_BigTreeLeaf`, 1198×548), cao **1024** px (`FxStage/Game/GumihoInfectionBoosterEffect`, 131×1024) | `kr_static` / confirmed | Mức header: danh sách kích thước ảnh của 13 pack KR |

#### Giới hạn kỹ thuật tự đặt (`design` / `design`)

| Mục | Nguồn / trạng thái | Ghi chú |
|---|---|---|
| `MAX_IMAGE_SIDE = 4096`: ảnh có cạnh vượt 4096 thì báo lỗi, không cấp phát bộ nhớ | `design` / design | Giới hạn của **bộ giải mã**, không phải quy tắc của client gốc; chọn để blob hỏng không gây cấp phát bộ nhớ vô hạn. Số liệu đi kèm là quan sát `kr_static` ở mục trên, không phải giới hạn do client đặt. Gặp asset vượt trần thì phải nâng và ghi lại nguồn. |

#### Đối chiếu với tool tham chiếu — và giới hạn của phép đối chiếu

RGBA giải ra **khớp từng pixel** với `i16.py` trên `fx/Item/Bomb` và
`FxCharacter/Character/C00-WalkDown_ALayerStatic` (6 ảnh), và trên **374.719 ảnh có dữ liệu
RLE** của 4 pack KR (`fx`, `FxCharacter`, `FxTile`, `CharTE`) — đủ cả ba chế độ màu. Lệnh so:
xem PR task #33.

Giới hạn của phép đối chiếu: `i16.py` là bộ giải mã tham chiếu viết lại từ disasm, **không
phải** client gốc. Hai bộ giải mã khớp nhau chỉ chứng minh chúng đọc cùng một byte theo cùng
cách, **không** chứng minh client gốc tiêu thụ `f0..f11` hay luồng alpha đúng như ta hiểu; task
này không có trace runtime nào của client KR/VN cho sprite. Nghĩa field `f0..f11` vẫn ở mức
bằng chứng của [MAPS §3](MAPS.md).

## Bundle nội dung

`crates/boom-assets/src/bundle.rs` dựng **bundle** cho client từ pack: ảnh đã giải mã
đóng gói thành trang atlas PNG, kèm metadata ô của map (ID tile/obj → sprite/frame, kích
thước, canvas, pivot), **clip animation**, **bảng phần tử UI** và **bảng âm thanh** (BGM + SFX)
cùng `content_hash` ổn định. Kiểu đọc nằm ở
`crates/boom-content/src/bundle.rs` để client dùng được mà không cần bộ giải mã pack.
Định dạng và cách đo: [BUNDLES](BUNDLES.md) (schema hiện hành: 4).

## Kết quả giải nén (`assets/`)

| Loại | Số lượng |
|---|---|
| PNG (từ 20.437 sprite) | 108.726 |
| ogg | 645 |
| map (`.map`, version 1000/1001) | 965 |
| khác (`.bin`: `Attr/*`, script, bảng…) | 512 |

Chưa làm ở bộ giải nén legacy `tools/idf.py` (nguồn chuẩn đã giải ở [MAPS](MAPS.md)):
- Frame record 48 byte: **đã** giải (12 × `i32`: offset, index ảnh, rect, canvas, pivot) ở
  [MAPS §3](MAPS.md); pack **không** mang thời gian animation (R-079) — bộ giải nén legacy
  không in ra các field này.
- Cấu trúc `.map`: **đã** giải ở [MAPS §1](MAPS.md) (loader `CGamePlayMap__Load` 0x7354FD,
  pseudocode ở `decomp/CGamePlayMap__Load.c`); bộ giải nén legacy chỉ xuất file thô.
- Cấu trúc `Attr/*`: dạng `u32 len + chuỗi` xen số u32, đọc được bằng mắt.
