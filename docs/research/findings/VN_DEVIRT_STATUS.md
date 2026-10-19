# Devirtualize hàm bị Themida ảo hoá (hướng 1)

Mục tiêu: chạy được bản dump `ca_fixed.exe` (không còn Themida) trong Wine trên host có GPU.
IAT đã sửa xong (`docs/research/findings/VN_RUNTIME_REFERENCE.md` → "Sửa IAT"). Việc còn lại là các đoạn code bị Themida ảo hoá.

## Bảng trạng thái khối VM (nguồn sự thật duy nhất)

Cập nhật 2026-10-18. `tools/devirt.py` `BLOCKS` hiện có **15 khối**; cột "cách" nói bằng chứng: **snap** = snapshot chụp tại site trên client gốc (`win/snap.bat`) + emulate/diff;
**emu** = emulate trên snapshot cũ với khung hàm dựng bằng `--pre-asm` (+ `--overlay` dump sống), không cần client gốc vào trận; **skip** = `jmp cont` có căn cứ tĩnh.

| site | hàm | cách | trạng thái | kiểm runtime (Wine) |
|---|---|---|---|---|
| 0x8F452B | `sub_8F4421` (init trong `CFxApp__InitMain`) | snap | ✅ viết lại | client chạy tới đăng nhập |
| 0x8F8F69 | `CStageMgr__ChangeStage` | snap | ✅ viết lại | đổi stage 1→10→11→12→13 |
| 0x8F4D46 / 0x8F54E4 / 0x8F5F40 | vòng lặp chính `sub_8F4D01` (init / đổi stage / định kỳ) | snap + emu | ✅ viết lại (bỏ anti-cheat) | chạy liên tục |
| 0x8F346C | `sub_8F3409` keep-alive 5 s | snap + emu | ✅ viết lại | opcode 15 mỗi 5 s |
| 0x9682E5 | `sub_9682DE` `Init(index)` | emu | ✅ viết lại (reset 12 thành viên, §Khối viết lại bằng emulate) | gdb: 4 lần vào/ra, không crash |
| 0x6EFD4A | `sub_6EFD24` (đặt bom → bản ghi bảng vòng `CGameMsgCache`) | emu | ✅ viết lại, **chưa kiểm gói C→S 111** | Space: không crash, bản ghi khớp tham số, bom hiện |
| 0x71581A | `sub_71580C` (handler gói 552) | emu + overlay | ✅ viết lại | gói 552: vào/ra không crash |
| 0x5A6E89 / 0x6F287E / 0x754C82 | guard `rand() % N` (anti-cheat lấy mẫu) | skip | ✅ plain skip = đường 99% của game gốc | chưa chạm 1% |
| 0x8E5519 | `CFxApp__vf6` (đuôi tắt app) | skip | ✅ chỉ chạy lúc thoát | chưa thử đóng game |
| 0x5788C9 | `CBombOwner__vf33` (1 lệnh `jmp 0x414543` bị đè) | skip | ✅ khôi phục đúng lệnh gốc (mẫu "jmp bị chiếm đoạt", `vmtriage.md` §3a) | chạy qua |
| 0x736C27 | `CGamePlayMap__Load` đuôi | skip + emu | 🟡 emulate với **object map thật** (dump 11 MB, `tools/gdb/dump_map.gdb`): chỉ gọi `sub_414543(ebx+0x8F480C)` + đặt biến đếm `[ebp-0x308]=0` (không đọc sau cont) = vòng lặp 0 lần với Village01/phòng 19 → skip tương đương; thân vòng cho map/loại phòng khác **chưa biết** | skip không chặn điều khiển |
| 0x966FEC | `sub_966FDE` | — | ⬜ hàm mồ côi (0 xref), chưa chạm | — |
| 0xA12E59–0xA17A24 (43 site) | anti-cheat/system-info | — | ⬜ skip khi chạm (cont = marker+0x12, `vmtriage.md` §3b) | — |

Khác: hook `CSelectServerStage__On_16` (`HOOKS`) dựng lại ô kênh; 59 điểm "unknown" trong `decomp/themida_entries.json` là code **mutate** (chạy native, vd `sub_711E7F`) — không phải VM.
Chi tiết từng khối mới: [vm_6EFD4A_inputs.md](../../archive/re/VM_6EFD4A_INPUTS.md) (§8), mục "Khối viết lại bằng emulate" cuối file này; triage tĩnh gốc: [vmtriage.md](../../archive/re/VM_TRIAGE_HISTORY.md);
công cụ devirt tổng quát (đang dở): [vmdevirt.md](VM_METHOD.md); hướng dẫn cho người mới: [devirt-themida-huong-dan.md](DEVIRT_METHOD.md).

### Ba đường thêm một khối mới (chọn từ rẻ đến đắt)
1. **skip** — khi native ngay trước site đã có `jge cont` với `rand()%N`, hoặc khối chỉ chạy lúc thoát, hoặc là 1 lệnh bị đè (tìm hàm song sinh).
2. **emu trên snapshot cũ** — `tools/vmemu.py <SITE> --snap logs/snap/8F5F40/pre.snap --run-natives --quiet-api [--writes] --pre-asm <khung hàm> [--overlay ADDR:FILE]`.
   Khung hàm: đặt `ecx`/thanh ghi, push tham số, `call <hàm>`; object game lấy từ dump sống (`tools/gdb/dump_stage.gdb`). Đổi giá trị tham số để biết tham số nào đi vào đâu, nhánh nào đổi.
   Bẫy: VM đi lạc khi object thiếu (con trỏ null, vtable null) → nạp thêm bộ nhớ thật; chuỗi `CALL` giống nhau với nhiều giá trị khác nhau = không điều kiện.
3. **snap tại site trên client gốc** — chỉ khi cần trạng thái runtime riêng; client gốc bị Themida giết khi bị patch (`runtime.md` → "Client gốc trong WinBoat").

## Trạng thái lịch sử (2026-10-16)

- Bản dump chạy trong Wine: intro → đăng nhập → chọn kênh (xem `progress.md` cho trạng thái sau đó).
- HackShield tắt bằng **chính công tắc của game**: `Feature_IsDisabled(212)` → 1 ở 22 chỗ gọi (`tools/patch_client.py`); BlackCipher bỏ qua.

## Quy trình snapshot + emulate khối VM

VM Themida trong bản dump không chạy được vì thiếu trạng thái runtime (stack/vùng nhớ của process gốc). Nhưng **chạy được trong Unicorn
trên một snapshot đầy đủ bộ nhớ của client gốc** chụp đúng tại site:

1. **Client gốc chạy được trong WinBoat** (không GPU) nhờ D3D9 phần mềm: `thirdparty/swd3d/` = `d3d9.dll`+`wined3d.dll` của Wine (PE)
   + `opengl32.dll`+`libgallium_wgl.dll` của Mesa (mesa-dist-win, llvmpipe), đặt cạnh `ca.exe`, `GALLIUM_DRIVER=llvmpipe` (`win/swd3d.bat`).
   Kiểm tra riêng: `tools/d3dtest/d3dtest.c` + `win/d3dtest.bat`.
2. **Snapshot**: `win/snap.bat <SITE> [CONT] [timeout] [CONT:oldbytes:00]` → `tools/launcher/snap.exe` (mingw, `snap.c`):
   patch site (`jmp rel32` vào stub VM) → cave lưu thanh ghi + TEB rồi quay vòng; injector suspend mọi thread, dump **mọi vùng committed**
   (~260MB) → `logs/snap/<SITE>/pre.snap`; nếu có CONT: cho chạy tiếp, chụp `post.snap` tại điểm tiếp tục (byte gốc ở CONT bị thay bằng `jmp`).
3. **Emulate**: `tools/vmemu.py <SITE> --snap logs/snap/<SITE>/pre.snap --run-natives --quiet-api [--skip ADDR=eax] [--pre-asm "..."]`
   - map toàn bộ snapshot, GDT để `fs:` → TEB, khôi phục byte gốc ở site;
   - log mọi `CALL` native từ VM (tên IDA, ecx, tham số) và giá trị trả về; code Windows chạy thật, **syscall WoW64 giả lập** (`jmp far 0x33`);
   - cuối: diff frame của caller + `.data` → so với `tools/snapdiff.py logs/snap/<SITE> ['--obj=*E6092C:0x200']` (diff pre/post thật).
   - `--pre-asm` chạy x86 trước site (ví dụ ép một lần chuyển stage: `sub_8F391D(10)`) để kích hoạt khối không tự chạy.
   - Đừng `--skip` hàm anti-cheat mà VM kiểm tra kết quả (`sub_A13A10`): VM rẽ sang nhánh phá stack (ghi 0 từ đỉnh stack xuống).
4. **Viết lại** chỉ phần có tác dụng với code native (biến cục bộ được đọc sau, field object, global), bỏ phần anti-cheat → `tools/devirt.py`.

## Ranh giới khối VM: marker của Themida SDK

Mỗi khối VM **kết thúc bằng marker VM_END của SDK**: `EB 10 57 4C 20 …` (`jmp $+0x12` qua 16 byte, có chữ `WL `), các byte còn lại bị
mutate (`89 xx` / `90`). **Code gốc tiếp tục ngay sau marker** (marker + 0x12). Khối = `[site, marker+0x12)`; vùng giữa là rác
(Themida ghi đè code gốc tại chỗ, nên độ dài vùng ≈ độ dài code gốc). Phần code gốc **sau** marker còn nguyên dù IDA không thấy
(không có xref) → disassemble thẳng bằng capstone.
Kiểm chứng: 0x8F452B → 0x8F45A7 (khớp vmtrace), 0x8F8F69 → 0x8F8FCA (khớp thanh ghi sau khối của vmtrace: `edi=0xE`, `ebx=0`).
73 marker trong `.text`. Ghép site với marker kế tiếp:

| site | hàm | tiếp tục | dài |
|---|---|---|---|
| 0x8E5519 | `CFxApp__vf6` | 0x8E5547 | 0x2E |
| 0x8F346C | `sub_8F3409` keep-alive | 0x8F38F3 | 0x487 ✅ |
| 0x8F452B | `sub_8F4421` | 0x8F45A7 | 0x7C ✅ |
| 0x8F4D46 | `sub_8F4D01` (vòng lặp chính) khởi tạo | 0x8F5468 | 0x722 ✅ |
| 0x8F54E4 | `sub_8F4D01` chuyển stage | 0x8F5DFA | 0x916 ✅ |
| 0x8F5F40 | `sub_8F4D01` định kỳ | 0x8F6908 | 0x9C8 ✅ |
| 0x8F8F69 | `CStageMgr__ChangeStage` | 0x8F8FCA | 0x61 ✅ |
| 0x966FEC | `sub_966FDE` | 0x9670BD | 0xD1 |
| 0x9682E5 | `sub_9682DE` | 0x968385 | 0xA0 |
| 0x5A6E89 | `CGOBomber__vf21` | 0x5A6F64 | 0xDB |
| 0x6EFD4A | `sub_6EFD24` | 0x6EFED9 | 0x18F |
| 0x6F287E | `sub_6F27EE` | 0x6F2923 | 0xA5 |
| 0x71581A | `sub_71580C` | 0x7158E4 | 0xCA |
| 0x736C27 | `CGamePlayMap__Load` | 0x736D6C | 0x145 🟡 skip chạy tốt |
| 0x754C82 | `CMapScriptStageClear__vf7` | 0x754D20 | 0x9E |
| 0x5788C9 | `CBombOwner__vf33` | — | 1 lệnh 🟡 skip chạy tốt |

Dòng 🟡 (2026-10-17): 2 khối mới thêm dạng **skip thí nghiệm** trong `tools/devirt.py` `BLOCKS`, chưa viết lại thật:
- `0x736C27 → jmp 0x736D6C` (`CGamePlayMap__Load` — thực ra là điểm `mov ecx,[0xE60884]` của đoạn dọn dẹp; luồng
  nạp map tiếp tục bình thường). Nhờ nó, client qua được nạp map vào trận. **Rủi ro (2026-10-17, chủ dự án nhắc):**
  vùng 0x736C2C..0x736D59 (~0x12E byte ≈ code gốc bị ghi đè) là việc thật đã bị bỏ qua khi skip — trạng thái mà
  phần sau dựa vào có thể thiếu; là **giả thuyết 2** cho hiện tượng "nhân vật không di chuyển" trong trận
  (`progress.md` §5). Giả thuyết 1 (thiếu gói/state khác) vẫn giữ song song. Skip `0x5788C9` thì an toàn được chứng
  minh (khối đúng 1 lệnh jmp, cont khớp song sinh).
- `0x5788C9 → jmp 0x414543` (`CBombOwner__vf33` — site nằm ngoài `themida_entries.json`, là 1 lệnh `jmp sub_414543`
  nguyên bản bị Themida đè bằng `jmp 0x26574543` (ngoài image → SIGSEGV khi chạy). Soi hàm song sinh
  `CBombOwner__vf31` (0x578963, không bị đè) xác nhận byte gốc = đúng jmp đó → skip khôi phục đúng ngữ nghĩa).
  Bằng chứng chi tiết trong `docs/archive/re/VM_TRIAGE_HISTORY.md` §3a và `docs/archive/re/VN_PROGRESS_LOG.md` §11. Cánh vẫn là **skip**: nếu sau này
  phát hiện 0x414543 (hub Feature-check, bản dump trả al=0) phải trả giá trị thật thì viết lại.

(Site nằm ngoài hàm IDA như 0x8F54E4/0x8F5F40 không có trong `decomp/themida_entries.json`; tìm bằng `jmp` → `push key; jmp 0x11EE128`.)

## Khối đã xong

### 0x8F452B — `sub_8F4421` (bước khởi tạo trong `CFxApp__InitMain`)
- Ranh giới: site 0x8F452B → tiếp tục ở **0x8F45A7** (`lea ecx,[esi+14h]; push ebx; call 0x413F32`). Byte giữa là rác.
- Quan sát (`tools/vmtrace.py`, client gốc trong WinBoat): thanh ghi edi/esi/ebp/ebx giữ nguyên; eax/ecx chết. Bộ nhớ:
  - `0xEED860[256]` = bảng CRC32 reflected, poly **0xDD10EC81** (chỉ code Themida ở 0x1330A69 đọc);
  - `0xEEE07C = 0x4A65000A`, `0xEEE088 = 1`, `0xEEE08C = GetTickCount()` (không có xref từ code game);
  - `0xEEE650..65C` = callback `0x8F2C5D, 0x8F30F7, 0x8F2C4F, 0x8F31F6` (nhiều hàm game gọi, ví dụ `sub_413F32`).
- Viết lại: `tools/devirt.py` `BLOCKS[0]` (dữ liệu ghi thẳng vào image + `GetTickCount` lúc chạy).

### 0x8F8F69 — `CStageMgr__ChangeStage(id)` 0x8F8EEC
- Hàm: nếu `id == [this+D8]` thì thôi; `[this+E8]=1`; stage cũ `vf45` (OnLeave, `[vt+0xB4]`) + dọn UI; `delete` stage cũ; **khối VM**;
  rồi code gốc (còn nguyên từ 0x8F8FCA): `switch(id)` → `malloc(size)` + ctor → `[this+D4]`; null → MessageBox + `App__SetExitError(5,0x1B4)`;
  `stage->vf44()` (OnEnter, `[vt+0xB0]`); CALog `[XRpt] Sg - INT(dne:%d)`; `[this+E8]=0`.
- Khối VM (0x61 byte) chỉ còn: `[this+DC] = [this+D8]` (stage trước, đọc qua `GetPrevStage` 0x8F889A), `[this+D8] = id`.
  Bằng chứng: vmtrace (`D8: 0x17 → 1`), ngoài ctor `sub_8F9800` không code native nào ghi D8/DC; `CStage__vf44` đọc `GetCurStageId` nên D8 phải đặt trước.
  Ngữ cảnh tại site: `esi`=CMainWnd, `ebx`=id, `edi`=0 (dùng làm tham số ở 0x8F8FD0).
- Viết lại: `tools/devirt.py` `BLOCKS[1]`.

## Stage (CMainWnd = `g_pMainWnd`)

`CMainWnd`: `+D4` stage hiện tại, `+D8` id hiện tại (`GetCurStageId` 0x8F8893, 151 nơi gọi), `+DC` id trước, `+E8` đang chuyển stage.
Id 0x17 = chưa có stage. Đổi stage: `sub_8F391D(dword_E6092C, id)` đặt id đích (giá trị được che), vòng lặp chính so với `GetCurStageId` rồi gọi ChangeStage.
Bảng `switch` trong code gốc 0x8F8FE3 (size = tham số `malloc`; `0x40176c(n)` = `Feature_IsDisabled(n)`):

| id | stage | size | ctor | ghi chú |
|---|---|---|---|---|
| 1 | `CLoginStage` | 0x26CC | 0x84F724 | ✅ chạy trong Wine |
| 3 | `CCreditStage` | 0x1024 | 0x68F731 | |
| 10 | `CSelectServerStage` | 0x7354 | 0x85817A | |
| 11 | lobby: loại 0 `CLobbySquareStage` (0x570F0, 0x4E56DF) / loại 2 `CLobbyStageWithSquareUI` (0x57CFC, 0x4C1F4D) / khác `CLobbyStage` (0x5525C, 0x6B3EDD) | | | loại = `sub_994378(server hiện tại)`, mặc định 1 nếu feature 0x2E tắt; 0x7B tắt UI square |
| 12 | phòng: `sub_8F8CFC` → `CPrepareStage` / `CHiddenCatch(Single)PrepareStage` | | | |
| 13 | ván: `sub_8F8C88` → `CGameStage` / `CHiddenCatchGameStage` | | | |
| 14 | `CShopStageNew` | 0xAD64 | 0x807358 | |
| 16 | `CChattingStage` | 0xC134 | 0x68EE57 | |
| 17 | `CExchangeStage` | 0x8D88 | 0x846220 | |
| 19 | `CUserMarketStage` | 0x11B4C | 0x85E898 | feature 7 |
| 20 | `CSquareStage` | 0x7048 | 0x86C926 | feature 8 |
| 21 | `CMagicBookMarketStage` | 0xF8A8 | 0x87F942 | feature 0x3C |

(Đọc từ code gốc, id 1 đã kiểm chứng; ctor là hàm **không** có caller native — hàm stage được gọi từ `vf0` deleting-dtor là **dtor**.
README/devirt cũ ghi `CLoginStage` ctor 0x84F4A7 là sai: đó là dtor.)

## Vòng lặp chính `sub_8F4D01` (CMainSystem::Run) — ✅

`CFxApp__vf5` 0x8E30DA → `sub_8F4D01(ecx = dword_E6092C)`, `[ebp-0x280]` = this. Kết quả emulate (diff khớp snapshot thật):
- **A 0x8F4D46** (trước vòng lặp): `sub_A12E50` (anti-cheat: thả `%TEMP%\drc*.tmp` rồi nạp), `[ebp-0xA8] = malloc(0x2B4)` zero
  (free lúc thoát 0x8F6CE2), bảng toàn vẹn 57 stub VM + `sub_A14800` (hash → `ebp-0xCC..-0x68`). Giữ lại: chỉ buffer.
- **B 0x8F54E4** (khi `CMainSystem::vf5` = stage hiện tại ≠ stage đích): `sub_A13A10` (checksum `.text` 0x401324+0x1266FA) rồi
  `CStageMgr__ChangeStage(g_pMainWnd, sub_8F3903(this))`.
- **C 0x8F5F40** (khi `elapsed > 0x3681`): `p = new(0xC)`; `sub_67C820(p, &[ebp-0x140])`; xoá `CMainSystem+0xF8` cũ (`sub_8F2C6B(1)`),
  `+0xF8 = p`; `sub_A13A10`; `[ebp-0x184] = sub_9995B1(sub_412B76())`; `sub_441A4C(&[ebp-0x134], 1)`.
- **0x8F346C** `sub_8F3409` (`CMainSystem::vf10`, mỗi 5 s): keep-alive `SendPkt 0xF {u32 time, [EA6FF0+0x1C], [EA6FF0+0x20]}`,
  `[EA6FF0+0x24]=1`, `CMainSystem+0x48 = time`; phần sau là kiểm tra HackShield ngẫu nhiên (bỏ).

## Cách Themida ảo hoá trong binary này

Một **khối code ở giữa hàm** bị thay bằng:
```
<site>:     jmp  0x13xxxxx          ; trong .text của game
0x13xxxxx:  push <key>              ; khoá chọn bytecode, mỗi site một khoá khác nhau
            jmp  0x11EE128          ; VM entry (CISC VM Themida 2.x)
```
VM entry `0x11EE128`: `pushfd; push …; pushad`-kiểu lưu context, hằng số rác xen kẽ (mutation).
Bộ thông dịch nằm trong section Themida của exe (`0x11E4000–0x16AB000`), bytecode cũng ở đó.
Trong bản dump, VM chết vì đọc dữ liệu chỉ có trong process gốc (địa chỉ stack của main thread gốc `0x19FF68`, v.v.).

Code game **không** bị ảo hoá thì vẫn chạy native; một số đoạn bị **mutate** (x86 xáo trộn, nhảy sang `0x13xxxxx` rồi quay về) — vẫn chạy native.

## Danh sách điểm vào (`decomp/themida_entries.json`, sinh bởi `tools/themida_scan.py`)

117 điểm game code nhảy vào section Themida (đã lọc bằng IDA):

| class | số | ý nghĩa |
|---|---|---|
| `vm` | 56 | đi tới VM entry `0x11EE128`, có `key` |
| `unknown` | 59 | gặp `call`/nhánh có điều kiện trước khi tới đích — gần như chắc là mutate (chạy native được) |
| `return` | 2 | quay về `.text` — mutate |

### 15 điểm VM trong hàm game (ưu tiên)

Thứ tự theo luồng khởi động (làm trước) rồi tới gameplay:

| Site | Hàm | Vai trò (đoán từ ngữ cảnh) | Ưu tiên |
|---|---|---|---|
| 0x8F452B | `sub_8F4421` | khởi tạo trong `CFxApp__InitMain` (0x8E5FF8 → gọi ở 0x8E6D79); **đang chết ở đây** | 1 |
| 0x8E5519 | `CFxApp__vf6` (0x8E50D1) | vòng đời app | 1 |
| 0x8F346C | `sub_8F3409` | gần login/mạng (`Login_ConnectMain` 0x8F3978 ở cạnh) | 1 |
| 0x8F4D46 | `sub_8F4D01` | gần `Net_DispatchMainSocket` 0x8F4094 | 1 |
| 0x8F8F69 | `sub_8F8EEC` | gần `CMainWnd`, gọi từ `sub_8F939A` (`CMainWnd__OnPacket`) | 1 |
| 0x966FEC | `sub_966FDE` | ? | 2 |
| 0x9682E5 | `sub_9682DE` | ? | 2 |
| 0xA15F57 | `sub_A15F50` | vùng anti-cheat/system info | 3 |
| 0xA1784E | `sub_A17810` | duyệt cửa sổ con (`GetWindow` GW_CHILD/HWNDNEXT) — anti-cheat? | 3 |
| 0x6EFD4A | `sub_6EFD24` | ? | 2 |
| 0x6F287E | `sub_6F27EE` | ? | 2 |
| 0x71581A | `sub_71580C` | gần `CGameStage__OnPacket` 0x71B631 | 2 |
| 0x736C27 | `CGamePlayMap__Load` 0x7354FD | **nạp map** | 2 |
| 0x754C82 | `CMapScriptStageClear__vf7` | script map: điều kiện qua màn | 2 |
| 0x5A6E89 | `CGOBomber__vf21` (0x5A6244) | **gameplay: nhân vật** | 2 |

### 41 điểm VM ngoài hàm IDA: `0xA12E59–0xA17A24`

Nằm sau `AntiCheat_ScanProcesses` (0xA12C00, chụp toolhelp quét process theo danh sách trong `cavn.ini`).
Giả thuyết: toàn bộ là anti-cheat tự viết → **bỏ qua cả khối** thay vì devirtualize. Cần xác nhận nơi gọi vào vùng này.

## Công cụ quan sát khối VM: `tools/vmtrace.py` + `tools/launcher/inject.c`

Chạy trên **client gốc** (Themida thật) trong WinBoat, nơi VM chạy đúng:
```sh
uv run --with pefile --with capstone --with keystone-engine python tools/vmtrace.py build <SITE> <CONT>   # -> logs/vmtrace/<SITE>/
# tạo win/vmtrace_run.bat gọi: win\vmtrace.bat <SITE> 30000000 230000 30000FF0   (RemoteApp không nhận tham số có dấu cách)
xfreerdp3 ... '/app:program:C:\Windows\System32\cmd.exe,cmd:/c \\host.lan\Data\boom-re\win\vmtrace_run.bat'
uv run --with pefile python tools/vmtrace.py diff <SITE>     # thanh ghi + object [esi] + stack + vùng data 0xE00000–0xF09000 trước/sau
```
- `inject.exe` khởi động `ca.exe`, đặt cave ở 0x30000000, patch HackShield + site + điểm tiếp tục khi Themida giải mã xong, chờ cờ = 2, dump.
- `flag=2` nghĩa là cả hai hook chạy → **điểm tiếp tục đúng**.
- Stack thay đổi **dưới ESP** là vùng nháp của VM, bỏ qua.
- Cần `boomsrv` + DNAT (để qua bước kiểm tra bản vá) nếu khối nằm sau đó trong luồng khởi động.
- Tìm điểm tiếp tục: sau site là byte rác; tìm đoạn x86 hợp lệ chạy tới epilog của hàm (`call 0x9DB5CB; ret`), kiểm chứng bằng vmtrace.

## Chạy bản dump

```sh
python3 tools/fakeserver.py --log logs/fakeserver_host.log &   # server giả trên host
tools/rebuild_wine.sh            # fix_iat -> devirt -> patch_client -> chạy dưới winedbg (cửa sổ hiện trên desktop)
tools/run_xvfb.sh ca_fixed_patched.exe 40   # chạy trong Xvfb :77 riêng, chụp logs/xvfb/shot_*.png (không đụng desktop)
```
Chỉ chạy **một** bản một lúc (mutex `Global\FxClient...`; bản thứ hai thoát mã 9). Dọn bằng `kill` đúng PID, đừng `pkill -f` chuỗi có trong lệnh của chính shell.

## Phương pháp đề xuất (cho từng site)

1. **Xác định ranh giới khối**: site là điểm vào; tìm điểm ra = địa chỉ trong hàm mà VM nhảy về
   (lệnh ngay sau khối bị thay; thường IDA thấy code "mồ côi" sau site, không có xref).
2. **Đọc ngữ cảnh tĩnh** trong IDA: biến/thanh ghi được dùng sau điểm ra, biến được ghi trước site → suy ra khối làm gì.
3. **Quan sát động trên client gốc** (Themida thật, VM chạy đúng) trong WinBoat:
   launcher patch bộ nhớ (`tools/launch.ps1`, `tools/launcher/boomlaunch.c`) có thể chèn `int3`/hook tại site
   và tại điểm ra để ghi thanh ghi + vùng nhớ trước/sau (so sánh = tác dụng của khối).
   Có thể cài x32dbg vào WinBoat nếu cần debugger tương tác.
4. **Viết lại bằng x86** trong section mới của bản dump, sửa `jmp` ở site trỏ sang code mới, cuối code mới `jmp` về điểm ra.
   Đưa vào pipeline: thêm bước trong `tools/fix_iat.py build` (hoặc tool riêng) + ghi lại trong file này.
5. Nếu khối chỉ là kiểm tra chống debug/anti-cheat → thay bằng `jmp <điểm ra>` (bỏ qua).

## Lệnh hay dùng

```sh
tools/rebuild_wine.sh                 # build ca_fixed(_patched).exe, copy vào wine/Boom, chạy winedbg, in backtrace + CALog
uv run --with pefile --with capstone python tools/themida_scan.py unpacked_ca.exe   # sinh lại decomp/themida_entries.json
# exception đầu tiên (first chance) trong winedbg:
printf 'set $BreakOnFirstChance=1\ncont\ncont\nbt\ninfo regs\nquit\n' | WINEDEBUG=-all wine winedbg 'Z:\home\admin\boom-re\wine\Boom\ca_fixed_patched.exe'
WINEDEBUG=+seh,-all wine ca_fixed_patched.exe     # liệt kê mọi exception
WINEDEBUG=+debugstr,-all wine ca_fixed_patched.exe  # OutputDebugString của game
```
Wine đã đặt `HKCU\Software\Wine\WineDbg\ShowCrashDialog = 0` (in backtrace ra console thay vì hộp thoại).

## Chẩn đoán màn hình logo (2026-10-16) — ĐÃ GIẢI QUYẾT (khối 0x8F8F69, xem trên)

Bản dump dừng ở màn hình intro. Thực chất process thoát mã 9 (`App__SetExitError(9)`), không treo.

- CALog: `116 : InitBeforePatch` rồi dừng.
- winedbg (break first-chance): page fault `EIP=0x1388`, tức một `call [reg+N]` với con trỏ rác.
- Nguồn: `CStageMgr__ChangeStage` (`0x8F8EEC`) có một khối ở site `0x8F8F69` (tiếp tục ở `0x8F9392`) chưa viết lại → cần làm block thứ 2.
- Đã quan sát block này trên client gốc: `tools/vmtrace.py build 8F8F69 8F9392`, dump ở `logs/vmtrace/8F8F69/dump.bin`.
  - reloc (5 byte site đè): `call 0x9DB5B7`; điểm tiếp tục thật là epilog `0x8F9392` (call `__EH_epilog3`), back `0x8F9397`.
  - Hàm ngữ cảnh: chuyển stage (Login → SelectServer → Lobby…). vtable stage: `CLoginStage` ctor `0x84F4A7`, `CStage` ctor `0x6A588F`.
  - diff: nhiều thay đổi trên object `CMainWnd` ([esi]) + ~18 dword global; cần đọc kỹ để tái hiện (chưa xong).

Các script phụ thêm: `tools/wine_bt.sh` (attach winedbg lấy bt), `tools/wine_dbg.sh` (break first-chance trong Xvfb).

## Khối viết lại bằng emulate trên snapshot cũ (2026-10-18, không cần client gốc vào trận) — chi tiết

Phương pháp: `tools/vmemu.py <SITE> --snap logs/snap/8F5F40/pre.snap --run-natives --pre-asm <khung hàm>` — snapshot cũ của client gốc đã có
đủ trạng thái runtime của Themida; khung hàm + đối tượng game dựng bằng `--pre-asm` hoặc nạp từ dump sống của client chạy trong Wine
(`tools/gdb/dump_stage.gdb` → `--overlay ADDR:FILE`). Đổi giá trị tham số/gói để xem tham số nào vào field nào, nhánh nào đổi.

| site | hàm | kết quả | kiểm runtime |
|---|---|---|---|
| 0x6EFD4A | `sub_6EFD24` | ghi bản ghi 68 B vào bảng vòng `CGameMsgCache` 13/17 (flush op 111) — chi tiết `vm_6EFD4A_inputs.md` §8 | Space không crash, bản ghi khớp tham số (gdb `msgcache_rec.gdb`) |
| 0x71581A | `sub_71580C` (gói 552) | không điều kiện: `MsgCache.vf3(1)`, `Decode4(packet)` (bỏ kết quả), `sub_711E7F`, `sub_708192`, `sub_414543(this+0x4AB4)` (box getter), `sub_705F63` | server gửi 552 (`BEGIN_OP=552`): vào/ra handler không crash (gdb `h552.gdb`) |
| 0x9682E5 | `sub_9682DE` Init(index) | gán index + reset 12 thành viên (box -1/0, sub-readers, object +0xB4) — thay bản tạm | gdb `init9682.gdb`: 4 lần vào/ra (2 ctor + reset idx 0/1), không crash |
| 0x5A6E89, 0x6F287E, 0x754C82 | guard `rand() % N` | plain `jmp cont` = đường đi ~99% của game gốc | build; chưa có lần chạy nào chạm 1% |

Ghi chú: `sub_711E7F`/`sub_708192`/`sub_705F63` nhảy vào section Themida (`jmp 0x13xxxxx`) nhưng là code **mutate** (chạy native, quay về .text),
không phải VM — 59 điểm "unknown" trong `decomp/themida_entries.json` thuộc loại này. Packet (`RecvPkt`): +0x10 buffer, +0x20 giới hạn,
+0x24 con trỏ đọc, +0x28 chế độ (khác 0 → số bị XOR), +0x30 cờ lỗi.

**0x736C27 (`CGamePlayMap__Load` tail) — emulate thử, chưa viết lại:** với `ebx` = object trống, khối chỉ gọi box getter `sub_414543(ebx + 0x8F480C)`
rồi thoát (không ghi gì) → chunk ~0x136 byte của hàm gốc rất có thể là vòng duyệt danh sách (rỗng với object trống). Muốn biết nó làm gì với map thật cần bộ nhớ
`CGamePlayMap` (object > 11 MB, `this+0xA832E8` là danh sách) — chưa dump được. Skip hiện tại không chặn điều khiển (đã kiểm), giữ nguyên.
**0x8E5519 (`CFxApp__vf6`)**: đây là đoạn tắt app (duyệt hết singleton gọi dtor), chỉ chạy lúc thoát → plain skip. **0x966FEC**: hàm mồ côi (0 xref), chưa chạm.

## Công cụ phân tích VM (nhánh `feat/vmdevirt`, 2026-10-18)

`tools/vmdevirt.py` tự dò cấu trúc VM thay vì hardcode, và đã kiểm chứng trên chính dump này:

| Việc | Lệnh | Kết quả |
|---|---|---|
| Liệt kê site + stub head | `discover` | 607 site / 390 head; **6/6 khối đã devirt tay đều được tìm thấy** |
| Tự dò VM context | `profile --snap ... --site ...` | ra `ctx = 0x11EDDEC` từ spinlock `0x11EECB0` |
| Bảng handler | `handlers --ctx 11EDDEC` | 178 entry trong `ctx+0x94..+0xF48`, tĩnh (khớp 3 snapshot + file) |
| Bản đồ opcode | `opcodes --snap ... --disp 11F9E81` | 46 opcode, 46/46 có handler, 44/46 có size, 3 opcode nhánh |
| Hiệu ứng mỗi VI | `vfx --snap ...` | diff ctx + thanh ghi giữa 2 dispatch, gom theo opcode |
| Trace / lift | `trace` / `lift` | ghi write ctx/image/heap, hoặc rút lệnh có ngữ nghĩa |

Kiểm chứng quan trọng: `[ctx + opcode*4]` so với EIP handler thực tế — **3.680/3.680 khớp, 0 lệch**.

Đã verify định lượng với snapshot thật: `8F346C` 32/52 write ngoài stack khớp `post.snap`, `8F4D46` 9/12,
`8F5F40` 2/4; call sequence khớp `devirt.py` đến từng tham số (kể cả con trỏ `ebp-0x140`). CFG: 24 block,
6 back-edge, hub `0x135B33B`. Bàn giao đầy đủ + việc còn lại: [vmdevirt.md](VM_METHOD.md) §18.

Điều **chưa** làm được (ghi rõ để khỏi tưởng nhầm): bytecode bị mã hoá rolling key nên không decode tĩnh
được toàn stream; ngữ nghĩa 46 opcode chưa xác định (mới có size/nhánh/handler). Chi tiết + báo cáo độ phủ:
[vmdevirt.md](VM_METHOD.md).

Đính chính (2026-10-18): `sub_414543` là **getter của box bool/u32 chống sửa bộ nhớ** (cùng họ `sub_578344`/`sub_413F32`), không phải "hub check Themida"; giải mã box: con trỏ dữ liệu ở `box+8`, 4 byte + byte khoá k (k=0→42; out=k^b; k+=b+42) — xem `tools/gdb/bomb_ids.gdb`.
