# Boom Online TCP protocol (VBNB_116 client)

Tìm từ `unpacked_ca.exe` (IDA: `unpacked_ca.exe.i64`). Code tham chiếu: `tools/boomproto.py`.

Độ tin cậy:
- ✅ **đã kiểm chứng**: chạy code thật của binary trong Unicorn và so với Python (`tools/verify_emu.py`).
- 🟢 **đọc từ decompile**: logic rõ ràng nhưng chưa chạy thử.
- ❓ **chưa chắc**.

## Lớp socket

| Class / hàm | Địa chỉ | Vai trò |
|---|---|---|
| `CClientSocket::Connect` | 0xA68850 | connect TCP |
| `CClientSocket::Read/Write` | 0xA69010 / 0xA69150 | `recv` / `send` |
| `CPacketSocket::RecvToBuffer` | 0xA66260 | đọc vào buffer |
| `CClientSocket::ProcessRecvPacket` | 0xA662E0 | kiểm seq byte → cắt frame → giải mã |
| `CPacketSocket::SendPacket` | 0xA663E0 | mã hoá → ghi seq byte → gửi |
| `CClientSocket::ReceivePacket` (vf20) | 0x884383 | wrapper, có chuỗi debug liệt kê tên trường |
| `Net_DispatchMainSocket` | 0x8F4094 | vòng nhận packet chính |
| `CMainWnd::OnPacket` | 0x8F995D | định tuyến packet |
| `Net_OnHandshake` | 0x999C02 | packet đầu tiên từ server |

Constructor `CClientSocket` (0x8842DD) đặt các trường:

| Trường | Tên (theo chuỗi debug) | Giá trị đầu |
|---|---|---|
| this[16] | header type | 0, server cấp qua handshake |
| this[18] / this[19] | base (r) / (s) | 102 / 192 |
| this[20] | modif | 231 |
| this[21] / this[22] | seq (r) / (s) | 40 / 40 |
| this[23] / this[24] | deltaseq (r) / (s) | 3 / 3 |

## Frame

```
[hdr: A byte][len: B byte][extra: C byte][payload ...][checksum: K byte]
```
Kích thước theo **header mode** (switch trong 0xA63EE0 / 0xA643F0 / 0xA64B70) — 🟢

| mode | A (hdr) | B (len) | C (extra) | type size |
|---|---|---|---|---|
| 0 | 1 | 2 | 0 | 1 |
| 1 | 1 | 4 | 0 | 1 |
| 2 | 2 | 2 | 0 | 2 |
| 3 | 2 | 4 | 0 | 2 |
| 4 | 1 | 2 | 1 | 1 |
| 5 | 1 | 4 | 4 | 1 |

`len` = độ dài payload + C. Mọi số nguyên đều **big-endian**.

**Checksum mode**:
- 0 → không mã hoá, không checksum; byte type đầu payload được tráo bằng bảng opcode (dưới).
- 1 → payload mã hoá XOR, checksum CRC32 (4 byte), `len` bị XOR-obfuscate.
- 2 → như 1 nhưng checksum CRC8 (1 byte).

Mode client dùng — ✅ (đọc từ bộ nhớ client đang chạy, `tools/peek.ps1`):

| Chiều | (cks, hdr) | Nguồn |
|---|---|---|
| Client → server | (2, 2) | `0xE5794C/50`, `0xE47F54/58` |
| Server → client, packet đầu (hello) | (0, 0) | `dword_EEC1BC/C0` (kết nối chính), `dword_EA6FE0/E4` (kết nối patch) — đều = 0 lúc runtime |
| Server → client, sau hello | (2, 3) | `0xE57944/48` |

## Seq byte (byte đầu header) — ✅ (khớp traffic thật)

Gửi (`MakeSeqByte` 0xA660A0):
```
b = deltaseq_s ? ((type + modif) ^ (base_s + seq_s)) & 0xFF : base_s
seq_s += deltaseq_s       // sau mỗi packet
```
Nhận (`CheckSeqByte` 0xA66030):
```
if chưa có header type:  type = ((b ^ (base_r + seq_r)) - modif) & 0xFF
else kiểm b == deltaseq_r ? ((type + modif) ^ (base_r + seq_r)) : base_r
seq_r += deltaseq_r
```
Với header mode 2/3 (A = 2) chỉ byte đầu được ghi → ❓ byte thứ 2.

## Mã hoá payload — ✅

`PacketEncryptXor` 0xA64240 / `PacketDecryptXor` 0xA642E0, key = seq hiện tại (seq_s khi gửi, seq_r khi nhận):
```
k = key ^ 0x53351D9C
c[0] = k ^ p[0]
for i >= 1:  k -= 0x63CAACE3;  c[i] = k ^ p[i] ^ p[i-1]     (dword little-endian)
byte lẻ cuối: c = p ^ (p[last_dword] >> 8*j)   (nếu không có dword nào: dùng k ban đầu)
```
Checksum tính trên **plaintext**, giá trị khởi đầu = key:
- CRC32: poly 0x04C11DB7, MSB-first, `crc = T[b ^ (crc>>24)] ^ (crc<<8)`, ghi big-endian.
- CRC8: poly 0x07, `crc = T[crc ^ b]`.

## Obfuscate số nguyên (mode 1/2) — ✅ (khớp traffic thật)

Khi checksum mode là 1/2, `len` và **mọi field** ghi bằng `Encode1/2/4` đều bị XOR:
- u8 `^ 0x5A`
- u16 `^ 0xA569`
- u32 `^ 0x96CA5395`

Chuỗi = u16 độ dài + byte, trong đó phần byte được XOR theo từng khối 4/2/1 (0xA26A10).

## Bảng tráo opcode (mode 0) — ✅ nghịch đảo nhau

16 hàng × 256 byte, hàng chọn theo header type:
- encode `g_OpcodeSubstEncTable` 0xE5CEE8
- decode `g_OpcodeSubstDecTable` 0xE5DEE8
- Dump: `decomp/net/opcode_{enc,dec}_table.bin`

## Payload — ✅

```
[opcode: 1|2 byte = "type size" của mode][fields...]
```
"Type" **chính là opcode**: dispatcher gọi `RecvPkt__Rewind` rồi `RecvPkt__Decode2` → đọc lại đúng trường đầu tiên.
Ở mode 2/3 opcode là u16 (XOR `0xA569`), ở mode 0 là u8 và được tráo qua bảng opcode.
Đọc bằng `RecvPkt__Decode1/2/4/8/Str/Buf` (0xA65670…), ghi bằng `SendPkt__Encode*` (0xA65220…).

## Hello (packet đầu từ server, mọi kết nối) — ✅ chạy được với client thật

Server gửi ngay khi client kết nối; client không gửi gì trước.
Cùng một định dạng cho kết nối patch (`PatchServer_CheckVersion` 0x8E426E) và kết nối chính (`Net_OnHandshake` 0x999C02):
```
format (0,0): [seq u8][len u16 BE][opcode u8 = 0 (tráo bảng)]
  u16 ?            (0)
  u16 version      phải = 116 (sub_8E2494); khác 116 → client chạy autoupdate.exe rồi thoát
  u32 header_type  hàng bảng tráo opcode (0..15) → SetHeaderType
  str ?            (u16 len + byte)
  u8  flag         1 → thêm: u8, str, u32, u8 (sub_8E3AA0)
```
Seq byte cho header_type `t`: `((t + 231) ^ (102 + 40)) & 0xFF` (t=0 → `0x69`).
Sau hello, phía nhận của client chuyển sang (2,3) và `seq_r = 43`.

Kết nối patch: nhận hello hợp lệ → client tự đóng socket (~0,1s) rồi mở kết nối chính tới server khác trong danh sách.

## Đăng nhập — ✅ chạy được với client thật

`Login_DoLoginSequence` 0x8E2DEC (gọi sau hello trên kết nối chính):

1. Client gửi **opcode 0** + 6×u32: `sub_65419B(E61024)`, `sub_98D908(..)`, `*(E60FBC+92)`, `sub_8EE552(E60A0C)`, `sub_9967C4(CMultiServer)`, `sub_98131D(E64B10)`.
   Chạy không qua launcher của VNG thì cả 6 đều = 0. Ví dụ packet thật:
   `0f f4 a5 73 | 11 74 a3 99 … 00 00 | 96` → hdr `0f f4`, len 26, payload, CRC8 0x96.
2. Server phải trả **đúng thứ tự**, mỗi packet tối đa 10 giây:

| Opcode | Handler | Nội dung | Tối thiểu |
|---|---|---|---|
| 13 | `CStage__On_13` 0x654ADE | u8 có dữ liệu?, blob → `fxdynimginfo.ssd` | `u8 0` |
| 2 | 0x98DFAB | u8, blob FxClientString | `u8 0` (dùng cache) |
| 12 | 0x9D3F17 | u8, blob FxQuestString | `u8 0` |
| 21 | 0x8F13E6 | u8, blob FxMessageTable | `u8 0` |
| 1 | 0x99933B | u8, blob `multisvr.ssd`; u8, blob `multisvrn.ssd` | `u8 0, u8 0` |
| 43 | 0x982A90 | u8, blob | `u8 0` |

Nhận đủ → hàm trả 2, cửa sổ game "Boom" mở; màn Login (xem dưới).

**Opcode 1 — danh sách server** (`sub_99933B`): mỗi phần là u8 cờ (1 = có dữ liệu mới, client ghi cache ra file) + blob.
Không có file `multisvr*.ssd` trong thư mục game → gửi `u8 0, u8 0` = dùng danh sách hardcode trong `CMultiServer__ctor`
(nhóm "Default Server List"; endpoint 120.138.77.x / 222.255.122.204 × 3838/4848/5858/6868, đã patch IP thành 127.0.0.1).
- blob `multisvr.ssd` (`sub_998CCF`): u8 `n` × {str, str, u32} (nhóm, `sub_9986F3`), u16 `m` × {u32 in_addr, u16 port, u8} (endpoint,
  `sub_998760`; mặc định u8 = 1), u8 cờ (`CMultiServer+0x3C`).
- blob `multisvrn.ssd` (`sub_998EA0`): u8 `n` × {u8, str, str}.
- ⚠️ Gửi `multisvr_blob()` (1 nhóm, 4 endpoint 127.0.0.1 với u8 = 1) → client **không mở kết nối chính**, hiện `ErrCannotLogin`.
  Còn thiếu ý nghĩa trường → `tools/fakeserver.py` tạm để `u8 0`. Nhớ xoá `wine/Boom/multisvr*.ssd` sau khi thử (client đọc cache).
Cài đặt mẫu: `tools/fakeserver.py` (`hello_packet`, `login_replies`, `Session`).

### Đăng nhập bằng tài khoản (màn Login) — ✅ chạy được với bản dump
- Client gửi **opcode 1**: u8 số người chơi, str id, str mật khẩu (cả hai còn một lớp mã hoá riêng), 6×u32 (gồm IP LAN). Kèm opcode 58.
- Server trả **opcode 4** (`CLoginStage__On_4` 0x8534F3): u8 kết quả (0 ok, 1 chưa đăng ký…); khi 0 → `sub_852BCA`:
  u8, u16, u8 `n` (= số người chơi 1P/2P), u64, `n` × ~40 trường (u32/u16/u8/str, thứ tự trong `login_ok()`), rồi 6 trường chung
  (cuối = cờ "đổi mật khẩu"). Client trả opcode 9 (u32).
- Server trả **opcode 15** (`CLoginStage__On_15`): blob {str host, 3×u16, u8 mode, u16} → hộp thoại chờ `CWaitForLoginDlg` đổi
  trạng thái 2→1→3 → chuyển **stage 10** (chọn kênh). Không có 15 → hết giờ → `ErrBadNetwork`.
- Rời màn Login (`sub_84FABC`): đặt tên file cache kênh `channel2_%03d%s.ssd` (`sub_993EF4`), client gửi opcode 7 (lớn, `sub_98E763`),
  rồi `sub_8F391D(10)`.

### Màn chọn kênh (stage 10, `CSelectServerStage`) — ✅ vào được, hiện danh sách (slot trống, chưa rõ ngữ nghĩa field)
- Vào stage, client gửi opcode 298 (str id đã mã hoá + u8) và **opcode 40** (u8 0). Server giả trả 14 + 16 khi nhận 40.
- Handler của stage: 7 (kết quả vào kênh: u8 mã lỗi 0/2/4/5/11..22), 16 (trạng thái kênh), 19 (chỉ thành viên guild), 41.
- **Opcode 14 — danh sách kênh** (`CStage__On_14` 0x99514B → `sub_99489D`, object `CChannelCache` = `dword_E61010`):
  u8 cờ + blob; cờ 1 → `RecvPkt__DecodeBlob` vào sub-reader `v5` → parse `sub_994800(this=cache, a2=v5)` rồi
  ghi cache ra file kênh (`sub_A62C20`, tên file chỉ có sau `sub_993EF4` → **phải gửi sau khi vào stage 10**;
  gửi sớm hơn thì ghi file thất bại → `CAppException` → client ngắt kết nối). Cờ 0 → đọc cache (`sub_A62D10`).
  - Blob inner **phải bắt đầu bằng u8 tag** (`sub_A62580` 0xA62580): 0 = copy thô (`memcpy`, `end = len-1`),
    1 = giải mã `sub_A0B480`; tag khác → sub-reader rỗng (base/end/pos = 0) mà không báo lỗi.
    Gửi thiếu tag (byte đầu = count `04 00…`) là nguyên nhân crash trước đây: count `sub_A62F70` đọc trên reader
    rỗng trả về `(u16)v5` (≈ 0xDE44 = 56868, quan sát `v5 = 0x11de44` rỗng tại entry `sub_994663`) → vòng lặp
    `v6 < count` tràn mảng 1024 bản ghi trong `CChannelCache` → crash trong helper field (`sub_414059` 0x4140C5 /
    `sub_414059+` 0x41418B, `movb %al,1(%ecx)` / `2(%ecx)` với ECX rác). Đã sửa: `channel_list()` thêm `b"\x00"`.
  - Sau tag: u16 `n` LE (`sub_A62F70`), `n` bản ghi (`sub_994663`, mỗi bản ghi stride 58 dword = 0xE8 byte,
    slot đầu = cache+20; `CChannelCache` malloc 0x4011C ở `CFxApp__InitSystems` 0x8E5DB9): tên str
    (u32 LE độ dài + byte, qua `sub_410655` → `sub_A63A40`), u32 (`sub_A62FB0` + `sub_413F32` vào field +0x1C),
    u8×7 (`sub_A62F40` + `sub_414059` vào +0x28,+0x34,+0xD0,+0x40,+0x4C,+0x58,+0x64), u16×2 (`sub_A62F70` +
    `sub_41411C` vào +0x70,+0x7C), u8×4 (vào +0x88,+0x94,+0xA0,+0xAC), u32×2 (vào +0xB8,+0xC4).
    Bố cục đọc từ disasm `sub_994663` (0x994663–0x9947DB); giá trị các field chưa rõ (server giả gửi 0 trừ tên + id).
  - Parse xong → ghi `channel2_001.ssd` (`"ssi1"` + zlib, 56 byte với 4 kênh mẫu) → `CStage__On_14` duyệt kênh
    qua `sub_9950DA` vào `CChannelInfo` (`dword_E60FD8`). Client ở lại stage 10, keep-alive opcode 15 đều.
  - ⚠️ Slot kênh hiện **trống** trên UI (đã nhận đủ 4 kênh + status, không crash). Nghi một field
    trong bản ghi là trạng thái/hiển thị (đang gửi 0), hoặc id status (0–3) phải khớp id bản ghi (1–4).
- **Opcode 16 — trạng thái kênh** (`CSelectServerStage__On_16` 0x854ADC, guard `sub_994A1D(CChannelInfo)` ≠ 0):
  blob **cũng cần tag** (`DecodeBlob` → `v6` + `sub_A62580`): u16 `n`, `n` × (u16, u32 LE) → `sub_9949C6(idx, a, b)`
  (`0x9949C6`: `ESI = idx*12 + ECX_base`, `sub_413F32(ESI+0x3A018, a)`, `sub_413F32(ESI+0x3D018, b)`).
  Thiếu tag → count rác (quan sát `EBX = 0x401` = vòng lặp chạy quá 1024) → crash `sub_413F32` (0x413FA1,
  `movb %al,4(%ecx)`, ECX = 0). Đã sửa: `channel_status()` thêm `b"\x00"`.

### Kiểu dữ liệu (mode 1/2)
- u64: 8 byte BE, XOR `0x0000000071B18589` (`RecvPkt__Decode8` 0xA26980).
- str: u16 độ dài + byte obfuscate theo khối 4/2/1: khối đọc little-endian, XOR khoá theo kích thước, ghi big-endian (0xA26A10).
- blob (`RecvPkt__DecodeBlob`): u32 N + N byte (obfuscate như str) = `[0x18][u32 LE len][dữ liệu]`; dữ liệu đọc **thô little-endian**
  (u8, u16, str = u32 LE độ dài + byte). `boomproto.put_blob`.

## Dispatch

`CMainWnd::OnPacket` lần lượt thử:
1. `sub_8F97CB` → factory `dword_EA78E8[opcode]` (packet template `enGameServerPacket`, đăng ký lúc runtime)
2. `sub_8F9796` → factory `dword_E64B20[opcode]` (`enGeneralServerPacket`)
3. `sub_694646`
4. `stage->OnPacket` (vf51) của stage hiện tại; stage con gọi tiếp `CStage::OnPacket` (0x6A978B, 191 opcode)
5. `sub_8F939A`

Bảng opcode → handler (520 opcode, 59 dispatcher): `decomp/opcode_dispatch.json`.
Handler được đặt tên dạng `<Stage>__On_<opcode>` trong IDA. Pseudocode của các stage: `decomp/handlers/`.

## Kiểm chứng

- Mã hoá/giải mã/CRC: emulate hàm thật của binary (`tools/verify_emu.py`) — 88/88 khớp.
- Phía client → server: giải mã packet thật của client, CRC8 khớp.
- Phía server → client: client thật chấp nhận hello + 6 packet login do `fakeserver.py` tạo.

## Câu hỏi còn mở

- Byte thứ 2 của header khi A = 2: client gửi `0xF4`; khi nhận client không kiểm tra (server gửi 0 vẫn được).
- Ý nghĩa 6×u32 trong opcode 0 (có lẽ session/account từ launcher VNG).
- Ý nghĩa các trường trong gói đăng nhập (opcode 4), bản ghi kênh (opcode 14), endpoint multisvr.
- Opcode 14 crash (xem trên); lobby, phòng, ván chơi.
