> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Triage khối VM còn lại (tĩnh, 2026-10-16)

> **Cập nhật 2026-10-18:** đây là triage TĨNH ban đầu; cột ưu tiên/"đã skip" trong các bảng dưới đã lỗi thời. Trạng thái hiện tại từng khối: [devirt.md](../../research/findings/VN_DEVIRT_STATUS.md#bảng-trạng-thái-khối-vm-nguồn-sự-thật-duy-nhất).
> Đính chính: các site `0x6EFD4A`, `0x71581A`, `0x9682E5`, `0x5A6E89`, `0x6F287E`, `0x754C82`, `0x8E5519` đã xử lý; `sub_711E7F`/`sub_708192`/`sub_705F63` là code mutate, không phải VM.

Rà soát 56 site `vm` trong `decomp/themida_entries.json` + 3 site ngoài JSON (8F54E4, 8F5F40, 5788C9).
Không chạy động, không sửa binary, không commit. DB IDA (`unpacked_ca.exe.i64`, idalib) chỉ đọc, không save.

## Quy ước nguồn / độ chắc chắn (ghi tắt trong bảng)

- `F` = hàm chứa do IDA (`db.functions.get_at`) — chắc.
- `X` = caller/xref do IDA (`db.xrefs.to_ea`) — chắc.
- `D` = disasm IDA (`instructions.get_between`) hoặc capstone trên `unpacked_ca.exe` — chắc (byte trước site còn nguyên; sau site là rác Themida).
- `J` = `decomp/themida_entries.json` (+ `tools/themida_scan.py`) — chắc.
- `N` = `decomp/ida_names.json` / `rtti.txt` / `decomp/opcode_dispatch.json` — chắc (tên), suy luận vai trò từ tên là trung bình.
- `M` = marker `EB 10 57 4C 20` quét bằng script (73 marker, ghép site→marker kế tiếp) — chắc là marker, nhưng "marker = cuối khối" là trung bình (có 2 lệch, xem §2).
- Suy luận vai trò không có mã `F/X/D` hỗ trợ: yếu.

Phân loại: **(a)** anti-cheat / bỏ qua được · **(b)** đường gameplay chính · **(c)** chưa rõ.
Ưu tiên: **P0** làm trước · **P1** · **P2** · **P3** làm khi chạm · **S** = ứng viên "thí nghiệm `jmp cont` trước, devirt sau nếu vẫn crash".

## 1. Đối chiếu số liệu với `docs/research/findings/VN_DEVIRT_STATUS.md`

- Đã xong: `tools/devirt.py` có **6 BLOCKS** (8F452B, 8F8F69, 8F4D46, 8F54E4, 8F5F40, 8F346C), devirt.md ghi "7 khối" — lệch 1, chưa rõ khối thứ 7 (có thể đếm nhầm). Nguồn: đọc `tools/devirt.py`. Chắc.
- Còn lại: 56 − 4 (site trong JSON đã xong: 8F452B, 8F8F69, 8F4D46, 8F346C) = **52 site** = 9 trong hàm game + 43 vùng A1xxxx. 3 site ngoài JSON: 8F54E4/8F5F40 (đã xong) và **5788C9** (skip 2026-10-17, bảng §3a): byte tại đó là `E9 …` nhảy vào vùng Themida (0x137C3D9 / 0x138B12D trong khoảng 0x11E4000–0x16AB000; còn 5788C9 nhảy ra ngoài image 0x26574543 — mẫu khác) — `themida_scan.py:follow()` không theo tới VM entry được (có thể pattern `push key` khác). Nguồn: đọc byte PE + `D`. Chắc.
- Vùng anti-cheat: JSON có **43** site từ 0xA12E59 đến 0xA17A24, trong đó 41 ngoài hàm IDA + 2 nằm trong hàm (`A15F57` trong `sub_A15F50`, `A1784E` trong `sub_A17810`) — khớp "41 điểm ngoài hàm IDA" của devirt.md. Nguồn: `J` + `F` (`get_at` trả None cho 41 site). Chắc.
- Marker: ghép site→marker kế tiếp khớp bảng devirt.md cho cả 9 site hàm game (cont = marker+0x12). Nguồn: `M`. Chắc.

## 2. Hai điểm cont trong bảng devirt.md cần hiệu chỉnh khi devirt

- `0x6EFD4A`: cont theo marker là 0x6EFED9, nhưng IDA tại đó hiện `dd` rác; code native thật tiếp tục ở **0x6EFEDC** (`pop edi; pop esi; pop ebx; leave; retn 0x30` — epilog của chính `sub_6EFD24`). Nguồn: `D`. Chắc.
- `0x736C27` (`CGamePlayMap__Load`): cont theo marker là 0x736D6C — **đã xem disasm**: 0x736D6C = `mov ecx, [0xE60884]; call sub_8EC54D; mov bl, 1; jmp loc_736Bxx` (đoạn dọn dẹp, rồi `mov al,bl` → epilog) — đây là điểm native thật, khớp cách skip đã chạy thành công (2026-10-17, xem §4 mục 3ia). Ghi chú cũ "0x736D90" là `mov [ecx], offset vt_TSingleton_Class_CPlayMapInfoList_` — đuôi TSingleton, một đường khác, không phải cont. Nguồn: `D` + chạy thật. Chắc.
- Kết luận: với 2 khối này, "code gốc ngay sau marker" không đúng từng byte; khi devirt phải lấy cont = địa chỉ native thật trên. Các cont còn lại khớp marker+0x12 và IDA disasm ra code hợp lệ.

## 3. Bảng toàn bộ site VM còn lại (52)

### 3a. Site trong hàm game (9)

| site | key | hàm chứa (IDA `F`) | vai trò đoán | loại | ưu tiên | nguồn / chắc chắn |
|---|---|---|---|---|---|---|
| 0x5A6E89 | D84CFF3 | `CGOBomber__vf21` (0x5A6244–0x5A700B) | check mẫu anti-cheat trên counter `[esi+0x2718]`: đếm 6 lần `(x%25==0)` rồi `rand()%1001 < ebx` mới vào VM, còn lại `jge` về cont 0x5A6F64 (~99% bỏ qua). Cont native tiếp tục gọi `sub_59C9A4/sub_5A04A4/…`. VM ~1% frame, không thể chứa logic gameplay chính | a | **S** (P2 nếu skip thất bại) | `F,J,D` chắc; suy luận a chắc (guard số học rõ) |
| 0x6EFD4A | D850BA8 | `sub_6EFD24` (0x6EFD24–0x6EFEE3) | helper dùng chung của bomber: 8 caller toàn họ bomber — `sub_4A4E70`, `CGOCAHeroGameBomber__vf12`, `sub_5B0053`, `sub_5B089D`, `CGOCAHeroGameBomber__vf6_2` (2 chỗ), `sub_6E3E58`, `sub_74E76F`, **`CGOSquareBomber__vf6`** (lobby square). Đầu hàm: `sub_41C791(arg_4)` → `sub_8EAC0E()` (cổng check, chưa rõ) → 0 thì vào VM, else `return 0`. Cont thật 0x6EFEDC (xem §2) | b | **P1** | `F,X` chắc; vai trò b trung bình-chắc (caller quyết định) |
| 0x6F287E | D85712F | `sub_6F27EE` (0x6F27EE–0x6F292B) | setter struct: chép `arg_14..arg_20 → [eax+10h..20h]`, `[eax+4]=1`; rồi `rand()%999 >= 10 → jge` cont 0x6F2923 (~99% bỏ qua; cont chỉ còn `EH_epilog; retn 0x28`). Caller: `CGOCAHeroGameBomber__vf6_2` + `sub_702DE4` | b | **S** (P1 nếu skip thất bại) | `F,X,D` chắc; suy luận skip chắc (guard số học rõ) |
| 0x71581A | D85B2FC | `sub_71580C` (0x71580C–0x71581F, chỉ 0x13 byte prefix) | **handler opcode 552 của `CGameStage__OnPacket`**: `dispatch[552] = sub_71580C` (indirect), gọi tại 0x71BC96 `push edi; mov ecx,esi; call` (ecx=CGameStage, edi=packet). Prolog `push 0x2C + EH_prolog3`, toàn bộ thân (0xCA) bị ảo hoá, cont 0x7158E4 chỉ còn epilog. Vùng sau có `RecvPkt__Decode2` là hàm khác | b | **P0** | `F,X,D,N` (dispatch) chắc |
| 0x736C27 | D85F6CD | `CGamePlayMap__Load` (0x7354FD–0x736C2C) | **nạp map**: trước site `malloc(0x90)` + ctor `sub_7D204A` + `sub_72A3C0`; 4 xref OFFSET trong `.rdata` (slot vtable). Cont thật 0x736D90 (xem §2). File đối chiếu: `decomp/CGamePlayMap__Load.c` | b | **P0** | `F,X,D,J,N` chắc |
| 0x754C82 | D864077 | `CMapScriptStageClear__vf7` (0x754BF6–0x754D34) | check điều kiện qua màn trong **vòng lặp** (`inc/cmp/jl` ở cont 0x754D20 quay về 0x754C31): `dynamic_cast esi → CGONpcObject` (RTTI trong lệnh, chắc là duyệt NPC), rồi `rand()%995 >= 10 → jge` cont (~99% bỏ qua) | b | **S** (P1 nếu skip thất bại) | `F,D,N` (RTTI) chắc; suy luận skip chắc |
| 0x8E5519 | D868BB4 | `CFxApp__vf6` (0x8E50D1–0x8E551E) | vòng đời app: kẹp giữa 2 lần gọi virtual có guard `Feature_IsDisabled (0x40176c)`; caller `sub_8E7292`, `WinMain`, 1 slot vtable. Khối nhỏ (0x2E). Client đã chạy tới stage 10 nên khối này hoặc hiếm khi chạy hoặc không chặn | b | P3 | `F,X,D` chắc; vai trò trung bình |
| 0x5788C9 | — | `CBombOwner__vf33` (0x5788B8–0x5788CE) + caller gián tiếp `CGOCAHeroGameBomber__vf33_0` (0x5A0274). **Không có trong themida_entries.json** — phát hiện khi crash thật | Đuôi wrapper: `push esi; mov esi,ecx; mov eax,[esi]; push 1; call [eax+0xC8]; lea ecx,[esi+4]; pop esi;` rồi byte nguyên bản tại 0x5788C9 (sđc `e9 CA BB E9 FF` = `jmp sub_414543`) bị Themida **đè bằng `e9 75 BC FF 25` = jmp 0x26574543** (ngoài image — vùng heap cấp phát lúc chạy, đã bị giải phóng trong bản dump → SIGSEGV). So sánh song sinh `CBombOwner__vf31` (0x578963, không bị đè): `...; lea ecx,[esi+0x1C]; pop esi; jmp sub_414543` — cùng dấu → khối = chính xác 1 lệnh jmp, **skip = `jmp 0x414543` khôi phục đúng ngữ nghĩa** | a | ✅ đã skip, chạy thật qua được (2026-10-17, xem progress.md §11) | `D` (byte song sinh) + gdb (`logs/xvfb/gdb.txt`: `[esi]=vt_CGOZzolmanBomber_off510` lành, EIP crash = 0x26574543 đúng bằng target của jmp) |
| 0x966FEC | D8AA5FC | `sub_966FDE` (0x966FDE–0x966FF1, 19 byte) | chưa rõ: prolog `push 0x24 + EH_prolog3`, **0 xref** (hàm mồ côi — gọi gián tiếp kiểu callback hoặc chết). Cont 0x9670BD gọi `sub_A62B10` (vùng packet-decode, chưa rõ). Vùng 0x965–0x968 là cụm thunk/helper nhỏ sau code UI (`CUserPageWnd/CUIConfig`, `N`) | c | P3 | `F,X` chắc (mồ côi); vai trò yếu |
| 0x9682E5 | D8AE77A | `sub_9682DE` (0x9682DE–0x9682EA, 12 byte) | helper kiểu dtor: `2× sub_413EED + sub_413F04` trên `[esi]`; caller `sub_71A9EC` (vùng code game-stage 0x71Axxx) + `sub_9683B4` (hàm kề). Cont 0x968385 khớp epilog (`pop edi/esi/ebp; ret 4`) | b/c | P2 | `F,X,D` chắc; phân loại trung bình (caller game-stage nhưng hàm nhỏ, có thể là helper thuần) |

### 3b. Vùng anti-cheat / system-info 0xA12E59–0xA17A24 (43 site, loại **a**, ưu tiên P3 — chỉ xử lý khi crash chạm)

Bối cảnh chung (áp dụng mọi dòng): sau `AntiCheat_ScanProcesses` 0xA12C00 (`N` devirt.md); IDA không tạo hàm nào bao các site (`F get_at`=None, trừ 2 dòng ghi rõ); mỗi site chỉ có 1 xref kề trước kiểu flow/fallthrough (`X`, type 21) — tức là **chuỗi khối VM nối tiếp nhau**, xen kẽ vài stub native tí hon (`CSystemInfo__vf0` chỉ dài 0x1E, `sub_A12E3E/44/4A`, `sub_A13330`, `sub_A160E0`). Lối vào duy nhất từ ngoài vùng là slot vtable (`X` OFFSET tới `CSystemInfo__vf0` từ `.rdata` 0xC95034). `sub_A15F50` chỉ được gọi từ 0xA164F1 (nội vùng); `sub_A17810` chứa liệt kê `GetWindow` (anti-cheat quét cửa sổ, theo devirt.md) và caller 0xA178CB nằm ngay trong chính nó. Nguồn: `F,X,D,J`. Suy luận (a): chắc.

| site | key | vị trí / hàm chứa | dài khối (site→marker+0x12) | ghi chú |
|---|---|---|---|---|
| 0xA12E59 | D8B2412 | ∅ — cụm G1 (sau stub `CSystemInfo__vf0` A12E20–A12E3E) | 0x37B | đầu chuỗi |
| 0xA131E7 | D8BBDA8 | ∅ — cụm G1 | 0xB5 | |
| 0xA13450 | D8C0658 | ∅ — cụm G2 (sau stub `sub_A13330` A13330–A13441) | 0x3F | |
| 0xA1349C | D8C3AFC | ∅ — G2 | 0x221 | |
| 0xA136D4 | D8C9811 | ∅ — G2 | 0x86 | |
| 0xA13769 | D8CD916 | ∅ — G2 | 0x150 | |
| 0xA138D7 | D8D2F9D | ∅ — G2 | 0x133 | |
| 0xA13A1C | D8D7AA6 | ∅ — G2 | 0x4B9 | khối dài nhất vùng |
| 0xA14319 | D8ED579 | ∅ — G2 | 0x145 | mẫu 0x145 lặp lại nhiều lần (check lặp?) |
| 0xA14479 | D8F39FB | ∅ — G2 | 0x182 | |
| 0xA14809 | D90420E | ∅ — G2 | 0x55 | |
| 0xA149B8 | D90F3CD | ∅ — G2 | 0x124 | |
| 0xA14B49 | D9158D2 | ∅ — G2 | 0xD6 | |
| 0xA14C36 | D919F5E | ∅ — G2 | 0x83 | |
| 0xA14CC9 | D91EBE5 | ∅ — G2 | 0x145 | mẫu 0x145 |
| 0xA14FD9 | D923755 | ∅ — G2 | 0x8D | |
| 0xA15089 | D92718B | ∅ — G2 | 0x283 | |
| 0xA15329 | D92CA99 | ∅ — G2 | 0x145 | mẫu 0x145 |
| 0xA1548C | D9324C9 | ∅ — G2 | 0x51E | khối dài thứ 2 |
| 0xA159C9 | D93BFEF | ∅ — G2 | 0x145 | mẫu 0x145 |
| 0xA15B39 | D9404A9 | ∅ — G2 | 0x14A | |
| 0xA15C99 | D9455A7 | ∅ — G2 | 0x145 | mẫu 0x145 |
| 0xA15DF9 | D94A3CC | ∅ — G2 | 0x145 | mẫu 0x145 |
| 0xA15F57 | D7F6AB3 | trong `sub_A15F50` (A15F50–A15F5C) | 0x168 | key D7F… khác dải (cũ hơn?); caller duy nhất nội vùng |
| 0xA160D9 | D7FBE66 | ∅ — cụm G3 (giữa `sub_A15F50` và stub `sub_A160E0`) | 0x4C | |
| 0xA16309 | D80DF62 | ∅ — cụm G4 (sau stub `sub_A160E0`) | 0x53 | |
| 0xA16534 | D819AAB | ∅ — G4 | 0x42 | |
| 0xA16589 | D81C8E4 | ∅ — G4 | 0x74 | |
| 0xA16619 | D82002B | ∅ — G4 | 0x51 | |
| 0xA1673D | D8272E0 | ∅ — G4 | 0x62 | |
| 0xA16875 | D82EC13 | ∅ — G4 | 0x93 | |
| 0xA16927 | D833669 | ∅ — G4 | 0x108 | |
| 0xA16A46 | D94E5A1 | ∅ — G4 | 0x99 | key quay lại dải D94… |
| 0xA16AF9 | D952CEE | ∅ — G4 | 0x145 | mẫu 0x145 |
| 0xA16C59 | D957DF3 | ∅ — G4 | 0x145 | mẫu 0x145 |
| 0xA16E89 | D95D43C | ∅ — G4 | 0xB9 | |
| 0xA16F59 | D962527 | ∅ — G4 | 0x145 | mẫu 0x145 |
| 0xA170B9 | D9673E5 | ∅ — G4 | 0x1E1 | |
| 0xA172C9 | D96D19F | ∅ — G4 | 0x1E5 | |
| 0xA174CC | D97332B | ∅ — G4 | 0x14C | |
| 0xA1784E | D978530 | trong `sub_A17810` (A17810–A178E9) | 0x61 | duyệt `GetWindow` GW_CHILD/HWNDNEXT — anti-cheat quét cửa sổ (`D` devirt.md; chắc) |
| 0xA178FA | D97CA83 | ∅ — cụm G5 (trước `App__HandleUpdateCmdline` A17AA0) | 0x98 | |
| 0xA17A24 | D984F8A | ∅ — G5 | 0x61 | cuối chuỗi |

## 4. Thứ tự devirt đề xuất cho chặng lobby → phòng → ván (stage 11/12/13)

Phòng (stage 12, `CPrepareStage`) không có site VM nào — qua free nếu lobby xong. Lobby mặc định loại 1 (`CLobbyStage`, feature 0x2E) nên nhánh square-bomber có thể không chạm; cứ thử tới đâu làm tới đó.

1. **Thí nghiệm rẻ trước (S): `jmp cont` 3 khối có guard `rand()%N`** — `0x5A6E89→0x5A6F64`, `0x6F287E→0x6F2923`, `0x754C82→0x754D20`. Cơ sở: game đã chạy đúng ~99% số lần qua các điểm này mà không vào VM (`D` chắc). Nếu Wine vẫn crash đúng tại site (tức lần đó rơi vào 1% vào VM) thì mới devirt thật. Riêng 0x754C82 nằm trong vòng lặp duyệt NPC mỗi lần check qua màn — skip là đường chung, an toàn cao.
2. **`sub_6EFD24` (P1)** — helper bomber dùng chung 8 caller, trong đó có `CGOSquareBomber__vf6` (lobby square). Snapshot+emulate theo quy trình devirt.md; chú ý cont thật là **0x6EFEDC** (§2) và cổng `sub_8EAC0E` ở đầu hàm (giữ nguyên native).
3. **`CGamePlayMap__Load::0x736C27` (P0)** — chặn vào ván (nạp map). Cont thật **0x736D90** (§2). Đối chiếu `decomp/CGamePlayMap__Load.c`.
4. **`sub_71580C` (P0)** — handler opcode 552 của `CGameStage__OnPacket`; toàn bộ thân là VM nên bắt buộc snapshot+emulate, không có đường skip. Cần fakeserver phát opcode 552 (xem dispatch `CGameStage__OnPacket@71B631`) để kích hoạt khi chụp.
5. **`sub_9682DE` (P2)** — khi chạm (caller vùng game-stage `sub_71A9EC`). Khối nhỏ (0xA0), snapshot+emulate.
6. **`sub_966FDE`, `CFxApp__vf6` (P3)** — khi chạm. 966FDE mồ côi xref nên có thể không bao giờ chạy; vf6 đã qua được tới stage 10.
7. **43 site vùng A1xxxx (P3)** — mặc định không đụng. Nếu Wine crash tại một site: `jmp` thẳng tới cont (marker+0x12 trong bảng §3b), vì toàn vùng là system-info/anti-cheat tự viết, entry duy nhất qua vtable `CSystemInfo`, và HackShield đã tắt bằng công tắc game. Không devirt từng khối trừ khi skip gây lỗi logic thấy rõ.

## 5. Ghi chú cho đợt sau

- **Mẫu "jmp bị chiếm đoạt" (5788C9)**: site có thể KHÔNG nằm trong themida_entries.json và là 1 lệnh jmp ngắn trong hàm nguyên bản bị Themida đè bằng jmp ra vùng heap. Cách nhận nhanh: hàm chứa hiện `JUMPOUT(<địa chỉ ngoài 0xB00000>)` trong Hex-Rays; chạy thật sẽ crash ở EIP = target. Cách sửa rẻ và đúng: tìm hàm song sinh có cùng đuôi (pattern `add ecx,N; jmp sub_414543` hoặc `lea ecx,[esi+M]; pop esi; jmp sub_414543`) để biết byte nguyên bản, rồi skip = khôi phục lệnh đó.
- Đừng tin `ida_names.json` nearest cho vùng A1xxxx: mọi site đều hiện `CSystemInfo__vf0+N` nhưng hàm đó chỉ dài 0x1E (`F`); các site thực sự nằm ngoài hàm IDA nào.
- 3 site có guard `rand()` là chỗ duy nhất trong 9 khối hàm game mà "bỏ qua" là đường chạy native thường xuyên — các khối còn lại (71580C, 736C27, 6EFD24, 9682DE, 966FEC, 8E5519) không có guard, skip là can thiệp hành vi, phải kiểm chứng bằng chạy thật.
- `sub_6EFD24` và `sub_702DE4` (caller của 6F27EE) vẫn chưa rõ vai trò chính xác (`N` không đặt tên) — khi devirt cần đọc caller trước để biết object/args.
