> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Phòng chờ (stage 12, CPrepareStage) — đặc tả tĩnh cho server giả

Phạm vi: phân tích tĩnh `unpacked_ca.exe` (bản copy DB riêng `boom-re-oc/ida/a-room.i64`),
tia tiếp docs/archive/re/VN_LOBBY_LOG.md (Cập nhật 6/7, Errata audit) và docs/specification/networking/LEGACY_VN_PROTOCOL.md. Độ tin cậy:
🟢 decompile rõ · ❓/CHƯA KIỂ = suy đoán.

## 0. Khung chung — vòng lặp sự kiện phòng chờ

- Handler sự kiện (click nút/ô) của **CPrepareStage**: `sub_6CB7B6` (0x6CB7B6). Gate đầu:
  vf(44) của `*(stage+15139)` (stage-mgr) trả 0 mới xử lý; `a2` = id sự kiện.
  Bản ghi event: `0xBBA`=chọn bản đồ→`sub_6BE40F`; `0xBBB`=→`sub_6CB78F`
  (đến `sub_8EA9B0()!=0` mới chạy `sub_6C95AD`); `3616..3624`(0xE20..0xE28)=cửa
  hàng/nhặt; `3620`=clear cờ; `3350`(0xD16+)=`sub_6C1A53`; `3500`(0xDAC)=BẮT ĐẦU→
  `sub_6C93B0`; `3617/3618/3619` hướng về các nút; `3616/3618`→`sub_6C3A62(3617)`/
  `sub_6587B8(447)`; `3200`(0xC80)=`sub_6C0585`; event đồng thời `0xCE3..0xCEB` →
  `sub_6C1947`. 🟢
- O kak raz: a2 ∈ [3400..3400+this+4776): click **ô slot hàng 1** → `sub_6C9413`;
  `a2 ∈ [3600..3615]`: **sub_6C62A4** (gửi opcode 212, ❓ mục 4).
- Đvény chờ/lock **`this[4777]`** (pre-stage):
  `0`=rảnh · `1`=đã gửi 88 (xin đổi chỗ/phòng) · `2`=đã gửi 94 (toggle slot) ·
  `4`=đã gửi 87 (BẮT ĐẦU).While != 0 các hàm gửi đều từ chối. 🟢
- `sub_8EA9B0()` (0x8EA9B0): trả 1 nếu **ØNG chơi+khiên (host) của phòng có bit1(0x2)**
  (? — đọc `CLobbyStage__On_218(dword_E60880)` số người chơi, duyệt mỗi slot qua
  `sub_8EA779(&3byte, i)` — slice trạng thái 3 byte = bit0 sẵn sàng, bit1 host, bit2
  một cờ trạng thái; dung trường hợp `state&2`). = "có host trực tuyến". ❓ "MỌI LUỒNG
  GỬI 88/87/94 ĐỀU ĐÒI HÀM NÀY ≠0" — tức chỉ người phòng nào đã trong ô mới gửi được. 🟢

## 1. Luồng Ô người chơi / SẴN SÀNG (opcode 94)

### 1a. Client gửi — `sub_6BE8AF` (0x6BE8AF) 🟢

Click **ô của chính mình** (sub_6CB7B6 × slot-range 3400 → `sub_6C9413`;
điều kiện `(state&0x1C)!=4` — không phải trạng thái bị khóa? ❓; và `sub_8EA9B0()!=0`,
cùng `sub_8DADA3()/sub_8DAD4C()` bỏ qua một vài chế độ ❓). Đổi state cục bộ, lật bit0
(sẵn sàng) rồi GỬI:

```
opcode 94 (client→server): bool slot (SendPkt__EncodeBool — 1 byte),
                           bool bit-sẵn-sàng-mới (1 byte)
```
- Giới hạn slot: chỉ nhận khi `slot ∈ 4..15` HOẶC mã mode đặc biệt (117; 128/125;
  133) theo 3 nhóm kiểm `sub_988B51/988B7D/988DB8(mode)` — nghĩa ❓ (slot 0..3(
  =peers) bị chặn gửi riêng).
- Sau gói (sub_999526 ≠0): tiếp tục đặt `this[4777]=2`, gọi **vf153(0)** (tắt nút
  BẮT ĐẦU chờ server_callback); `sub_5A71F(slot, state-payload-6-byte)` áp cục bộ.
- Hằng phi đễn: `sub_6918DD` (0x6918DD, sibling của CSessionStage — đưга
  `sub_68D847`, nhánh slot hàng 2 3450+ dùng point khác `sub_68C244` ❓) gửi cùng
  format và đặt `this[4772]=2`. ⚠ Chưa suy luận được tại sao tồn tại song song
  2 luồng gửi 94 (khả năng 2 component khác nhau render cùng UI). CHƯA KIỂ.

### 1b. Server phải trả — **On_94 → `sub_6C450D` (0x6C450D)** 🟢 (đổi thông tin phòng)

CPrepareStage__On_94 = `sub_6C9241` (0x6C9241): nếu `sub_8DAB40()/8DAD87/8DAD95` bất kỳ
≠0 → nhánh đặc biệt `sub_6C43BA` (❓ chưa đọc); ngược lại `sub_6C450D` — đọc:

```
opcode 94 (server→client): str A, str B, u8 (= số người chơi? cộ слот phòng — trong
                           code so với field +118 của room-object), u8 (= mã mode)
```
- So: nếu str B rỗng-none… logic: nếu `(str B khác cache) || sub_8E75EA(E60884)!=u8#1
  || sub_8E78DB(E60884)!=u8#2` → thông báo `aRoominfochange` ("room info change");
  else nếu str A đổi → `aRoomnamechange`. 🟢
- Luôn áp vào room object `dword_E60884`: tên (`sub_8EB853(str B)`), tít cũ
  (`sub_8EB8CD(str A)`), `sub_8E75D2(u8#1)` (=field +118, clamp >2→1 — mode/chế độ số
  người chơi), `sub_8E78CB(u8#2)` (=field +228, mã phòng/mode); `sub_6BFC2A(u8#2)`;
  nếu `sub_8EA9B0(E60884)!=0`: `v=sub_6DF84B(...)` → **gửi tiếp opcode 88 (u16, u32)**
  (`sub_6C3557`); nếu mode cũ `+228==16` và mới ≠16 → `sub_6C0E47(u8#2,0)` (rơi UI
  team/свой chế độ ❓).
- ⚠ Hệ quả QUAN TRỌNG cho fakeserver: sau khi nhận **94 (toggle slot)** hoặc khi
  client đổi mode qua chọn map → server phải **trả 94 (broadcast phòng: str, str,
  u8, u8)** trước, client sau đó còn gửi tiếp **88**(nếu có người đang sẵn sàng/ô
  chuyển — điều kiện trong hàm, ❓ chính xác điều kiện nhánh).

## 2. Chọn bản đồ (nút 0xBBA → sub_6BE40F → vf152; gói 88)

### 2a. `CPrepareStage__vf152` (0x6D0E02) — hộp thoại chọn map 🟢

- Gate: `sub_8DAD7B()!=0` và **mode phòng `sub_8E78DB(E60884)` (=field +228) ≠14**
  (14 = được phép qua nhánh riêng? — chỉ gọi getter, no-op rõ ràng — ❓).
- Mở dialog map-list theo **mode hiện tại**:
  `+228==14 → sub_647774(1)`; `==16` (14+2) → `sub_9B63AA(0,1)` + `sub_6477DF(…,1)`;
  `==17` (14+3)?? — decomp thứ ba: `sub_9B63C3(0)` + `sub_647816(…)` (nhánh mode
  đặc biệt). Đang xác định `(mode==14 or mode==17)` → `sub_647774`.
  Nghĩa: 14 = map tự chọn thường/17 = "chief map" — 16 = ❓. Đánh dấu „mode 14/16/17"
  là 3 **bộ định dạng bộ chọn map** khác nhau.
- Dialog mod: khi OK (`CBuyItemListDlg__vf38(0,1,1)==3`) → `v33=sub_6483EE(v36)`
  = **id map đã chọn**; nếu mode hiện tại ==17: kiểm "chief prev" (`sub_9C0F6C`)
  → quét 12 slot: người có "khoảng 7 điểm" quá cao bị đuổi (`sub_41C229`, ❓ chi tiết
  rule); nếu đuổi-xoá → thông báo `aCannotselectma` ("cannot select map") GỬI LỆNH
  ❓ — cùngimar till packets? — không packet ở bước chỉ xong — hàm cuối cùng gọi
  **`sub_6C3557(this, id_map, 1)`**.
- Cancel (vf38==4) → vf65(this,-1): không gói.
- ⟹ **khi chọn map client gửi `opcode 88`** (xem 2b).

### 2b. `sub_6C3557` (0x6C3557) — gửi opcode 88 🟢

```
opcode 88 (client→server): u16 (sub_6DC06C(id) = u16 tại bản ghi-map[id]+112
                           = "chỉ số map theo nhóm"?), u32 (id = id-đCV chọn)
```
- Điều kiện: `this[4777]==0` và `sub_8EA9B0()!=0`.
- Chặn 2P: nếu `sub_6DC339(id)==247 && dword_E60848>=2` → chỉ hiện
  `a2pcannotenterg` ("2P cannot enter game…"), **không gửi**.
- Send ok → `this[4777]=1`, **vf153(0)**.
- Caller khác của 6C3557: `sub_6C450D` (On_94 — tự gửi lại 88 khi mode đổi),
  `CPrepareStage__vf149`, `vf150`, `vf152`, `sub_6D5834`. ⟹ **opcode 88 được dùng
  chung cho mọi "cosoj/forum/you/вход phòng / đổi map / đổi mode"** — task phần (3)
  "opcode 88 (u16,u32) khi vào phòng" trớ hộp; field = (u16 chỉ số-nhóm-map,
  u32 id). Hoàn; nghĩa server trả lại = **94** (sub_6C450D) cập nhật phòng. CHƯA KIỂ
  cho giá trị sample.

### 2c. Map/theme id hợp lệ 🟢/❓

- `sub_6DC339(id)` (bản ghi+41): mode/loại của map; `sub_6DC06C(id)` (+112): u16 chỉ số.
- Đến lượt bắt đầu trận, server trả On_501 chứa str tên map; client kiểm:
  `sub_736FE3(field, u16_mapid)` trả **chuỗi mẫu (mảng 56-byte-str `this+56*id`)**
  so sánh phần prefix bằng `_mbsnbcmp` với tên map trong gói, và `sub_6DE36E(u32)`
  kiểm `0<=id<sub_414543(this+72)` + hợp lệ qua `sub_6DD49B` (7 kiểm con
  `sub_6DC65E/6DC706/6DC73E/6DC6CE/6DCBA2/6DC696/6DC776`) — false → dialog
  `aInvalidmapretu` ("invalid map returned"). 🟢 nghĩa field:
  `sub_6DD49B(id)!=0` → lưu `sub_8E7E46(cờ)`. ❓ đối tượng `this+72` chưa định danh.
- Nhóm "bản đồ konnte chuẩn": chuỗi list ở mảng `this+56*i` — cần dump runtime.
  KHÔNG KHIẾT từ file tài nguyên tĩnh — CHƯA KIỂ (xem §5 việc còn lại).

## 3. Luồng BẮT ĐẦU trận (tuần tự toàn bộ) 🟢 (trừ ghi chú riêng)

### 3a. Bấm BẮT ĐẦU — event 3500 (`sub_6C93B0`) → `sub_6C869F` (0x6C869F) [gửi **opcode 97**]

- `sub_6C93B0` (0x6C93B0): nếu `sub_8EA9B0()!=0` (có host trong phòng) → `sub_6C869F()`;
  else chỉ khi a2==0: feature 4 (`sub_9B6F75(4)`) bật → `sub_6BEA68()` (CHƯA KIỂ),
  else `sub_A7A121(stage+47964)` + `sub_6C31ED(1)` (vào nhánh "87 chế độ khác" — mục 3b).
- `sub_6C869F` (0x6C869F): gửi **opcode 97** — ĐÃ ĐỌC, chi tiết §7.2.
- Dialog/trạng thái: sau gửi thành công `this[4777]=4` + `vf153(0)`. (Chú ý §7.2
  ghi `=3` trên nhánh 97 — CHƯA KIỂ 2 luồng set giá trị khác nhau; xem §7.2.)

### 3b. `sub_6C31ED` (0x6C31ED) — gửi **opcode 87** [nhánh không-host/khác]

Điều kiện vào: `this[4777]==0 && sub_8EA823(E60884)!=0`. Chặn thoát về stage cũ
(`sub_8F391D(*(this+15141)) + sub_8F2F14()` — sub_8F2F14 gửi opcode 81, không field)
nếu: số người chơi ∉{2,3}; hoặc trạng thái slot (`(state>>2)&7`, đọc qua `sub_8EA779`)
— nếu thấy slot "trạng thái" =1 → v21=0; =7 → bỏ qua; còn lại → thoát; kiểm tất cả
slot "đúng trạng thái không kháng" `((state&0x1C)==4)==v21` → thoát.
Ngược lại: kiểm `sub_8DAB89()` (fail → `aInvalidgradeto`), `sub_8DAFD7(1)` (fail →
`aInvalidgradese`) rồi GỬI:

```
opcode 87 (client→server): blob (sub_A629B0 + sub_8FD1D5 — dữ liệu tải准备的
                           trận/neกมคธรรม?), bool (v21 = "mọi người sẵn sàng"?),
                           u32 (GetNumNestedThreadSubscriptions(E60A20))
```
Send ok → `this[4777]=4`, vf153(0). Ngoài caller 0xDAC còn là luồng vf65/vf82/vf149/
vf77/vf44 của CPrepareStage — cùng packet. **Nội dung blob: ĐÃ ĐỌC — §7.1** (bool×2
+ list nhóm × sub_8FAD2D; field trong của sub_8FAD2D vẫn ❓).

### 3c. Server xác nhận BẮT ĐẦU — **On_501 `sub_6C4978` (0x6C4978)** 🟢 (phần lớn — format đủ)

Trình tự đọc server→client (mọi mode):

```
opcode 501 (server→client, "bắt đầu trận"):
  u8 result            !=1 → chỉ hiện dialog "Cannot launch as..." (aCannotlaunchas),
                       KHÔNG đổi stage
  [chỉ khi (sub_8DAB40()/8DAD87/8DAD95) — mode 2P/team] i = stage[4770]:
      i × { u8 flag;
            flag!=0 → 3-byte buf (DecodeBuf 3) → sub_8E977D(slot, …) — state ô ❓
            flag==0 → u32 id; blob (template OnPkt_multi_344_553: nội dung ❓);
                      3-byte buf (vị trí❓); u8; ... dựng actor per-player (malloc 0x198,
                      sub_679A78/46B55B), vf592(stage) —CHƯA KIỂ chi tiết }
  u32 A            → sub_8E77E4 (field +145 room: "seed/mã trận"? CHƯA KIỂ)
  u16 B            → sub_8EAB95: **map id** (u16) + kế ổn get; (v31 = B
                   trong sub_6DE3 executed cử tương — wait — Detailed: B dùng trong
                   kiểm (xem 2c)) — TỔNG HỢP: B = id map/theme chuẩn клиентов ❓
  str  mapName     → kiểm prefix với chuỗi поколения của sub_736FE3(table, B) —
                     "Map/xxx.map" hợp lệ; KHÔNG hợp lệ → dialog aInvalidmapretu
                     (không đổi stage)
  u32 C            → sub_6DE36E(C): 0<=C<count(this+72) & hợp lệ (7 sub-check
                     sub_6DD49B) — "theme id"? CHƯA KIỂ
  u8  D            → sub_8E78CB (room +228 = **mode mới** khi thi trong trận — same as On_94)
  u8  E            → sub_8E7636 (field +263 = sáng/tối? CHƯA KIỂ)
  u8  F (=n)       → số bản ghi tiếp theo; n × :
      u8  slot     → sub_8E8EFF(slot,0); sub_8E8EBB(slot, u8#3)
      u8  ×2 (v15, v16)
      u32 ×3       → sub_8E99A3(&entry, slot): nếu ≠0: u32#1→sub_67900E, u32#2→
                     sub_67901A; u32#3=v31: nếu slot==mình → sub_9B69F7(1P, v31),
                     nếu 2P && slot==second → sub_9B69F7(2P, v31)
                     ⟹ v31 = "điPTY/trận-hàng/cột cho phép"? CHƯA KIỂ (khoảng
                     "đơi nhận điểm vị trí"?)
  [loop 16× sub_6BE6D0(slot) — tìm chỗ của mình] (≤16 vòng) →
      u8 flag      → ≠0 → sub_985703(packet) đọc thêm (blob thụ động ❓ — CHƯA KIỂ)
      v17 = sub_6DC025(E60930); sub_6DDB4B(var64, v17); sub_98579D(E60A20, state)
                     (điền entrance — từng người + sub_971177-та bảng vị trí)
      sub_6734D5(21, 1, 0)                          (tнат/disable cờ nội bộ #21 ❓)
      sub_8F391D(13)                                → ChangeStage(13)→ **thành stage 13**
```
- **Sau việc dưới đây type-2 cụt: thành công → sub_8F391D(13)** (disasm 0x6c4fe9: `push 0Dh` — id đích = **13**);
  FAIL (sai map / F!=1) → `sub_8F391D([stage+0xEC94])` (id đích = id hiện tại, tức
  TRỞ LẠI stage hiện tại, kh 而 itта ranh hành trọng tất cả chế độ) + `sub_8F2F14()`
  = **gửi opcode 81** (không field, ❓ nghĩa — "hwnба đặt”？). 🟢
- ⟹ fakeserver: khi nhận 87/97 (tùy mode) phải trả **501** như trên, với
  `result u8 = 1`, mapName đúng creek (với id B — kiểm thế list), mode D khớp
  phần chết/phong hiện tại, F = 0 (đơn giản nhất: bản ghi cặp player trống
  — ❓ sub_98579D có thể cần bảng "in game state" đầy đủ để hoạt ý tưởng đunci).

### 3d. Stage tiếp theo — id 13 🟢

- `CStageMgr__ChangeStage` (0x8F8EEC, khối VM tại 0x8F8F69 — xem devirt.md §73):
  `sub_8F8C88` tạo `CGameStage` (`sub_8DE532(E60928)==0`, malloc 0x5710, ctor
  `sub_70CBAF`) hoặc `CHiddenCatchGameStage` (`sub_8DE532==1`, malloc 0x5F40,
  ctor `sub_47C726`). `sub_8DE532` = loại trận người dùng đã vào (gán góc II từ
  vòng đầu/regard bảo — CHƯA KIỂ).
- gói client gửi ngay khi vào stage 13: sẽ cần phân tích riêng (trên docs,
  thuộc mục "c-game": vf44 của CGameStage. Dispatch nhanh: CGameStage__vf51 ==
  opcode 20 (trả về), 25 → sub_7127B3, 81 → sub_70C8C1, 106 → sub_7019AE,
  107 → sub_703D01, 119, 128 → sub_71458E, 135+... — đầy đủ cần thêm phiên
  c-game). Trong phạm vi a-room: **CHỚ chưa đề ra gói trả để trận chạy — CẦN TIẾNH thêm nó.**

## 4. opcode 88 — chốt phần hỏi (3) của task 🟢/❓

- Sender duy nhất: `sub_6C3557(this, id, a3)` (0x6C3557). Wire = **`[88][u16][u32]`**
  — u16 = `sub_6DC06C(id)` = giá trị tại **bản ghi-map[id]+112** của bảng truy
  (object `dword_E60930`, bản ghi list qua `sub_6DB901(id)`); u32 = `id` đầy đủ.
- Điều kiện gửi: `this[4777]==0` và `sub_8EA9B0()!=0`; chặn 2P (`sub_6DC339(id)==247
  && dword_E60848>=2` → chỉ hiện `a2pcannotenterg`, không gửi).
- 6 caller: `sub_6C450D` (On_94 — gửi lại 88 ngay sau khi nhận room-info-change nếu
  `sub_8EA9B0(E60884)!=0`), `sub_6C5469` (được gọi từ **CPrepareStage__On_106** —
  tạo/bật nút PRE_Bt_Start rồi gửi 88), `vf149/vf150/vf152` (chọn map/mode),
  `sub_6D5834` (❓).
- ⟹ Giải nghĩa task (3): opcode 88 khi vào phòng KHÔNG phải gói "xin vào phòng"
  chung, mà là **"đồng bộ vị trí/chế độ của mình với server"** dùng chung cho các
  thao tác (chọn map → đổi mode → xin chỗ). Giá trị hiển thị của cả 2 field
  liên quan đến bản ghi-triangle trong bảng nói trên — đối với test động, giữ
  bảng w/ spoofing sample. **Server cần trả khi nhận 88 = opcode 94** (room-info
  change, định dạng mục 1b) — để cập nhật room UI + khóa `this[4777]=0`
  (quay về rảnh). 🟢/❓

## 5. Việc còn lại (một số điểm CHƯA KIỂ)

1. ~~`sub_8FD1D5` (blob của opcode 87/97), `sub_6C43BA` (nhánh On_94 khác)~~ —
   ĐÃ ĐỌC (xem §7.1, §7.3). Còn lại: `sub_6BEA68` (nhánh feature-4 của 0xDAC),
   `sub_985703` (đọc gói phụ của 501).
2. Bảng tên/bản ghi map (mảng 56 byte `this+56*i` trong sub_736FE3) cần runtime dump
   để có map id chuẩn (tiếng "Map/..." + theme id).
3. Luồng stage 13 (CGameStage — QDialog đong, đọc `CGameStage__vf51` hay rotate
   vf44/vf5 của game stage cho một bảng đầy đủ: gói client gửi khi vào (OnEnter),
   server trả map/vị trí/player — thuộc phiên "c-game" (thư mục khác).
4. Sự kiện các nút `sub_6C1A53` (3350+), `sub_6C0585` (0xC80), `sub_6C1947`
   (0xCE3-0xCEB), `sub_6C62A4` (3600+i, gửi **opcode 212**) — chưa đọc.
5. sub_6918DD vs sub_6BE8AF (2 luồng gửi 94) — chưa truy thứ cấp.

## 6. Bảng định dạng gói (tóm tắt a-room)

| opcode | hướng | fields (thứ tự đọc) | nguồn |
|---|---|---|---|
| 94 | client→server | bool slot, bool bit-sẵn-sàng (2×u8 qua EncodeBool) | `sub_6BE8AF`/`sub_6918DD` 0x6BE8AF/0x6918DD |
| 94 | server→client | str A, str B, u8 (mode +118), u8 (mode +228) — đổi thông tin phòng | `sub_6C450D` 0x6C450D (nhánh khác `sub_6C43BA` 0x6C43BA: chỉ str A + str B — §7.3) |
| 85 | client→server | str, str, bool, bool (58?) — sửa thông tin phòng từ dialog (event 0xBBB) | `sub_6C21F6` 0x6C21F6 |
| 87 | client→server | blob (đã giải mã — §7.1), bool, u32 — luồng BẮT ĐẦU (chế độ thường) | `sub_6C31ED` 0x6C31ED |
| 97 | client→server | blob (cùng `sub_8FD1D5` — §7.1), u32 (GetNumNestedThreadSubscriptions) — luồng BẮT ĐẦU host (event 0xDAC); trước gửi: kiểm grade `sub_8DAB89`/`sub_8DAFD7(1)` | `sub_6C869F` 0x6C869F |
| 88 | client→server | u16 (chỉ số nhóm map), u32 (id map/mode) — chọn map/đổi mode | `sub_6C3557` 0x6C3557 |
| 212 | client→server | ❓ — event 3600+i | `sub_6C62A4` 0x6C62A4 |
| 501 | server→client | u8 result(=1); [2P: n×bản ghi]; u32, u16(map id), str(map name), u32(theme), u8, u8, u8 n, n×(u8 slot, u8, u8, u32×3); [u8 + (≠0: phụ blob)] → ChangeStage 13 | `sub_6C4978` 0x6C4978 |
| 81 | client→server | (không field) — gửi khi On_501 THẤT BẠI (quay lại stage cũ) | `sub_8F2F14` 0x8F2F14 |

## 7. Ghi chú bổ sung — 3 hàm trước đây "CHƯA KIỂ" (đọc thêm) 🟢

### 7.1 `sub_8FD1D5(a2)` (0x8FD1D5) — nội dung blob của opcode 87/97 🟢/❓

- `this` = object "chuẩn bị trận" (state chuẩn bị của stage), dau: blob §3b gói từ
  `sub_A629B0` (constructor/clear) + `sub_8FD1D5(this, blob)`. Trình tự encode Greenland
  (mọi field qua EncodeBool/Encode4):
  1. `EncodeBool(blob, this[3] != 0)` — CH4/boolean ❓ (this[3] = *(_DWORD*)
     (state + 12) — cờ "team-lőada"? field `+12` — CHƯA KIỂ nghĩa).
  2. `EncodeBool(blob, this[3])` — bool thô của cùng ô (cast to bool).
  3. duyệt list node tại `this[2]` (linked list node 9 nút đầu `v4[4]`→`v4[5]`
     stride 36 byte/node (vector<string→,? node>)):
     n = (end−begin)/36 → `Encode?` số n (sub_A634C0 — bool/u4 tảng: n <= chu kỳ
     in nếu n≠0) rồi n × `sub_8FAD2D(a2)` (encode 1 bản ghi "tổ trong/nhóm trận"
     chấm; nội dung record 36 byte — CHƯA KIỂ field bên trong).
- ⟹ blob là **bảng phân nhóm các đội/groups** (theo mode team) — server giả có thể
  chấp nhận nguyên khối như "opaque" vì cho phép bỏ qua pass-through, vẫn nên
  decode: bool×2 + list nhóm × sub_8FAD2D. ❓ field timaster của sub_8FAD2D cần
  runtime/2P test để xác định.

### 7.2 `sub_6C869F` (0x6C869F) — luồng "BẮT ĐẦU" gửi **opcode 97** 🟢

- Điều kiện vào: `this[4777]==0` (else thoát); grade-check `sub_8DAB89()` (fail →
  dialog `aInvalidgradeto...`) và `sub_8DAFD7(1)` (fail → `aInvalidgradese...`).
- Sau đó `v3 = sub_6C0990()` = "id chế độ vào trận": nếu `sub_8DADA3()!=0 &&
  sub_9B6F75(4)!=0` → v3 = 0. Dải hợp lệ: {0..8, 10, 12..22, 23..26} (ngoài
  dải thoát). Hợp lệ → `sub_6734D5(dword_E6083C, 20, 1, 0)` (cảnh/trạng thái #20 —
  ❓) + `sub_6C710D(v3, 0)` rồi thoát, **KHÔNG gửi 97** — tức 97 chỉ đi trong
  một số chế độ (đơn giản `v3` = mode nhận từ 6C0990 dẫn về đường "ẩn khác").
  ❓ dải và nghĩa cần runtime.
- Khi GỬI 97: trình tự giống 87 §3b — `sub_88849F(97)`; blob `sub_A629B0 +
  sub_8FD1D5` (§7.1) qua `EncodeBlob`; `sub_8FE6C7()`; `sub_6DB42C(this[14221],
  this[14222])` (2 field 35//35, `+14221/+14222` — CHƯA KIỂ); nếu
  `sub_6DF9FB(blob-writer)!=0` (❓ có gì vừa ghi vào packet) → `sub_6C710D` (lỗi)
  **bỏ field u32**; ngược lại thêm `u32 GetNumNestedThreadSubscriptions(E60A20)`.
  Gửi ok (`sub_999526`) → `this[4777]=3` (khác 87: =4), gọi vf(+612)(this,0).
  Close: `sub_A62B10`, `sub_8884DC`.
- ⟹ **97 = biến thể BẮT ĐẦU của host trước khi vào mode ANOTHER (v3 →), dùng
  blob phân nhóm §7.1 + u32 như 87 nhưng KHÔNG có field bool v21**.
  fakeserver: trả **501** như mục 3c khi nhận 97. 🟢

### 7.3 `sub_6C43BA` (0x6C43BA) — nhánh thứ 2 của On_94 (server→client) 🟢

- Trình tự: `DecodeStrInto(Str)` (str A), `DecodeStrInto(Buf2)` (str B); đọc chuỗi
  hiện hành của room `sub_8EB2DC(dword_E60884, v11)`; so `Buf2` == v11
  (`sub_40E371`) → nếu khác: nếu `Str` == tiêu đề cũ (*) `sub_8E8E8A(room)`:
  hiện dialog `aRoominfochange` ("Room info change...") hoặc `aRoomnamechange`
  ("Room name change..."). Sau đó áp dụng: `sub_8EB8CD(Str)` (setter str A) và
  `sub_8EB853(Buf2)` (setter str B) trên phòng.
- ⟹ nhánh này = **server phản hồi cho opcode 85 (sửa tên/mô tả phòng, mục
  "sửa thông tin phòng")**: 2 chuỗi = (tên mới?, mô tả mới?) — phần client فقط
  nhận+sập+thông báo, KHÔNG có u8 mode. Các setter `sub_8EB8CD/8EB853` tương ứng
  với 2 str của luồng sửa thông tin — thứ tự trường cần đối sánh luồng §85.
  ❓ A = tên hay mô tả — c Alex nêu: sub_8E8E8A (đọc từ phòng) sẽ quyết định theo
  tiếng tại runtime.
