# Đo tĩnh KR 00967 — item/cap (2026-10-21, cập nhật 2026-10-22)

Metadata cho lượt điều tra [#149](https://github.com/aquaplatinumberlitz/boom-rs/issues/149) / Q-020.
**Chưa tạo claim `kr_static` có consumer**: chưa định vị consumer của hiệu ứng Bomb/Fire/Roller,
trần chỉ số (cap), hay vai trò bảng `ItemAttr` (R-051/R-052). R-150 là **blocker** cho phần
`BomberAttr`/item (xem bổ sung cuối file), không phải claim giá trị gameplay.

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `/home/admin/boomkr-re/out/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Input đọc là **bản sao** `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`. Không mở DB gốc, không chạy client.

## Đã định vị

| Hạng mục | VA | RVA | Ghi chú |
|---|---|---|---|
| Chuỗi đường dẫn pack `Attr/ItemAttr` | `0x0178E7F4` | `0x0138E7F4` | xref duy nhất: `sub_1196A70` |
| **Bộ nạp `ItemAttr`** | `0x01196A70` | `0x00D96A70` | 1.638 B; đọc header 3 dword, rồi bảng item |
| Chuỗi `Item/Bomber%d` | `0x0178DE88` | `0x0138DE88` | tài nguyên hiển thị item, không phải hiệu ứng |
| Chuỗi `Item/BomberSuLuxma` | `0x0178F558` | `0x0138F558` | như trên |

### Cấu trúc bảng đọc được từ `sub_1196A70` (tóm tắt tự viết, không phải nguyên văn)

```
header:  3 × i32
mỗi item: i32 id ; tên ≤ 63 ký tự (copy vào +20, NUL-terminated) ; i32 ; 2 × i32 ; 5 × i32
         bản ghi cách nhau 192 B, nằm ở vùng +244048 của đối tượng quản lý
sau đó:  bảng thứ hai, stride 244 B, nằm ở +28
```

Cấu trúc trên chỉ cho biết bộ nạp đọc bảng thứ nhất (id + tên + 8 dword, stride 192) rồi một bảng
thứ hai stride 244. **Vai trò của cả hai bảng đều `unknown`**: layout `id + tên + 8 dword` **không** loại
trừ việc bảng thứ nhất chứa trường trọng số hoặc quan hệ nhóm, và chưa có consumer đọc các trường này.
Nếu giữ giả thuyết bảng thứ hai liên quan "vai trò bảng" của R-051/R-052 thì đó chỉ là **`kr_static` /
`candidate`** — lý do: loader đọc một bảng riêng stride 244 B ngay sau bảng item, nhưng chưa có consumer
chứng minh vai trò; giả thuyết này **không** dùng để loại trừ vai trò bảng thứ nhất.
Chưa đọc được ngữ nghĩa 8 dword và bảng thứ hai ⇒ **không** claim.

## Lượt thử này không đóng được

| Việc | Trạng thái |
|---|---|
| Consumer hiệu ứng Bomb/Fire/Roller (tăng số bom / tầm / tốc) | **chưa định vị** |
| Trần chỉ số (base/cap, nhặt tại cap) | **chưa định vị** |
| Nghĩa f2–f7 của `BomberAttr` cho C00/C01 (R-042) | **chưa có consumer**; vẫn là ánh xạ theo web (`web`/candidate) |
| Vai trò **cả hai bảng** `ItemAttr` (R-051/R-052) | mới có **cấu trúc loader**, chưa có consumer ⇒ `unknown` |

## Provenance công cụ (ghi rõ)

- **IDA MCP trực tiếp** trên DB **bản sao**: `search_bytes` cho các chuỗi (`Attr/ItemAttr`,
  `Item/Bomber%d`, …) và decompile `sub_1196A70`. Lý do fallback: adapter IDA của REA không khai báo
  `read_bytes`/`search_bytes` (đã ghi ở các lượt trước), và `search_strings` của adapter trả toàn bộ
  bảng chuỗi.
- **REA** (`open_binary` bản sao, `provider_id: ida`) đã mở lại cho đúng hàm đã chọn, nhưng
  `procedure_pseudo_code 0x1196A70` trả **`unreadable_output`** — lỗi cụ thể:
  `IDA MCP returned malformed or unsupported producer data: success: Invalid input: expected true; session: Invalid input: expected object, received undefined`.
  Không retry (không phải lỗi tạm). Vì vậy bằng chứng của lượt này là **IDA trực tiếp**, không phải REA.
- Bundle export trước `close_binary`: `/home/admin/boom-team/tmp/re-kr/evidence-item-149.json`
  (105 B, **0 record** vì lời gọi REA thất bại). `close_binary` → `{"result":null}`.

## Bước tiếp (theo thứ tự)

1. Đọc reader của bảng item: xref tới getter của vùng `+244048` (stride 192) và `+28` (stride 244) để
   lấy ngữ nghĩa 8 dword và bảng thứ hai — trả lời R-051/R-052.
2. Tìm consumer tăng số bom / tầm nổ / tốc độ: đi từ `CGOBomber` (vtable `0x01745FE8`) hoặc từ
   `CBomberAttr` (`0x0134FD40`), rồi đối chiếu với item "Bomb/Fire/Roller".
3. Trần chỉ số: tìm nơi so sánh giá trị hiện tại với max trước khi cộng (thường là `min(cur+1, max)`).
4. Nếu các hàm trên nằm trong `.themida` thì dừng và ghi entry VM theo
   [DEVIRT_METHOD](../../../research/findings/DEVIRT_METHOD.md).

## Bổ sung 2026-10-23 — R-150 blocker: BomberAttr → consumer

REA MCP `provider_id: ida` trên bản sao `kr-full.exe` SHA-256 `cf72da11…e63b`; bundle
`.re-work/evidence-r150-bomberattr-149.json` (20 record, không commit). Không mở DB gốc, không chạy
client/server.

| Kiểm | VA | Kết quả |
|---|---|---|
| `CBomberAttr` ctor | `0x134FD40` | vtable/khởi tạo object; không đọc stat |
| Loader row | `0x13500D0` | đọc `name; f0,f1,f2..f10`; f2/f3 speed, f4/f5 bomb, f6/f7 range (thứ tự + `bomber_attr.json`, R-042 vẫn **candidate**) |
| Kẹp f5 (bomb max) | `0x1350438`–`0x1350450` | `cmp eax,0xA`; nếu >10 ghi 10 — kẹp **bảng**, không phải runtime |
| Singleton | `dword_19D8A90` | chỉ 4 xref: getter `0x559BA0`, lifecycle `0x1354E20`/`0x1355B60`, check `0xFBCD20` |
| Row accessor | `0x1356500` → `0x1351D10` | chỉ đọc `row+408`; 9 caller dùng availability/khởi tạo (`0xB349A0`, `0x6E27B0`), không đọc f2–f7 |
| Item path KR 158 | `0xE5D55E` → `0xE5F450` → `0xE14360` → `0xE9E660` | đọc count/u16/u8; nhánh vị trí/effect; `E9E660` đã đọc **hết** (207 dòng/0x644 B), **không** có `cur+1`/`min(cur+1,max)`/setter stat |
| VN 158 | `sub_706E0A` | chỉ lấy ItemAttr row `244*kind+base+28`, clear pending `rec+16`, không cộng stat (`vn_static`) |
| Roller/Shoe | kind 2 / 3 | không consumer KR; **không** đánh đồng Roller với Shoe |
| Speed | — | không thấy `cấp → px/bước`; R-125 giữ `unknown` |

**Kết luận:** `kr_static` confirmed thứ tự loader + kẹp f5 ≤ 10; **blocker** vì không có consumer
f2–f7 và không có đường tăng/kẹp stat. Đã đọc hết thân `sub_E9E660` (207 dòng/0x644 B); các callee
effect chưa mở hết; chưa có runtime. **Đề xuất profile:
không đổi** — giữ `characters.*` cấp/level (R-117), không đổi sang px.

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_a7b10e81a75c73a095fb6051a1b8b920bea040539d54a3caf8affb5a0c964657` | `batch_decompile` `0x13500D0` | loader/thứ tự/kẹp f5 |
| `ev_6a793b7a68430d9e91f10ca72bb7aacf103d8581bd1ff5b6cd15a8f6f3ad8519` | `batch_decompile` `0x134FD40`/`0x1196A70` | ctor, ItemAttr |
| `ev_1d52291e32045de5b78dea123f1dc11097fac622be61a15b29422eff02f54e7b` | `xrefs` `0x19D8A90` | 4 xref |
| `ev_eb051ef2d534b2541ef7f32da4e1e1ce5783a11768ad8c875446a81750c2bd0d` | `batch_decompile` `0x1356500` | row lookup |
| `ev_ad4a51ff039e6fb8605395e7ecdfdcbd031e9efe6eb2fc427ee226f35d505bb9` | `batch_decompile` `0x1351D10` | `row+408` |
| `ev_68acf8f616736e3cde275314a9e3eb581fb6d5e155a4df919b1cb4ae18917350` | `xrefs` `0x1351D10` | 9 caller availability |
| `ev_0a1951685f0a6bb893535ee7c8077ffbb533ab557fa9816291abcf8766479a30` | `batch_decompile` `0xE5F450` | KR 158 |
| `ev_f8628d13735542ad5390e5587da2fb9032d33849175ed875ad4a5851aa5f5504` | `batch_decompile` `0xE14360` | nhánh item |
| `ev_f8c6effc1e1b414d322e7a193d7ee3f7467efca4b6dd6ac0254bd11846fcacb8` | `procedure_pseudo_code` `0xE9E660` | **đọc hết** 207 dòng/0x644 B; không `cur+1`/`min`/setter stat |
| `ev_61f31d6d657f4f1237d38bf40978dfb09b0a542391817ad8a3ba1da3b478e05b` | `batch_decompile` `0xE9E660` | bản batch cũ |
