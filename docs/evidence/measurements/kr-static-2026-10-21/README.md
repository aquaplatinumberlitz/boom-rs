# Đo tĩnh KR 00967 — movement/va chạm (2026-10-21)

Metadata của phép đọc tĩnh dùng cho claim `kr_static` R-119…R-125
([RULES_V1 §12](../../../specification/gameplay/RULES_V1.md#12-re-kr-00967--movement-và-va-chạm-r-119r-125)).
Không chứa binary, decompile nguyên văn hay dữ liệu pack. Kết luận và cách suy luận nằm ở
[KR_RE_MOVEMENT](../../../research/findings/KR_RE_MOVEMENT.md).

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `/home/admin/boomkr-re/out/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |
| VN `VBNB_116` (đối chiếu) | `/home/admin/boom-re/unpacked_ca.exe` | — | `13a0a33cccc8f5f5124cdd8dac863b68118e6dda83340649b897fc41faed364a` | `0x00400000` | `VA − 0x00400000` |

Image base lấy từ database (`0x400000`); `.text` bắt đầu `0x401000`. Mọi địa chỉ dưới đây là VA trong
không gian module đã unpack, tức `RVA = VA − 0x400000`.

Hai database IDA được đọc là **bản sao**, không phải DB gốc:

- `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe.i64`
- `/home/admin/boom-team/tmp/re-kr/ida/unpacked_ca.exe.i64`

Không mở `.i64` gốc của `boomkr-re`/`boom-re` bằng công cụ có thể ghi. Không chạy client, không kết nối
server, không đụng `.themida`.

## Kết quả tra cứu (VA → RVA)

| Vai trò | KR VA | KR RVA | VN VA | Cơ sở khớp |
|---|---|---|---|---|
| `CGameContext::Init` | `0x00E0EEC0` | `0x00A0EEC0` | `0x006ED78A` | chuỗi trace nhúng `CGameContext::Init` |
| `CGameContext::MakeBombers` | `0x00E12A30` | `0x00A12A30` | `0x006EB9A9` | chuỗi trace nhúng `CGameContext::MakeBombers` |
| Input dispatch của context (vf +0x20) | `0x00E13D00` | `0x00A13D00` | `0x006E5E05` | cùng slot vtable + hành vi (lấy bomber theo slot → controller vf3(dir), case 4 đặt bom) |
| `CMovable::MoveSimple` | `0x01310200` | `0x00F10200` | `0x0095199B` | chuỗi trace nhúng `CMovable::MoveSimple` |
| Trượt vuông góc trục X (trợ góc) | `0x01310D30` | `0x00F10D30` | — | slot 1 vtable `CMovable`; cùng họ với `MoveSimple` |
| Trượt vuông góc trục Y (trợ góc) | `0x01310EE0` | `0x00F10EE0` | — | slot 2 vtable `CMovable` |
| Kiểm "đã thẳng hàng ô" trên trục di chuyển | `0x01310C50` | `0x00F10C50` | — | được `CGOBomber::Move` gọi trước khi trượt |
| `CGOBomber::Move` (vòng lặp di chuyển) | `0x00B93890` | `0x00793890` | — | gọi `CMovable::MoveSimple` + hai hàm trượt + bảng hướng |
| Cổng vào `CGOBomber::Move` | `0x00B93830` | `0x00793830` | — | gọi `0x00B93890` |
| Cập nhật điều khiển bomber (lấy dir + độ dài) | `0x00B95180` | `0x00795180` | — | gọi `0x00B93830` với `(dir, len)` từ controller `bomber+0xE0` |
| Truy vấn ô bị chặn | `0x00EA0FD0` | `0x00AA0FD0` | — | được `MoveSimple`/trượt gọi `(row, col, ctx)` |
| Lấy object ô lưới + kiểm lớp tile | `0x00E9F9B0` | `0x00A9F9B0` | — | RTTI `CGOTileFixed`/`CGOTilePassable` |
| Bảng vector hướng (4 hướng × `dx,dy`) | `0x017929B0` | `0x013929B0` | — | được `MoveSimple`/`CGOBomber::Move` chỉ số hoá `2*dir` |
| vtable `CMovable` | `0x0179B198` | `0x0139B198` | — | RTTI `??_7CMovable@@6B@` |
| vtable `CGameContext` | `0x017695B8` | `0x013695B8` | `0x00C5466C` | RTTI `??_7CGameContext@@6B@` + `CGameContext::Init` ở slot 4 (VN slot 3) |
| vtable `CHiddenCatchGameContext` | `0x01704574` | `0x01304574` | `0x00C1768C` | RTTI `??_7CHiddenCatchGameContext@@6B@` |

## Cách tái tạo

1. Mở bản sao DB (không mở DB gốc): `kr-full.exe.i64`.
2. Liệt kê chuỗi khớp `^C[A-Za-z0-9_]+::[A-Za-z0-9_~]+$`, lấy hàm chứa xref của từng chuỗi → tên hàm.
   KR có 314 hàm tự khai tên theo cách này, VN có 186; phần giao cho cặp địa chỉ VN↔KR.
3. Đọc vtable theo RTTI đã có sẵn trong DB (`??_7...@@6B@`) để lấy slot; so slot cùng hành vi giữa
   hai build (VN `CGameContext` có 11 slot, `Init` ở slot 3; KR có 12 slot, `Init` ở slot 4).
4. Từ `CMovable::MoveSimple`, đọc bảng vector hướng `0x17929B0` (4 cặp `dx,dy`) và hằng số `40`/`39`
   trong thân hàm; đọc tiếp hai hàm trượt `0x1310D30`/`0x1310EE0` để lấy ngưỡng `20/14/26`.

## Giới hạn

- Chỉ tĩnh: chưa có trace runtime KR cho bất kỳ claim nào ở đây; không suy ra hành vi mạng.
- Ngưỡng trợ góc đọc từ hằng số trong code, chưa đo trong trận; "tối đa 34 px" là suy ra từ hình học
  `r ∈ [6,34)`, không phải số đo.
- Chưa xác định hàm quy đổi **cấp tốc độ → px mỗi bước**; `CMovable::MoveSimple` chỉ nhận độ dài còn lại
  do caller cấp (xem `KR_RE_MOVEMENT` §5).
- Polarity của `CGOTileFixed`/`CGOTilePassable` trong `0xE9F9B0` chưa chốt (candidate).
- Không có bằng chứng cho bomb pass / đứng trên bom trong phép đọc này.
