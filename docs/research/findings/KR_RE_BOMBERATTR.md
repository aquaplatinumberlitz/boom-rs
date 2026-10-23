# KR 00967 — BomberAttr → Bomb/Fire/Roller/speed (R-150, blocker)

Lượt RE [#149](https://github.com/aquaplatinumberlitz/boom-rs/issues/149) / Q-020. Claim **R-150** (dải phiên 1
R-150…R-199), tiếp sau
[evidence item](../../evidence/measurements/kr-static-item-2026-10-21/README.md).
Mục tiêu: ánh xạ f2–f7 của `BomberAttr` vào base/max có **consumer**, đường nhặt Bomb/Fire/Roller
(tăng, kẹp trần, nhặt khi đã trần), và consumer tốc độ (mở R-125). Không đánh đồng Roller với Shoe.

## 10.1 Build / phạm vi

- Build KR `00967`, bản sao `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`, SHA-256
  `cf72da11…e63b`; image base `0x400000`; REA MCP `provider_id: ida`.
- VN đối chiếu bằng IDB bản sao `.re-work/vn/unpacked_ca.exe.i64` (không mở DB gốc).
- Đã đọc: `CBomberAttr` ctor `0x134FD40`, loader `0x13500D0`, row/accessor `0x1351D10`/`0x1356500`,
  singleton `dword_19D8A90`, các consumer `0xB349A0`/`0x6E27B0`; item path KR 158 `0xE5D55E` →
  `sub_E5F450` → `sub_E14360` → `sub_E9E660`, và VN `sub_706E0A`.
- Không chạy client/server; không mở DB gốc. Chưa thấy `.themida` trên các đường này.

## 10.2 Loader `0x13500D0`: thứ tự trường

Loader đọc từng row theo thứ tự:

```
name (độ dài + byte)
f0, f1
f2, f3, f4, f5, f6, f7, f8, f9, f10
```

Theo thứ tự này và `boomkr-re/data/kr/bomber_attr.json` (`field_map`: `2-3` speed, `4-5` bomb,
`6-7` blast), có:

| Cặp | Vị trí đọc | Nghĩa **candidate** (R-042) |
|---|---|---|
| f2/f3 | sau f0/f1 | speed min/max (cấp) |
| f4/f5 | kế tiếp | bomb min/max (cấp) |
| f6/f7 | kế tiếp | range min/max (cấp) |

`f5` (bomb max) bị **kẹp ≤ 10** ngay khi nạp: `cmp eax,0xA` tại `0x1350443`, nếu lớn hơn thì
`push 0xA` và ghi lại (`0x1350448`–`0x1350450`). Đây là **kẹp bảng**, không phải kẹp runtime khi
nhặt item. f8–f10 được đọc tiếp (f8 = 10, f9 = 100, f10 thay đổi) nhưng không nằm trong câu hỏi.

## 10.3 Consumer: không tìm thấy cho f2–f7

- Singleton `dword_19D8A90` (CBomberAttr) chỉ có **4 xref**: getter `0x559BA0`, lifecycle
  `0x1354E20`/`0x1355B60`, và check `0xFBCD20` — không có consumer stat.
- `sub_1356500` (row lookup) chỉ có **2 caller**: loader `0x13500D0` và `0x1351D10`.
- `0x1351D10` chỉ đọc field **`row+408`** (`sub_1356500(...)+408`); các caller của nó
  (`0x6E27B0`, `0x853A70`, `0xB349A0`, `0xDBFAC0`, `0xFB6920`, `0xFB8250`, `0x11499A0`,
  `0x1211140`, `0x12DA350`) dùng giá trị này như **availability/khởi tạo character** (ví dụ
  `0xB349A0` chọn nhánh theo giá trị; `0x6E27B0` lặp 0x2000 để lập bảng), **không** đọc f2–f7.
- Vì vậy **không có consumer** cho f2–f7 trong phạm vi đã quét; không được dùng bảng này làm
  base/max cho Bomb/Fire/Roller hay speed. R-042 vẫn là `candidate` (nghĩa trường), R-117 vẫn giữ
  cấp (level), không đổi sang px.

## 10.4 Item path Bomb/Fire/Roller

- KR opcode 158 → `0xE5D55E` → `sub_E5F450`: đọc `u8 count`, mỗi item `u16`, `u8`, rồi gọi
  `sub_E14360`.
- `sub_E14360`: resolve object theo `u16`; nhánh `a3==1` đặt vị trí `(40*x, 40*y+39)` qua slot
  `+116`; nhánh `a3==0` gọi `sub_E9E660(v28, a4)`. Không thấy phép `cur+1`/`min(cur+1,max)` cho
  bomb/range/speed trong thân đã đọc.
- `sub_E9E660` đã đọc **hết** (0x644 B, 207 dòng; `ev_f8c6effc…`): ghi
  `*(this+176)=sub_B37E50(a2)`, tính thời lượng/effect, loop tile (`sub_E9F9B0`, `B323D0`,
  `B72400`, `FF6930`), rồi `EB8000`/`EB7A60`/`EB9840` và virtual `+104`/`+80`. **Không có**
  `cur+1`, `min(cur+1,max)`, `sub_55FD30` setter stat hay đọc f2–f7. Các callee effect chưa mở
  hết, nên đây vẫn là blocker có phạm vi, không phải “toàn bộ đường item không cộng”.
- VN 158 `CGameStage__On_158` gọi `sub_706E0A(slot,kind)`; `sub_706E0A` chỉ lấy row ItemAttr
  (`244*kind + base + 28`), kiểm `rec+16==1`/`row+160>0` rồi **clear pending** — không cộng stat.
  Đây là `vn_static`, không phải `vn_runtime`; chưa dùng làm consumer KR.

## 10.5 Roller / Shoe / speed

- `ItemAttr` kind: 0 Bomb, 1 Fire, 2 Roller, 3 Shoe (VN `docs/items.md`, `vn_static`). Không có
  consumer KR cho kind 2/3 trong phạm vi này ⇒ **không** đánh đồng Roller với Shoe, không gán
  “Roller = tốc độ/Shoe”.
- Không tìm thấy consumer `cấp tốc độ → px/bước`; R-125 giữ `unknown`. Không đổi
  `characters.c00_speed`/`c01_speed` hay bất kỳ giá trị profile nào.

## 10.6 Evidence

Bundle `.re-work/evidence-r150-bomberattr-149.json` (20 record, không commit).

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_a7b10e81a75c73a095fb6051a1b8b920bea040539d54a3caf8affb5a0c964657` | `batch_decompile` `0x13569A0`/`0x13568AB`/`0x13500D0`/`0x13555E0` | loader, thứ tự trường, kẹp f5 |
| `ev_6a793b7a68430d9e91f10ca72bb7aacf103d8581bd1ff5b6cd15a8f6f3ad8519` | `batch_decompile` `0x134FD40`/`0x1196A70` | `CBomberAttr` ctor, ItemAttr loader |
| `ev_545d2e06d89fc84fc69a4fd0b8cc1822758b890911ad5c0404c49ed938edd9f0` | `batch_decompile` `0x559BA0`/`0x1353EB0`/`0x1354B10`/`0x1354B20` | singleton getter |
| `ev_1d52291e32045de5b78dea123f1dc11097fac622be61a15b29422eff02f54e7b` | `xrefs` `0x19D8A90` | 4 xref, không consumer stat |
| `ev_eb051ef2d534b2541ef7f32da4e1e1ce5783a11768ad8c875446a81750c2bd0d` | `batch_decompile` `0x1352E20`/`0x1352E90`/`0x1356500` | row lookup |
| `ev_ad4a51ff039e6fb8605395e7ecdfdcbd031e9efe6eb2fc427ee226f35d505bb9` | `batch_decompile` `0x1351D10`/`0x13557D0`/`0x1355A30` | accessor `row+408` |
| `ev_68acf8f616736e3cde275314a9e3eb581fb6d5e155a4df919b1cb4ae18917350` | `xrefs` `0x1351D10` | 9 caller availability/init |
| `ev_3f642ac880d4479466459a8a32b8a287d8475561b29f48982657acd896a1c52c` | `batch_decompile` `0xB349A0`/`0x6E27B0` | dùng `row+408` như availability |
| `ev_0a1951685f0a6bb893535ee7c8077ffbb533ab557fa9816291abcf8766479a30` | `batch_decompile` `0xE5F450` | KR 158 handler |
| `ev_f8628d13735542ad5390e5587da2fb9032d33849175ed875ad4a5851aa5f5504` | `batch_decompile` `0xE14360` | nhánh vị trí/effect |
| `ev_f8c6effc1e1b414d322e7a193d7ee3f7467efca4b6dd6ac0254bd11846fcacb8` | `procedure_pseudo_code` `0xE9E660` | **đọc hết** 207 dòng/0x644 B; không `cur+1`/`min`/setter stat |
| `ev_61f31d6d657f4f1237d38bf40978dfb09b0a542391817ad8a3ba1da3b478e05b` | `batch_decompile` `0xE9E660` | bản batch cũ (đối chiếu) |
| `ev_debd4e7bb5e3432e67388c9c25cf84f8ed108f81b6d8042cf6ea333e91cdb801` | `batch_decompile` `0xE5F2C0` | opcode 164 (đối chứng, không phải pickup) |
| `ev_54c04bf2107fea29663d77c9dbce0b53d65b03126231404ad481c544651ce806` | `batch_decompile` `0xB79DC0`/`0xB73410` | bomber ctor, không đọc f2–f7 |

## 10.7 Kết luận / bước tiếp

- `kr_static` **confirmed**: loader thứ tự row và kẹp f5 ≤ 10.
- **Không tạo claim riêng cho `E9E660`**: đã đọc hết nhưng không có consumer/`cur+1`/`min`; R-150 giữ blocker.
- **Blocker**: không có consumer f2–f7; không có đường +bomb/+range/+speed, kẹp runtime, nhặt khi
  trần, Roller/Shoe, hay speed→px/bước trong phạm vi đã đọc. `E9E660` đã đọc hết và không có phép
  cộng; các callee effect chưa mở hết.
- Bước tiếp: (1) mở các callee effect (`EB8000`, `EB7A60`, virtual `+104`) và ItemAttr
  (`row+160/184`); (2) tìm hàm set stat của `CGOBomber` (không chỉ ctor) và so `cur/max`; (3) nếu
  gặp `.themida`, dừng + ghi entry VM; (4) cần runtime/`read_bytes` để kiểm nhặt tại trần.
- **Đề xuất profile: không đổi.** `characters.*` giữ cấp/level (R-117); nếu tìm được consumer
  base/max mới mở PR profile riêng theo ADR 0013.

## R-153 — ItemAttr reader và bảng trọng số (2026-10-23)

Tiếp nối #149/Q-007/Q-020, V1-016. Lượt này chỉ đọc `CItemResourceMan`/ItemAttr; **không lặp**
`CBomberAttr`/`E9E660` của R-150. Build KR `00967`, REA MCP bản sao `kr-full.exe`
SHA-256 `cf72da11…e63b`.

### Loader và hai bảng

`sub_1196A70` (VA `0x01196A70`, RVA `0x00D96A70`, segment `.priv`) là loader native (197 dòng,
0x666 B), caller `sub_1231BD0` tại `0x123292B`. Nó đọc `Attr/ItemAttr` và dựng:

| Bảng | Base | Stride | Count | Row đã thấy |
|---|---|---|---|---|
| Item (`CItemResourceMan`) | `+244048` (`0x3B950`) | `0xC0` = 192 | `+244036` (`0x3B944`) | `+8` key, `+20` tên (≤63), `+84` (`0x54`) **item id**, `+96..+180` 8 dword |
| Group | `+28` (`0x1C`) | `0xF4` = 244 | `+16` (`0x10`) | `+8` container, `+20`/`+84` tên, `+148` (`0x94`) **số slot**, `+96/+108/+120`, `+184..+232` 5 dword |

- `sub_1197900` tìm row item theo **`+84`**: `for i < count(+244036): if sub_57A870(row_i + 84) == id return row_i`; không thấy thì trả row 0.
- Xref của lookup có **7 hàm**: `0xE84CD0`, `0x6B3FA0`, `0xB978B0`, `0xB98250`, `0xFFFA70`, `0x1191DD0`,
  `0x1192700`. **Chỉ 4 thân đã đọc**: `E84CD0` đọc byte user-info/character rồi tạo model; `6B3FA0`,
  `B978B0`, `B98250` tạo/đọc item model/UI từ id. Trong 4 thân này **không** thấy đọc weight/placement.
  **3 thân còn lại** (`FFFA70`, `1191DD0`, `1192700`) và tác dụng bắc cầu của helper/virtual vẫn **unknown**;
  xref 7 hàm không phải 7 thân đã đọc.
- `B479E0` gọi `B978B0(this+513, id, 0)` — item model gắn vào bomber (slot/held); **chưa thấy** đường map drop.

### Bảng trọng số 0x8000

Loader cấp `0x8000` byte tại `+436048` (`0x6A750`), fill `0xFF`, rồi với từng group:

```
offset = 0
for group in groups:
    n = group[+148]                  // số slot
    for i in offset .. offset+n-1:
        table[i] = group_index
    offset += n
    if offset > 0x7FFF: error
```

- **Không chia tổng**, không nhân 100, không dùng `weight/sum`; đây là **bảng chỉ số group**
  (slot count), không phải phần trăm. Nếu tổng < 0x8000, phần còn lại giữ `0xFF`.
- Quét **literal displacement** `+0x6A750` trên disassembly: chỉ **2** tham chiếu, **cả hai trong loader**
  (khởi tạo/ghi). **Chưa thấy reader trong phạm vi quét literal này**; không loại trừ alias/base đã
  điều chỉnh, địa chỉ tính toán, helper, indirect hay VM.
- Vì vậy không chứng minh client chọn item/drop; bảng có thể để dành cho VM/server/feature khác.
  `weight` không phải `%`; `placement` không phải xác suất gạch.

### Kết luận / giới hạn

- `kr_static` **confirmed**: loader native, stride/count, item id `+84`, lookup `1197900`, và việc
  loader fill bảng 0x8000. **Không** confirmed sự vắng mặt reader toàn bộ.
- `candidate`: vai trò group table và 6 tên đặc biệt `off_19A6354` (một tên `LucciPocketGold`).
- `unknown`: reader ngoài phạm vi literal, 3 thân caller chưa đọc và bắc cầu helper/virtual; client
  chọn drop hay chỉ hiển thị placement server; nghĩa 8 dword item `+96..+180`; cap nhặt; xác suất 35%
  (không suy từ weight). Không đọc thêm 35% server.

### Evidence

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_fbc144c7328649f1257d84f59211000a8f77c9e09d3ef83c6addfbfcb375e827` | `procedure_pseudo_code` `1196A70` | loader 197 dòng |
| `ev_180b89234db3ee43884ec244ad85aa9149fa01e8a592de2ccf70a45181a063b5` | `procedure_assembly` `1196A70` | offset/stride/field |
| `ev_180777980cc9e987726f3bde0ac105722f37b04e90af024cd3fccefe00bbf096` | `batch_decompile` `1194A00`/`1196830` | ctor, container stride 192/244 |
| `ev_5443f41efdd99dd89fbdf4a21fb353de409446fb26eaa645e7ebc18e163af082` | `batch_decompile` `1197340`/`1197C40` | group name→index, decrypt |
| `ev_584dffb25e7eb71990bb6f3e50663c6c29ee83765161796cb8523af156559181` | `batch_decompile` `1196960`/`11970E0`/`1197900` | lookup id `+84` |
| `ev_04618990e2e75ab6277b4cf65fa2d0d01c245685b1374e711c332e9f4707ec95` | `batch_decompile` `E84CD0`/`B978B0`/`B98250`/`6B3FA0` | consumer model/UI |
| `ev_563814aa2dd0ce9050c19671db0233f91fd6decff136c9c1c7b504a9c82e08f0` | `batch_decompile` `B479E0`/`B4C9D0` | item model bomber |
| `ev_be2b7b9e7c911bab723341f747844a7fd1576ef3793f088b1118fba1b43d7682` | `xrefs` `1197900` | 7 xref; chỉ 4 thân đã đọc |
| `ev_21c2be2fcc6cc7f66194b179dc3f4baa59c4845e26a91db432e97cf46870f4e6` | `xrefs` `1196A70` | caller `123292B` |
| `ev_3234408243c47bec6d8034fb6a1f347331e1673e45bd46527a127ec24dc8562c` | `xrefs` `B978B0` | 2 caller model |
