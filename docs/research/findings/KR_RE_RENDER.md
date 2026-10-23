# RE tĩnh VN→KR — render: accessor frame, neo tile, lớp Static/Adjust

Kết quả đọc tĩnh cho [#201](https://github.com/aquaplatinumberlitz/boom-rs/issues/201) / Q-002/Q-003,
V1-030/031 (mục 7 hàng đợi RE). Claim ở
[RULES_V1 § (đánh số khi merge)](../../specification/gameplay/RULES_V1.md#-đánh-số-khi-merge--re-kr-00967--render-accessor-frame-neo-tile-và-lớp-staticadjust-r-209r-211)
(R-209…R-211; dải phiên R-209…R-249). Định danh build, VA/RVA và giới hạn công cụ ở
[evidence KR render](../../evidence/measurements/kr-static-render-2026-10-23/README.md).

**Kết luận chính:** ở client VN, lớp `CDataI16Alpha` có **accessor** cho frame 48 B: `vf8` trả con trỏ
tới khối **f4..f11**, `vf9`/`vf10` là rộng/cao rect, `vf11`/`vf12`/`vf13` là **f0/f1/f2**, `vf14` **ghi
f0/f1**, `vf15` lấy **index từ `f3`** rồi gọi `vf16` (getter phần tử bảng theo index; nghĩa
kiểu/handle/kích thước **chưa chứng minh**). Neo tile/object là `x = cột·40`, `y = hàng·40 + 39` (px).
Bảng tên lớp có `<tên>LayerStatic`/`<tên>LayerAdjust`; loader VN và KR đều **ghép `LayerStatic`** — **chưa**
chứng minh mỗi object nạp/giữ/vẽ đủ cặp sprite, và KR **chưa** có code xref `LayerAdjust`. **Chưa**
tìm được hàm vẽ dùng `f8`–`f11`, nên **công thức đặt ảnh so vị trí thế giới, thứ tự Static/Adjust,
clip và alpha vẫn `unknown`** — blocker có phạm vi, không đủ cơ sở đổi BUNDLES §7.

## 1. Phạm vi, build, phương pháp

| Mục | VN | KR |
|---|---|---|
| File | `unpacked_ca.exe` | `kr-full.exe` (bản sao `.re-tmp/`) |
| SHA-256 | `13a0a33cccc8f5f5124cdd8dac863b68118e6dda83340649b897fc41faed364a` | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` |
| Image base | `0x00400000` | `0x00400000` |
| Byte | 18.887.168 | 26.074.624 |

Đường chính: IDA MCP (idalib) trên **bản sao** DB `.i64` của hai build; decompile + đọc disasm + xref.
REA MCP đã thử trên VN (`open_binary` OK, SHA khớp) nhưng `procedure_pseudo_code` **timeout** hai lần
(DB VN lớn, worker cold) nên **không có `evidence_id`** cho lượt này; mọi kết luận dưới đây là
IDA-decompile với địa chỉ tuyệt đối, ghi rõ trong evidence. Không chạy client, không dựng ảnh.

## 2. `CDataI16Alpha` — accessor frame (VN, R-209)

`CDataI16Alpha__Load` `0x00A3D850` (đã có trong PACKS §Sprite). Các method của lớp (vtable
`0xC79714`…, slot 8..16) cho thấy layout truy cập:

| Method | Địa chỉ | Công thức (decompile) | Nghĩa suy ra |
|---|---|---|---|
| `vf8(frame)` | `0x008D50DF` | `48*frame + *(this+14) + 16` | con trỏ tới **f4**; khối f4..f11 liên tiếp |
| `vf9(frame)` | `0x008D50F3` | `[frame+24] − [frame+16]` | `f6 − f4` = **rộng rect** |
| `vf10(frame)` | `0x008D510B` | `[frame+28] − [frame+20]` | `f7 − f5` = **cao rect** |
| `vf11(frame)` | `0x008D5123` | `[frame+0]` | **f0** |
| `vf12(frame)` | `0x008D5136` | `[frame+4]` | **f1** |
| `vf13(frame)` | `0x008D514A` | `[frame+8]` | **f2** (trả nguyên, không diễn giải) |
| `vf14(frame,a,b)` | `0x008D515E` | `[frame+0] = a; [frame+4] = b` | **ghi f0/f1** — registration thay đổi được |
| `vf15(frame)` | `0x008D5180` | `(*this+64)(this, [frame+12])` | lấy **index từ f3** rồi gọi `vf16` |
| `vf16(index)` | `0x008D519B` | `*(this[16] + 4·index)` | **getter phần tử bảng theo index**; nghĩa kiểu/handle/kích thước: **unknown** |

vtable của lớp có 7 bản (alpha/character/item/…) cùng trỏ các method trên (xref `0xC79714`,
`0xC79794`, `0xC797E4`, `0xC79834`, `0xC79894`, `0xC79934`), nên đây là **lớp dữ liệu dùng chung**,
không phải renderer.

**Điểm chưa có:** không method nào trả `f8/f9/f10/f11`; renderer phải đọc chúng từ con trỏ `vf8`
(`p[4..7]` = f8..f11) hoặc đọc trực tiếp frame. Lượt này **không tìm được** hàm như vậy (xem §5).

## 3. Neo tile/object (VN, R-210)

`CGOTile::vf33` `0x005BED20` (slot 33 vtable `CGOTile` `0xC36A00`-ish):

```
*(this + 36) = a2;
(*(*this + 20))(this, a3, -1);
sub_413F32(this + 19, 40 * a5);      // this[19] = 40 * cột
sub_413F32(this + 22, 40 * a4 + 39); // this[22] = 40 * hàng + 39
*(this + 37) = a7; *(this + 38) = a6;
(*(*this + 12))(this, 1);
```

`CGOTileFixed__vf33` `0x005BEE9E` gọi lại hàm trên rồi lưu thêm hàng/cột (`this[39]/[40]`). Đơn vị
**px**, ô 40; mốc `y` là **1 px trên mép dưới ô** (khớp R-009 cho nhân vật: `x=cột·40`,
`y=hàng·40+39`). Đây là **neo thế giới**; chưa nối tới phép cộng f0/f1 hoặc canvas f8/f9.

## 4. Lớp `LayerStatic`/`LayerAdjust` (R-211)

- VN: loader map/object `sub_5C7F8E` (`0x5C8107`) và `sub_5C9741` (`0x5C9F66`) ghép tên
  `<Destination> + "LayerStatic"` bằng `strcat` trước khi nạp sprite. Bảng tên lớp ở `0xE48CA0`
  trỏ `"LayerStatic"` (`0xC3797C`), `0xE48E1C` trỏ `"LayerAdjust"` (`0xC79760`) — **bảng tên lớp có cả
  hai tên**; chưa chứng minh object nạp/giữ/vẽ đủ cặp.
- KR: `sub_BA3100` (`0xBA3384`) và `sub_BB2620` (`0xBB34C5`, `0xBB3545`) cũng ghép `LayerStatic`;
  chuỗi `LayerAdjust` KR `0x17902D8` chỉ thấy trong bảng dữ liệu `0x19A64B4`, **không** có code xref
  trực tiếp ⇒ **consumer `LayerAdjust` của KR: unknown**. Cơ chế cặp sprite theo từng object là
  `candidate`, không phải confirmed.
- **Chưa chứng minh** thứ tự vẽ (`Static` trước `Adjust`?), nơi clip theo viewport và cách áp alpha
  (surface alpha/colorkey `0xF81F` hay alpha stream). Không suy từ tên.

## 5. Phần chưa nối (blocker có phạm vi)

Đã thử các hướng sau, ghi lại để người sau không lặp:

1. **Gọi virtual slot 0x20 (vf8)**: quét 833 site `call [reg+20h]` trong VN; không site nào vừa lấy
   con trỏ f4..f11 vừa đọc `+16/+20/+24/+28` (f8..f11) trong cùng hàm. Các ứng viên lớn thuộc UI/D3D,
   không phải draw map object.
2. **Tìm theo `imul 48`**: 70 hàm VN có `imul …, 30h`; chỉ gồm getter `vf8..vf11`, loader
   `sub_8D5617`/`sub_8D5463` (đọc blob), và UI; **không** có renderer.
3. **`Static`/`Adjust`**: tên lớp tìm được, nhưng **không** tìm được vòng lặp vẽ hai lớp; code
   `vf148`/`vf116` của tile chỉ là state/logic (ví dụ `CGOTilePassable__vf21`).
4. **KR `CDataI16Alpha`**: RTTI `.?AVCDataI16Alpha@@` `0x19CFD30` còn, nhưng **không có xref/vtable**
   trong DB KR (RTTI stripped/không dùng), nên chưa xác nhận accessor KR tương ứng. KR có
   tên/loader `<name>LayerStatic` (như §4) nhưng **không** có code xref `LayerAdjust` và chưa có hàm vẽ dùng f8–f11.

Hệ quả: câu hỏi hẹp của #201 (công thức f0/f1/f2/f8…f11 so vị trí thế giới, thứ tự Static/Adjust,
clip/alpha) **chưa trả lời được**; Q-003 giữ `partial`. Không đổi BUNDLES §7 hay code.

## 6. Đề xuất cho BUNDLES §7 (chỉ đề xuất, không sửa)

1. Giữ nguyên công thức `design` hiện tại (canvas canh đáy, pivot_x trục trái) — **chưa có** bằng chứng
   consumer; đừng nâng thành `confirmed`.
2. Ghi thêm dữ kiện đã kiểm: neo tile `(cột·40, hàng·40+39)`; f0/f1 **mutate được** qua `vf14` nên
   không nên coi là hằng số sprite; bảng tên lớp có `LayerStatic`/`LayerAdjust` và loader ghép
   `LayerStatic`, nhưng **cặp sprite theo object chưa chứng minh**.
3. Nếu cần chốt: bước tiếp là tìm **draw list/render pass** của map object (nơi gọi surface blit với
   screen rect + clip), hoặc đo runtime theo Q-009; khi có thì sửa BUNDLES + client theo task riêng.

## 7. Giới hạn

- Chỉ tĩnh, không chạy client, không dựng ảnh; không có `evidence_id` REA cho lượt này (timeout VN,
  xem evidence README).
- Accessor VN đọc trên `unpacked_ca.exe`; KR **chưa** xác nhận accessor tương ứng (RTTI còn nhưng
  không có consumer/vtable xref).
- `f2` mới chỉ biết là `i32` trả nguyên; chưa có nghĩa.
- `Static`/`Adjust`: mới xác nhận **tên lớp + loader ghép `LayerStatic`**; cặp sprite theo object,
  code xref `LayerAdjust`, thứ tự vẽ, clip/alpha **unknown**.
- Không kết luận gì về BUNDLES/design hiện tại của V1.

## 8. Bước tiếp

1. Tìm render pass của map object trong VN: hàm duyệt danh sách object/tile và gọi surface blit;
   kiểm xem nó đọc f8–f11 từ `vf8` hay copy frame; từ đó lấy công thức + clip.
2. Kiểm thứ tự hai lớp bằng cách tìm nơi lưu 2 sprite (`LayerStatic`/`LayerAdjust`) và vòng lặp vẽ.
3. Nếu không tìm được tĩnh: đo runtime theo Q-009 (chụp màn hình/ghi blit) — chỉ khi có mới sửa
   BUNDLES §7.
