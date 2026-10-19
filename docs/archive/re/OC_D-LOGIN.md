> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Đặc tả opcode 4 (kết quả đăng nhập) & opcode 15 (thông tin mạng) — phân tích tĩnh

Nguồn: IDA Pro, DB `ida/d-login.i64` (bản copy riêng của nhiệm vụ, unwrap của `unpacked_ca.exe` VNG
VBNB_116 nhưng có symbols/handler-adapter khác bản China — thấy `CLoginStageChina__On_4` 0x84DE0E
tồn tại song song). Chỉ phân tích tĩnh; mọi địa chỉ là ground truth của bản VNG này.

Quy ước đọc: ✅ = đã xem disasm chính xác (offset byte đã chuẩn, thứ tự field đã chốt);
🖊️ = chỉ dựa decompile (offset có thể lệch kiểu nhưng đã đối chiếu disasm khi ghi cụ thể);
❓ = ngữ nghĩa chưa giải (CHƯA KIỂ). Người gửi/trả: op 4 là S→C, op 15 cũng S→C.

Định danh đối tượng toàn cục dùng trong bài:

| Ký hiệu | Ý nghĩa |
| --- | --- |
| `E60880` | stage/DB người chơi chính của màn Login–Lobby (`CLobbyStage__On_218(E60880)` = số người chơi 1P/2P, toàn bộ UI đọc getter trên đây) |
| `E60848` | store "động" theo người chơi (`sub_9B64F7` ghi vtable singleton `CUserDynamicInfo`) |
| `E623D4` | singleton thứ ba, 2 slot (`i ≤ 1`) dùng khi lọc dải giá trị ở lobby |
| `E60E54` | `CShareContext` (đích các trường đuôi gói) |
| `E64AEC` | vùng 16 byte chứa u64 đầu gói + dấu thời gian `{low, high, timeGetTime(), 1}` |
| `E60928`, `E608C4`, `E61004`, `E61010` | GUI/stage phụ (`sub_8DDB82` Lastselectedid; dialog chờ; store hai danh sách đuôi; đích opcode 9) |

## 1. Cơ chế lưu trữ

- **Value-box**: giá trị nhận từ gói S→C (và phần lớn dữ liệu người chơi) không ghi thẳng vào field
  thường mà qua setter box:
  - u32-box 12 byte — `sub_413F32` / `sub_44AD23` (disasm thấy `memcpy` 12 byte, tag/cksum
    tạo bởi `sub_A21D00`, cks 16-bit quay vòng `0x9A65`);
  - u8-box 6 byte — `sub_414059` / `sub_4D6096` (disasm `new 6`);
  - u16-box — `sub_41411C`.
  Getter: `sub_414543` (u32 từ box), `sub_414625` (u8/u16 từ box). Offset box ghi ở bài là **byte
  offset** dạng `12i + C` (i = chỉ số người chơi) hoặc `(i + N)*12` (tương đương; disasm hay gặp
  `add ecx,N; imul 0Ch` — chú ý một số hàm `imul` **trước** `lea +C`, khi đó offset là `12i + C`).
- **Chuỗi an toàn** (tên / str#2 / str#3): mỗi slot 16 byte {…, ptr mảng box-ký-tự (+4), cache
  char* (+8), độ dài (+12)}; `sub_911168 → sub_9110BB` nạp từng ký tự vào một box riêng
  (`sub_4D6096`, 12 byte/ký tự); getter `sub_9112A8` ghép lại thành `char*` (cache ở +8, trả
  `&szProxyBypass` nếu rỗng). Ba vị trí:
  - **tên**: set `sub_99A746` / get `sub_99A731` → slot `E60880 + 0x18 + 0x10i` (✅ disasm);
  - **str#2**: set `sub_99A817` / get `sub_99A832` → slot `+0x244 + 0x10i` (✅ disasm);
  - **str#3**: set `sub_99A783` → slot `+0x224 + 0x10i` (✅ disasm); getter tương ứng
    (`+0x224`, `sub_9112A8`) **không tìm thấy** trong DB ❓.
- Box + 0x118: `sub_99AC36(i)` trả `char*` của một std::string 28 byte tại
  `E60880 + 0x118 + 0x1Ci` (SSO, capacity ở +20) — xem mục 3.4.

## 2. Pipeline opcode 4

`CLoginStage__On_4` 0x8534F3 → `sub_852BCA` 0x852BCA (bản VNG — chuẩn wire; bản China
`CLoginStageChina__On_4` 0x84DE0E có layout khác: thêm 1 str đầu, 3 str + 1 u16 mỗi player,
không có u64/sub_99C99F/sub_99C4CB — chỉ dùng đối chiếu ngữ nghĩa).

`u8 result` đầu gói được `sub_99C389` lưu vào `CShareContext+264` (chỉ lưu). Khi
result ≠ 0 nhánh lỗi (mục 5). Result == 0 → parse như dưới.

### 2.1 Đầu gói

| # | Kiểu | Handler | Đích | Ghi chú |
| --- | --- | --- | --- | --- |
| H1 | u8 **result** | `sub_852BCA` đọc, `sub_99C389` → `CShareContext+264` | — | 0 = đi vào phần parse đầy đủ; ✅ |
| H2 | u16 | `sub_99C497` → `CShareContext+228` | getter `sub_99C48F`; ==0 → msg `aForbiddenauthe` ❓ | ✅ |
| H3 | u8 **playerCount** | phải == `CLobbyStage__On_218(E60880)` | lệch → dialog `aErrloginunknow` (`ErrLoginUnknown`, tham số 0x4C/0xA1/0x10000000), thoát | ✅ |
| H4 | u64 | `RecvPkt__Decode8` → `sub_6ABB29(dword_E64AEC, lo, hi)` | object tĩnh `dword_E64AEC`: `{low, high, timeGetTime(), 1}` — người đọc object này ❓ | ✅ |

### 2.2 Mỗi người chơi i = 0..playerCount-1 (31 field, thứ tự disasm tuyệt đối)

| # | Kiểu | Handler (địa chỉ) | Đích lưu | Getter / nơi đọc | Ý nghĩa | Độ tin cậy |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | u32 | `sub_9B698F` (0x9B698F→0x44AD23) | `E60848` u32-box `12i+0xC0` | — | ❓ | ✅ offset |
| 2 | u32 | `sub_9B69AB` (→0x44AD23) | `E60848` u32-box `12i+0xD8` | — | ❓ | ✅ |
| 3 | u16 | `sub_9B69AB` (→0x41411C) | `E60848` u16-box `12i+0xF0` | — | ❓ | ✅ |
| 4 | u16 | `sub_9B69AB` (→0x41411C) | `E60848` u16-box `12i+0x108` | — | ❓ | ✅ |
| 5 | u32 | `sub_99A71D` | dword `E60880 + 0x168 + 4i` | `sub_99A70C`; find-by-id 0x4690FB / 0x8405A7; gửi lại trong **opcode 6** (`sub_999FE6`/`sub_99A11E`) | **id người chơi** | ✅ |
| 6 | str | `sub_99A746` | slot chuỗi `E60880 + 0x18 + 0x10i` | `sub_99A731`; tên dùng trong op 6, cấu hình `Lastselectedid%02d` (`sub_8DDB82`/`sub_8DDBEC`), chặn whisper `sub_8DF209`/`sub_8DF30B`, yêu cầu vào guild "op 367" @0x83F053 | **tên hiển thị** | ✅ |
| 7 | u8 | `sub_99AC50` (→0x413F32) | u32-box `E60880 12i+0x150` | — | head/"chỉ số MultiServer": `i==0` && feature 212 (stub không tắt ⇒ luôn xảy ra) → `sub_900AF1(tên)` + `sub_900E2E(head)` → `sub_900D8C` nạp file multiserver qua `sub_AAD577` | ❓ (cơ chế nạp ✅) |
| 8 | str | `sub_99A817` | slot `+0x244 + 0x10i` | `sub_99A832` → đọc trong `sub_99BB9B` (guard `sub_9B6F75(E60848,3)`) và `sub_6A22E6` (CBuyItemListDlg) | ❓ | ✅ |
| 9 | str | `sub_99A783` | slot `+0x224 + 0x10i` | (getter không tìm thấy) | ❓ | ✅ offset |
| 10 | u8 | `sub_99A84A` | dword `+0x264 + 4i` | `sub_99A85E` (dword idx 153) | ❓ | ✅ |
| 11 | u32 | `sub_9B68EB` | `E60848` u32-box `12i+0x50` **và** u8-box `12i+0x38` = **value/24** | — | ❓ (v/24 → đoán thời hạn tính theo giá trị 24-đơn vị) | ✅ |
| 12 | u8 | `sub_9B6B6B` (→0x413F32) | u32-box `E60848 12i+0x68` | — | ❓ | ✅ |
| 13 | u8 | `sub_9B6B84` (→0x413F32) | u32-box `E60848 12i+0x4C8` | — | ❓ | ✅ |
| 14 | u16 | `sub_9B68CF` (→0x41411C) | u16-box `E60848 12i+0x12C` | — | ❓ | ✅ |
| 15 | u8 | `sub_9B619D` (i≤1, →0x414059) | u8-box `E623D4 12i+0x1C` | `sub_9B6233` (+ tiền điều kiện `sub_9B5F8A`) | ❓ | ✅ |
| 16 | u8 | `sub_9B61F7` (i≤1, →0x414059) | u8-box `E623D4 12i+0x64` | `sub_9B6302`; đọc trong `sub_66F4E4`/`sub_8DAFD7`: đối chiếu khoảng [min,max] lấy từ record thông tin qua `sub_994BBE`/`sub_994BDA` (gốc `sub_9995C6(dword_E60784)`) | ❓ (khả năng là giá trị bị lọc theo dải — phòng/kênh) | ✅ |
| 17 | u32 | `sub_9B6A92` (→0x413F32) | u32-box `E60848 12i+0x2D8` | — | ❓ | ✅ |
| 18 | u32 | `sub_9B6AAE` (→0x413F32) | u32-box `E60848 12i+0x2F4` | — | ❓ | ✅ |
| 19 | u8 | `sub_9B6A3E` (→0x414059) | u8-box `E60848 12i+0x1D4` | — | ❓ | ✅ |
| 20 | u8 | `sub_9B6A5A` (→0x414059) | u8-box `E60848 12i+0x228` | — | ❓ | ✅ |
| 21 | u8 | `sub_9B6A76` (→0x4D6096) | u8-box `E60848 (i+0x2B)*12` = `12i+0x204` | — | ❓ | ✅ |
| 22 | u16 | `sub_99AD91` (→0x413F32) | u32-box `E60880 12i+0x1B8` | — | ❓ (tính năng 6; stub luôn "không tắt" ⇒ luôn đọc) | ✅ |
| 23 | u16 | `sub_99AD75` (→0x413F32) | u32-box `E60880 12i+0x1A0` | — | ❓ (như trên) | ✅ |
| 24 | u32 | `sub_99AC6C` (→0x413F32) | u32-box `E60880 12i+0x170` | `sub_99AF0F` (12i+0x170); logic "AlreadyInGuild" 0x8AA212; "NotGuildMaster" qua `sub_99B18B` (bậc, 3=p trưởng); gửi op 362 {bool, guild id} @0x83C00D; theo guild "op 367" @0x83F053 | **guild id** (0 = không có guild) | ✅ |
| 25 | u8 | `sub_99ACCF` (→0x414059) | u8-box `E60880 12i+0x278` | — | ❓ | ✅ |
| 26 | u8 | `sub_99ACEB` (→0x414059) | u8-box `E60880 12i+0x290` | — | ❓ | ✅ |
| 27 | u32 | `sub_99ACB3` (→0x413F32) | u32-box `E60880 (i+0x27)*12 = 12i+0x1D4` | — | ❓ | ✅ |
| 28 | u8 | `sub_99ADAD` (→0x414059) | u8-box `E60880 (i+0x2B)*12 = 12i+0x204` | — | ❓ | ✅ |
| 29 | u8 | `sub_99A963` | byte `E60880 + 0x470` | — | ❓ (⛔ KHÔNG theo i — mỗi vòng lặp ghi đè cùng byte) | ✅ |
| 30 | u32 count → count × (u32, u32) | `sub_6A3ACE` (push cặp vào vector) → `sub_9B7FE0(i, vec)` (i≤1) → `sub_9B7B6B` | gán vector cặp {key, value} vào slot i của store trên `E60848` | `CStage__On_369` (0x6A448D) cập nhật từng cặp: key = box `E60848 12i+0x258` % 10 (`sub_9B72D1`), value mới → `sub_413F32(obj = sub_679EDD(i) +196)` | ❓ (map trạng thái key→value) | ✅ cơ chế |
| 31 | u8 | `sub_99A9C4` | dword `E60880 + 0x47C + 4i` | — | ❓ | ✅ |

Sau field 31 cho từng i còn chạy phần "sổ tay" ngoài wire: `sub_928BCD(i)` (object `E60888
+ 0x80i + 0x10` → `sub_928659` reset); tên (`sub_99A731`) và chuỗi `sub_99AC36(i)`
(`+0x118+0x1Ci`) → khi một dword kiểm tra (`[object tại var_C8] + 0x21C`) != 0 →
`sub_8DDB82(i+1, …)` ghi cấu hình
`Lastselectedid%02d` với i+1 — mọi thứ chỉ nội bộ config, không thêm field wire ❓.

### 2.3 Đuôi gói (trước và sau opcode 9)

| # | Kiểu | Handler | Đích | Ghi chú |
| --- | --- | --- | --- | --- |
| T1 | (không wire) | `push var_CC; sub_99ACA4(E60880, v67)` | box `+620` (E60880) | v67 (var_CC) khởi 1, bị `and` về 0 khi (i) playerCount sai (đầu gói) hoặc (ii) `sub_99AF0F(i) == 0` với bất kỳ i; box này lưu "toàn bộ hợp lệ" ✅ |
| T2 | u8 | `sub_99C3A7` | `CShareContext+268` | getter `sub_99C3A0` dùng trong `sub_8E28F4` (toggle khi rời màn Login) ❓ |
| T3 | u8 | `sub_99C99F` | `CShareContext+340` | getter chưa tìm ❓ |
| T4 | u8 | `sub_99C3BE` | `CShareContext+352` (byte) | getter `sub_99C3B7` dùng trong `sub_73849B` / CHiddenCatchPrepareStage ❓ |
| T5 | u16 | `sub_99C42A` | `CShareContext+452` | getter `sub_99C43C` trong `CWaitForConnSvrDlg__vf36` 0x859FA4 ❓ |
| T6 | u8 | `sub_99C4CB` | `CShareContext+456` | **số tab category màn chọn kênh** (builder `sub_854A7D`; tab đầu phải tồn tại để danh sách ô kênh dựng slot) ✅ |
| T7 | u8 | `sub_852BCA` | — | cờ "đổi mật khẩu": ≠0 → local `"NeedToChangePassword"` → `sub_98DB56` → `sub_93E801`/`sub_467AFB` (dialog, object `dword_E608C4` → `sub_42F17C` nếu có) ✅ |

Sau T7, client **gửi return-opcode 9**: `SendPkt__Begin(9)` + `SendPkt__Encode4(sub_993F73(dword_E61010))`
(= giá trị box tại `*(E61010+3)`) rồi `SendPkt__Send` ✅; `sub_612CB1(this + 0x1BC0)` đổi trạng thái
wait-dialog (2 → chạy) trước đó ngay tại 0x853401 ✅.

Và client đọc **tiếp tục hai danh sách cuối cùng trong cùng gói op 4** (sau opcode 9):

| # | Đọc khi | Handler | Đích | shape |
| --- | --- | --- | --- | --- |
| T8 | luôn (`sub_943275` = !`Feature_IsDisabled_stub0` ⇒ true) | `sub_943114` → `sub_98889C(packet)` | `dword_E61004` (đọc tại +4 và +168) | u32 count; count × (u32, u16, u8, str, u8) ❓ |
| T9 | feature 47 (`0x2F`) — stub ⇒ luôn | `sub_943147` → `sub_411CAE(packet)` | `dword_E61004` | cùng shape ❓ |

Khuyến nghị: fakeserver nên append `u32 0` ×2 sau field T7 (hai danh sách rỗng) — client hiện đọc vào
EOF (helper decode trả 0) nên chưa crash nhưng mang rủi ro.

## 3. Các ghi chú bổ sung

### 3.1 Sửa sai lệch ghi trước

- Trước đây ghi "box i+0x27 và i+0x2B bị ghi 2 lần (ghi đè)". Thực ra các setter cùng **chỉ số
  công thức** nhưng khác **object**: `sub_9B6A3E` (u8-box, `E60848` 12i+0x1D4) vs `sub_99ACB3`
  (u32-box, `E60880` 12i+0x1D4); `sub_9B6A76` (u8, `E60848` 12i+0x204) vs `sub_99ADAD`
  (u8, `E60880` 12i+0x204). Không có ghi đè cùng ô.
- Offset hai field E623D4 sửa thành byte `12i+0x1C` và `12i+0x64` (không phải "(i+7)*12 / (i+25)*12"
  như ghi cũ — disasm `imul` **trước** `lea +1Ch/+64h` cho `sub_9B619D`/`sub_9B61F7`/getters).

### 3.2 Trật tự hai u16 tính năng 6

`sub_99AD91` (đích `12i+0x1B8`) đọc **trước** `sub_99AD75` (`12i+0x1A0`) — `fakeserver` gửi cả hai
= 0 nên hoán vị không đổi hành vi.

### 3.3 value/24 của field 11

`sub_9B68EB(v)`; u32-box giữ v, u8-box giữ v/24. Gợi ý: 24 là số giờ → đoán "thời gian còn lại" hoặc
"giờ chờ làm mới"; chưa tìm getter UI ❓.

### 3.4 Slot chuỗi +0x118 (`sub_99AC36`)

Mỗi người chơi một std::string 28 byte tại `E60880 + 0x118 + 0x1Ci`. Các nơi đọc đã thấy:
- gửi trong **opcode 6** (`sub_999DBB` 0x999DBB: `EncodeWStr(tên)` rồi `EncodeWStr(slot+0x118)`
  cho mỗi player khi `sub_8DE2A6(E60928) == 0`, kèm `SendPkt__EncodeBuf` 16 byte + 2×u32 …);
- gửi trong **opcode 34** (`sub_84A092` 0x84A092 — luồng danh sách mua đồ, opcode 1024/1025);
- trong chính op 4: đọc lại để ghi config `Lastselectedid%02d`.

Nguồn ghi slot này (bên gửi chuỗi thứ hai) **chưa xác định** ❓; đoán là "tên từ tùy chọn hiển thị".

### 3.5 CShareContext+268 trong sub_8E28F4

`sub_8E28F4(frame)`: nếu `sub_99C3A0(E60E54) != 0` → `sub_8DCB9E(&v2,&v3)`; khi `v2 != 0` và
`On_218(E60880) == 1 && v3 == 0` → `v3=1`, `sub_8DCBBB(v2, 1)`; trả `sub_9A3EC9(v3 == 0)`.
Ngược lại (getter ==0) → lấy HWND `**(E60A08+8)` → `sub_9A21A6(hwnd)` (ẩn/hủy window).
Diễn giải thô: cờ T2 cho phép tiếp tục khởi động luồng lobby sau khi rời màn Login ❓.

## 4. Opcode 15 (S→C, thông tin mạng) — đã xác minh lại với IDA ✅

`CLoginStage__On_15` 0x84FE35: `RecvPkt__DecodeBlob` → sub-reader:

- str host (≤64 byte qua `sub_A63A40`) + u16 a + u16 b + u16 c → `sub_8E289C(this, host, a, b, 0, 0)`:
  `strcpy_s(dest+872, 0x200, host)`; `dest+1384 = a; +1388 = b; +1392 = c; +1400 = 0; +1404 = 0`
  (host + 3 cổng cho kết nối chính/vào kênh).
- u8 **chế độ** (`sub_A62F40`) → `*(dword_E63214 + 0xC)`:
  - `1` → str s1 (`sub_907F51`) + str s2 (`sub_907F8F`) ❓ (đoán: thông điệp/URL hỗ trợ);
  - `2` → u16 (`sub_907CB7`) ❓;
  - `0` → không field.
- u16 ĐUÔI bắt buộc (bị tiêu, không dùng) ✅ — khớp docstring `login_net_info` trong `fakeserver`
  (blob {str host, u16×3, u8 mode, u16}; mode + u16 cuối đã gửi đúng).
- `if (dword_tại_this + 0x1F78 != 0) sub_612CD0(this + 0x1BC0)` — đẩy dialog chờ
  (`CWaitForLoginDlg`, trạng thái 2→1→3, stage `sub_84FABC` → stage 10 theo protocol.md;
  `sub_84FABC` chưa decompile được) ✅ cơ chế + protocol.md xác thực runtime.

## 5. Kết quả result ≠ 0 (nghi, đối chiếu bản China)

Chỉ dùng mẫu/error-string của `CLoginStageChina__On_4` + corpus `ca_strings.txt`, không có
main logic disasm riêng ở đó: mức tin cậy trung ❓.

| code | string/hành vi |
| --- | --- |
| 0 | ok (đi tiếp parse đầy đủ) |
| 1 | `ErrLoginNotregistered` |
| 2 | `ErrLoginAlready` — kick: client trả op 11 {str tên} cho mỗi player |
| 3 | `ErrLoginConnect` — thêm (u8, u16, u16) |
| 4 | `ErrLoginUnknown` |
| 5 | thêm u8 n + n × u8 (`sub_641549` Dropbox ❓) |
| 6 | thêm (u16, u16) |
| 7 | `ErrWrongreserve` |
| 8+ | `ErrLoginDigitiz`, 9 `NotAllowedCountry`, 10 `OneClickFail`, 11 `ErrDynamicPass`, khác → `SdoWrongAuthIn + u8` |

## 6. Khuyến nghị cho `tools/fakeserver.py`

1. `login_ok()` khớp hoàn toàn thứ tự disasm lần này (31 field/player + đuôi) — chỉ một kiến nghị:
   sau field T7 (cờ đổi mật khẩu), append **`u32 0` × 2** (T8 + T9, mỗi list u32 count = 0).
   Hiện thiếu nhưng client đọc EOF→0 nên vẫn chạy.
2. Field #7 (head u8, i=0) gửi 0 = "chỉ số MultiServer 0" — muốn thử nhóm/endpoint khác trong
   danh sách multiserver thì thử giá trị khác 0 (hành vi idx lạ CHƯA KIỂ) ⚠️.
3. `login_net_info()` đã khớp đặc tả (có cả u16 đuôi), không cần sửa.
4. Guild id = 0 (không guild) phù hợp luồng đơn giản; muốn thử hành vi guild thì nhớ
   `sub_99AF0F(i) == 0` → xóa cờ hợp lệ T1 (v67) + "ConfirmGuildJoin" gửi op 367.

## 7. Câu hỏi còn mở

- Ý nghĩa cụ thể của các field per-player #1–4, 8, 9, 10–21, 25–31 (hầu hết là u8/u16/u32
  của hai store E60848/E60880, đọc lác đác trong UI lobby nhưng chưa ghép được tên + ý nghĩa).
- Getter str#3 (+0x224+0x10i) chưa có; nguồn ghi slot +0x118+0x1Ci chưa rõ.
- Người đọc object `dword_E64AEC` (16 byte, lưu u64 head) chưa lần theo.
- Shape chính xác và semantics hai list đuôi (`sub_98889C`, `sub_411CAE`) và toàn bộ
  `dword_E61004` store.
- Không giải được placeholder "aName01" trong lobby.md — tên hiển thị thực sự của player trong
  giao diện lobby sau load (khả năng lớn là slot +0x118, nhưng chưa chứng minh).
- Giá trị đọc tại `sub_9B72D1` (box `E60848 12i+0x258`) — ai ghi trước lần op 369 đầu tiên.

— Cập nhật lần 1: toàn bộ field opcode 4 đã disasm-verify (offset byte), opcode 15 đã xác minh
từ decompile của `CLoginStage__On_15`/`sub_8E289C`.
