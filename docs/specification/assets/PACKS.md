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
nFrames × 48 byte          frame record (chưa giải)
nImages × (u32 len, RLE data)
```
RLE (`CWriteRLE` 0xA2AC00, color 0xA29F90, alpha 0xA2C970):
- Byte đầu `&0xC1 == 0xC0` → pixel 24-bit BGR, không alpha.
- Byte đầu `&0xC1 == 0xC1` → pixel 24-bit BGR, alpha stream ở `offset = u32 @+1`, màu bắt đầu ở +5.
- Còn lại → pixel 16-bit RGB565, màu trong suốt `0xF81F`.
- Byte điều khiển: `run = b & 0x3F`; bit7 = trong suốt, bit6 = lặp 1 pixel, 00 = pixel liên tiếp.
- Alpha stream cùng kiểu; giá trị là độ trong suốt (alpha thật = 255 − v).

## Kết quả giải nén (`assets/`)

| Loại | Số lượng |
|---|---|
| PNG (từ 20.437 sprite) | 108.726 |
| ogg | 645 |
| map (`.map`, version 1000/1001) | 965 |
| khác (`.bin`: `Attr/*`, script, bảng…) | 512 |

Chưa làm:
- Frame record 48 byte (toạ độ, thời gian animation).
- Cấu trúc `.map`: loader là `CGamePlayMap__Load` (0x7354FD), pseudocode ở `decomp/CGamePlayMap__Load.c`.
- Cấu trúc `Attr/*`: dạng `u32 len + chuỗi` xen số u32, đọc được bằng mắt.
