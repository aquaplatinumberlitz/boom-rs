# Chạy client: Themida, IAT, patch, môi trường

Ghi lại những gì đã thử để **chạy được client thật** nối vào server giả, và vì sao từng cách thành/bại.

## Trạng thái (2026-10-16; cập nhật 2026-10-18 ở cuối file)

| Cách chạy | Mạng / login | Hiển thị | Ghi chú |
|---|---|---|---|
| `ca.exe` gốc (Themida) trong WinBoat + patch bộ nhớ | ✅ login xong | ❌ `init renderer falied. hr=0x887600aa` | VM không có GPU |
| `ca.exe` gốc trong Wine (host, RTX 3060) | — | — | ❌ chết ở anti-VM Themida trước WinMain |
| `ca_fixed_patched.exe` (dump + IAT sửa) trong Wine | — | — | 🟡 chạy qua CRT, vào `WinMain` (`CALog`: `InitBeforePatch`), chết ở **hàm bị Themida ảo hoá** |

## Thứ tự khởi động (`CFxApp__vf4` 0x8E6FEE)

1. `sub_A17AA0`: tự cập nhật, chỉ chạy khi có `-update "..."` trong command line.
2. `sub_8E5FC7()` luôn trả 1 → `CFxApp__InitHackShield` 0x8E3883 + BlackCipher (`sub_4134BF`).
   **Bỏ qua: `0x8E701D` `74 29` → `EB 29`.**
3. `sub_8E59D8`: khởi tạo hệ thống, gồm `sub_8E56E2` → `PatchServer_CheckVersion` 0x8E426E (kết nối patch, chờ hello).
4. Nạp pack (`sub_8E4A03`), renderer, stage Login → `Login_DoLoginSequence` 0x8E2DEC.

Lỗi được dồn lại qua `App__SetExitError(code, sub)` 0x8F280B và hiện ở cuối bằng `App__ShowExitError` 0x8E4778:
1 = không nối được login/patch server ("Login Failed"), 3 = HackShield (`ErrHackShieldInit%02d`), 4 = GameGuard, 5/6/7/8/9 = khác.
Exception C++ không bắt được → `CrashDlg` ("An error has occurred. Don't panic!") trong `WinMain` 0x8F2146.
Log của client: `CALog.log` trong thư mục game (`sub_A25BE0`).

## Danh sách server (`CMultiServer` 0x998FD5)

- IP hardcode: `222.255.122.204` (default), `120.138.77.22–25`, `120.138.77.83–85`, mỗi IP các port 3838/4848/5858/6868.
- Client chọn ngẫu nhiên; patch và login có thể rơi vào port bất kỳ → server giả phải nghe cả 4 port.
- `118.102.5.126`, `175.207.3.35` cũng hardcode (gần HackShield / `sub_96A44A`).

## Patch trong bộ nhớ (client gốc, Themida)

Themida giải mã image trong khoảng 0,8–1s rồi mới chạy tới `CFxApp__vf4`.
Launcher đọc liên tục, thấy byte gốc thì ghi đè ngay:
- Windows: `tools/launch.ps1` (PowerShell 32-bit, `win/watch2.bat`).
- Wine/Windows: `tools/launcher/boomlaunch.c` (mingw), đồng thời đổi IP server.

Themida **không** phát hiện patch (đã chạy được tới login).

## Mạng trong WinBoat

- Windows guest = `172.30.0.2`, NAT qua container (`172.18.0.2`) ra mạng docker `winboat_default` (`172.18.0.0/16`).
- Host bật `ufw` → guest không gọi được host `172.18.0.1`. Giải pháp: chạy server trong container riêng `boomsrv` (`172.18.0.50`) cùng mạng.
- Chuyển hướng IP cũ bằng DNAT trong container WinBoat (mất khi restart container):
  `iptables -t nat -I PREROUTING -s 172.30.0.0/24 -d <ip> -p tcp -j DNAT --to-destination 172.18.0.50`
- Chạy lệnh trong guest: `xfreerdp3 /app:program:cmd.exe,cmd:/c \\host.lan\Data\boom-re\win\X.bat` (RemoteApp), kết quả ghi về `\\host.lan\Data\boom-re\logs`.
  Chụp màn hình trong phiên RDP bằng PowerShell `CopyFromScreen` (QMP screendump chỉ thấy console, không thấy phiên RDP).

## Wine

- CachyOS Wine 10 chỉ có **WoW64 kiểu mới**; không tạo được prefix `win32`.
- Themida chạy lệnh đặc quyền (dò VM) ở 0x14790C9 (bắt được) rồi 0x11E5F62 → `EXCEPTION_PRIV_INSTRUCTION` không tới được SEH 32-bit → process chết (`0xC0000096`).
  Chạy `ca.exe` gốc không patch cũng chết y hệt → do Wine, không do patch.
- Bản copy để chạy: `wine/Boom` (reflink của `~/Downloads/Boom`, không đụng bản gốc).

## Bản dump `unpacked_ca.exe` và IAT

unlicense dựng IAT giả ở 0x1900000 (433 import) và sửa các lệnh `call [IAT]`,
nhưng **IAT gốc 0xC0F000–0xC0F7FF vẫn nguyên giá trị lúc dump**. Code đọc IAT gốc qua con trỏ
(`mov reg, ds:dword_C0Fxxx` rồi `call reg`) → nhảy vào wrapper Themida không còn tồn tại → crash
(`0xC0000005` tại 0x06D40A36).

IAT gốc chia theo DLL, ngăn cách bằng ô 0 (24 nhóm, 489 ô):
advapi32, (4 nhóm 1–2 ô chưa rõ), gdi32, imm32, iphlpapi, kernel32 (196), (2 chưa rõ), oleaut32, shell32,
user32 (97), version, (17 chưa rõ), (3 chưa rõ), ws2_32, binkw32, (1), (2), ijl15, imagehlp, mss32, nmco3.

Giá trị trong mỗi ô lúc dump là một trong:
1. wrapper Themida (vùng cấp phát riêng, ví dụ 0x06xxxxxx–0x07xxxxxx);
2. địa chỉ API thật (chỉ đúng trong lần boot lúc dump);
3. địa chỉ trong DLL đi kèm game.

Công cụ:
- `tools/dump_exports.ps1`: liệt kê export (kèm forwarder) của các DLL trong Windows **cùng lần boot**.
- `tools/live_dump.ps1`: chạy `ca.exe` gốc, dump IAT gốc + vùng nhớ thực thi + danh sách module.
- `tools/fix_iat.py`: giải từng ô, rồi `build` thêm import descriptor cho IAT gốc để loader tự điền.

Wrapper Themida là stub 4KB nhảy ngược vào section của chính exe → emulate phải có cả image `ca.exe` trong bộ nhớ.

## Sửa IAT (hướng 3) — kết quả 2026-10-16

`tools/fix_iat.py build unpacked_ca.exe ca_fixed.exe` (rồi `tools/patch_client.py`, hoặc gộp: `tools/rebuild_wine.sh`).
Kết quả giải đã chốt ở `decomp/iat_resolved.json` (git); `build` chỉ cần file này + `unpacked_ca.exe`.
Muốn giải lại (sửa `OVERRIDES`…) thì cần `logs/live/` + `unlicense_ca.log`, chạy `report` rồi `export`.

1. Dump process gốc **gồm mọi image module** (`tools/live_dump.ps1` → `logs/live/`): IAT gốc, vùng wrapper, image `ca.exe` + 64 DLL.
2. Mỗi ô IAT gốc, lấy tên theo thứ tự ưu tiên:
   - trỏ thẳng vào DLL → tên export (đọc bảng export từ chính image đã dump, kể cả DLL game: `dwk.dll`, `cawh.sys`, `d3dx9_30`…);
   - wrapper → emulate bằng Unicorn tới khi **tail-jump** vào DLL (ESP về đúng giá trị ban đầu); wrapper kết thúc bằng `jmp [IAT của DLL]` nên phải map cả image DLL;
   - Themida "stolen bytes" (tự chạy prologue rồi nhảy vào giữa API) → export gần nhất phía trước (≤ 0x40 byte);
   - còn lại → kết quả của unlicense (cùng lần boot);
   - **24 ô xác định thủ công** (`OVERRIDES`, có ghi lý do) — Themida chép nguyên thân API vào vùng riêng, không có điểm nhảy:
     dò theo cách game dùng (tham số, hằng số), số syscall (`NtUserEmptyClipboard` → `EmptyClipboard`), mã CRT đã biết.
   - 2 ô của `ngclient.aes` (BlackCipher) → stub `xor eax,eax; ret` (không nạp DLL anti-cheat).
3. Chọn DLL cho mỗi nhóm = DLL export được nhiều tên nhất (ưu tiên DLL "cổ điển" hơn kernelbase/ntdll/win32u) — để chạy được trên Wine.
4. Thêm section `.imp2` chứa import descriptor cho IAT gốc → loader tự điền.
5. **IAT giả của unlicense cũng sai 3 mục**: Themida dùng một bộ giả lập chung cho nhiều API
   (`InterlockedIncrement/Decrement/SetHandleCount`, `InterlockedExchange/ExchangeAdd`) nên unlicense gán bừa
   (`D3DXGetVertexShaderProfile`, `D3DXGetShaderConstantTable`, `VarUI4FromI8`). 38 lời gọi được sửa từng cái
   (`CALL_FIXES`) để gọi qua ô IAT gốc đúng, phân loại theo mã nguồn CRT MSVC / cách dùng kết quả.

Kết quả trong Wine: qua CRT startup, vào `WinMain`, nạp D3D9 + D3DX9 thành công, rồi chết ở hàm bị ảo hoá.

## Hàm bị Themida ảo hoá (VM) — chặn hướng 3

→ **Đang làm theo hướng 1, chi tiết ở [devirt.md](VN_DEVIRT_STATUS.md).**

Có 98 điểm code game nhảy vào section Themida (`0x11E4000–0x16AB000`):
- Phần lớn nhảy tới `0x131xxxx–0x132xxxx` rồi quay về: nhiều khả năng chỉ là code **mutate** (x86 bị xáo), chạy native được.
- **~12 hàm vào VM** qua `push <khoá>; jmp 0x11EE128` (VM entry, kiểu CISC VM của Themida 2.x):
  `CFxApp__vf6`, `sub_8F3409`, `sub_8F4421` (khởi tạo app — đang chết ở đây), `sub_8F4D01`, `sub_8F8EEC`,
  `sub_966FDE`, `sub_6EFD24`, `sub_6F27EE`, `sub_71580C`, `CGamePlayMap__Load`, `CMapScriptStageClear__vf7`, `CGOBomber__vf21`.
- Bộ thông dịch VM trong bản dump đọc dữ liệu chỉ có ở process gốc (ví dụ địa chỉ stack `0x19FF68`) → page fault.

Lựa chọn:
1. **Devirtualize** từng hàm (hiểu chức năng qua ngữ cảnh rồi viết lại bằng x86) — tốn công, mỗi hàm một bài toán.
2. Tái tạo môi trường mà VM cần (dữ liệu stack/vùng nhớ Themida lúc chạy) — chưa rõ khả thi.
3. Quay về client gốc (Themida chạy thật, VM chạy đúng) + patch bộ nhớ, giải bài toán hiển thị:
   Wine bản có WoW64 cũ (runner Lutris/wine-ge có lib32) hoặc Windows có GPU.

## Debug bản dump: dùng `winedbg --gdb`, không dùng winedbg tương tác

Kết luận thực nghiệm (đã kiểm chứng 2026-10-16): vấn đề nằm ở **frontend điều khiển**,
không phải engine đọc bộ nhớ (winedbg luôn đọc được). Cùng một động cơ, đổi tay lái:

| Chế độ | Cách gọi | Đánh giá |
|---|---|---|
| winedbg tương tác | lệnh `break/cont/info regs` qua stdin | ❌ chập chờn: breakpoint báo `Invalid address`, process chết trước khi hit, script `cont/bt` lệch pha sau exception đầu tiên |
| `winedbg --gdb` + gdb batch | `winedbg --gdb` làm gdb-proxy (spawn gdb 18.x thật, chuyển lệnh sang Win32 API), nạp sẵn file lệnh gdb | ✅ ổn định, lặp lại được: `break *0x8F2146` hit đúng `0x008f2146`, `x/2xb 0x400000` → `0x4d 0x5a` |

Cách dùng chuẩn (`tools/wine_gdb.sh <gdb_cmd_file> [user] [pass] [secs]` → `logs/xvfb/gdb.txt`):
```sh
printf 'set pagination off\nbreak *0x8F2146\ncontinue\nx/2xb 0x400000\nquit\n' > /tmp/cmds.gdb
tools/wine_gdb.sh /tmp/cmds.gdb tester 1234 80
```
Script tự dựng Xvfb :77 riêng và gõ đăng nhập lại vài lần (retry) trong nền — vì login
tự động tới đúng màn hình là chuyện riêng, không liên quan độ ổn định của debugger.
Phạm vi đã khẳng định: đặt breakpoint + đọc bộ nhớ theo script. Chưa khẳng định:
sửa hết mọi vướng runtime (VD: `set`-ghi bộ nhớ, watchpoint phần cứng vẫn cần thử
riêng từng cái).

## Client gốc trong WinBoat (2026-10-18)

Dùng để chụp snapshot bộ nhớ lúc VM chạy đúng (`win/snap.bat`, `tools/launcher/snap.c`). Kết quả thực tế:
- Cửa sổ game trong guest là **800x600** và bị **minimize** lúc mở → script click phải khôi phục cửa sổ và nhân tọa độ 1024x768 với 0.78125 (`tools/battle_drive.ps1`).
- Phải có **server trả đúng địa chỉ**: server Rust `SERVER_IP=172.18.0.50` (container `boomsrv` cùng mạng `winboat_default`); 127.0.0.1 = chính guest → "không vào được kênh".
- HackShield: `snap.bat` phải vá 13 chỗ `Feature_IsDisabled(212)` lúc đăng nhập (xem `win/snap_bisect.bat`) + `0x8E701D` + `0x69E636`; thiếu thì client thoát 0.5 s sau đăng nhập
  (khung anticheat). Vá thêm 9 chỗ ở sảnh (0x595372, 0x59E869, 0x59E98A, 0x68BB28, 0x690AEC, 0x69E62E, 0x6AA235, 0x6B0070, 0x6B0278) thì khối VM chống sửa code
  zero thanh ghi + stack rồi nhảy tới 0 (dump WER: EIP 0, ESP trỏ vào vùng toàn 0).
- Client gốc bị giết ngẫu nhiên (exit `0xC0000005`, không có dump WER) khi code đã bị patch, thường lúc đổi màn; tới stage 13 vẫn chết sau 501. `tools/snap_retry.sh` thử lại nhiều lần.
  **Kết luận:** không đưa được client gốc vào trận ổn định → dùng snapshot cũ + dựng khung hàm trong `tools/vmemu.py` (`docs/research/findings/VN_DEVIRT_STATUS.md`).
- RDP vào WinBoat hay bị reset: `docker restart WinBoat` rồi nạp lại luật DNAT (mất khi restart); `/app:program:cmd.exe,cmd:/c <bat>` chỉ chạy được khi không có phiên RDP khác.
- Bẫy shell: `pkill -f xfreerdp3` có thể giết luôn shell → dùng `pkill -x xfreerdp3`.
