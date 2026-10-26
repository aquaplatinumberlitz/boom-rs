# VN UI control coords F-11e — Login/Room/Lobby (2026-10-26)

Bằng chứng tĩnh; không chạy VN/Wine, không commit binary/decompile.

## Nguồn

- VN `ca_fixed_patched.exe` SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`.
- Login constructor `0x0084cd94`; control region `0x84d114..0x84de0d`; local disassembly
  `/tmp/vn-ui.dis` (xóa sau lượt), control dump `/home/admin/boom-team/shots/fidelity/F-11/vn-ui-controls.txt`.
- Image R-227 login (`01-login-initial.png`, 1024×768, game800×600 scale1,28); R-228/R-229 lobby/room captures.

## Login — xác nhận

| control | id | local x,y | screen (x+52,y) | ảnh |
|---|---:|---:|---:|---|
| `LO_BtLogNewId` | 0x3e9 | 240,556 | 292,556 | ĐĂNG KÝ left292, y556 |
| `LO_BtLogOk` | 0x3ea | 350,556 | 402,556 | ĐĂNG NHẬP left402, y556 |
| `LO_BtLogExit` | 0x3eb | 460,556 | 512,556 | THOÁT RA left512, y556 |
| `LO_BtEkey` | 0x6009 | 695,491 | 747,491 | không hiện |
| `LO_BtPT` | 0x6010 | 695,517 | 747,517 | không hiện |

- Literal local x/y: confirmed. `0x10000000` at `-0xac/-0xa8` **không phải w/h**; w/h code unknown.
- Offset +52 x, +0 y khớp ba nút nhưng **chưa định vị control cha `LO_LoginBox`**; screen mapping là derived, chưa đủ để gọi parent là confirmed.

- Constructor là `thiscall`; string refs `LO_BtLogNewId`/`LO_BtLogOk`/`LO_BtLogExit`.
- Ảnh R-227 cho visual width ~106, height ~35 logic (button); không nâng thành code w/h.
- Code control id/name: `0x3e9` NewId, `0x3ea` Ok, `0x3eb` Exit.
- Ảnh login local: `/home/admin/boom-team/shots/fidelity/F-11/01-login-initial.png`.

## Room / Prepare

- Tên `Prepare/*`, `PRE_Bt_Ready`, `PRE_Bt_Start`, `PRE_Bt_Roominfo`, `PRE_*` có trong string table.
- Không tìm thấy constructor/control table tương ứng trong pass này; R-152 KR vẫn là
  `dword_19D8A58` cells `(20,90)…` và Ready `512,494` trong **KR executable**.
- R-229 image room `BẮT ĐẦU` visual xấp xỉ `(509,509)`; không dùng R-152 `512,494` làm confirmed
  mapping vì khác nguồn/khác màn và y lệch khoảng15.
- Không claim VN room x/y/w/h.

## Lobby / Square

- Tên `LobbySquare/RoomUI_*`, `Lobby/LB_*`, `Square/SQ_*` có trong string table.
- Không tìm thấy constructor/control table tương ứng trong pass này; R-228 ảnh vẫn là nguồn duy nhất.
- Không claim VN lobby x/y/w/h.

## Giới hạn

- Chỉ Login có layout literal; Room/Lobby/HUD/Result chưa có code locator.
- Không mở binary/pack khác; không chạy client; không sửa code/profile/asset.
- D-04/D-05 giữ nguyên.
