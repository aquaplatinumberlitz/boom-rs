# Đo tĩnh KR 00967 — item/cap (2026-10-21, cập nhật 2026-10-22)

Metadata cho lượt điều tra [#149](https://github.com/aquaplatinumberlitz/boom-rs/issues/149) / Q-020.
**Chưa tạo claim `kr_static` mới**: chưa định vị consumer của hiệu ứng Bomb/Fire/Roller, trần chỉ số
(cap), hay vai trò bảng trọng số `ItemAttr` (R-051/R-052). Ghi lại mỏ neo + giới hạn + bước tiếp.

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

Hệ quả có thể dùng ngay: `ItemAttr` là **bảng theo từng item** (id + tên + 8 dword), không phải bảng
trọng số theo nhóm; bảng thứ hai (stride 244) mới là ứng viên cho phần "vai trò bảng" của R-051/R-052.
Chưa đọc được ngữ nghĩa 8 dword và bảng thứ hai ⇒ **không** claim.

## Lượt thử này không đóng được

| Việc | Trạng thái |
|---|---|
| Consumer hiệu ứng Bomb/Fire/Roller (tăng số bom / tầm / tốc) | **chưa định vị** |
| Trần chỉ số (base/cap, nhặt tại cap) | **chưa định vị** |
| Nghĩa f2–f7 của `BomberAttr` cho C00/C01 (R-042) | **chưa có consumer**; vẫn là ánh xạ theo web (`web`/candidate) |
| Vai trò bảng `ItemAttr` (R-051/R-052) | mới có **cấu trúc bảng**, chưa có consumer ⇒ `unknown` |

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
