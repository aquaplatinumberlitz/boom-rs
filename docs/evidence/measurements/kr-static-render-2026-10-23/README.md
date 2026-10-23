# Đo tĩnh render VN→KR — accessor frame, neo tile, lớp Static/Adjust (2026-10-23)

Metadata cho lượt điều tra [#201](https://github.com/aquaplatinumberlitz/boom-rs/issues/201) /
Q-002/Q-003, V1-030/031 (mục 7 hàng đợi RE). Claim:
[RULES_V1 § (đánh số khi merge)](../../../specification/gameplay/RULES_V1.md#-đánh-số-khi-merge--re-kr-00967--render-accessor-frame-neo-tile-và-lớp-staticadjust-r-209r-211)
(R-209…R-211); suy luận/giới hạn/đề xuất ở [KR_RE_RENDER](../../../research/findings/KR_RE_RENDER.md).

## Định danh build

| Build | File | Byte | SHA-256 | Image base |
|---|---|---|---|---|
| VN (client giải nén) | `unpacked_ca.exe` (`/home/admin/boom-re/unpacked_ca.exe`) | 18.887.168 | `13a0a33cccc8f5f5124cdd8dac863b68118e6dda83340649b897fc41faed364a` | `0x00400000` |
| KR `00967` | `kr-full.exe` (bản sao `.re-tmp/`) | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` |

`RVA = VA − 0x400000` cho cả hai. Không commit binary/DB/ảnh.

## Công cụ và provenance

| Bước | Tool | Kết quả |
|---|---|---|
| Đọc VN + KR | IDA MCP (idalib) trên bản sao DB `.i64` | decompile, disasm, xref, quét byte |
| REA trên VN | `open_binary` `provider_id: "ida"` | OK, SHA khớp `13a0a33c…` |
| REA `procedure_pseudo_code` VN | 2 lần | **timeout** (worker cold/DB lớn) ⇒ **không có `evidence_id`** cho lượt này |
| Chạy client/dựng ảnh | không | chỉ tĩnh |

Vì REA timeout, bằng chứng dưới đây là **IDA-decompile + địa chỉ tuyệt đối**, kiểm lại được trên DB
bản sao của hai build; không có `evidence_id` để trích. Đây là giới hạn công cụ, không phải kết luận
từ evidence rỗng.

## Mỏ neo (VA → RVA)

| Vai trò | Build | VA | RVA |
|---|---|---|---|
| `CDataI16Alpha__Load` | VN | `0x00A3D850` | `0x63D850` |
| `vf8` trả con trỏ f4..f11 | VN | `0x008D50DF` | `0x4D50DF` |
| `vf9` rộng rect (f6−f4) | VN | `0x008D50F3` | `0x4D50F3` |
| `vf10` cao rect (f7−f5) | VN | `0x008D510B` | `0x4D510B` |
| `vf11` = f0 | VN | `0x008D5123` | `0x4D5123` |
| `vf12` = f1 | VN | `0x008D5136` | `0x4D5136` |
| `vf13` = f2 | VN | `0x008D514A` | `0x4D514A` |
| `vf14` ghi f0/f1 | VN | `0x008D515E` | `0x4D515E` |
| `vf15` lấy index từ f3 → vf16 | VN | `0x008D5180` | `0x4D5180` |
| `vf16` getter phần tử bảng theo index | VN | `0x008D519B` | `0x4D519B` |
| vtable `CDataI16Alpha` (7 bản) | VN | `0x00C79714`, `0xC79794`, `0xC797E4`, `0xC79834`, `0xC79894`, `0xC79934` | `0x879714`… |
| `CGOTile::vf33` neo tile | VN | `0x005BED20` | `0x1BED20` |
| `CGOTileFixed::vf33` | VN | `0x005BEE9E` | `0x1BEE9E` |
| loader ghép `LayerStatic` | VN | `0x005C7F8E` (+`0x5C8107`), `0x005C9741` (+`0x5C9F66`) | `0x1C7F8E`, `0x1C9741` |
| bảng tên `LayerStatic`/`LayerAdjust` | VN | `0x00C3797C` / `0x00C79760`, refs `0xE48CA0`/`0xE48E1C` | `0x83797C`/`0x879760` |
| ghép `LayerStatic` | KR | `sub_BA3100` (`0xBA3384`), `sub_BB2620` (`0xBB34C5`, `0xBB3545`) | `0x7A3100`, `0x7B2620` |
| chuỗi `LayerStatic` / `LayerAdjust` | KR | `0x17482D8` / `0x17902D8` (bảng `0x19A64B0`/`0x19A64B4`) | `0x13482D8` / `0x13902D8` |
| RTTI `.?AVCDataI16Alpha@@` | KR | `0x19CFD30` | `0x15CFD30` |

## Accessor frame (VN, R-209)

| Method | Công thức | Nghĩa |
|---|---|---|
| `vf8(frame)` | `48*frame + *(this+14) + 16` | con trỏ tới f4 (khối f4..f11) |
| `vf9(frame)` | `[frame+24]−[frame+16]` | f6−f4 (rộng rect) |
| `vf10(frame)` | `[frame+28]−[frame+20]` | f7−f5 (cao rect) |
| `vf11/vf12/vf13` | `[frame+0]/[frame+4]/[frame+8]` | f0/f1/f2 |
| `vf14(frame,a,b)` | ghi `[frame+0]=a`, `[frame+4]=b` | f0/f1 **mutate được** |
| `vf15(frame)` | `vf16([frame+12])` | lấy **index từ f3** rồi gọi `vf16` |
| `vf16(i)` | `*(this[16] + 4*i)` | **getter phần tử bảng theo index**; nghĩa kiểu/handle/kích thước: **unknown** |

## Neo tile (VN, R-210)

`CGOTile::vf33` `0x5BED20`:
`this[19] = 40*a5` (x, a5 = cột), `this[22] = 40*a4 + 39` (y, a4 = hàng). Đơn vị px, ô 40;
mốc 1 px trên mép dưới ô.

## Lớp Static/Adjust (R-211)

- VN loader ghép `strcat(name, "LayerStatic")` tại `0x5C8107`/`0x5C9F66`; bảng tên lớp có
  `LayerStatic` (`0xC3797C`) và `LayerAdjust` (`0xC79760`). **Chưa** chứng minh mỗi object nạp/giữ/vẽ
  đủ cặp sprite.
- KR ghép `LayerStatic` tại `0xBA3384`/`0xBB34C5`/`0xBB3545`; `LayerAdjust` `0x17902D8` chỉ thấy
  trong bảng dữ liệu `0x19A64B4`, **không** có code xref ⇒ consumer `LayerAdjust` KR: **unknown**.
- Chưa có vòng lặp vẽ hai lớp, chưa chứng minh cặp theo object, nên **thứ tự/clip/alpha unknown**.

## Phạm vi đã tìm cho consumer f8–f11 (âm tính có giới hạn)

| Cách tìm | Kết quả |
|---|---|
| 833 site `call [reg+20h]` (vf8) trong VN | không site nào đồng thời đọc `+16/+20/+24/+28` trên con trỏ trả về |
| 70 hàm có `imul reg,30h` (48 B) trong VN | chỉ getter `vf8..vf11`, loader `sub_8D5617`/`sub_8D5463`, UI — không renderer |
| vòng lặp 2 lớp `LayerStatic`/`LayerAdjust` | không thấy |
| KR `CDataI16Alpha` | RTTI còn, **không** xref/vtable trong DB ⇒ chưa xác nhận accessor KR |

Đây là negative **có giới hạn** (theo danh sách trên), không phải chứng minh không tồn tại.

## Giới hạn

- Không có `evidence_id` REA (timeout VN); bằng chứng là IDA-decompile + địa chỉ, kiểm lại được.
- Không chạy client/không dựng ảnh; không đo runtime.
- `f2` chưa có nghĩa; f8–f11 chưa có consumer; cặp sprite theo object, consumer `LayerAdjust`,
  thứ tự/clip/alpha chưa có.
- KR chưa xác nhận accessor tương ứng; chỉ xác nhận **tên + loader** `LayerStatic`, không có code xref
  `LayerAdjust`.
- Không sửa BUNDLES §7/code/profile; đề xuất ở findings §6.
