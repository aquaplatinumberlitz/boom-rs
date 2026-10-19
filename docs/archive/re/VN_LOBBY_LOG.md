> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Lobby (vào kênh → stage 11) — đặc tả tĩnh cho server giả

Phạm vi: phân tích **tĩnh** client `unpacked_ca.exe` (nhánh `VBNB_116`), phục vụ viết
đặc tả `tools/fakeserver.py` để đi tiếp từ màn chọn kênh (stage 10) vào lobby (stage 11).
Không chạy động, không sửa code.

Quy ước độ tin cậy: ✅ đã kiểm chứng động · 🟢 đọc rõ từ decompile (chưa chạy) ·
❓ suy đoán / chưa rõ. Mỗi khẳng định ghi nguồn (địa chỉ hàm).

## 0. Bối cảnh, stage id

- Bảng `switch` trong `CStageMgr__ChangeStage` (0x8F8EEC, code gốc từ 0x8F8FCA;
  xem `docs/research/findings/VN_DEVIRT_STATUS.md` → "Stage"): **10 = `CSelectServerStage`**
  (size 0x7354, ctor 0x85817A), **11 = lobby**.
- Stage 11 có 3 biến thể, chọn theo `sub_994378(server hiện tại)` (nguồn `docs/research/findings/VN_DEVIRT_STATUS.md`):
  loại 0 → `CLobbySquareStage` (size 0x570F0, ctor 0x4E56DF),
  loại 2 → `CLobbyStageWithSquareUI` (0x57CFC, 0x4C1F4D),
  còn lại → `CLobbyStage` (0x5525C, ctor 0x6B3EDD). Mặc định loại 1 nếu feature 0x2E tắt;
  0x7B tắt UI square. 🟢
- Rời màn Login (`sub_84FABC` 0x84FABC): `sub_98E763()` → `sub_8E28F4` →
  `sub_84F482()` → `sub_993EF4(cache kênh)` → `sub_8F391D(dword_E6092C, 10)`
  (đặt id đích) → `sub_8FE6C7`. Vào stage 10 client gửi opcode 298 + 40
  (xem `docs/specification/networking/LEGACY_VN_PROTOCOL.md`). 🟢

## (a) Luồng vào kênh — opcode 7 (`On_7`)

### a1. Client gửi opcode 7 — `sub_98E763` (0x98E763) 🟢

```
sub_481256() → đường dẫn "%s\%s.log" trong thư mục Windows
                (tên file từ chuỗi toàn cục dword_E25550)
CreateFileA(..., OPEN_EXISTING) → ReadFile (tối đa 0xFFFF byte)
SendPkt__Begin(7) + SendPkt__EncodeStr(nội dung file) + SendPkt__Send
CloseHandle + DeleteFileA (xóa file sau khi gửi)
```

- Body client→server: `[opcode u16 = 7][str]` (str = nội dung file, mã hoá chuỗi
  mode 1/2 như thường lệ). Nếu file không tồn tại → **không gửi gì cả**.
- Nội dung/kẻ ghi file này ❓ chưa truy (có thể là token phiên do bước login viết;
  cần test động: log body opcode 7 trong fakeserver để xem).
- Gọi từ duy nhất `sub_84FABC` (rời màn Login, **trước** `ChangeStage(10)`),
  và file cache tên `channel2_%03d%s.ssd` (`sub_993EF4` 0x993EF4) cũng chỉ có từ đây —
  khớp ghi chú trong `docs/specification/networking/LEGACY_VN_PROTOCOL.md` rằng opcode 14/16 phải gửi sau khi vào stage 10.

### a2. Server trả opcode 7 — `CSelectServerStage__On_7` (0x8558C4) 🟢

Dispatcher: `CSelectServerStage__vf51` (0x859C70) → opcode 7 → `sub_8558C4`.
Điều kiện tiên quyết (không phải mã lỗi): `this[5915] == 1`, ngược lại hiện
`ErrDisconnect` + `sub_854C2B` (thoát màn). Field đầu tiên là **u8 mã kết quả**:

| Mã (u8 đầu) | Chuỗi hiển thị (tên symbol IDA — nghĩa suy từ tên, ❓) | Hành động |
|---|---|---|
| 0 | `ErrCannotConnect` | dialog + `sub_8549D5` (reset `this[5915]=0`) |
| 2 | `PremiumRecommend` | dialog + reset |
| 4 | `ErrVersionMismatch` | dialog + reset |
| 5 | `ErrServerNotActive` | dialog + reset |
| 11 | `ErrCheckingServer` | dialog + reset |
| 12 | `ErrEtcChecking` | dialog + reset |
| 13 | `AdultOnlyCh` | dialog + reset |
| 14 | `NexClubRegNoti` | dialog + reset |
| 15 | + thêm **str** | dialogข้อความ đó + reset |
| 16 | `TournamentAlready…` | dialog + reset |
| 17 | `2PCannotEnterT…` | dialog + reset |
| 18 | `TournamentChan…` | dialog + reset |
| 19 | + thêm **u8, u32, u32** | dialog `ChannelNotEnoughLucci%d` (format `aChannelnotenou`, tra cứu kênh qua `sub_994AB9`/`sub_994D51`) + reset |
| 20 | `DontHaveEntran…` | dialog + reset |
| 21/23/24/25 | `2PCannotEnterB/S/H/R…` | dialog + reset |
| 22 | `BigMonsterChan…` | dialog + reset |
| 26 | `ShutdownTermin…` | dialog + `sub_854CC5` |
| 27/28 | `HiddenCatch2PC…` / `WinningMode2PC…` | dialog + reset |
| 29/30/31 | `ConnectResult_O/S/N…` (khớp `ConnectResult_OneClick/SimpleGameUser/NoNexonId…ConnectFail` trong `ca_strings.txt`) | dialog + reset |
| **mặc định (gồm 1, 3, 6–10…)** | **thành công** | đọc tiếp **u8, u32, u32, u16**; nếu `u32 thứ hai != 0 && u16 != 0` → lưu vào object stage (`this[6403]=u8`, `[6404]=u32`, `[6405]=u32`, `+12812=u16`) → `sub_855254()`; ngược lại `ErrCannotConnect` + reset |

- Giá trị các mã: 🟢 (đọc trực tiếp từ `switch` trong decompile 0x8558C4).
  Nghĩa tiếng Việt chính xác của chuỗi: ❓ (chỉ suy từ tên symbol; cần mở dialog thật).
- Nhánh thành công `sub_855254` (0x855254): hiện dialog chờ (`sub_859F12`,
  chuỗi `SelectServerStage` + `WaitDialog`), gọi hàm ảo `(this+24484, 0, 1, 1)`;
  nếu trả 2 → `sub_854CC5` (không dịch được pseudocode — ❓, khả năng là yêu cầu
  đổi stage; cần xem thêm hoặc bắt động).
- Hai handler **cùng wire-format** ở stage khác (để đối chiếu, 🟢):
  - `CLobbyStage__On_7` (0x6AFEDF): cùng `u8, u8, u32, u32, u16`, điều kiện thành công
    giống hệt; khi thành công gọi `sub_9999A9` + `sub_999702(u8, u32, u32, u16)`
    (lưu địa chỉ/mã server vào `dword_E60784`: `+16=u8`, `+20=u32`,
    `+24=sub_994AB9(u8,u32)` = **chỉ số kênh tra từ cache opcode 14**,
    `+28=u32`, `+32=u16`), rồi **gửi opcode 6** (xem b1) — nếu `sub_8F6D35(1)=0`
    thì `SetExitError(1,0)` + thoát. Nhận opcode 7 khi đã ở lobby ≈ đổi kênh
    trong lobby (❓, cần kiểm động).
  - `OnPkt_multi_7_7` (0x690A31, dùng chung cho `CSessionStage`/`CExchangeStage`):
    cùng layout, thành công → `sub_8F314D` + `sub_999702` + `sub_8F6D35(0)`.

### a3. Hệ quả cho server giả: nên trả gì khi nhận opcode 7

- Reply tối thiểu để đi tiếp (🟢 từ nhánh default ở a2):
  `opcode 7 : u8 <mã thành công> , u8, u32, u32, u16`
  với `u32_thứ_hai != 0 && u16 != 0`. Mã thành công **không nằm** trong bảng lỗi
  trên — **`u8 = 1` là ứng viên đầu tiên** (1 không xuất hiện ở nhánh lỗi nào).
- Giả thuyết cần kiểm (❓): các field sau mã phải **khớp bản ghi kênh đã gửi ở
  opcode 14**, vì `sub_999702`/`sub_994AB9` (0x994AB9) dò cache kênh theo cặp
  `(u8, u32)` — dò trượt (`return -1`) ở nhánh 19 đã dẫn tới dialog lỗi, và ở nhánh
  thành công giá trị `-1` có thể gây lỗi sau. Cụ thể `sub_994AB9` duyệt
  `sub_414543(this+12)` bản ghi, khớp `sub_994A5A(i)==u8 && ...==u32`.
  Fakeserver hiện gửi bản ghi 14 với `u32 = i+1`, còn lại 0 — nên thử
  `u8/u32 = (0?, i+1)` trước, rồi biến thể theo kết quả động.
- Sau reply thành công, client dự kiến: `sub_855254` (dialog chờ) → đổi stage 11 →
  xem (b).

## (b) Các opcode lobby chờ sau khi vào (stage 11)

### b1. Client gửi khi vào lobby (server phải **nhận**, không phải gửi) 🟢

1. **Opcode 6** — gửi ngay trong `CLobbyStage__On_7` thành công (0x6AFEDF),
   trước khi đổi stage (vẫn ở ngữ cảnh stage cũ):
   `Begin(6); EncodeBool(số_người_chơi);` lặp theo `CLobbyStage__On_218`
   (= số người chơi 1P/2P, `sub_99AEE3` 0x99AEE3 chỉ đọc `this+12`):
   `EncodeStr(sub_99A731(i)) + Encode4(sub_99A70C(i))` (tên + u32 chưa rõ).
   Cấu trúc 🟢, ngữ nghĩa từng field ❓ (ngoài count).
2. **Opcode 303** — gửi khi **OnEnter mọi stage** (`CStage__vf44` 0x6A3E34):
   nếu giá trị `GetCompletionList(g_pMainWnd)` ∈ {11,12} → `v11=1`, == 20 → `v11=0`;
   lặp theo số người chơi: `Begin(303); EncodeBool(v11); EncodeBool(i); Send`.
   Giá trị 11/12/20 trùng id stage (11 lobby, 12 phòng, 20 square?) nên đây chính là
   "chào lobby" của client. Tên import `GetCompletionList` nghe không liên quan —
   có thể IDA đặt tên sai, nhưng hằng số so sánh là bằng chứng trực tiếp 🟢/❓.
   Server không có handler 303 ở lobby (dispatch `CGameStage` mới có 303) →
   chỉ cần nhận/log, chưa cần trả lời (❓ có thể cần, kiểm động).
3. Sau đó, các gửi theo hành động người dùng (tìm bằng xrefs tới
   `SendPkt__Begin` 0x888462 / `SendPkt__Send` 0x888609, 🟢 tồn tại, ngữ nghĩa ❓):
   opcode **9 + 84** (`CStage__On_75` 0x697D72), **77** (`sub_6B6441` 0x6B6441 —
   tạo/vào phòng: `Begin(77) + sub_6B61F0(...)`), **78** (chấp nhận mời, xem opcode 26),
   và nhiều gửi từ dialog/UI lobby (`sub_6B9867`, `sub_6B9F39`, `sub_6B76D0`, … chưa xem).

### b2. Server gửi → client: bảng opcode lobby

Nguồn dispatcher: `CLobbyStage__vf51` (0x6B2ED4) + `opcode_dispatch.json`
(`CLobbyStage__OnPacket@6B2ED4`). Lưu ý luồng chung trong `vf51`: sau (gần như)
mọi opcode đều chạy tiếp `RecvPkt__Rewind → sub_4D24BB → sub_512B66 →
CChatStage__OnPacket` (chat base) rồi nhánh `v3-218==0 → sub_99AEE3` (đếm người chơi)
hoặc `v3==234 → sub_6B0D7C`. Tức opcode **218** (bare, không field) và **234**
(không field) được xử lý ở đuôi chung — server có thể gửi opcode trần.

| Opcode | Handler | Field server→client (thứ tự đọc) | Nhận định |
|---|---|---|---|
| 7 | 0x6AFEDF | `u8==0` fail; ok: `u8, u32, u32, u16` (u32#2≠0, u16≠0) | 🟢 đã rõ (a2); đổi kênh trong lobby |
| 19 | 0x6B0357 | **không đọc field**; kênh guild-only → `SetExitError(5,1999)` + thoát | 🟢 |
| 26 | 0x6B167A | `u16 roomId, u32, str inviter, str, [u8…]` (có nhánh `sub_8DAD14`); nhánh chấp nhận → **client gửi opcode 78** (`Encode2 + Encode4 + Bool(0) + Bool(count) + mỗi người Bool`) | 🟢 cấu trúc; ❓ ngữ nghĩa (mời vào phòng) |
| 125 | 0x6B1358 | **không field** (huỷ ladder) | 🟢 |
| 218 | (đuôi vf51) | **không field** (`sub_99AEE3` đếm người chơi) | 🟢 |
| 234 | 0x6B0D7C | **không đọc packet** (kiểm tra guild từng người chơi + chuyển UI `sub_8DD721(100)`…) | 🟢 — ứng viên "đẩy client đi tiếp" tốt nhất: gửi opcode trần, không sợ sai field |
| 259 | 0x6B27BB | `str, u32×6` (thông báo hạng/điểm; chỉ hiển thị) | 🟢 |
| 265 | 0x6B147E | `u16 n, n×str` ( afforded mỗi str hiện dialog) | 🟢 |
| 268 | 0x6B25E3 | `u8 idx(<số người chơi), str, u32 (1..6), u32` | 🟢 |
| 363 | 0x6ADF1C | → `CStage__On_363` + `sub_67C5C9/67BDA0` (không đọc trực tiếp ở đây) | ❓ chưa xem sâu |
| 368 | 0x6AE770 | `u32, u32` (u32#1 ≤ 0x12 → `sub_9B70B9/sub_67A5AB`) | 🟢 |
| 372 | 0x6ADFAD | → `sub_67BA96` (không đọc trực tiếp) | ❓ |
| 406 | 0x6AEF18 | `u8` | 🟢 |
| 418 | 0x6B1FEC | `str, u32 n, n×u32` (hết hạn item) | 🟢 |
| 419 | 0x6B224E | `str, u32 n, n×(u32,u32)` (bán item) | 🟢 |
| 426 | 0x6B2433 | `u8 + blob` (vòng lặp `sub_A62FB0` lần, magic-book?) | 🟢 một phần |
| 499 | 0x6AE72A | **không field** → `sub_8E79B2(1)` | 🟢 |
| 505 | 0x6B011E | `u8==1` ok: `u8, u32, u32, u16` (địa chỉ square: `hostlong/port`; cả hai 0 → lỗi `SquareErrCannot…`); thành công → `sub_999702` + `sub_99A207(...,0x402,...)` | 🟢 — giống họ opcode 7, liên quan square |
| 553 | (inline vf51) | `blob → sub_6A5A5E` (điều kiện `sub_40176C(47)==0 && sub_8DACD5()`) | 🟢 một phần |
| 554 | 0x6AE817 | rewind + forward cho `dword_E6405C` | 🟢 |
| 576 | 0x6AE836 | → `CStage__On_576` (+ `sub_51607D` nếu feature mở) | ❓ |
| 32/89/124/289 | — | có trong dispatch, chưa đọc pseudocode (289 → virtual subclass) | ❓ |
| base | `CChatStage__OnPacket` + `CStage` (191 opcode chung) | luôn chạy sau mỗi packet lobby (chat, 40, 75, 77, 82, …) | 🟢 tồn tại, chưa liệt kê field |

Biến thể square (`CLobbySquareStage__vf51` 0x4E5A38, `CSquareStage__vf51` tương tự)
xử lý thêm chùm 466–512, 576, 612–625 — chỉ liên quan nếu lobby loại 0;
mặc định (loại 1) dùng bảng trên. ❓ loại lobby thực tế khi vào bằng fakeserver.

Danh sách phòng (room list) ❓: **chưa xác định** opcode nào đẩy danh sách phòng.
Không có handler lobby nào đọc dạng "u16 n + n bản ghi phòng" trong số đã xem;
26 là lời mời đơn lẻ, 234 không field. Khả năng: (i) room list đi qua opcode base
(`CStage`/`CChatStage`) chưa đọc, (ii) client tự xin bằng opcode 77/78/84 sau khi
nhận 234/bare trigger, (iii) thuộc chùm square. Cần test động mục (c).

## (c) Đề xuất test động tiếp theo (thứ tự)

Giả thiết môi trường như README: `python3 tools/fakeserver.py --log ...` +
`tools/xvfb_login.sh` (bản `_dbg` để bắt exception), client đã ở stage 10
(chọn kênh, đã nhận 14+16).

1. **Log request opcode 7**: fakeserver hiện chỉ log hex body — ghi thêm
   `DecodeStr` của body 7 để lộ nội dung file `%Windows%\<?>.log`
   (xem nó là token/auth gì; đối chiếu với `login_ok`/`login_net_info`).
2. **Reply opcode 7 tối thiểu** (khi nhận opcode 7 từ client):
   `obf_body(7, u8=1, u8=0, u32=?, u32=NN≠0, u16=MM≠0)`.
   Thử `NN/MM` theo bản ghi kênh đã gửi ở opcode 14 (đang gửi `u32=id=i+1`):
   cặp `(u8=0, u32=id kênh user đã click?)` rồi biến thể `(u8=1..4, u32=id)`.
   Vì click kênh nào là input người dùng, cách nhanh nhất là auto-click + thử ma trận
   nhỏ (4 kênh × vài cặp field), quan sát dialog lỗi (mã nào trong bảng a2) hay
   chuyển stage. Kỳ vọng: dialog lỗi → sai field; chuyển stage 11 → đúng.
3. **Sau khi sang stage 11**: log và **chưa trả lời vội** opcode 6 + 303 của client;
   chụp màn hình xem client đứng ở UI nào / có timeout dialog gì (tên chuỗi dialog
   sẽ chỉ ra opcode còn thiếu).
4. **Đẩy bare trigger**: gửi `opcode 234` trần (không field) rồi `opcode 218` trần;
   nếu UI lobby đổi (theo `sub_6B0D7C`: kiểm guild + `sub_8DD721(100)`), ta đã có
   "mồi" vào lobby mà không cần room list.
5. **Room list**: nếu sau (4) lobby hiện nhưng trống phòng, thử gửi 26 một bản ghi
   mời (`u16 roomId=1, u32, str inviter, str`) xem dialog mời có hiện không
   (→ client tự gửi 78, ta học tiếp luồng tạo/vào phòng `CPrepareStage`/stage 12);
   song song đọc nốt `CStage`/`CChatStage` opcode phòng (77/78/84 quanh `sub_6B6441`,
   `CStage__On_75`).
6. **Loại lobby**: log xem ctor nào chạy (0x6B3EDD vs 0x4E56DF vs 0x4C1F4D) qua
   breakpoint/winedbg ở test động — quyết định dùng bảng opcode nào long-term
   (bảng (b2) là `CLobbyStage`; square có thêm chùm 466+).

Thứ tự ưu tiên code fakeserver (khi test động xác nhận): `on_opcode_7 → reply 7`
⇒ `on_opcode_6/303 → log (+ sau này reply 234/218)` ⇒ room list (chưa rõ, để sau).

## Phụ lục: địa chỉ tra cứu

- Gửi 7: `sub_98E763` 0x98E763 (tên file: `sub_481256` 0x481256); rời login:
  `sub_84FABC` 0x84FABC; tên file cache kênh: `sub_993EF4` 0x993EF4.
- Nhận 7: `CSelectServerStage__On_7` 0x8558C4 (dispatcher 0x859C70),
  `CLobbyStage__On_7` 0x6AFEDF (dispatcher 0x6B2ED4),
  `OnPkt_multi_7_7` 0x690A31.
- Thành công → `sub_855254` 0x855254 (dialog `sub_859F12` 0x859F12),
  reset-flag `sub_8549D5` 0x8549D5, chuyển stage `sub_8F391D` 0x8F391D /
  `CStageMgr__ChangeStage` 0x8F8EEC (khối VM, xem `docs/research/findings/VN_DEVIRT_STATUS.md`).
- Lưu server: `sub_999702` 0x999702; tra kênh: `sub_994AB9` 0x994AB9;
  đếm người chơi: `CLobbyStage__On_218`/`sub_99AEE3` 0x99AEE3.
- OnEnter gửi 303: `CStage__vf44` 0x6A3E34 (`SendPkt__Begin` 0x888462,
  `SendPkt__Send` 0x888609). Stage 11: ctor `CLobbyStage` 0x6B3EDD (size 0x5525C).
- IDA DB chỉ đọc qua MCP idalib, không save. Disasm dài không in ra chat theo quy ước.

## (c) Tại sao kẹt ở "vào kênh" — điều tra 2026-10-16 (chưa xong)

Triệu chứng: vào được stage 10 (CALog `Sg - INT(dne:10)`), nhận opcode 14 (ghi `channel2_001.ssd`,
không crash), **nhưng lưới 8 ô kênh (4×2, 5 trang) trống** → không click vào kênh được → kẹt.

### Điều kiện click được một kênh — `sub_8570DB` (0x8570DB) 🟢
Duyệt `CChannelInfo` (`dword_E60FD8`), chỉ nhận kênh `i` thỏa **cả 3**:
- `sub_994A5A(info, i) == 10` — `0x994A5A`: đọc field `+0x4C` (u8), trả **10 nếu == 18**, else trả giá trị thô.
  ⇒ **field +0x4C của record phải = 18** mới click được. **⚠ SAI — xem "Errata audit" cuối file.**
- `sub_994D51(info, i) == a2` — `0x994D51`: field `+0x40` (u8); `a2` = loại kênh click (trong `sub_856BAC` so
  với `0x80/0x82/0xC8`) ⇒ thử `+0x40 = 0x80`. **⚠ SAI — xem "Errata audit" cuối file.**
- `sub_994C91(info, i) == 0` — `0x994C91`: field `+0xC4` (u32), phải 0.

Trong record opcode 14 (`sub_994663`), 7 u8 lưu theo thứ tự vào `+0x28,+0x34,+0xD0,+0x40,+0x4C,+0x58,+0x64`.
Nên u8 thứ 4 = `+0x40`, u8 thứ 5 = `+0x4C`. Đã sửa `channel_list()`: `u8×7 = [0,0,0,0x80,18,0,0]`. **⚠ SAI — xem "Errata audit" cuối file.**

### Vẫn trống sau khi đặt kind=18 — phần chưa giải 🟡
- Dữ liệu opcode 14 vào **`CChannelCache` (`dword_E61010`)**; click/join đọc **`CChannelInfo` (`dword_E60FD8`)**
  — hai object khác nhau. `CStage__On_14` (0x99514B) copy bằng vòng `sub_9950DA(this=CChannelInfo, i, -1,
  sub_993F7B(i))` nhưng `sub_9950DA` (0x9950DA) chỉ ghi value-box vào **mảng chỉ mục** `this+3*i+59398` /
  `+62470`, **không** copy record 232-byte sang. Vậy record 232-byte trong `CChannelInfo` được điền ở chỗ khác
  (chưa truy ra — có thể `sub_994FB3` khi record đã tồn tại, hoặc broadcast opcode 7/16).
- Lưới ô kênh dựng ở `sub_858AB4` (0x858AB4, chuỗi `aChBtChannel`/`ChannelTab`) — lớn (~7.6KB decompile), chưa đọc hết.
- Field đều là **value-box** (`sub_413F32` set / `sub_414625` get, 6 byte, có `sub_A21D00` + XOR) → đọc runtime
  phải giải mã, không đọc thẳng bộ nhớ được.

### Luồng join sau khi click — `sub_856BAC` (0x856BAC) 🟢 (chưa cần tới vì chưa click được)
Gọi mỗi frame từ `CSelectServerStage__vf5`. Khi chọn kênh hợp lệ: gửi **opcode 71** (`sub_88849F(71)` +
`EncodeBool`), tính toán, **đặt `this[5915]=1`**, `sub_8F391D(dword_E6092C, 11)` (vào lobby) hoặc `(…, 20)` (square),
gửi thêm 1 packet. ⇒ client tự đổi sang stage 11; server cần xử lý opcode 71 + trả `opcode 7` thành công
(định dạng ở mục (a2)/(a3)) để không bị `ErrDisconnect` (do `On_7` đòi `[5915]==1`).

### Bước tiếp đề xuất
1. Runtime: break `sub_858AB4` hoặc dump `CChannelInfo`/`CChannelCache` sau On_14, giải value-box để xem field
   nào quyết định "kênh vào ô nào" (nghi một field = số thứ tự ô/trang).
2. Tìm nơi điền record 232-byte của `CChannelInfo` (xref ghi `dword_E60FD8 + 232*i`).
3. Khi ô kênh hiện + click được: thêm handler opcode 71 + reply opcode 7 (thành công) vào `fakeserver.py`.

### Cập nhật: hàm vẽ ô + điều kiện copy (2026-10-16)
- **Hàm vẽ lưới ô kênh = `CSelectServerDlg__vf19` (0x872672) + `sub_8729D0` (0x8729D0)** (đọc `dword_E60FD8`,
  class `CSelectServerDlg` 0x872xxx). Đây là nơi cần đọc tiếp để biết field nào quyết định ô hiện/trống.
- `sub_9950DA`: nếu `sub_414543(cache_record + 0x1C) == -1` → chỉ set mảng chỉ mục; **else `sub_994FB3`** (0x994FB3)
  copy record cache→Info (value-box các offset +40..+220). Record fakeserver gửi có `+0x1C = i+1` (≠ -1) ⇒ đáng lẽ
  chạy nhánh copy. Cần xác nhận runtime nhánh nào chạy và vì sao `vf19` vẫn vẽ trống (nghi field quyết định ô/trang
  hoặc 1 field hiển thị khác chưa set). Getter kênh (đều value-box, cùng stride 232): `sub_994A5A`(+0x4C kind),
  `sub_994D51`(+0x40 loại), `sub_994C91`(+0xC4), `sub_994A86/994A9F/9949F3`(chuỗi: tên/…).

### Cập nhật 3: hàm vẽ THẬT của stage 10 + gate cuối (2026-10-16, qua gdb-proxy)
Dùng `winedbg --gdb` + gdb batch (xem `tools/wine_gdb.sh`; ổn định, đọc bộ nhớ chuẩn — `x/2xb 0x400000`=`4d 5a`)
đã xác định runtime:
- **`CSelectServerDlg__vf19` (0x872672) KHÔNG phải hàm vẽ stage 10** (không hit). Hàm vẽ thật = **`sub_854E1D`**
  (0x854E1D, gọi count-getter `sub_994A1D` 5360 lần/các frame từ 0x854EDD). Hàm **dựng mảng slot** = **`sub_85718D`**
  (0x85718D). Mảng slot ở **`stage+0x4C6C` (this+4891 dword)**; `vf44` memset -1 lúc OnEnter.
- **Dữ liệu kênh trong `CChannelInfo` (0xE60FD8) ĐÚNG HẾT** (giải value-box khớp giá trị gửi):
  CH0 type(+0x40)=**50**, kind(+0x4C)=**18**, max(+0x34)=**100**. ⇒ record opcode 14 + bộ giải box chính xác. **⚠ SAI — xem "Errata audit" cuối file.**
- **Gate cuối khiến ô trống** (trong `sub_85718D`): thêm kênh vào ô chỉ khi
  `(u8)sub_994A9F(ch) == *(stage+0x4C68)` — **category kênh == tab đang chọn**. Runtime: **tab = -1**
  (mặc định lúc dựng), **category kênh = 0** (= u8 thứ 3 của record, offset **+0xD0**, mình đang để 0)
  → `0 != -1` → không kênh nào vào ô. (Kèm `sub_994C91==0` ok, `sub_994D51` so 0xC8, kind∈{10,18} ok.)
  `sub_994A9F` đọc `info+232*i+232` = record[i]+0xD0 (record base = info+0x18).
- **Tab (stage+0x4C68) chỉ đổi khỏi -1 qua UI**: `vf15` (0x857458+, gọi `sub_85718D(this, tab±1)` = nút mũi tên **⚠ SAI — xem "Errata audit" cuối file.**
  đổi tab category), `vf5` (0x8592FA, click). Không thấy tự chọn tab lúc kênh tới. Vì (u8)category ∈ 0..255 không
  bao giờ == tab -1, nên **phải có bước chọn tab category** (đưa tab về 0..255 khớp category kênh).
- **Việc còn lại để kênh hiện**: (1) đặt category record (+0xD0, u8 thứ 3) = một giá trị, rồi (2) làm tab khớp —
  hoặc kích hoạt `vf15` (phím đổi tab), hoặc tìm chỗ client tự chọn tab mặc định khi kênh tới (có thể cần 1 field/
  packet nữa). Thử nhanh: đặt +0xD0=0, vào stage 10, ép `sub_85718D` nhận a2=0 (hoặc bấm phím đổi tab) → ô sẽ nhận id.

### ✅ XÁC NHẬN: kênh hiển thị được (2026-10-16, qua gdb-proxy)
Ép mảng slot `stage+0x4C6C = {0,1,2,3}` + tab `stage+0x4C68 = 0` **sau khi On_14 nạp kênh** → 4 ô hiện
**"Kenh 1".."Kenh 4"** với icon kênh (ảnh `docs/evidence/images/vn/channel_display_ok.png`). Chứng minh: record opcode 14 + field
(type=50/kind=18/max=100/category=0) + hàm vẽ `sub_854E1D` đều ĐÚNG; thiếu mỗi việc **điền mảng slot**. **⚠ SAI — xem "Errata audit" cuối file.**

**Nút thắt gốc (đã chốt):** mảng slot `stage+0x4C6C` dựng bởi `sub_85718D`:
- `vf44` (OnEnter) gọi `sub_85718D(this,-1)` → dựng lúc `CChannelInfo` còn RỖNG (kênh tới sau qua opcode 40→14).
- `vf5` (mỗi frame) gọi `sub_85718D(this, v4)` **chỉ khi** `v4 != *(this+4854)`, với `v4 = sub_A84DFE(this+1878)` **⚠ SAI — xem "Errata audit" cuối file.**
  = **lựa chọn của tab-control** (this+1878). Tab-control mặc định chọn -1 → `v4` không đổi → **không dựng lại**
  khi kênh tới → ô trống.
- Ngoài ra `vf15` (0x857458+, nút mũi tên đổi tab) cũng gọi `sub_85718D(this, tab±1)`. **⚠ SAI — xem "Errata audit" cuối file.**

**Fix sạch (chưa làm):** làm client dựng lại mảng slot với tab khớp category kênh sau khi nhận kênh. Hướng:
1. Tab-control (this+1878) phải có mục category khớp `category kênh (+0xD0)` và được chọn → `v4` đổi → `vf5` dựng lại.
   Cần tìm hàm điền tab-control (nghi trong `sub_858AB4` vf44, chuỗi `aChBtChannel`) + category value mặc định.
2. Hoặc gửi kênh (opcode 14) SỚM hơn (trước khi `vf44` dựng mảng slot) — nhưng client gửi opcode 40 lúc OnEnter nên khó.
3. Hoặc (tạm, để chơi được): sau khi vào stage 10, người chơi bấm phím đổi tab (`vf15`) → dựng lại → kênh hiện.
Record opcode 14 hiện mang type=50/kind=18/max=100/category(+0xD0)=0; cần chốt category + cơ chế chọn tab. **⚠ SAI — xem "Errata audit" cuối file.**

### Cập nhật 4: NGUỒN số tab + cơ chế dựng lại (2026-10-16) — cơ chế đầy đủ
- **Số tab category = `sub_854A7D()` = `*(CShareContext + 456)`** (`CShareContext` = `dword_E60E54`, singleton session).
  Đặt bởi **`sub_99C4CB(v50)` trong `sub_852BCA` (xử lý login opcode 4)** → **số tab đến từ GÓI LOGIN**, không phải
  channel data. Trong gói login (sau vòng lặp người chơi): `u8,u8,u8,u16,`**`u8 = số tab`**`,u8(đổi mật khẩu)`.
  Server giả cũ gửi = 0 → 0 tab → không bao giờ dựng lại ô. **Đã sửa `login_ok()`: u8 số tab = 1.**
- **`CSelectServerStage__vf5` là handler SỰ KIỆN** (a2 = mã sự kiện): a2∈0x5DC.. = hover/chọn ô; a2∈0x9C4.. = click ô;
  **a2 == 900 (0x384) = SỰ KIỆN ĐỔI TAB** → `v4 = sub_A84DFE(this+1878)` (lựa chọn tab-control); nếu
  `0 ≤ v4 < sub_854A7D()` và `v4 != *(this+4854)` → `sub_85718D(this-36, v4)` **dựng lại ô** với tab = v4.
  ⇒ **ô kênh chỉ dựng lại khi CHỌN một category tab** (bắn sự kiện 900). Không có tương tác → ô giữ nguyên (trống
  từ lúc OnEnter khi kênh chưa tới).
- Builder `sub_85718D` lọc: `(u8)sub_994A9F(ch)(category +0xD0) == tab v4` + `sub_994C91(ch)(+0xC4)==0` +
  `sub_994D51(ch)(type +0x40)` so 0xC8 + kind(+0x4C)∈{10,18}. ⇒ **category kênh (+0xD0) phải == chỉ số tab được chọn.** **⚠ SAI — xem "Errata audit" cuối file.**

**Để kênh tự hiện cần:** (1) số tab login > 0 (đã đặt 1); (2) category kênh (+0xD0) khớp một tab; (3) tab được chọn
→ bắn sự kiện 900 → vf5 dựng lại. Còn phải xác minh: tab-control (this+1878) có tự chọn tab 0 khi vào màn không, hay
người chơi phải click tab/mũi tên category (nút `<` `>` ở đầu màn?) để bắn sự kiện 900. Đây là bước cuối cùng.

### ✅ Cập nhật 5: kênh TỰ HIỆN — patch client + sửa offset (2026-10-16)
Ảnh: `docs/evidence/images/vn/channel_autoshow.png` (login → màn kênh hiện "Kenh 1".."Kenh 4", **không click gì**).

1. **Patch client (hook `.devirt`)** — `tools/devirt.py`, danh sách `HOOKS` (cơ chế mới: trampoline, không cần VM jmp):
   site `0x854B5B` (cuối `CSelectServerStage__On_16`, lệnh `call sub_A62530` = huỷ blob, `ecx=&v6` đã đặt sẵn) →
   `jmp cave`. Cave: chạy lại `call sub_A62530`; `pushad/pushfd`; `stage = *(*(0xE60838)+0xD4)` (bỏ qua nếu 0);
   `push 0; mov ecx, stage; call sub_85718D` (__thiscall, ret 4); `popfd/popad`; `jmp 0x854B60` (epilog gốc).
   An toàn vì On_16 (0x854ADC) chỉ được gọi từ `CSelectServerStage__OnPacket` (0x859CDA) → stage hiện tại chắc chắn là
   CSelectServerStage. Không có nhảy nào vào 0x854B5B; nhánh "0 kênh" (`jz 0x854B60`) bỏ qua hook (không cần dựng).
   Kiểm chứng gdb: `sub_85718D` được gọi 2 lần — OnEnter `a2=-1` và hook `a2=0` (mode `[stage+0x64F0]=0`).
2. **Lỗi offset khiến hook dựng ra 0 ô** (gdb trên gate của `sub_85718D`): `sub_994A9F`=0 khớp tab 0,
   `sub_994C91`=0, `sub_994D51`=0, nhưng **`sub_994A5A`=100** (cần 10/18). Lý do: getter đọc `info+232*i+G`, record lưu ở
   `info+0x18+232*i+S` ⇒ **G = S + 0x18**; getter `+0x4C` = S `0x34` = **u8[1]** (fakeserver để 100). Sửa u8[1]=18 → 4 ô.

### Errata audit các commit "Claude Opus 4.8" (7e4a395, 3447c1f, ba8badc, 4235925, 03b7b2c, e4c591c, bb2b2ca)
Lỗi gốc chung: **lẫn hai hệ offset**. Offset lưu trong `sub_994663` (`[esi+S]`, esi = record = `info+0x18+232*i`)
thì đúng; nhưng offset đọc ra từ getter (`this+232*a2+G`, this = `info`) lại bị ghi như thể là S. Bảng đúng
(kiểm trong IDA: `lea ecx,[esi+S]` của `sub_994663` + decompile getter; kiểm runtime bằng gdb):

| field gói 14 | S (record) | G (getter) | getter | ý nghĩa đã kiểm |
|---|---|---|---|---|
| u32 (sau tên) | 0x1C | 0x34 | `sub_994D38` (u32) | **MAX USERS**: renderer `sub_854E1D` + `sub_857458` coi kênh dùng được khi `cur < max` |
| u8[0] | 0x28 | 0x40 | `sub_994D51` | **loại kênh**: join `sub_856BAC` so 0x80/0x82/0xC8; builder mode 0 cần ≠0xC8; renderer: 50..65/70 → thêm check `sub_66F4E4`; ==0x82 đổi skin |
| u8[1] | 0x34 | 0x4C | `sub_994A5A`/`sub_994A86` | **kind**: 18 → đọc ra 10; builder/click cần 10/18 |
| u8[2] | 0xD0 | 0xE8 | `sub_994A9F` | **category** == tab đang chọn |
| u8x4[3] | 0xAC | 0xC4 | `sub_994C91` | phải 0 |
| opcode 16 u16 | — | info+0x3A018+12i | `sub_994D7E` | **CUR USERS** |
| opcode 16 u32 | — | info+0x3D018+12i | `sub_994DA3` | chưa rõ |

Các khẳng định sai cụ thể (đã gắn ⚠ ở trên):
- "+0x4C = u8 thứ 5 = kind", "+0x40 = u8 thứ 4 = type", "max = u8[1] (+0x34)", "sub_994C91 = u32 thứ 2" — sai hệ
  offset; đúng theo bảng trên. Hệ quả: fakeserver đặt kind=18 vào u8[4] (vô dụng) và 100 vào u8[1] (chính là kind) →
  builder loại mọi kênh. `max` thật là u32, fakeserver để `i+1`.
- "type phải ∈ 50..65 hoặc 70 mới hiện (0x80 bị loại)" — lấy từ `CSelectServerDlg__vf19` (hàm của dialog khác, chính
  4.8 sau đó xác nhận không chạy ở stage 10). Trong renderer thật, `sub_66F4B4` chỉ thêm một điều kiện disable; type=0
  vẫn hiện (đã chạy thử: u8[0]=u8[3]=0 vẫn hiện đủ 4 kênh).
- "giải value-box khớp ⇒ record + bộ giải chính xác" — chỉ chứng minh u8 round-trip ở offset lưu, không chứng minh ý nghĩa.
  **`tools/wine_peek.py`: bộ giải u32 sai** (dùng khoá `buf[1]`, getter `sub_414543` dùng `buf[4]`) — đã sửa + self-test.
  Kiểm runtime (gdb, `x/8xb` buffer box): CH0 max (G 0x34) giải ra **100**, cur (info+0x3A018) ra **10** = đúng giá trị gửi; builder slotcount=4.
  Đồng thời `/proc/PID/mem` **không đọc được** bộ nhớ game trên new-wow64 (EIO) — docstring cũ nói "reliable" là sai.
- "`vf15` (0x857458) = nút mũi tên, gọi `sub_85718D(this, tab±1)`" — sai: `sub_857458` tìm kênh có type==a2, `cur<max`,
  `sub_994C15>=a3`, rồi gọi `sub_85718D(this, *(this+6116))` (giống chọn kênh nhanh), không phải ±1.
- "`vf5` gọi mỗi frame" (Cập nhật 3) — sai, chính Cập nhật 4 đã sửa: `vf5` là handler sự kiện (900 = đổi tab).
- opcode 16 (`channel_status`, có từ commit OpenCode b80cc27, 4.8 audit cho qua): gửi `(u16 i, u32 users)` — sai nghĩa;
  index là bộ đếm vòng lặp, u16 = cur users. Đã sửa thành `(u16 users, u32 0)`.
- `tools/wine_gdb.sh`: cửa sổ login cố định ~42 s quá ngắn khi chạy dưới gdb (khởi động chậm ~3×) → nhiều lần không vào
  được stage 10. Đã đổi thành thử lại tới khi CALog có `dne:10`. `pkill -f ca_fixed_patched` đổi sang `[c]a_…` để
  không tự giết shell gọi nó.

Những phần 4.8 làm **đúng** (đã kiểm lại): tag byte opcode 14/16 (xác nhận OpenCode); renderer thật = `sub_854E1D`,
builder = `sub_85718D`, mảng slot `stage+0x4C6C`, tab `stage+0x4C68`; category = `sub_994A9F` = S 0xD0; số tab =
`*(CShareContext+456)` từ gói login (runtime: với 1 tab, `sub_85718D(…,0)` được chấp nhận); sự kiện 900 = đổi tab;
offset lưu trong `sub_994663`; kỹ thuật `winedbg --gdb`.

**Bước tiếp (vào kênh):** click ô → `sub_8570DB` cần `sub_994D51 == loại click` (u8[0], hiện 0; `sub_856BAC` gợi ý 0x80
= kênh thường) → join gửi opcode 71 → server trả opcode 7 thành công.


### ✅ Cập nhật 6: VÀO ĐƯỢC KÊNH → stage 11 (sảnh quảng trường) (2026-10-17)
Chạy `tools/xvfb_join.sh` (login → chờ `dne:10` → click ô "Kenh 1"): CALog `NTFC - SCEstbl` → `SSS - CNTM(SCed)` →
`LbySS - INT` → **`Sg - INT(dne:11)`**. Ảnh: `docs/evidence/images/vn/channel_selected.png` (thanh tải kênh hiện), `docs/evidence/images/vn/lobby_loading.png`
(stage 11, khung "Đang tải").

Luồng (đã code trong `tools/fakeserver.py`):
1. Opcode 16 entry = `(u16 cur users, u32 state)`; **state phải = 1** (online): renderer `sub_854E1D` vẽ tên + thanh tải
   khi `sub_994DA3()==1`, check vào kênh `sub_67051F` trả 0 nếu state==0 (3 = từ chối đặc biệt).
2. Click ô → `vf5` (sự kiện 1500+ô) → `sub_856BAC(stage, kênh)`. `v30` = "kênh thuộc server đang nối"
   (`sub_994F71(kênh)` == `E60784+0x14` và kind == `E60784+0x10`): nếu đúng → đổi thẳng stage 11 (không gói nào).
   Nếu không: `[5915]=1`, gửi **opcode 5** `{bool (kind qua sub_58FDF8/box), u32 chỉ số kênh trong cùng kind (sub_994F71)}`.
3. Server → **opcode 7** (`CSelectServerStage__On_7` 0x8558C4): `u8 kết quả` (0 = ErrCannotConnect; 2,4,5,11..31 = các lỗi
   có chuỗi; 26 = shutdown; giá trị khác = OK, dùng 1), `u8 kind (10)`, `u32 chỉ số kênh trong kind`, `u32 IP`, `u16 port`
   (IP/port thứ tự host, client tự htonl/htons; cả hai ≠ 0). Lưu `stage+0x640C..0x6418`; ngắt kết nối cũ (`sub_8F314D`).
4. `sub_859D8A`→`sub_855717`: `sub_999702(E60784, kind, idx, ip, port)`, connect (`sub_8F6D35`, nhận hello mới), rồi
   `sub_999FE6` gửi **opcode 6** `{u8 số người, bool, mỗi người: str tên, u32, u32, u64; bool, bool}`; đặt `[5915]=2`.
   Fakeserver trỏ IP/port về chính nó (cùng port).
5. Server → **opcode 17**: `CStage__OnPacket` case 15 (case = opcode−2) → `vf55`; `CSelectServerStage__vf55` chỉ chạy khi
   `[5915]==2` → `0x69A5F0`: parse `sub_69A420`/`sub_4D7963` rồi **`sub_8F391D(.., 11)`** (20 = square nếu `sub_8DB058`).
   Thân: SYSTEMTIME (8×u16), u32, u8×9, u8 n (≤6), n×(u32,u32), u8, u8, u8 — fakeserver gửi giờ hiện tại + toàn 0.

### Hiện trạng stage 11 (`CLobbySquareStage`, OnEnter `vf44` 0x4DED62) 🟡
Kẹt ở "Đang tải". Lúc vào, client gửi 298, 303, 319, 64, 476, 477, 395 (server giả chưa trả). Handler đã xác định:
- **On_466** (0x4E52B5) = thông tin khu: `u32 area id` (`this+5541`, -1 = đặc biệt), blob {u32 n, n×{u32 m, m×5×u32}} (vật thể,
  `sub_4E4EFD/sub_4E134C`), `str`, `str` (dùng làm tên **theme** — `"%s%s"` với `aTheme`, phải là tài nguyên có thật),
  `u32` → `sub_4E1FBE`; nạp map theme (`sub_8DA6F5`).
- **On_467** (0x4E4100) = danh sách người trong quảng trường: `u16 n`, n× `sub_4E3F29`; cuối hàm đặt **`this[4775]=1`
  (đã load)** và gửi opcode 396. Chặn bởi `sub_4D82DE()` (cũng chặn 476/477).
- On_476: `u16 id, u32, str` (chat/notice theo id); On_477: `u16 n, n×{u16, u8 k, k==0 ? u32}`.
Bước tiếp: xem client gửi gì trong 395/298/303/319/64 (cái nào là "xin vào quảng trường"), tìm tên theme hợp lệ trong
assets, gửi 466 rồi 467 (có chính người chơi) để qua "Đang tải".

### ✅ Cập nhật 7: SẢNH LOAD XONG (2026-10-17)
Ảnh `docs/evidence/images/vn/lobby_loaded.png`: sảnh quảng trường hiện đủ UI (TẠO PHÒNG, ĐI TẮT, lưới phòng, tab chat AllTab/GuildTab/
Square_ChatTab, khung nhân vật) kèm lớp hướng dẫn lần đầu (HD Câu cá, nút "Đóng lại"). Đứng ổn định ≥70 s, keep-alive
(opcode 15) đều. Chạy: `SHOTS=5 tools/xvfb_join.sh`.

1. **Opcode 395** (rỗng, `sub_4D5C25` gọi từ `CLobbySquareStage__vf44`) = xin thông tin quảng trường. Fakeserver trả
   **466** rồi **467** (`square_info()`, `square_users()`):
   - 466: `u32 area=1`, blob `{u8 tag 0, u32 n=0}`, `str map = "LobbySquare01.map"` (**phải có đuôi** — nạp bằng
     `"Map/%s"`, file `assets/fx/Map/LobbySquare01..09.map`), `str theme = "Square"` (`"Theme/"+theme`,
     `assets/fx/Theme/Square`), `u32 0`.
   - 467: `u16 n = 0` → client đặt `this[4775]=1` và gửi **396** (= đã load).
2. **Crash khi nạp map** (gdb: `sub_4BECBE` → `sub_6EBC30(0x19)` → … → `sub_592BFA` → `eh vector constructor iterator` gọi
   "ctor" rác → nhảy vào địa chỉ stack, AV tại `0x0012D69F`). Nguyên nhân: **10 byte tại `0x592C09` hỏng ngay trong dump gốc**
   (`unpacked_ca.exe`): byte 1..6 của `push offset dtor; push offset ctor` bị thay bằng `FF 15 58 06 90 01`
   (`call [0x01900658]`, vùng không có trong dump). Khôi phục `68 E8684C00 68 90684C00` (dtor `sub_4C68E8` theo hàm sinh đôi
   `sub_592C2D` + unwind funclet `0xB1F8C0`; ctor `sub_4C6890` — hàm duy nhất còn lại ghi `vt_CMotionBlurFloorEffect`;
   byte 0 và 7..9 khớp sẵn) — patch trong `tools/patch_client.py`. Đã quét `.text` tìm `FF 15` trỏ vào vùng không ánh xạ:
   5 kết quả, 4 cái còn lại là byte trùng ngẫu nhiên trong lệnh/dữ liệu hợp lệ (0x5F48FD, 0x6E5E5B, 0xD0EA7F, 0xDF6367).
3. Còn để ý: tên hiển thị là `aName01` (chưa có tên thật — có thể từ login/467 rỗng); 298/303/319/64/476/477 client gửi
   vẫn chưa được trả lời (chưa thấy ảnh hưởng). `tools/wine_gdb.sh` có thêm `JOIN=1` (click "Kenh 1" sau khi vào màn kênh).
   Ảnh chụp đen 1 lần (`import` lúc đổi khung) không lặp lại ở lượt sau.

**Bước tiếp:** đóng hướng dẫn, thử TẠO PHÒNG (xem opcode client gửi) → trả lời để vào phòng chờ; điền tên người chơi.

### ✅ Cập nhật 8: TẠO PHÒNG → phòng chờ stage 12 (2026-10-17)
Ảnh: `docs/evidence/images/vn/create_room_dialog.png` (hộp thoại TẠO PHÒNG), `docs/evidence/images/vn/waiting_room.png` (phòng "DefaultRoomTitle2", 8 ô, chọn
nhân vật/đội/bản đồ, nút SẴN SÀNG). Chạy: `CLICKS="975,38,3 385,50,4 437,612,10 512,384,8" tools/xvfb_join.sh`
(đóng hướng dẫn → TẠO PHÒNG → XÁC NHẬN). CALog `Sg - INT(dne:12)`.

Tìm đường click bằng gdb (`tools/wine_gdb.sh`, biến mới `CLICKS=`): sự kiện nút **không** qua `CStage::vf5`.
`CCtrlButton__vf14` (0xA7A3BD, WM_LBUTTONUP 514) gọi `ICtrlEventHandler` slot +20 của cửa sổ cha `[nút+0x54]` (hoặc của
delegate `[cha+0x88]` nếu khác 0) với mã nút `[nút+0xF8]`. Nút sảnh: cha = **`CLobbyRoomUIWnd`** (vt 0xC23D1C,
ICtrlEventHandler ở +0xF4 → slot 5 = **`sub_4D3512`**): 2011 (0x7DB) = TẠO PHÒNG → `sub_4CB4C3`, 2012 = ĐI TẮT → `sub_4CB599`.

1. **Nút TẠO PHÒNG im lặng** nếu `CLobbyRoomUIWnd[+0x4585C]==0` (chưa nhận danh sách phòng) hoặc `[+0x45858]!=0`.
   `+0x4585C=1` chỉ đặt trong `sub_4D0F56` = **opcode 88 danh sách phòng**: `u16 n`; n≠0 thì blob n×(u16 session, u32 →
   `sub_6ACBB5`). Fakeserver gửi 88 với n=0 ngay sau 466/467 (`room_list()`).
   Bộ phân phối gói phòng `sub_4D24BB` (gọi từ `CLobbySquareStage__OnPacket`/`CLobbyStage__OnPacket`): 83 `sub_4CE9FC`,
   84 `sub_4CF53C`, 85 `sub_4CC007`, 86 `sub_4CBCE6`, 87 `sub_4D108E`, 88 `sub_4D0F56`, 90 `sub_4CABC7`, 330 `sub_4D1F27`,
   331 `sub_4C869E`, 334 (cờ `+300108`), 490 `sub_4CBD84`.
2. Bấm TẠO PHÒNG → `this[252]` (= `CNormalLobbyStrategy`, vt 0xC52BCC) vf40 0x6B8DD8 mở hộp thoại. XÁC NHẬN → client gửi
   **opcode 77**: `str tên phòng, str mật khẩu, u32×7 (mặc định 0,1,5,19,0,0,0), u32 -1, u32 0, u8 0` (ý nghĩa chưa gán;
   `parse_create_room()` giải và ghi log).
3. Server → **opcode 83** (`create_room_ok()`): `u8 0` (OK; 1 SameRoomExist, 2 NoEmptySlot, 3 CantCreateRoom, 4 NoAuthority,
   5..19 lỗi khác — `sub_4CE9FC`) rồi `CNormalLobbyStrategy` vf11 0x6B7B55 đọc `u16 số phòng, u32, u8, u8, u8` →
   **`sub_8F391D(12)`**. (`sub_4DD398` cùng định dạng nhưng là đường của sảnh quảng trường, cần `CLobbySquareStage[4793]`.)
4. Vào stage 12 client gửi 298, 303, 337, 476 (chưa trả lời). **Bước tiếp:** gói danh sách người trong phòng (để chính người
   chơi hiện trong ô) + nút SẴN SÀNG / bắt đầu trận — đang có agent OpenCode `a-room` phân tích tĩnh stage 12.

### ✅ Cập nhật 9: người chơi hiện trong phòng chờ (2026-10-17)
Ảnh `docs/evidence/images/vn/waiting_room_player.png`: nhân vật ở ô 1, kèm cờ (chủ phòng). Stage 12 = **`CPrepareStage`** (OnPacket rồi
chuyển tiếp `CSessionStage__OnPacket` → `CChatStage__OnPacket`). `CSessionStage__OnPacket`: 103 → vf141, 104 → vf143,
105 → vf144, 7, 81, 143, 144, 212, 394, 400 (`sub_6913F1`). `CPrepareStage__OnPacket` riêng: 94, 106, 107, 109–111
(vf146/147/151), 113–127, 134, 188, 209, 253, 289, 324, 372, 397, 406, 407, 409, 498–501, 538, 553.
- **Opcode 103 = đặt người chơi vào ô** (`0x692446` → `CSetPlayerOnPreparePacket` = `sub_573146` + `sub_572A05`), định dạng
  ghi trong `room_player()` của `tools/fakeserver.py`. `DecodeBuf` 3 byte (bitfield sẵn sàng/nhân vật/đội) dùng cùng mã
  hoá theo cụm như chuỗi (`sub_A26A10`) → thêm kiểu field `("buf", bytes)` vào `obf_body`. Blob được đệm byte 0.
  Fakeserver gửi 103 (ô 0, id 1000 = id trong login) khi client gửi **337** (lúc vào stage 12).
- Opcode 104 (vf143) = rời phòng (`u8 slot`; nếu là mình thì hiện Banned… và về stage trước).
- Còn thiếu: dòng chat "CoupleEntrance" (một field 103 để mặc định), bản đồ trống, nút vẫn là "SẴN SÀNG" (client chưa coi
  mình là chủ phòng) và bấm không gửi gì → cần gói thông tin phòng / chủ phòng / bản đồ (agent `a-room` đang phân tích).

### Cập nhật 10: nút BẮT ĐẦU bật (2026-10-17)
Ảnh `docs/evidence/images/vn/waiting_room_start.png`: ô 1 "TRƯỞNG PHÒNG", nút **BẮT ĐẦU**, khung nhân vật ("NHANH NHẨU").
- **Id người chơi = field 5 của gói login** (`sub_99A71D`; xác nhận theo `docs/archive/re/OC_D-LOGIN.md` của agent d-login + IDA).
  Trước đây fakeserver gửi 0 ở đó (giá trị 1000 nằm ở field 1 `sub_9B698F` – khác). Phải khớp user id của opcode 103,
  nếu không client không coi ô đó là của mình.
- Trưởng phòng = bit1 byte0 của trạng thái ô (`sub_8EA9B0` duyệt ô của các người chơi của mình).
- **Opcode 106** (`room_slot_state()`): `u8 slot, 3 byte trạng thái (DecodeBuf), u8`. Ô của mình + bit sẵn sàng không đổi →
  `CPrepareStage__vf153(this, 1)`: bật các nút (`sub_A7195B`), sprite `PRE_Bt_Start` nếu là trưởng phòng
  (`PRE_Bt_Ready` nếu không). Thiếu gói này thì nút phòng bị tắt — bấm không có gì.
- Sự kiện nút phòng: `CPrepareStage__vf5` → `sub_6CB7B6` (chặn nếu `this[15139]->vf44()!=0`, không lấy được trạng thái ô,
  hoặc `this[4777]!=0`). `0xBBA` (chỉ trưởng phòng) → vf152 hộp thoại chọn bản đồ; `0xBBB` → `sub_6C95AD` → `sub_6C63B6`.
- Bấm BẮT ĐẦU một mình → hộp thoại **"SmallPlayerNo"** (client tự chặn: thiếu người).
- Sau 106, client gửi opcode 88 (`u16, u32`) — chưa trả lời.

### Cập nhật 11: bảng map (opcode 234 + 231), map của phòng (113) (2026-10-17)
Ảnh `docs/evidence/images/vn/map_dialog_village.png`: phòng có map **Village01** (ảnh xem trước thật, tối đa 8 người).
- **Bảng map do server gửi** (`CDynamicMapMan` = `dword_E60930`, cache `fxdynmapinfo.ssd`): **opcode 231** (`CStage__On_231`)
  blob → `sub_6E077C`; chỉ nhận nếu checksum blob (`sub_A0E1E0` = adler32 khởi tạo 0, trên tag+data, lưu bởi `sub_A62580`)
  == `CDynamicMapMan[3]`, giá trị này đặt bởi **opcode 234** (`CStage__On_234`: blob {u8,u16,u32,u8 n,n×u8}, `u32` checksum
  map động → `sub_6E0F64`, `u32` checksum stage-map → `sub_6ABA21`, `u8`, `u32`). Định dạng bản ghi 231 + offset bản ghi 128
  byte: xem `dyn_map_blob()` trong `tools/fakeserver.py`. **Chuỗi trong sub-reader blob = u32 độ dài + byte** (`sub_A63A40`).
- `u16` ở +112 của bản ghi map = **chỉ số trong `CPlayMapInfoList`** (`dword_E60A10`, `sub_7376D3` nạp lúc khởi động mọi mục
  `Map/*` của `fx.idx` theo thứ tự idx = thứ tự chữ cái, 1019 mục; Village01 = 954, Forest01 = 223, Camp01 = 104). Số người tối
  đa lấy từ file `.map` (`sub_6DD119` → `sub_736E92`, mục 56 byte, +40).
- Danh sách map cuối (`sub_6E06A0`) chỉ giữ map có byte +4 == `CDynamicMapMan[27]` (`sub_4D6FC6`: 0 kênh thường).
- **Opcode 113** (`CPrepareStage__On_113`) = đặt map phòng: `u16 0, u16, u32 chỉ số map` → `sub_6DBFBE` (map hiện tại).
- Điều kiện số người khi BẮT ĐẦU (`sub_6C0990` → vf155 → `sub_988F2D`): tối thiểu 2 người/2 đội, trừ **loại phòng**
  (u8 thứ 4 của gói 83, `sub_8E78CB`/`sub_8E78DB`) ∈ {14,16,17,19,22,32,33} → 1/1. Loại 14 dùng danh sách map riêng của người
  chơi (`sub_9B63AA`), nên fakeserver mặc định loại 19 (`ROOM_TYPE`). Lỗi BẮT ĐẦU hiển thị bởi `sub_6C710D(mã)`
  (2 CantDecideMap, 3 AllMustReady, 4 SmallPlayerNo, 5 SmallMapPlayer[2], 6 SmallTeamNo, …), cũng dùng cho **opcode 116**.
- Nút BẮT ĐẦU = `sub_6C869F` (vf77): kiểm tra → gửi **opcode 97** (blob + thông tin map `sub_6DF9FB`) → `this[4777]=3`.
- Hiện tại: "CantDecideMap" — hộp thoại map vẫn liệt kê 0 map (bộ lọc `sub_6DEE48` loại hết; đang đo bằng gdb).

### Cập nhật 12: bắt đầu trận do server chủ động (501) → CGameStage (2026-10-17) — DỪNG
- **Opcode 501** (`CPrepareStage__On_501` 0x6C4978, định dạng trong `match_start()`; đối chiếu `docs/archive/re/OC_A-ROOM.md` của
  agent a-room + IDA) không đòi client gửi 97 trước, nên server tự bắt đầu trận được (`AUTO_START=<giây>`). Điều kiện: tên
  map khớp tiền tố mục `CPlayMapInfoList` (`sub_736FE3`), chỉ số map hợp lệ (`sub_6DE36E`), và cả 16 ô qua `sub_6BE6D0`
  (ô có người cần u8 thứ hai của bản ghi ≠ -1). Client gửi 298 rồi tạo `CGameStage` (stage 13).
- Khối VM `sub_9682DE` (0x9682E5, Init(chỉ số) của object thành viên 2 instance, gọi từ ctor `sub_9683B4`) được thay bằng
  bản tối thiểu trong `tools/devirt.py` — **suy ra từ ctor/dtor, chưa kiểm bằng trace**.
- Sau đó client crash trong vùng VM khi game context nạp map (backtrace qua 0x6ED8A6 → vùng `CGamePlayMap__Load`).
  **Dừng tại đây**: phần này nằm ở khối bảo vệ của Themida; chủ dự án đang tự xử lý (xem `docs/archive/re/VN_PROGRESS_LOG.md` mục 5).
- Hộp thoại chọn map vẫn liệt kê 0 map dù bộ lọc `sub_6DEE48` cho qua cả 3 map (gdb: RET=1) — vấn đề ở bước nhóm map,
  chưa xử lý (không cần cho đường 501).

### Cập nhật 13: hộp thoại chọn map — đo lại (2026-10-18, Claude Opus 5.5)
- Danh sách của `CMapSelectDialog` (vf36 → `sub_648EA4` → `sub_648E3D` → `CDynamicMapMan::sub_6E0D2C(all)`) chỉ thêm map
  khi: **bộ lọc `sub_6DF1B0` = `sub_6DEE48(map, 1, all, 1)` ≠ 0**, có nhóm (`sub_6DC8AF` ≠ −1) và nhóm đang mở
  (`grp[0] == 1`) hoặc `sub_6DC3FD(map)`.
- Nhóm (vector 48 byte ở `CDynamicMapMan+0x5E58`) dựng **một lần mỗi tiến trình** (`dword_EA4E68`) trong `sub_6E048D`
  (gọi từ parser 231): bản ghi có byte +40 thoả `sub_9889F4` (1,5,6,7,12,13,18,21,22,24,25,26,29) là tiêu đề nhóm; map
  thường tìm nhóm theo (+4, +110, +41) (`sub_6DBB28`), không có thì tự tạo nhóm mới (mở, đếm 1).
- gdb (`tools/gdb/mapdialog.gdb`, 3 map mặc định): nhóm dựng đúng (1 nhóm mới, 2 map vào nhóm), nhưng **bộ lọc trả 0 cho
  cả 3 map** ⇒ ghi chú cũ "bộ lọc cho qua (RET=1)" là đo nhầm biến thể khác (`sub_6DF1DB` = `sub_6DEE48(map,0,1,1)`).
- Điều kiện con của `sub_6DEE48` (mode `sub_6DC035` mặc định): `sub_6DCD82`, `sub_6DCF5B(map,0)` (item/khóa map),
  `sub_6DD516(map, room type)`, `sub_6DC81E`/`sub_8E7F2E`, `sub_6DC856` (+108 == 3), `sub_6DE4CD` (khi all = 0),
  `sub_6DC43E` (`sub_988A3D(+40)`). Script đo từng điều kiện: `tools/gdb/mapfilter.gdb` (lần chạy bị phiên khác
  dùng chung Xvfb/client cắt ngang — chạy lại khi rảnh: `JOIN=1 CLICKS="975,38,6 385,50,8 437,612,20 512,384,15
  900,575,15" sh tools/wine_gdb.sh tools/gdb/mapfilter.gdb tester 1234 480` với fakeserver chạy sẵn).
- fakeserver `ALL_MAPS=1`: bảng 231 đủ 1019 map (`all_battle_maps()`, theme theo tiền tố thư mục `Theme/*`, không khớp →
  `ThemeDefault`); client nhận (fxdynmapinfo.ssd ghi), vẫn vào trận Village01 bình thường; hộp thoại vẫn rỗng (cùng lỗi
  bộ lọc trên).

## Cập nhật 13 (2026-10-18 đêm) — danh sách phòng + vào phòng người khác ✅ runtime (2 client)
- **S→C 88** (`sub_4D0F56`): `u16 n`; nếu n ≠ 0: blob (tag 0 + little-endian) n × bản ghi (`sub_6ACBB5`):
  `u16 số phòng (session, < 560/1049 — hiện "S02" cho 1)`, `u32 id phòng`, `u32 len + tên`, `u8 +76 có mật khẩu`, `u8 +88`, `u8 +104 đang chơi`,
  `u16 +116`, `u8 +128 số người`, `u8 +140`, `u8 +152 tối đa`, `u8 +92`, `u32 +176`, `u8, u8` (`sub_97C722`), `u8 +200`, `u8 n2` (n2 × {u8 k; k==0: u32}, `sub_6ACA64`).
  Bấm được khi `+128 < +152` và `+104 == 0` (`sub_6AC879`). Trường không ghi chú: CHƯA KIỂM (gửi 0).
- Bấm ô phòng (nút 2500+i, `sub_4D1436` → `sub_4CB829`) → **C→S 78**: `u16 số phòng`, `u32 id phòng`, [`bool 1` nếu `sub_8DAD14`], `str mật khẩu` (hỏi khi +76 ≠ 0),
  `u8 n người chơi trên máy`, n × `u8` (nhân vật). Server trả **83** như tạo phòng → client vào phòng chờ → 337 → server xếp vào slot kế tiếp.
- Server Rust (`hub.rs`): 88 liệt kê phòng chung ("Phong 1", người/8, WAITING/đang chơi), gửi lại cho mọi client đang ở sảnh khi có người vào/rời phòng.
  Chạy thật (`logs/duo/t5`, `docs/evidence/images/vn/lobby_room_list.png`): A tạo phòng; B thấy "S02 Phong 1 WAITING 1/8", bấm vào → C→S 78 → 83 → phòng chờ → cả hai vào trận.
