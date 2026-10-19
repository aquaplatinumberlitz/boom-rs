> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# c-square — Sảnh quảng trường (CLobbySquareStage, stage 11)

Nguồn: IDA db riêng `~/boom-re-oc/ida/c-square.i64` (unpacked_ca.exe). Quy ước xem `docs/specification/networking/LEGACY_VN_PROTOCOL.md`:
handler `On_N` = opcode N; `CStage__OnPacket` case = opcode−2; đọc `RecvPkt__Decode*`, gửi
`SendPkt__Encode*`; gửi opcode = `sub_88849F(op)` hoặc `SendPkt__Begin`+`Encode*`; field game = "value
box" (`sub_413F32` set / `sub_414625` get u8 / `sub_414543` get u32). Độ tin cậy: 🟢 = đọc từ
decompile, ❓ = chưa kiểm (CHƯA KIỂM).

Đối chiếu runtime: `docs/archive/re/VN_LOBBY_LOG.md` Cập nhật 6/7 (client vào được stage 11, trả lời 466+467 là load xong).

## 1. CLobbySquareStage__vf44 (OnEnter) 0x4DED62 — 🟢

Flow (decompile, theo thứ tự gọi; `this` = stage):
1. Log `xrptLbyssInt`; `this[4776] = sub_A28F50("LobbySquareSta…")`, `this[4777] = …` (chuỗi log/label).
2. Gọi `CChatStage__vf44(this)`; bằng 0 → log lỗi + thoát (không qua sảnh).
3. `sub_6B7221(buf)`; qua `*(this[5542])` vf+12 (`this[5542]` = object con/particle), có `dword_E61020`/`dword_E60A0C` → `sub_8EE556(i,…)` ×6.
4. `sub_6551CB(4,2)`, `sub_8E78CB(0)`, `sub_4DE792`, `sub_4D5AA8(this)`, `sub_4D6B56`.
5. **Gửi 476** (`sub_692C71`) khi `Feature_IsDisabled_stub0()==0`; `sub_71C46A(2)`, `sub_89FA44`.
6. `sub_93BEE1`, `sub_467AFB`, `sub_940F55` (khởi tạo UI).
7. **Gửi 319** (`sub_67A132`, guard `this[3085]` — chỉ 1 lần).
8. `sub_4D6FC6(this)`, **Gửi 477** (`sub_692CC8`), **Gửi 395** (`sub_4D5C25`).
9. Gọi `(*this[5542] vf+80)`; khối nếu !Feature: kiểm `sub_9B6F75(3)`, `sub_9B6F75(4)`, `sub_8DAD87`/`sub_8DAD95`/`sub_8E9E73(dword_E60884)` = chế độ tournament — hiện dialog "BundleCreatetourname…" (UI, không packet).
10. `sub_8E7C8E(0)`, `sub_8E796D(dword_E60884)==0` → `sub_8EB853` ("gameserver connect"?); `sub_4D727A(this)`, `sub_4D7C16(0)`; if `sub_41C730(dword_E60840)` → `sub_93C767`, `sub_467AFB`, `sub_697E70`.
11. Log "gameserver conncnt"; return vf41 (`sub_9DB5CB`).

⇒ Thứ tự gửi khi bước vào: **476 → 319 → 477 → 395** (đúng quan sát runtime). 298/303/64 gửi từ chỗ khác trong stage / CStage base.

## 2. Các gói client → server lúc vào sảnh — 🟢

Quy ước: payload là các dòng sau `Encode*`, đọc thứ tự ghi (big-endian, obfuscate theo mode 1/2 — xem protocol.md).
Opcode ghi như u16 (XOR 0xA569 khi mode 2/3).

### 298 — `sub_627FCC` (site 1) / `sub_9ADBDF` (site 2)
- Site 1 `sub_627FCC(this)`: `SendPkt__EncodeStr(str = *(this+2706))`, `SendPkt__EncodeBool(false)`. `this+2706` = 0x2A38 (❓ nội dung trường — kiểu str lưu trong stage; so sánh CSelectServerStage gửi str id đã mã hoá — CHƯA KIỂM nghĩa).
- Site 2 `sub_9ADBDF()`: n = `CLobbyStage__On_218(dword_E60880)` (số mục nào đó trong lobby, runtime n≤1);
  - n ≤ 1: gửi **1 gói** `{str sub_99A731(0), bool true}`;
  - n > 1: gửi **n gói** `{str sub_99A731(i), bool true}` i=0..n−1.

### 303 — gửi từ `CStage__vf44` 0x6A3EDA (base CStage; CLobbySquareStage kế thừa)
Điều kiện: `id stage hiện tại = *(dword_E60880+0xD8)` (getter 0x8F8893, mọc tên
"?GetCompletionList@UMSSchedulerProxy…" — thực ra là `return *(this+54)` dword):
`∈ {11, 12}` → `m = 1`, `== 20` → `m = 0`, khác → không gửi.
Và `n = CLobbyStage__On_218(dword_E60880)` = giá trị box `*(CMainWnd+12)` (u32).
Mỗi i = 0..n−1: `sub_88849F(303)` rồi **2 × `SendPkt__EncodeBool` (0xA65240, đã soi cả 2 call
= cùng đích)**: `{bool m, bool i(byte index)}`.
Nghĩa tổng thể 303 = ❓ (phản hồi chỉ có case ở CGameStage — xem §4).

### 319 — `sub_67A132(this)` : gói rỗng (0 field) 🟢
Guard `this[3085]` (chỉ gửi 1 lần/nhập, reset khi ra vào lại sảnh); trả về 0 nếu đã gửi ⇒ **yêu cầu 1 lần**, không lặp.

### 64 — `sub_679EF4(this, a2)` : `u16` field 🟢
Giá trị = `a2 + *(this+1257)`. Kẹp: `a2 + *(this+1257) >= 0` và `*(this+1258)==0` mới gửi; sau gửi `*(this+1258)=1` (lần đầu trong stage).
Nghĩa: lật trang/nhóm danh sách tin gì đó — nghĩa đối diện ở handler On_64 (xem §3).

### 476 — `sub_692C71` : rỗng (gọi từ vf44 và có thể từ chỗ khác) 🟢
### 477 — `sub_692CC8` : rỗng 🟢
### 395 — `sub_4D5C25` : rỗng 🟢 (→ server trả 466 + 467 theo lobby.md Cập nhật 7)
### 396 — client gửi sau khi gói 467 xử lý xong (cuối On_467 đặt `this[4775]=1`) — xác nạp xong (chưa soi field).

## 3. Server → client — opcode 466/467 (thông tin sảnh + người chơi)

### 466 = thông tin khu/map (On_466 0x4E52B5 — khớp lobby.md Cập nhật 6)
```
opcode 466 (server→client): u32 area, blob, str, str, u32
```
- `u32 area` → `this[5541]`; `sub_8E7C8E(18<=area<=19)` = cờ chế độ khu.
- blob: `u32 n, n × {u32 m, m × u32×5}` (vật thể khu, `sub_4E4EFD(sub_4E134C)`).
- `str` = tên map (`"Map/%s"`), `str` = theme (`"Theme/%s"`, `sub_8DA6F5` nạp), `u32` (❓, nạp cùng map).
- **Tên khu trên thanh trái** khi area ≠ -1: key `"LobbySquare_AreaName%02d"` (aLobbysquareAre;
  ref từ On_466 0x4E545E, On_27 0x4E0DE8, CChatStage__On_619 0x681AB3) + area → `sub_415B2F`
  (so số `%`-placeholder rồi sprintf `sub_A205F0`; không format được → `sub_4014BC`) rồi
  `sub_4D3EBE` vẽ. area = -1 → dùng tên kênh (`sub_993FD6(sub_9995C2(dword_E60784))`).
  Nhánh ≠ -1 còn ghép `"%s-%s"` (aSS_3 0xC24FD0, `sub_42A941`) với tên kênh —
  ❓ chuỗi đó đổ vào element nào.

### 467 = danh sách người trong sảnh (On_467 0x4E4100)
```
opcode 467 (server→client): u16 n, n × {u16 slot, u32 uid, blob "profile", u8, u8, u16 x, u16 y,
  u8, blob, bool [blob], bool(feature74) [blob], u32 m, m × blob}
```
- Chặn `sub_4D82DE()` (đang nạp map); mỗi record `sub_4E3F29`: đọc **u16 slot** — bỏ nếu
  `slot >= sub_6E3D75()` (số slot tối đa `*(object+54)`).
- `sub_4E2D7E(this, slot, a3)` (2004 byte) đọc lần lượt:
  1. `u32` → `obj[+36]` (truyền vào `sub_679434`), obj = `malloc 0x198` + `sub_469AC8`.
  2. **blob "profile" → `sub_679434`**, thứ tự trong blob (các field lưu qua value box `sub_413F32`):
     - **str tên** → `sub_40E909(obj+8)` — field đầu blob! Getter tên = `sub_678ED7`
       (thunk `sub_A67960` = `return this+8`).
     - u8, u8, u32, u8, u32, u16, u32 (box), u32 → obj+188 (this[47]
     - u32, u32 (bán qua `sub_44AD23` — ❓ kiểu), u8, u32, **str thứ 2** → obj+160
       (❓ tên phe "handle"; dùng cho UI khác),
     - u16 ×3 → box, u16 đọc nhưng đây chỉ dùng do dump ❓ không rõ đâu,
     - u32 → `sub_679056` (❓), u32 (biên 0..1) → box, u32 ×4 → obj+392..404 (this[98..101]).
  3. `u8` (v39), `u8` (v42): bitfield — build `j` = mask **0x3F (bits 6..11) từ v39,
     0x1F (bits 13..17) từ v42**; sau đó `sub_8E76A3(slot, v39)`, `sub_8E8EBB(slot, v42)` — ❓
     hai u8 = hành vi "đi/vào khu" hay trạng thái hiển thị.
  4. `u16 x, u16 y` — pixel; **tile = x/40, y/40** (lọc `sub_6EB788(x/40, y/40)`) → **tọa độ**.
  5. `u8` (v5) — bitfield thứ 3 (`sub_8EA539(slot, …)` trước đó đã xoay `j` mask `0xC0E0`: ❓).
  6. `blob` → `sub_8EABEC(slot)`; nếu u8 đầu blob (`sub_A62F40`) ≠ 0 → `sub_97EDBC(blob,0)` rồi
     `sub_8ED30A(slot, …)` — ❓ nghĩa (phần UI text?).
  7. `bool` → nếu 1: `blob` → obj new 0x74 (`sub_4720DD`, `sub_9C9EB6`) → `sub_8EAEFA(slot, …)` — ❓.
  8. `bool` (feature74, stub luôn 0) → nếu 1: `blob` → `sub_40D664`, `sub_4BE6CD`, `sub_42C75D`,
     `sub_4D4787` ❓ (nhánh bỏ khi feature bật).
  9. `u32 m` → m × blob (mỗi blob: nếu u32 đầu == 1 → u16, u32 → `sub_86493F`, `sub_8664D7` — ❓ buff).
  10. feature76 stub==0: dùng đã đọc xong — `sub_8E8014/8E7F9A/8E7FD7/8EB468` + `sub_5A2BCD`
      (không đọc thêm từ wire).
  - Cuối: `sub_4DC5C2(slot, obj)` = gắn bản ghi vào "bảng slot người chơi", và
    `sub_4D8156(name…)` → `sub_8DD45B(name)` = **tìm tên trong bảng 64-entry × n tab tại
    `dword_E60880+676`, mỗi entry 21 byte (stride 1344/tab, n = `CLobbyStage__On_218(CMainWnd+12)`
    — box)**; thấy → `sub_86472A(1)`. ❓ băng đó là gì trong GUI.
- Kết thúc handler: bật UI self (`sub_592ADE(1)`; key "Game/SelfIndicator" — nghĩa ❓), rồi:
  - **`sub_679EF4(dword_E61018, 0)` = GỬI op 64 (a2=0, guard `this[1258]` lần 2 bị chặn)**,
  - đặt `this[4775] = 1` (flag "đã nạp danh sách", để thoát "Đang tải"),
  - **gửi op 396 rỗng** = "xac nhận nạp xong".

### Vì sao thanh trên trái hiện "aName01" — không phải tên người chơi
- KHÔNG phải tên người chơi. On_466 vẽ thanh khu/top-left bằng key **`"LobbySquare_AreaName%02d"`**
  với `area` chèn vào; fakeserver gửi area = 1 ⇒ chuỗi `LobbySquare_AreaName01`; phần đầu string
  dài cho ô hiển thị nên chỉ thấy phần đuôi "**aName01**" (cách cắt chính xác: CHƯA KIỂ).
  Bản dịch không tồn tại: key KHÔNG thấy trong `assets/fx/ClientData/ClientString.bin`
  (nội dung opcode 2 FxClientString) — server thật dùng FxMessageTable (opcode 21) làm bảng dịch,
  client không có cache nên hiển thị thẳng chuỗi đã format từ key.
- Tên người chơi thật lấy từ **2 nguồn**:
  1. **Login opcode 4** (`sub_852BCA`): mỗi player có 2 str (getter `sub_99A746`/`sub_99A817`
     theo d-login; player id `sub_99A71D` phải khớp uid) — **tên của chính mình** trong UI.
  2. **Opcode 467 record** (mỗi người khác trong map): **str đầu trong blob "profile"** →
     `sub_40E909(obj+8)`; getter tên `sub_678ED7` → `sub_A67960` (`this+8`).
     Với 467 rỗng (n=0 như fakeserver) không ai hiện trên map; UI self vẫn vẽ.

## 4. Handler server → client trong quảng trường (CLobbySquareStage__OnPacket 0x4E5A38)

Dispatch riêng của stage: 26, 27, 32 (vf240), 79, 234, 265, 289, 394, 419, 426, 466, 467, 469,
470–481 (trừ 479 → mặc định return), 482–489, 492, 493, 496, 506 (vf233), 507, 508, 576,
612–617, 622–625; cuối gọi chain phụ `sub_42DA98 → sub_4D24BB → sub_4D39EB → sub_4C011B →
CChatStage__OnPacket` (chat riêng 618+, không lặp vào đây). Tất cả (trừ 467) chặn bởi `sub_4D82DE()`.

**Chuyển động / trạng thái người chơi**:

| Op | handler | format (opcode N (s→c): ...) | ý nghĩa |
|---|---|---|---|
| 469 | 0x4E36B1 | u16 slot → `sub_4E2D7E(slot,…)` | **người mới xuất hiện** (cùng bản ghi 467) |
| 470 | 0x4DA834 | u16 slot → `sub_4D8DBB(slot)` + `sub_4D88AD()` | **người rời sảnh** |
| 471 | 0x4D8E98 | u16 n, n × 8B: `u16 id`, `u16 → this[3183] = dword0>>16`, `u32 → 2 box (dword1, byte2 của dword1)` | **lô cập nhật vị trí/param nhiều người cùng lúc** — mỗi mục tìm obj tên qua `sub_4BE6CD(id_lo16)` (mục 0 bỏ qua; cần vf+388 trả khác 0), rồi `sub_4D5560`: ghi giá trị + nếu `this[3513]==0` đặt `timeGetTime()`, gọi `sub_4D51F6` (cập nhật chuyển động) — 8B trọn vẹn 🟢, nghĩa từng số ❓ |
| 472 | 0x4D8F32 | u16 slot, u8 act, u16 val, u8 flag, [flag≠0 → u8 → `sub_593D85`] | **lệnh hành động/đổi trạng thái cho 1 người** — flag≠0 thì chạy hiệu ứng `sub_593D85`; flag=0 thì chỉ khi state obj `(charobj+1296)` vf+40 < 0 mới tiếp (`sub_59C47F`→`sub_970BF0`→`sub_595D7A(act,val,0)` = bắt đầu hành động, kiểm "đang bận" vf+196, gọi vf+220(-1) + vf+260) — tên hành động ❓ |
| 473 | 0x4DA862 | u16 slot, u8, (chỉ khi `(charobj+1296)` vf+40 < 0) → `sub_4D944D(slot,u8)` | **đổi trạng thái hiển thị của 1 người**: bitfield qua `sub_8EA71F`/`sub_8E977D` (byte mask 0xE0/0xC0, XOR vào cánh 0x1F), sau đó cập nhật 13 mục + 9 slot nội bộ (`sub_8D4A0F`, `sub_8E9814/8E983C`) — cùng họ với bitfield 2×u8 của 467; nghĩa bit ❓ |
| 474 | 0x4E26BB | u16 slot, u8, [blob + bool + [bool1 → blob]] | **"467 thu gọn" cho người đã có** (bản như 469-full nhưng thiếu `u32+blob profile` đầu): cùng chuỗi cập nhật 13 mục + 9 slot + blob + `bool` → blob nếu 1 thì nạp object UI (`sub_9C9D9A`/`sub_9C9EB6`), feature 41/76; 1731B chủ yếu là 13×9 vòng update — nghĩa blob ❓ |
| 475 | 0x4DF1F8 | u16 slot, u8 s → `sub_4DC5C2(slot, s)`: s = 0 off / 1 on / 2 hiện text "LobbySquare_…Ing" | **trạng thái "đang chơi/đang làm gì đó"** của người trên slot (dot trạng thái + text) 🟢 |
| 493 | 0x4D92F2 | u16 slot | ❓ |

**Các case còn lại của dispatch (476–481, 492) — handler xác nhận 🟢, nghĩa cả enum ❓**:

| Op | handler | format | ghi chú |
|---|---|---|---|
| 476 | 0x4DAC78 | u16 slot, u32, str | lấy bản ghi `sub_8E99A3(slot)`, `u32 → sub_6790BC`, `str → sub_67962A` rồi áp lên char obj (`sub_59C91D`) — nghi "đổi tên/chỉ số hiển thị" ❓ |
| 477 | 0x4D9060 | u16 n, n × {u16 id, u8, [u8=0 → u32]} → `sub_6E9994(id,u8,u32)` | danh sách mục trạng thái/cần nạp (n cuối gọi `sub_875139`) — ❓ |
| 478 | 0x4D90F5 | u16 slot, u32, u32 (sub_44AD23), u32, u8×3 (box), u32 (box), u16 (sub_44AD23), u16 (box) → `sub_6E9AF9(slot,…,-1,-1,-1,…)` | hồ sơ/điểm số cho slot; hợp lệ → `sub_6EFC1C(slot)` — ❓ |
| 480 | 0x4E14B1 | u32 id, u8 n, n × {u16, u16, u8, u8} → `sub_6EBAE0(0, id, …)` | danh sách cặp giá trị theo id — ❓ |
| 481 | 0x4DA8E7 | u16 n, n × {u32, u8, u8, u16 m, m×u8, [u8=2 → u32]} | reset `this+19176` trước (`sub_87461A`), mỗi mục `sub_875C11/87541A` + có thể `sub_8753A8`; cuối `sub_4D9379()` — ❓ |
| 492 | 0x4E242D | (không đọc) → **GỬI 419**: bool, bool, u16 n, n × {bool, bool} | server yêu cầu client gửi lại **419** (trạng thái từng người, qua `sub_726BA7/726BB2`); lưu ý opcode 419 dùng cả 2 chiều ❓ |

**Chat/khu trong sảnh** (cửa chat thuộc CChatStage; nhóm 612–617 là handler riêng của CLobbySquareStage, 618+ là chat):

| Op | handler | format | ý nghĩa |
|---|---|---|---|
| 612 | 0x4DE0C0 | u8 mode | mode 0 = **load lại khu**: hiển thị "AreamoveLoadin", this[5541]=-1, **gửi lại 395**; mode 1 = `sub_4C1393` |
| 613 | 0x4E36D2 | u8, u16 slot, u32, blob, u8, [sub_572A05 tự đọc thêm], blob, bool, [bool1 → blob] | **refresh bản ghi 1 người có sẵn** (khung như 474 + thêm `u32 + blob profile` đầu = parse qua `sub_679434`, giống blob "profile" của 467) + trên hết📎 vẽ UI self: xâu key "Game/SelfIndicator" → vf+152(this, u8-đầu, chuỗi) = **bật indicator của chính mình**; kết thúc gọi vf+424(this, 24, 1972) — nghĩa u8 đầu ❓ |
| 614 | 0x4D9C6F | u32 ×3 → `sub_4BFDA5(u32,u32,u32)` | chặn bởi `this[4775]` (đã nạp); nếu số tab kênh ≠ 2 và UI điều kiện hợp lệ thì bỏ qua (lọc theo tên tab đầu `sub_99A731(0)`) — ❓ (nghi bộ đếm/thông báo) |
| 615 | 0x4D5E7B | (rỗng) | `sub_4BFE37(this+337280)` — không đọc packet 🟢, nghĩa ❓ |
| 616 | 0x4D9D56 | u32, u32, u32, u32, str | **tìm người theo uid qua các tab kênh**: so `u32#2` với uid từng thành viên tab (`sub_99A70C(i)`) + đúng tên (`sub_40E0D6`) — nếu không khớp ai trong các tab hoặc không có tab → `sub_4C0D80(u32×4)`; chức năng 4×u32 ❓ |
| 617 | 0x4D5E89 | u32 → `sub_4C0957` | ❓ |
| 622 | 0x4BCB30 | u16, u8, u32, u16 + timestamp | store vào struct (`this+85019` của stage): `this[0]=timeGetTime()`, `[1]=u32`, `[2]=u16`, `[3]=u16`, `[4]=u8` — ❓ (bộ đếm giờ của sự kiện gì đó) |
| 623 | 0x4BCB79 | (không đọc packet; reset struct) | đặt `[0]=-1, [1]=[2]=0, [3]=[4]=-1` — xoá trạng thái 622 🟢 |
| 624 | 0x4BCC41 | str, str → `sub_5FB0E4(tên1, chuỗi2, màu (0,76,161))` | **hộp thông báo 2 chuỗi** (chuỗi 1 qua `sub_98DB56` sao chép/gói, chuỗi 2 hiển thị thẳng, màu tím nhạt) — nội dung ❓ |
| 625 | 0x4BCB8F | u16 → struct[2] (`this+85019`) | đặt giá trị của mục 2 trên struct 622 (không chữ khác) 🟢 |

**Sự kiện khác**:

| Op | handler | format | ý nghĩa |
|---|---|---|---|
| 26 | 0x4E061B | u16, u32, str, u8, str, u8 → mời đi cùng "ReceiveInvite…/RoomDestroyed" | mời/mời refund khu |
| 27 | 0x4E0D5B | u32, str | mời đi cùng (dialog "LobbySquare_InviteUserArea", kiểm sub_8DD45B) |
| 79 | 0x4DAD83 | u32 slot, str | feature76: slot hợp lệ (≤ `sub_6E3D95`) → `sub_6E5C2B(slot)` + str → `sub_5A2C2B(str,…,76)` — gán thông tin dạng chuỗi cho 1 người ❓ |
| 234 | 0x4DBC0D | (sub-parse; các chuỗi "GuildJoinSuccess2", "GuildAcceptedButLimitOver"…) | kết quả vào guild |
| 265 | 0x4DEA21 | u16 n, n × str | **danh sách chuỗi hiển thị**: mỗi str sao chép → `sub_98DB56`, rồi `sub_5FABCC(…, màu (0,76,161))` — thông báo/tên ❓ |
| 289 | 0x4D5BAE | (không decode) → `sub_6952A5(pkt)` = parse trong sub; 0 → vf268(this,0) | ❓ |
| 394 | 0x4DF9A8 | str, u8, u8, u8 | **danh sách tên có highlight**: phân mảnh chuỗi (`sub_4DCFC5`, tối đa 2 mục 0x1C), mỗi mục tìm người `sub_6E7AE0`; màu vàng (255,216,0) nếu u8#1 ≠ 0, màu (235,160,255) nếu str#0 so sánh đúng. Ý nghĩa qua ngữ cảnh người gọi — ❓ |
| 419 | 0x4E0FB7 | str, u32, u32, u32 → "UMItemSold1" "[%s] " | **báo bán đồ trên chợ (User Market)** |
| 426 | 0x4E119C | u8, blob → "MagicBookMarket_SoldItem" | báo bán sách phép (MagicBook) |
| 482/483 | 0x4DC39E/0x4DC4B0 | u8 → "DominoAlreadyIgnite/Full/Invalid" | sự kiện domino |
| 486 | 0x4DFC08 | u16 slot, u32, u32, u16, u16, u8 | domino-event hoàn thành |
| 487 | 0x4DF224 | rỗng | no-op |
| 488 | 0x4DF798 | u16 → "PlayWithYou" | lời mời chơi cùng |
| 489 | 0x4DF238 | u16 slot, u8 type (thêm str = tên người đọc từ bản ghi slot) | **kết quả PlayWith**: type 0 → `sub_4D6B6B` + `sub_4DD17B` (bắt đầu/thoát playing); type ≥ 1 → dialog lỗi "PlayWithInvalid…" màu — ❓ map type→chuỗi |
| 496 | 0x4DAF64 | u32, u32, u8 | so 2×u32 với **hash tên** (`sub_A1FE70(tên)`) của từng người trên map; người nào khớp → tạo object 0xB8 + `sub_576289(charobj, sub_6E206E(slot), str…)`. Nghi "làm nổi/khoá theo tên" — ❓ |
| 507 | 0x4DDB1E | u8 → "NoEmptySlot" | lỗi tạo phinaire |
| 508 | 0x4DDBC5 | u8 → "CantFindEmptyRoom" | lỗi tạo phòng |
| 576 | 0x4D6E91 | (CStage__On_576 + sub_51607D(CMainWnd)) | giữ hành vi base |

**Phản hồi các gói client gửi khi vào sảnh**:
- 298: phản hồi chỉ tồn tại tại **CGameStage__OnPacket (71B631) case 298** → `sub_7035C4` — thuộc
  stage gameplay, không rút phản hồi cho sảnh (chắc server **không trả** 298 ở sảnh, gói gửi
  tuỳ gói từ CNM/SMS dialog — ❓).
- 303: phản hồi analog chỉ có tại CGameStage case 303 (`sub_70727E`) — ở sảnh không trả.
  Ở sảnh nghĩa 303 = tiềm năng "hỏi trạng thái từng kênh/tab" vì điều kiện gửi từ base stageEnter.
  per-tab index bool — =❓ (từ `CStage__vf44` và chỉ khi stage ∈ {11,12} hoặc 20).
- 319: **CStage__OnPacket case 319** (0x6A978B) → parse dạng "multi_0_1_4_6" (`OnPkt_multi_0_1_4_6`
  0x40176C + `sub_695FE5`) = gói dữ liệu kiểu multi: kiểu `bool/u8/u32/u16` chọn từ case ❓.
  Server NÊN trả gói 319; chạy trong sảnh lần 1.
- 64: **CStage__OnPacket case 64** → `CStage__On_64` 0x6948BE: `blob` → `sub_8B25A2`:
  `u16 n, n × record` (record parse trong `sub_8A1514`, obj malloc 0x238), cache ra **fxstock.ini**
  (`sub_A62C20(aFxstockIni)`); n = 0 thì giữ dữ liệu cũ + ghi file + `this[8]=1` + `sub_8B2064(...)`.
  Tên file "fxstock" gợi **bảng giá/mua bán (stock item)** — nghĩa record `sub_8A1514` ❓.
- 476/477/395: trả **476 / 477 / (466+467)** như trên (chính handler cùng opcode).
- 396: không có handler phía client — client chỉ GỬI; server có thể bỏ qua.

## 5. Kết luận trả lời trực tiếp câu hỏi task

1. **Client gửi gì khi bước vào sảnh (vf44)**: `476 (rỗng) → 319 (rỗng, 1 lần) → 477 (rỗng) → 395 (rỗng)`;
   sau đó từ base/flow khác thêm `303 (bool,bool ×n tab, khi stage∈{11,12}→m=1; stage 20→m=0)` và
   `64 (u16, lần đầu khi guard this[1258]=0)`; 298 (str+bool ×n) chỉ khi mở dialog SMS/CNM.
   **Client PHẢI nhận gì để nạp xong**: `395 → 466 + 467` (467 có thể rỗng n=0 — UI self vẫn vẽ),
   `476 → 476`, `477 → 477`, `319 → 319`, `64 → 64` (có thể rỗng blob). Khi client xử lý xong 467 nó
   **gửi 396 rỗng** (ack nạp xong) — server có thể bỏ qua. Sau đó server có thể bắn 469/471/472… tùy.
2. **Khung bản ghi 467/469/613/474** — cùng một chuỗi thân đọc bởi `sub_4E2D7E`:
   `u16 slot, u32, blob(profile: str tên đầu → obj+8), u8, u8, u16 x, u16 y, u8, blob, bool+[blob],
   bool+[blob], u32 m, m×blob`; 613 thêm tiền thân `u8 mode, u16 slot, u32` (hoặc tương đương),
   474 không có phần blob profile. record = obj 0x198, gắn bảng slot bằng `sub_4DC5C2`.
3. **"aName01"** — chuỗi key `LobbySquare_AreaName01` (area=1) vẽ trên thanh khu (top-left),
   dài hơn ô hiển thị nên chỉ thấy đuôi "aName01"; KHÔNG phải tên người chơi. Fakeserver muốn
   đẹp thì gửi `area` khác/không gửi, hoặc thêm bản dịch của key qua bảng FxMessageTable (op 21).
4. **Sự kiện di chuyển**: các handler 467/469/613/474 trong sảnh đều là thông báo do server chủ
   động bắn; gói client gửi để di chuyển chính mình trong task này KHÔNG soi (❓) — kết luận: thông
   báo người khác di chuyển bằng **471** (lô 8B ×n).
5. **Chat**: handler chat nằm ở `CChatStage__OnPacket` (618+); sảnh riêng chỉ có 612–617 (612 =
   mode reload khu → client gửi lại 395).
