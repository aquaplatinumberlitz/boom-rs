> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Boom Online client — reverse engineering

Dịch ngược client **Boom Online** (VNG, bản Việt của Nexon *Crazy Arcade / BnB*, nhánh source `VBNB_116`).
Server đã chết, mất source; chỉ còn client. Mục tiêu dài hạn: hiểu đủ để viết **server giả lập** (sẽ viết bằng **Rust**) và chơi lại được.

> **Vào làm tiếp?** Đọc theo thứ tự:
> 1. **[docs/archive/re/VN_PROGRESS_LOG.md](VN_PROGRESS_LOG.md)** — tiến độ, cách chạy thử, chỗ đang kẹt, việc tiếp theo (viết cho người mới)
> 2. [Bắt đầu phiên làm việc](#bắt-đầu-phiên-làm-việc) (kiểm tra môi trường)
> 3. Chi tiết từng bước đã làm: [docs/archive/re/VN_LOBBY_LOG.md](VN_LOBBY_LOG.md)

Tài liệu:

| File | Nội dung |
|---|---|
| [docs/specification/networking/LEGACY_VN_PROTOCOL.md](../../specification/networking/LEGACY_VN_PROTOCOL.md) | Protocol TCP: frame, mã hoá, seq, hello, đăng nhập, dispatch — phần lớn **đã kiểm chứng với client thật** |
| [docs/specification/assets/PACKS.md](../../specification/assets/PACKS.md) | Pack `.idx/.idd`, mã hoá entry, sprite → PNG |
| [docs/specification/FEATURE_CATALOG.md](../../specification/FEATURE_CATALOG.md) | Catalog KR: 46 nhóm bằng chứng, 114 mục có quyết định phạm vi |
| [docs/planning/SCOPE_V1.md](../../planning/SCOPE_V1.md) | Scope V1 đã chốt: 37 mục, Battle cá nhân 2–4 người, nội dung và gate nghiệm thu |
| [docs/planning/TEAM_WORKFLOW.md](../../planning/TEAM_WORKFLOW.md) | Workflow team/agents, nguồn luật Hàn–Trung và hợp đồng giao task |
| [l10n/README.md](../../../l10n/README.md) | Việt hóa KR: catalog, thuật ngữ, câu đố dân gian ĐBSCL và 23 mục text còn review |
| [docs/specification/localization/IMAGE_REVIEW.md](../../specification/localization/IMAGE_REVIEW.md) | Trang duyệt 3.038 tài nguyên ảnh KR/VN: mở ảnh, xác nhận và lưu kết quả CSV |
| [docs/research/findings/VN_RUNTIME_REFERENCE.md](../../research/findings/VN_RUNTIME_REFERENCE.md) | Chạy client: thứ tự khởi động, mã lỗi, patch bộ nhớ, mạng WinBoat, Wine, **sửa IAT bản dump** |
| [docs/archive/re/VN_PROGRESS_LOG.md](VN_PROGRESS_LOG.md) | **Tiến độ + handoff**: đang ở đâu, chạy thử, việc tiếp theo |
| [docs/archive/re/VN_LOBBY_LOG.md](VN_LOBBY_LOG.md) | Kênh → sảnh → phòng chờ → bắt đầu trận: nhật ký từng bước, địa chỉ hàm |
| [docs/research/findings/VN_DEVIRT_STATUS.md](../../research/findings/VN_DEVIRT_STATUS.md) | **Bảng trạng thái 15 khối VM đã xử lý** + 3 đường viết lại (skip / emulate trên snapshot cũ / snapshot tại site) |
| [docs/archive/re/VM_TRIAGE_HISTORY.md](VM_TRIAGE_HISTORY.md) | Triage tĩnh 56 site VM (lịch sử; trạng thái hiện tại ở devirt.md) |
| [docs/archive/re/VM_6EFD4A_INPUTS.md](VM_6EFD4A_INPUTS.md) | Khối đặt bom `0x6EFD4A`: khung hàm, tham số, **kết quả viết lại (§8)** |
| [docs/research/findings/VM_METHOD.md](../../research/findings/VM_METHOD.md), [docs/research/findings/DEVIRT_METHOD.md](../../research/findings/DEVIRT_METHOD.md) | Công cụ devirt tổng quát `tools/vmdevirt.py` (đang dở) + hướng dẫn cho người mới |
| [docs/specification/assets/MAPS.md](../../specification/assets/MAPS.md) | Định dạng map |
| `docs/oc_*.md` | Ghi chép của agent OpenCode (chưa audit hết, xem `docs/archive/re/VN_PROGRESS_LOG.md` mục 7) |

---

## Trạng thái

Cập nhật 2026-10-18. Bản ngắn gọn cho người mới: [docs/archive/re/VN_PROGRESS_LOG.md](VN_PROGRESS_LOG.md).

| Phần | Trạng thái | Ở đâu |
|---|---|---|
| Unpack Themida `ca.exe` | ✅ `unpacked_ca.exe` (OEP 0x9E5EFE) | [Unpack](#1-unpack-themida) |
| IDA: phân tích + đặt tên từ RTTI | ✅ 3699 vtable, 13.800 hàm ảo, ~20.7k tên | `decomp/ida_names.json` |
| Lớp mạng TCP | ✅ kiểm chứng: emulate binary + traffic thật | `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `tools/boomproto.py` |
| Packet → handler | ✅ 520 opcode / 59 dispatcher | `decomp/opcode_dispatch.json`, `decomp/handlers/` |
| Hello + chuỗi đăng nhập | ✅ client thật đăng nhập được vào server giả | `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `tools/fakeserver.py` |
| Đăng nhập bằng tài khoản (opcode 1 → 4, 15) | ✅ tới màn chọn kênh | `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `tools/fakeserver.py` |
| Màn chọn kênh (stage 10) | ✅ kênh tự hiện (hook), click vào kênh được | `docs/archive/re/VN_LOBBY_LOG.md` |
| Sảnh quảng trường (stage 11) | ✅ load xong (sửa 10 byte hỏng 0x592C09), TẠO PHÒNG chạy | `docs/archive/re/VN_LOBBY_LOG.md` |
| Phòng chờ (stage 12) | ✅ người chơi, trưởng phòng, map Village01, nút BẮT ĐẦU bật; hộp thoại chọn map còn trống | `docs/archive/re/VN_LOBBY_LOG.md` |
| Trận đấu (stage 13) | 🟡 vào trận, nạp map, HUD, nhân vật nhận phím, **đặt bom chạy**; chưa gửi gói bom/di chuyển lên server (C→S 111?) | `docs/archive/re/VN_PROGRESS_LOG.md`, `docs/research/findings/VN_DEVIRT_STATUS.md` |
| Pack `.idx/.idd` + sprite → PNG | ✅ 108.726 PNG, 965 map, 645 ogg | `docs/specification/assets/PACKS.md`, `tools/idf.py`, `tools/i16.py` |
| Định dạng `.map`, `Attr/*`, frame animation | ⬜ | `decomp/CGamePlayMap__Load.c` |
| Client gốc (Themida) + patch bộ nhớ | ✅ chạy tới đăng nhập trong WinBoat; ❌ không render (VM không GPU); ❌ Wine (anti-VM Themida) | `docs/research/findings/VN_RUNTIME_REFERENCE.md` |
| Bản dump chạy trong Wine (**hướng đang theo**) | ✅ **chạy tới phòng chờ**: đăng nhập → kênh → sảnh → phòng chờ | `tools/rebuild_wine.sh`, `docs/research/findings/VN_DEVIRT_STATUS.md` |
| Hàm bị Themida ảo hoá | 🟡 **15 khối** trong `tools/devirt.py` (viết lại hoặc skip có căn cứ); còn `0x736C27` (skip), `0x966FEC`, 43 site anti-cheat A1xxxx | `docs/research/findings/VN_DEVIRT_STATUS.md` (bảng trạng thái), `tools/devirt.py`, `tools/vmemu.py` |
| Devirt tổng quát (`tools/vmdevirt.py`) | 🟡 bảng handler/ctx/opcode có; chưa emit C | `docs/research/findings/VM_METHOD.md` |
| Client gốc trong WinBoat có hình | ✅ D3D9 phần mềm (wined3d + Mesa llvmpipe) | `win/swd3d.bat`, `thirdparty/swd3d/` |
| Server thật (Rust) | 🟡 khung `server/` chạy được tới trận (login → kênh → sảnh → phòng → 501/189/190); `tools/fakeserver.py` là đặc tả sống | `docs/archive/re/VN_PROGRESS_LOG.md` mục 12 |

### Đã kiểm chứng trên client thật
- Codec `boomproto.py` giải mã đúng packet thật của client (CRC8 khớp); seq byte, obfuscate số đúng.
- Định dạng nhận: (0,0) cho hello, (2,3) sau đó; gửi (2,2). Đọc thẳng từ bộ nhớ process.
- Luồng: kết nối patch → hello (version 116) → client đóng → kết nối chính → hello → client gửi opcode 0 →
  server gửi 13, 2, 12, 21, 1, 43 (mỗi cái `u8 0`) → đăng nhập xong, cửa sổ "Boom".

### Câu hỏi còn mở
- Byte thứ 2 của header 2 byte (client gửi `0xF4`, khi nhận không kiểm tra).
- 6×u32 trong opcode 0 (có lẽ session từ launcher VNG; chạy trần thì toàn 0).
- Packet sau đăng nhập.

---

## Việc tiếp theo

Danh sách đầy đủ, có thứ tự ưu tiên: [docs/archive/re/VN_PROGRESS_LOG.md](VN_PROGRESS_LOG.md#6-việc-tiếp-theo-theo-thứ-tự-ưu-tiên). Tóm tắt:

1. Gói **C→S 111** (flush bản ghi đặt bom): tìm điều kiện flush của `CGameMsgCache::vf8`.
2. Dump `CGamePlayMap` để viết lại thật khối `0x736C27`; xử lý site anti-cheat A1xxxx khi chạm.
3. Mở đủ map (1019), sảnh/phòng nhiều người, các gói trong trận; audit `docs/oc_*.md`.
4. Hoàn thiện server **Rust** (`server/`) dùng `tools/fakeserver.py` + `docs/` làm đặc tả.

---

## Bắt đầu phiên làm việc

### Kiểm tra nhanh
```sh
cd ~/boom-re
python3 tools/boomproto.py                                   # -> roundtrip ok
python3 tools/idf.py ~/Downloads/Boom                         # liệt kê 4 pack
uv run --with unicorn --with pefile python tools/verify_emu.py unpacked_ca.exe   # -> fails: 0
tools/rebuild_wine.sh                                         # build dump đã sửa + chạy trong Wine (backtrace + CALog)
```

### File không có trong git (nặng / bản quyền) — đang nằm trên máy

| Thứ | Ở đâu | Cách tạo lại |
|---|---|---|
| Client gốc | `~/Downloads/Boom/` | bản cài game |
| `unpacked_ca.exe` (18MB) | `~/boom-re/` | [Unpack](#1-unpack-themida) |
| IDA database `unpacked_ca.exe.i64` (283MB) | `~/boom-re/` | [IDA](#2-ida) |
| `assets/` (665MB) | `~/boom-re/assets/` | `python3 tools/idf.py extract ~/Downloads/Boom assets` (~5 phút) |
| `tools/unlicense.exe` | `~/boom-re/tools/` | [unlicense 0.4.0](https://github.com/ergrelet/unlicense/releases/tag/0.4.0) `py3.11-x86` |
| `ca_fixed.exe`, `ca_fixed_patched.exe` | `~/boom-re/` | `tools/rebuild_wine.sh` (cần `unpacked_ca.exe` + `decomp/iat_resolved.json`) |
| `wine/Boom/` | bản copy reflink của client để chạy Wine | `cp -a --reflink=auto ~/Downloads/Boom wine/` |
| `logs/live/` (77MB), `logs/exports.tsv` | dump bộ nhớ client gốc + export DLL | `win/live_dump.bat` trong WinBoat. **Chỉ cần nếu giải lại IAT**; kết quả đã lưu ở `decomp/iat_resolved.json` |
| `unlicense_ca.log` | log unpack (fix_iat dùng khi giải lại) | sinh ra khi unpack |

### Trạng thái môi trường (không bền — kiểm tra lại mỗi phiên)

- **WinBoat** (Windows trong docker `dockur/windows`): `docker ps`; port RDP **đổi sau mỗi lần restart container** → `docker port WinBoat`.
  User/pass trong `~/.winboat/docker-compose.yml`. Thư mục home của host = `\\host.lan\Data` trong Windows.
  Bị treo (RDP reset ngay) → `docker restart WinBoat`.
- **Server giả**: container `boomsrv` (`172.18.0.50` trên mạng `winboat_default`) nếu đang chạy; trên host thì `python3 tools/fakeserver.py --log logs/fakeserver_host.log` (nghe 3838/4848/5858/6868).
- **Luật iptables DNAT** trong container WinBoat (chuyển IP server cũ → `boomsrv`) **mất khi restart container**; lệnh ở [Chạy client](#chạy-client-với-server-giả).
- Host bật **ufw** → Windows guest không gọi được host; dùng container cùng mạng.
- **Wine**: CachyOS Wine 10 (chỉ WoW64 mới), prefix `~/.wine`, đã đặt `ShowCrashDialog=0`.
- **IDA MCP**: database mở bằng idalib; lần đầu mở mất ~70 phút auto-analysis, trong lúc đó mọi lệnh bị treo.

### 1. Unpack Themida

`ca.exe` bị pack Themida/WinLicense 2.x. Phải **chạy** nó trong Windows để dump.
```sh
docker port WinBoat        # lấy port 3389
xfreerdp3 /u:<USER> /p:<PASS> /v:127.0.0.1 /port:<PORT> /cert:ignore \
  '/app:program:C:\Windows\System32\cmd.exe,cmd:/c \\host.lan\Data\boom-re\win\unpack.bat'
```
`win/unpack.bat` copy client vào `C:\boom` (bỏ `*.idd`, `hackshield`, `blackcipher`), chạy `unlicense ca.exe`, chép `unpacked_ca.exe` + log về.
Mọi script `win/*.bat` chạy cùng cách (RemoteApp), ghi kết quả về `logs/`.

### 2. IDA

- Đã có `.i64` thì mở thẳng.
- Làm lại từ đầu: mở `unpacked_ca.exe` (~70 phút) rồi chạy `tools/ida_apply_names.py` (nạp `decomp/ida_names.json`).
- Sau khi đặt thêm tên: chạy `tools/ida_export_names.py` trong IDA → cập nhật `decomp/ida_names.json`, rồi commit.

---

## Chạy client với server giả

**Client gốc (Themida)** — mạng/đăng nhập chạy, không render (WinBoat không GPU):
```sh
docker run -d --rm --name boomsrv --network winboat_default --ip 172.18.0.50 \
  --user "$(id -u):$(id -g)" -v "$HOME/boom-re:/work" -w /work python:3.12-slim python -u tools/fakeserver.py --log /work/logs/fakeserver.log
docker exec WinBoat sh -c 'for d in 222.255.122.204 120.138.77.0/24 118.102.5.126 175.207.3.35; do
  iptables -t nat -I PREROUTING -s 172.30.0.0/24 -d $d -p tcp -j DNAT --to-destination 172.18.0.50; done'
xfreerdp3 ... '/app:program:C:\Windows\System32\cmd.exe,cmd:/c \\host.lan\Data\boom-re\win\watch2.bat'   # launch.ps1 patch bộ nhớ + chụp màn hình
```

**Bản dump đã sửa IAT** trong Wine (đang làm, xem `docs/research/findings/VN_DEVIRT_STATUS.md`):
```sh
python3 tools/fakeserver.py --log logs/fakeserver_host.log &   # IP đã patch thành 127.0.0.1
tools/rebuild_wine.sh
```

---

## Đã tìm được gì (tóm tắt)

### Client
- `ca.exe`: PE32 i386, build 2015-03-27; `v2.49/ca.exe` bản cũ hơn. PDB `M:\Source\VBNB_116\Binary\intl\Vietnam\client\CA.pdb`.
- MSVC C++ (CRT VS2008), boost, lua_tinker, TinyXML, zlib 1.1.4. RTTI: **2558 class** (`rtti.txt`).
- Phiên bản client = **116** (`App__GetVersion_116` 0x8E2494).
- Anti-cheat: HackShield (`hackshield/`), BlackCipher (`ngclient.aes`, `blackcipher/`), anti-cheat tự viết (quét process theo `cavn.ini`, `AntiCheat_ScanProcesses` ~0xA12C00), `cawh.sys` (thật ra là DLL hook cửa sổ của game).
- Log client: `CALog.log` trong thư mục game.

### Mạng (chi tiết `docs/specification/networking/LEGACY_VN_PROTOCOL.md`)
- Frame `[seq][len][payload][checksum]`, big-endian; client gửi XOR nối chuỗi (`seq ^ 0x53351D9C`, mỗi dword `-= 0x63CAACE3`) + CRC8.
- Seq: base 102/192, modif 231, đếm từ 40, +3 mỗi packet. Opcode = trường đầu payload.
- Server giả lập: `tools/fakeserver.py` (`hello_packet`, `login_replies`, `Session`).

### Tài nguyên (chi tiết `docs/specification/assets/PACKS.md`)
- Pack `fx`, `charte`, `emote`, `fxasset`; entry `Map/*`, `Attr/*` mã hoá (CRC32), `_mm` = zlib; sprite RLE + alpha.

### Địa chỉ quan trọng (`unpacked_ca.exe`)

| Địa chỉ | Tên | Ghi chú |
|---|---|---|
| 0x9E5EFE | OEP | |
| 0x8F2146 | `WinMain` | `CrashDlg` khi có C++ exception không bắt |
| 0x8E6FEE | `CFxApp__vf4_Init` | thứ tự khởi động, `docs/research/findings/VN_RUNTIME_REFERENCE.md` |
| 0x8E701D | jz HackShield/BlackCipher | patch `74 29` → `EB 29` |
| 0x8E426E | `PatchServer_CheckVersion` | kết nối patch, chờ hello |
| 0x8F280B / 0x8E4778 | `App__SetExitError` / `App__ShowExitError` | mã lỗi khởi động |
| 0x998FD5 | `CMultiServer__ctor` | danh sách IP/port |
| 0x8842DD | `CClientSocket__ctor` | seq params |
| 0xA662E0 / 0xA663E0 | `ProcessRecvPacket` / `SendPacket` | |
| 0xA64B70 / 0xA643F0 | `RecvPkt__Deserialize` / `SendPkt__Serialize` | |
| 0xA642E0 / 0xA64240 | `PacketDecryptXor` / `PacketEncryptXor` | |
| 0xE5CEE8 / 0xE5DEE8 | bảng tráo opcode enc / dec | |
| 0x999C02 | `Net_OnHandshake` | hello trên kết nối chính |
| 0x8E2DEC | `Login_DoLoginSequence` | opcode 0 → chờ 13,2,12,21,1,43 |
| 0x8F995D | `CMainWnd__OnPacket` | định tuyến packet |
| 0x6A978B | `CStage__OnPacket` | 191 opcode chung |
| 0xA388C0 / 0xA37880 | `PackFS__AttachIdf` / `CData__LoadFromPack` | pack |
| 0x7354FD | `CGamePlayMap__Load` | map (có khối bị ảo hoá) |
| 0x8F8EEC | `CStageMgr__ChangeStage` | bảng id → stage trong `docs/research/findings/VN_DEVIRT_STATUS.md` |
| 0x8F4D01 | vòng lặp chính (`CFxApp__vf5` gọi) | 3 khối VM lớn — đã viết lại (`docs/research/findings/VN_DEVIRT_STATUS.md`) |
| 0x11EE128 | VM entry Themida | `docs/research/findings/VN_DEVIRT_STATUS.md`; khối kết thúc ở marker `EB 10 57 4C 20` |
| 0xC0F000–0xC0F7FF | IAT gốc | `decomp/iat_resolved.json` |
| 0x1900000 / 0x16AD000 | IAT giả của unlicense | 3 mục sai, xem `docs/research/findings/VN_RUNTIME_REFERENCE.md` |

---

## Cấu trúc repo

| Đường dẫn | Nội dung |
|---|---|
| `docs/` | tài liệu (xem bảng đầu file) |
| `tools/boomproto.py` | codec packet (tự test khi chạy trực tiếp) |
| `tools/fakeserver.py` | server giả: đăng nhập → kênh → sảnh → phòng chờ → bắt đầu trận (biến `ROOM_TYPE`, `MAP_INDEX`, `AUTO_START`) |
| `tools/verify_emu.py` | chạy hàm crypto thật của binary bằng Unicorn, so với `boomproto.py` |
| `tools/idf.py`, `tools/i16.py` | đọc/giải nén pack, sprite → PNG |
| `tools/patch_client.py` | patch file: bỏ HackShield, bỏ patch-check, đổi IP |
| `tools/fix_iat.py` | giải IAT gốc (`report`/`export`) và build `ca_fixed.exe` (`build`) |
| `tools/rebuild_wine.sh` | fix_iat → devirt → patch → chạy bản dump trong winedbg |
| `tools/devirt.py` | khối VM viết lại / skip (`BLOCKS`, 15 khối) + `HOOKS` → `ca_devirt.exe` |
| `tools/vmdevirt.py` | devirt tổng quát (discover/profile/handlers/opcodes/trace) — `docs/research/findings/VM_METHOD.md` |
| `tools/match_gdb.sh`, `tools/gdb/*.gdb` | chạy cả trận (đăng nhập → phòng → 501… → phím + Space×3) dưới gdb; các script gdb theo chủ đề |
| `tools/battle_drive.ps1`, `tools/snap_retry.sh`, `win/snap_bisect*.bat` | tự động hoá client gốc trong WinBoat để chụp snapshot (`docs/research/findings/VN_RUNTIME_REFERENCE.md`) |
| `server/` | server Rust (`cargo run --release`, env như fakeserver + `SERVER_IP`) |
| `tools/vmtrace.py`, `tools/launcher/inject.c`, `win/vmtrace.bat` | quan sát một khối VM trên client gốc (cave + snapshot + diff) |
| `tools/launcher/snap.c`, `win/snap.bat` | snapshot toàn bộ bộ nhớ client gốc tại một site VM (pre/post) |
| `tools/vmemu.py`, `tools/snapdiff.py` | emulate khối VM trên snapshot (Unicorn), so với diff thật — quy trình trong `docs/research/findings/VN_DEVIRT_STATUS.md` |
| `tools/xvfb_join.sh` | đăng nhập → vào kênh → sảnh; `CLICKS=` click tiếp (tạo phòng…) + chụp màn hình |
| `tools/wine_gdb.sh` | chạy dưới `winedbg --gdb` + gdb, tự đăng nhập (`JOIN=1`, `CLICKS=`) |
| `tools/xvfb_login.sh`, `tools/xvfb_login_dbg.sh` | chạy dump trong Xvfb, tự gõ đăng nhập, chụp màn hình (bản `_dbg`: dưới winedbg) |
| `tools/run_xvfb.sh` | chạy bản dump trong Xvfb riêng + chụp màn hình |
| `tools/themida_scan.py` | liệt kê điểm vào section Themida → `decomp/themida_entries.json` |
| `tools/launch.ps1`, `tools/launcher/boomlaunch.c` | patch bộ nhớ client gốc (PowerShell / exe mingw) |
| `tools/live_dump.ps1`, `tools/dump_exports.ps1`, `tools/peek.ps1` | dump bộ nhớ / export DLL / đọc biến lúc chạy (PowerShell 32-bit) |
| `tools/ida_apply_names.py`, `tools/ida_export_names.py` | nạp / xuất `decomp/ida_names.json` (IDA) |
| `win/*.bat` | chạy trong Windows qua RDP RemoteApp: `unpack`, `watch2` (client gốc + patch + chụp màn hình), `live_dump`, `peek`, `run_*`, chẩn đoán (`diag`, `evlog`, `getcrash`, `status`) |
| `decomp/ida_names.json` | toàn bộ tên + comment đã đặt trong IDA |
| `decomp/iat_resolved.json` | IAT gốc đã giải (489 ô) |
| `decomp/themida_entries.json` | 117 điểm vào Themida + phân loại |
| `decomp/opcode_dispatch.json`, `decomp/handlers/`, `decomp/net/` | dispatch, pseudocode handler, hàm mạng + bảng opcode |
| `rtti.txt`, `ca_strings.txt` | RTTI, strings |

## Lưu ý / bẫy đã gặp

- Hằng số trong pseudocode Hex-Rays là **thập phân** — đổi hex cẩn thận (đã sai `1674226915` ≠ `0x63CA5EE3`; đúng là `0x63CAACE3`).
- Biến toàn cục = `0xFFFFFFFF` trong dump là chưa khởi tạo lúc OEP → đọc lúc runtime (`tools/peek.ps1`).
- `Map/*` dword thứ 2 = 2 → dễ nhận nhầm là sprite; nhận dạng bằng dword đầu = 1000/1001.
- Python `-I` không thêm thư mục script vào `sys.path` (tools đã tự xử lý).
- unlicense: không sửa IAT gốc; và **gán sai** những wrapper mà Themida giả lập chung nhiều API (xem `docs/research/findings/VN_RUNTIME_REFERENCE.md`).
- Dump IAT/export **phụ thuộc lần boot của Windows** (địa chỉ DLL hệ thống); kết quả đã chốt ở `decomp/iat_resolved.json`.
- QMP screendump chỉ chụp console, không thấy phiên RDP → chụp bằng PowerShell trong phiên.
- Lệnh `pkill -f <chuỗi>` có thể giết luôn shell đang chạy lệnh đó nếu chuỗi khớp chính nó.
