# VN UI control coords từ executable — F-11e (2026-10-26)

Bằng chứng tĩnh; không chạy VN/Wine. Không commit binary/decompile.

## Nguồn

- VN `ca_fixed_patched.exe`, SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`.
- Login constructor **`0x0084cd94`** (thiscall), region controls `0x84d114..0x84de0d`; string refs
  `LO_BtLogNewId`/`LO_BtLogOk`/`LO_BtLogExit`; local disassembly `/tmp/vn-ui.dis`, dump local
  `/home/admin/boom-team/shots/fidelity/F-11/vn-ui-controls.txt`.
- Image baseline R-227: `/home/admin/boom-team/shots/fidelity/F-11/01-login-initial.png`,
  1024×768, game800×600 scale1,28.

## Bảng control Login

| control | id | local x,y (code) | ảnh logic (x,y) | kết luận |
|---|---:|---:|---:|---|
| `LO_BtLogNewId` | 0x3e9 | 240,556 | 292,556 | khớp với offset chung +52 |
| `LO_BtLogOk` | 0x3ea | 350,556 | 402,556 | khớp với offset chung +52 |
| `LO_BtLogExit` | 0x3eb | 460,556 | 512,556 | khớp với offset chung +52 |
| `LO_BtEkey` | 0x6009 | 695,491 | — | không thấy trong capture login |
| `LO_BtPT` | 0x6010 | 695,517 | — | không thấy trong capture login |

- Ba nút cuối dùng **cùng parent offset +52 x, +0 y** suy ra từ ba điểm; chưa định vị được
  control cha `LO_LoginBox` nên đây là **derived** cho screen mapping, còn literal local x,y là
  `vn_static confirmed`.
- Các word `0x10000000` đặt ở `-0xac/-0xa8` **không phải w/h**; không suy width/height từ chúng.
  Ảnh R-227 cho visual width ~106, height~35 logic (button); không nâng thành code w/h.
- Code control id/name đúng; `0x3e9` NewId, `0x3ea` Ok, `0x3eb` Exit.

## Các màn khác

- `PRE_Bt_Ready`/room/lobby/HUD/result: không tìm thấy constructor/bảng literal tương ứng trong
  phạm vi pass này. R-152 (KR) vẫn là `dword_19D8A58` room cells `(20,90)…` + Ready `512,494`
  trong executable; chưa đối chiếu được VN.
- Không claim x/y cho lobby/room/HUD/result; R-227…R-230 hình ảnh vẫn giữ nguyên.

## Giới hạn

- Chỉ một hàm login; parent `LO_LoginBox` chưa định vị; font/size glyph chưa đo.
- Offset +52 là suy ra từ ba nút, không phải hằng số parent đã đọc; nếu control cha đổi thì phải đọc lại.
- Không commit binary, disassembly, ảnh; D-04/D-05 giữ nguyên.
