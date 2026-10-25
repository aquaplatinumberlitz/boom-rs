# CItemResourceMan — hai bảng và startup alias (metadata)

Fixture `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, base `0x400000`.
Đây là metadata nguồn cho R-186/R-187; không chứa binary, decompile dài, asset, raw log hay pack byte.

## RTTI / typed global

- vtable `C77D70`; RTTI `COL/CD1C24`, signature0, offset0 → `TDE4854C`
  `.?AVCItemResourceMan@@`.
- `E60934` là con trỏ toàn cục startup của manager; không suy kiểu row/ItemAttr group từ tên.

## Block source (chỉ prefix, `complete_function=false`)

| Block | Requested | Retained | Range | Terminal | SHA-256 |
|---|---:|---:|---|---|---|
| 89F436 | 256 | 256 | 89F436..89F536 | null | `c4a4da61f5da587d7ec2775478b4605e5863f0ba309cc02db5094d9f977bf9c1` |
| 89F536 | 160 | 160 | 89F536..89F5D6 | null | `c1e1697b0fb905b6bb30049332a3298bc845697bad66336602aab2c1b5fed0b7` |
| 89F5A1 | 256 | 254 | 89F5A1..89F69F | null | `0ec5c11e53fed407dd5b8d61eeaa23d54ff840520c0c9c4a59deebdf3b905fe6` |

Các block chỉ là instruction prefix; không kết luận function boundary, toàn loader hoặc toàn parse.

## Cấu trúc đã thấy

- Bảng1: count `[this+0x3B94C]`, base inline `this+0x3B9AC`, stride **0xC0=192**; block89F536 có
  `add edi,0xC0`.
- Bảng2: count `[this+0x10]`, base inline `this+0x1C + idx*0xF4`, stride **0xF4=244**; block89F5A1
  có `[this+0x10]`, `imul eax,0xF4`, `lea edi,[eax+ebx+0x1C]`.
- Tên/field trong hai block chỉ được ghi là parser call/offset; chưa gán ItemAttr, group, weight,
  drop hoặc item-vs-ship/model.

## Startup R1 (scoped)

`host-item-manager-alias-149-r1` (DSH2 review: VALID PARTIAL STARTUP):

- probe `9afec8a6f96d742a54a16e9e991296503428d9750fddd95a6c29cd0a9669977d`
- harness `d17856a83990806fd3dac3c9ffb38150e09b172a6e7c562b0599b2616390e909`
- server `ef688ad5588bed2390231728d3ab7e722c3af619e90501bf030eb84cb3a1158b`
- Loader object `0x109CEAE8`, vtable `C77D70`; entry89F3DE, OpenSuccess89F436 EAX1, literal
  `Attr/ItemAttr` tại C77D7C; `E60934=0x109CEAE8` cùng vtable tại Open.
- S191 thiếu nên **không có lateBoot/Boot**; run EXIT2, không rerun. Chỉ khẳng định startup
  loader/global alias; không persistence, whole parse, field/group/drop.

## Unknown giữ nguyên

Nghĩa hai row, quan hệ bảng244 với R-153 group244 và caller89D113 `+0xBC`; field role, ItemAttr
group/weight/drop, native pack identity, lateBoot/persistence. Legacy items.md chỉ là parser/locator;
không dùng tuyên bố weight/drop làm runtime confirmed.

## R-188/R-189 — bảng 244 và caller 89D113

| Quan sát | VN CItemResourceMan | KR R-153 (group) | Kết luận |
|---|---|---|---|
| stride | 0xF4=244 (89F5A1) | 0xF4=244 | cùng stride, chưa đủ identity |
| count | `[this+0x10]` | `+16` | lệch0xC |
| base | inline `this+0x1C` | `+28` | lệch0xC |
| slot field | row `+0x94` ghi cùng `+0xA0/+0xAC` | group `+148`=0x94 slot | trùng offset field, chỉ candidate |
| caller field | `89D113` đọc `manager+idx*0xF4+0xBC`, nonzero→`vtable+0x18`, zero→89CAC8 | chưa có consumer tương ứng | role unknown; không `item.*` |

Không có `item.*`/cap/group reader trong các block/cache đã đọc. `+0xBC` không phải phép +1; không
dùng crosswalk để đổi profile hay gọi ItemAttr group confirmed. Bảng0x8000/weight từ R153 chỉ là
parser/locator, weight≠drop%; caller89D113 chưa nối drop/placement. Không binary/asset/raw log.
