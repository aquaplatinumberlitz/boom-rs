> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# e-game: Stage TRẬN ĐẤU (stage 13) — phân tích tĩnh (IDA `~/boom-re-oc/ida/e-game.i64`, bản copy riêng)

Tiền đề (docs/archive/re/VN_LOBBY_LOG.md Cập nhật 10): phòng chờ = stage 12 `CPrepareStage`; bấm BẮT ĐẦU → `CPrepareStage__vf5` →
`sub_6CB7B6` (0xBBA = chọn bản đồ, 0xBBB) → `sub_6C95AD` → `sub_6C63B6`. Quy ước: handler `On_N` = opcode N;
`CStage__OnPacket` case = opcode−2; đọc bằng `RecvPkt__Decode1/2/4/8/Str/Blob`, gửi bằng `sub_88849F(op)` +
`SendPkt__Begin` + `SendPkt__Encode*` (mọi field bị obf XOR như protocol.md). Field game là "value box"
(`sub_413F32` set / `sub_414625` get u8 / `sub_414543` get u32); offset getter = offset lưu + 0x18 (Errata).

## 1. Nhận diện stage trận

### `sub_6C95AD` (0x6C95AD) → `sub_6C63B6` (0x6C63B6)
- `sub_6C95AD`: guard `sub_8DAB40`, `sub_8DADCA`, `sub_8DADE6`, `sub_8DAD87`, `sub_8DAD95()`: nếu `sub_8DAB40` ≠0 → đường
  `sub_6C2888`; 3 cái giữa ≠0 → `sub_6C2B69`; tất cả 0 → `sub_6C63B6` (đường trận thường).
- `sub_6C63B6`: kiểu chơi `v18 = sub_8E75EA(dword_E60884)` (room type?) và `v19 = sub_8E78DB(dword_E60884)` (battle kind?);
  `sub_975083(v19) != 0` → đường thường `sub_6C5EC8(v20, tên, v23, &v18, &v19)` (chỉ dựng cấu hình UI, không thấy gửi
  opcode); v19 ∈ {15, 17} → đường `CSimpleModifyRoomInfoDlg__vf44` + `sub_6C5D8C`. Bấm BẮT ĐẦU chỉ đổi UI — **không thấy
  gửi gói xin bắt đầu**; server tự quyết bằng gói 501. CHƯA KIỂ: xem `sub_6CB7B6` 0xBBB có gửi gói gì.

### On_501 → stage 13 ✅
`CPrepareStage__On_501` (0x6C4978): nhánh bản đồ hợp lệ → **`sub_8F391D(13)`**; nhánh tên bản đồ lệch (`aInvalidmapretu`,
hộp thoại "Invalid map...") → `sub_8F391D(this[15141])` + `sub_8F2F14` (quay về phòng chờ).
⇒ **Trận thường = stage 13, lớp `CGameStage`** (bảng vtable +: vf44 OnEnter 0x71B519, vf51 OnPacket 0x71B631,
kế thừa `CChatStage` → cuối OnPacket chuyển tiếp `CSessionStage__vf51`).

## 2. OnEnter — `CGameStage__vf44` (0x71B519) → `sub_71A9EC` (0x71A9EC) ✅
CALog: chuỗi `"[XRpt] Sg - INT(dne:%d)..."` (0xC7B204) — pattern stage 11/12 đều log `Sg - INT(dne:<stage-id>)`;
log cho stage 13 suy theo cùng pattern (CHƯA KIỂ xref trực tiếp). Luồng theo thứ tự code:
1. gọi vtable **vf146** (offset 584) của this (`CGameStage__vf146` 0x700F05);
2. `a1[4926] = 0`, rồi `CChatStage__vf44(a1)` (stage 13 kế thừa CChatStage);
3. `a1[155] = 1`; `sub_6FFC52`, `sub_71C46A(0)`, `sub_7056BD(a1)`;
4. build mảng 16 slot: `v25[i] = sub_8E9BF6(i)` (i = 0..15; getter đọc box `*(this+121)+124+580*i`) — dữ liệu người chơi
   đã lưu từ On_501 qua các setter `sub_8E977D` / `sub_8EA539` / `sub_8E8F21` (định dạng trong §4) — OnEnter KHÔNG cần
   gói bổ sung;
5. `v7 = (u16)sub_8E9977(dword_E60884)` = **map id đang chọn của phòng** (box `this+508`);
6. gọi vf147 (offset 588) = `CGameStage__vf147` 0x718AC8 với `(v7, v25)`: trả 0 → `sub_708FFE(a1, 1, 0)` (thoát về trước);
   trả ≠0 → nhóm lệnh dựng trận:
   `sub_6FFFDB`/`sub_6FFCC4`, `sub_701277`, `sub_7012A3`, `sub_70139A(v7)`, `*(this+4985)+12)`; nếu
   `sub_8E7E3F(dword_E60884)` và `a1[5417] != 0` → `sub_6E1F0C(120000)` (timeout vào trận 120 s);
   `a1[5342] = sub_6E204D(sub_6FFFDB)`, `sub_6FFC94` (nếu arg0==0), `sub_70478D`, `sub_6FFCE2`, `sub_7012AF`,
   `sub_6FFCED`, `sub_71A6E1`, `sub_7EF7DB(dword_E63FD0)`, `sub_7012BD`, `memset(a1+5378, 0x40)`,
   `sub_6E1F0C(120000)` (chế độ ngắm), duyệt `this[5413]` gọi vf15 (offset 60) từng phần tử, reset
   `a1[5400/5401/5536..5539]`, `sub_9682DE(this+5555×2)`, `sub_5ADE62()`.
- **OnEnter không gọi `sub_88849F`/`SendPkt__Begin`** (các nhánh chính) → trận không gửi gói nào lên server chỉ vì vào
  stage (CHƯA KIỂ: chỉ kiểm các nhánh chính, không đệ quy vào từng sub-call).

### vf147 (0x718AC8) — tạo/nạp trận
- `v4 = sub_736FE3(mapid)` = **tên file map theo id từ bảng cache** (bảng `this+4`, mảng std::string stride 0x38);
  `v17 = sub_8E7A7F(dword_E60884) != -1`; `v5 = sub_6FFFDB(this)` (game context);
- `Block = vf158(this, mapid, v17)` — vf158 (0x701316) → `sub_576D99(kind, this, v17)` = **factory theo kind**
  (`sub_8E78DB`) của chế độ chơi;
- nếu vf tại offset 12 của `v5` ≠0: `sub_6EBC30(Block)` (→ `sub_6EACE5` + `sub_6EB9A9` — nạp map/hiệu ứng; chính là hàm
  trong crash trace của Cập nhật 7), vf(v5+16), `sub_6EC237(v11, 1)`, `sub_6E3AFE(v5)`, `sub_70135D(this)`,
  vf offset 640 (=vf160), `sub_70D493(mapid, v17)`, `sub_715C3A(mapid, danh sách slot)`, `sub_70B578`, `sub_701368`,
  `sub_672E6A(1)`, phóng object cũ `this[5394]`, rồi `this[5394] = sub_6F987B(sub_414543(this+4781), sub_9B6F75(4))`
  (object trận mới; malloc 0xE2B8), `sub_704ED0(this)`.

### Map/theme nạp từ đâu
- **Tên map** tra từ bảng cache (`sub_736FE3`; `this = dword_E60A10` ở vf155) — bảng nạp bởi gói **231**
  (`CStage__On_231` 0x6E0C5A, cùng kiểu gói 13): `DecodeBlob` → parse `sub_6E077C` → ghi cache `fxdynmapinfo.ssd`
  (`sub_A62C20`; lỗi mở file → `CAppException` "Cannot open a file..." phải cẩn thận như gói 14).
- Blob của 231 (đọc qua subreader obf như gói 14): `u16 count` (`sub_A62F70`), rồi **check `sub_A62570 == this[3]`**
  (khác → `sub_6DB3ED` rồi thoát); mỗi bản ghi: `u32` (`sub_A62FB0`, dùng `sub_911168`), `u8`, `str` **tên file map**
  (nạp `"Map/%s"`), `str` (theme?), `u8`, `u8` (mã theme: 89 = "Theme/SweetCandy", 138 = "Theme/ThemeDefault", khác →
  `"Theme/%s"`), `str`, `str`, `u8`, `u8`, `u32` màu (sub_A2A5E0(B2,B1,B0)), `u8` (style: `/10==1` → `%=10` + cờ),
  `u8`, `u8`, `u16`, `u8`, `u32`. Sau mỗi bản ghi `sub_6E03A8(&record)`; cuối `sub_6E048D`, `sub_6E06A0`.
- BGM: vf155 (0x70EEF3) chọn `"%s.mp3"` theo map id (`sub_6DDBAB` từ bảng).
- CHƯA KIỂ: ý nghĩa từng field của bản ghi 231; nhưng **phải gửi 231 (đúng tên map id) trước 501** để `sub_736FE3`
  trả tên không rỗng (rỗng thì `_mbsnbcmp` với tên gói 501 lệch → rơi nhánh aInvalidmapretu).

## 3. `CGameStage__OnPacket` (vf51, 0x71B631) — bảng opcode → handler ✅
Đọc opcode bằng `RecvPkt__Decode2` (u16). Phân rẽ 4 dải: op ≤ 214 → case = op−20 (1 switch); 215..396 (nhánh so sánh
riêng); 397..578 → case = op−397; op 214/517 bắt riêng. Kết thúc mỗi nhánh (kể default) là fallback:
nếu `this[5417]` ≠ 0 → `RecvPkt__Rewind` rồi gọi vf12 (offset 48) của object đó; tiếp theo Rewind + gọi vf16 (offset 64)
từng phần tử danh sách `this[5413]`; cuối cùng `CSessionStage__vf51` (chuyển tiếp, có `sub_8883BE(a2)`).

| opcode | handler | địa chỉ |
|---|---|---|
| 25 | `CGameStage__On_25` (str → token → cửa sổ/nhạc) | 0x7127B3 |
| 81 | `CGameStage__On_81` | 0x70C8C1 |
| 106 | `sub_7019AE` | 0x7019AE |
| 107 | `CGameStage__On_107` | 0x703D01 |
| 119 | vf148 (offset 592, thiscall với packet) | — |
| 128 | `sub_71458E` | 0x71458E |
| 135 | xử lý trong dispatcher (u8 → `sub_9B5934`, có thể `sub_6FFBD4(5)`) | — |
| 145 | `sub_701A48` | 0x701A48 |
| 146 | `sub_70DE4C` | 0x70DE4C |
| 147 | `CGameStage__On_147` | 0x701AB1 |
| 148 | `sub_704CD8` | 0x704CD8 |
| 149 | `sub_704D77` | 0x704D77 |
| 150 | `sub_6FFF78` | 0x6FFF78 |
| 151 | `CGameStage__On_151` | 0x70205B |
| 152 | `CGameStage__On_152` | 0x7064AC |
| 153 | `CGameStage__On_153` | 0x70679A |
| 155 | `CGameStage__On_155` | 0x701E8B |
| 157 | `CGameStage__On_157` | 0x7068DE |
| 158 | `CGameStage__On_158` | 0x715E90 |
| 159 | `sub_703F52` (không đọc packet) | 0x703F52 |
| 160 | `CGameStage__On_160` | 0x7164BB |
| 161 | `CGameStage__On_161` | 0x704962 |
| 163 | `CGameStage__On_163` | 0x716735 |
| 164 | `CGameStage__On_164` | 0x701765 |
| 165 | `sub_7018D7` | 0x7018D7 |
| 166 | `CGameStage__On_166` | 0x708EB0 |
| 168 | `sub_719D08` | 0x719D08 |
| 169 | `sub_7090AD` | 0x7090AD |
| 170 | `sub_70FF80` | 0x70FF80 |
| 171 | `sub_7069DF` | 0x7069DF |
| 173 | `sub_708FFE(0,0)` nếu `sub_8DB058()`, khác vf149 (offset 596) | 0x712E49 |
| 174 | `sub_70264F` | 0x70264F |
| 176 | `sub_706C42` | 0x706C42 |
| 177 | `sub_706D3C` | 0x706D3C |
| 178 | `sub_7150E7` | 0x7150E7 |
| 184 | `CGameStage__On_184` | 0x709101 |
| 189 | `sub_70E5C4` — **LỆNH ROUND (quan trọng nhất, xem §5a)** | 0x70E5C4 |
| 190 | `sub_70E7D6` | 0x70E7D6 |
| 191 | `sub_709A38` | 0x709A38 |
| 192 | `sub_7173A5` | 0x7173A5 |
| 193 | `sub_715210` | 0x715210 |
| 194 | `sub_706823` | 0x706823 |
| 195 | `CGameStage__On_195` | 0x701D15 |
| 196 | `sub_707101` | 0x707101 |
| 204 | `sub_7071AC` | 0x7071AC |
| 205 | `CGameStage__On_205` | 0x701B11 |
| 209 | `sub_70EE54` | 0x70EE54 |
| 214 | `sub_70DD0E` | 0x70DD0E |
| 215 | `sub_7129C4` | 0x7129C4 |
| 216 | `sub_70DDB4` | 0x70DDB4 |
| 267 | `sub_69BABE` | 0x69BABE |
| 298 | `sub_7035C4` | 0x7035C4 |
| 303 | `sub_70727E` | 0x70727E |
| 311 | `sub_719A3C` | 0x719A3C |
| 312 | `sub_70AF9C` | 0x70AF9C |
| 313 | `sub_707D14` | 0x707D14 |
| 314 | `sub_7101FA` | 0x7101FA |
| 315 | `sub_7158EC` | 0x7158EC |
| 324 | `sub_703E0C` | 0x703E0C |
| 339 | `sub_7100B2` | 0x7100B2 |
| 356 | `sub_7155DA` | 0x7155DA |
| 358 | `sub_707673` | 0x707673 |
| 359 | `sub_70777C` | 0x70777C |
| 366 | `sub_70058C` | 0x70058C |
| 394 | `CGameStage__On_394` | 0x712567 |
| 396 | `u8 → sub_9B5648` (dispatcher) | — |
| 397 | `sub_704EF2` | 0x704EF2 |
| 398 | `sub_70BA8F` | 0x70BA8F |
| 399 | `sub_70813F` | 0x70813F |
| 502 | `sub_70BA63` | 0x70BA63 |
| 503 | `sub_70F090` | 0x70F090 |
| 517 | `sub_717791` | 0x717791 |
| 518 | `sub_70F7A3`; nếu `this[5395] ≠0` và `sub_6FA585()==0` → `sub_6FC976(39, 371, this)` | — |
| 519 | `sub_70054E` | 0x70054E |
| 523 | `sub_717B87` | 0x717B87 |
| 526 | `sub_711B3A` | 0x711B3A |
| 529 | `CGameStage__On_529` (guard `this[5395]`), thất bại → `sub_6FC976(39, 371, this)` | 0x6FA585 |
| 531 | `sub_70A158` | 0x70A158 |
| 532 | `sub_70A04B` | 0x70A04B |
| 536 | `sub_70FC1B` | 0x70FC1B |
| 538 | `sub_703951` | 0x703951 |
| 539 | `sub_71798F` | 0x71798F |
| 540 | `sub_703921` | 0x703921 |
| 545 | `sub_717EFC` | 0x717EFC |
| 547 | `sub_707E78` | 0x707E78 |
| 548 | `sub_71824D` | 0x71824D |
| 552 | `sub_71580C` | 0x71580C |
| 553 | `DecodeBlob → sub_6A5A5E` | A62490 |
| 555 | `sub_703A20` | 0x703A20 |
| 568 | `sub_705016` | 0x705016 |
| 569 | `sub_7038CD` | 0x7038CD |
| 570 | `sub_70C70D` | 0x70C70D |
| 571 | `sub_71ADEC` | 0x71ADEC |
| 572 | `sub_709D5E` | 0x709D5E |
| 578 | `CGameStage__On_578` | 0x70065D |


## 4. Gói opcode 501 (S→C "bắt đầu trận", handler 0x6C4978) — đọc từ decompile; CHƯA KIỂ runtime
```
opcode 501 (S→C):
  u8 ok                       // Decode1; ==0 → hộp thoại aCannotlaunchas, KHÔNG đổi stage

  // ---- phần 1: người chơi, lặp this[4770] (số người đã lưu từ opcode 103) ----
  u8 c                        // c != 0: chỉ DecodeBuf 3 → sub_8E977D(slot, buf)
                              // c == 0: entry đầy đủ:
                              //   u32 player_id        (sub_679434 + blob)
                              //   blob                 (thông tin người, như 103)
                              //   DecodeBuf 3          (bitfield sẵn sàng/nhân vật/đội; chia cho sub_691BEE)
                              //   u8                   (sub_8E8F21)
                              //   sub_572A05 + CGameStage-vf149 offset 592 (this)
  // ---- phần 2: cấu hình trận (guard sub_8DAB40/8DAD87/8DAD95 đều 0) ----
  u32 seed                    → sub_8E77E4(seed)   ; sub_65412D(3, seed)
  u16 map_id                  → sub_736FE3(map_id) ; sub_8EAB95(map_id)
  str map_name                // u16 độ dài + byte; so _mbsnbcmp với tên map_id: khác → aInvalidmapretu
  u32 type                    // sub_6DE36E(type) != 0 bắt buộc; sub_6DD49B(type) != 0 → cờ (sub_8E7E46)
  u8 p0                       → sub_8E78CB(p0)
  u8 ready_bits               → sub_8E7636(ready_bits) (CHƯA KIỂ)
  u8 pcount                   → sub_8E9027(pcount) (số người phần 3)
  // ---- phần 3: pcount × { u8 slot, u8 x, u8 y, u32, u32, u32 } ----
  u8 slot → sub_8E8EFF(slot, 0)
  u8 x → sub_8E76A3(slot, x)
  u8 y → sub_8E8EBB(slot, y)
  u32 → sub_67900E (dùng làm ô/x?; CHƯA KIỂ)
  u32 → sub_67901A (CHƯA KIỂ)
  u32 → sub_9B69F7(0 nếu là người thứ 0, 1 nếu thứ 1, giá trị) (CHƯA KIỂ)
  // ---- phần 4: lặp 16 slot (0..15) ----
  mỗi slot có người: u8 != 0 → sub_985703(a2)  (sub-packet, CHƯA KIỂ)
  // ---- hợp lệ ----
  sub_6734D5(0x15, 1, 0); sub_8F391D(13)       // đổi stage 13
```
Nhánh lệch bản đồ: hộp thoại `aInvalidmapretu` rồi `sub_8F391D(this[15141])` (quay về phòng chờ) + `sub_8F2F14`.
CHƯA KIỂ: ý nghĩa các u32 loại c/d/e của phần 3 và sub-packet của phần 4.

## 5. Các gói chạy trận (đọc từ decompile; field theo thứ tự Decode/Encode chính xác, ý nghĩa CHƯA KIỂ)

### 5a. Opcode 189 (S→C) — LỆNH LƯỢT (`sub_70E5C4` 0x70E5C4) ✅ decompile đủ (2026-10-17)
```
opcode 189 (S→C) — thứ tự đọc: u8 mode, u32, u16 map_id, rồi field theo mode:
  u8 mode (Decode1)
  u32 -> this[4887]      (Decode4) = **KHOÁ PHIÊN** (2026-10-18, §5a-4): byte thấp mở đầu mọi gói trận C→S
                         (`sub_6FFFA0`), gói nhận có byte đầu khác bị `vf61` bỏ
  u16 map_id             (Decode2; dùng cho sub_8EAB95/vf147 — giống map id trong 501)
  mode 1: u8×16 (Block) → sub_70E357: sub_70C5DC(1) (phase this[5402]=1), vf147(map_id, Block)
          (nạp lại round — đi qua skip 0x736C27), tạo 1 bomber cho ta (malloc 0xA18 + sub_5862A5 +
          sub_6E17DB vào E608CC), rồi C→S **139** {bool 1}
  mode 2: u16 a4 + u8 a5 + u32 a6 → sub_70DF00: sub_70D4B7 (phase 3), sub_70319B, vf147(map_id, Block16
          Block16 = 16 × giá trị sub_8E9BF6(k)[: 580*k + this[121] + 124 của room ctx, đọc client-side]),
          sub_70A385(a4, a5, a6), nếu a6 != 0 → sub_69280F(0, a6), rồi C→S **140** {bool 1}
  mode 4: u16 a4 + u8 a5 + u8 a6 → sub_70DFF9, 2 nhánh + đuôi dùng chung:
          nhánh A (a4 < (this[5410]-this[5409]) >> 2, đếm phần tử list this[5409..5410]): vf155(1) +
          vf157(map_id, Block);
          nhánh B (ngược lại): nếu a4 != (*(*this+608))(this, 4) [getter slot 152 arg 4] → sub_708FFE(5,0) và
          bỏ tiếp tục (KHÔNG gửi 140); nếu bằng → vf147(map_id, Block);
          **cả hai nhánh chảy vào đuôi chung**: nếu this[5419] là CGameMiniMapWnd → sub_7E397C(a6, map_id); vòng
          12 bomber: hồi ghi blob 96 byte đã thu thập lúc đầu hàm ([vt+728] thu, [vt+732] ghi lại), gọi
          [vt+424](0, 9430) nếu sub_58D4DA != 0; rồi gửi C→S **140** {bool 1}.
          ⇒ 140 là phản hồi CỦA CẢ nhánh A và nhánh B (đã chạy thật: mode 4 (1,1,1) → client trả 140 sau 89 ms,
          thấy ở cả run của agent lẫn run tái-build của chủ dự án).
  mode 0: (không field riêng — chỉ header u8/u32/u16; chỉ khi this[5402]==2 → sub_70E511: phase 3, vf155(1),
          resume đồng hồ trận — xem §5a-3, đã kiểm runtime 2026-10-18)
```
Đã chạy thật (fakeserver `TURN_MODE`): mode 4 với a4=1 → bị cổng từ chối (im lặng); mode 1 → client gửi 139 ✓ +
nạp lại round (renderer đổi layout, nhân vật về spawn); mode 2 (a4=1,a5=1,a6=12345) → client gửi 140 ✓ (màn hình
đen ~5 s là loading, rồi map mới hiện). Nhân vật **vẫn chưa tự di chuyển** — gợi ý còn thiếu state/gói khác (xem §12 cập nhật).
CHƯA KIỂ: ý nghĩa thật của a4/a5/a6 và this[4887]; sau 140 cần gì nữa để mở điều khiển.

### 5a-2. Opcode 190 (S→C) — "GO" (`sub_70E7D6` 0x70E7D6, dispatch case 190) ✅ decompile (2026-10-17)
```
opcode 190 (S→C):
  u32 v27   (Decode4; CHƯA KIỂ — có vẻ số lượt/seed; được truyền vào sub_5A8DA5 nếu this[5417] != 0)
  u8  v28   (Decode1; CHƯA KIỂ — kind, cũng truyền sub_5A8DA5)
  u32 t1    (Decode4; giây → this[19736] = 1000*t1)
  u32 t2    (Decode4; giây → this[19740] = 1000*t2; this[19744] = 0)
  u8 flag   (Decode1)
    flag != 0: (không đọc thêm) — duyệt bomber 0..1: lấy [vt+40] của obj+1524 → sub_414F70
    flag == 0: u8 n → n × u8 → sub_41D3CB từng giá trị
  Đầu hàm: sub_70C5DC(2) [phase = 2]; giữa hàm: sub_70319B, loop bomber: sub_592ADE(1) + [vt+424](24, 10120);
  cuối: tính số lượng item shield từ bảng item; nếu this[5417] là CGameMiniMapWnd → sub_7E397C(a6, map_id)
```
Đã chạy thật: `GO_START=4` (4 s sau 189) → client hiện overlay trung tâm "**THỜI GIAN**" + số 0 → tức phase 2 đã
set. Đồng hồ bên (GIỚI HAN) vẫn 00:00 và nhân vật vẫn chưa điều khiển được — bộ đếm/điều khiển có nguồn khác
(CHƯA KIỂ — nghi gói 356/176 theo §5g, hoặc bomber cần state từ [vt+424]/loop nguồn khác).
fakeserver: `round_go()` + env `GO_START`/`TURN_*`.

### 5a-3. Mở điều khiển: phase + đồng hồ trận ✅ runtime (2026-10-18, Claude Opus 5.5, gdb)
Kết luận: kẹt điều khiển là **thiếu gói (giả thuyết 1)**, không phải do skip 0x736C27. Chuỗi đúng:
**501 → 189 mode 1 → 190 → 189 mode 0** (fakeserver: `TURN_MODE=1 PLAY_START=4`).

Đường phím (đã kiểm từng bp, `tools/gdb/input_gates.gdb` + `input_dispatch.gdb`):
- `CGameStage__vf15` 0x7120F6 (WM_KEYDOWN 0x100/UP 0x101/CHAR 0x102; cổng `this[91] >= 256` — runtime 0x10D, qua)
  → `sub_70DA84` → bảng phím `sub_8DD392` (←↑→↓ = action 0/1/2/3 (VK 25/26/27/28), Space = 4; slot 0)
  → `sub_708CC8(key, action, slot, down)`: cổng `byte this[0x570A+slot]` (ctor `sub_70CBAF` gán sẵn 01/01 cho slot 0/1;
  bật/tắt qua sự kiện nội bộ `CGameStage__vf1` 0x7085E5 type 7/6 → `sub_7037E7`) — runtime 01, qua
  → strategy list `this[5413]` vf17 (+0x44) (obj vt 0xC668AC, trả 0) → game context `this[5407]` (`CGameContext`
  vt 0xC5466C) vf8 (+0x20) = `sub_6E5E05`: bomber `sub_6E5C77(slot)` → controller `bomber+1524` vf3(dir) /
  case 4 đặt bom → `CGOCAHeroGameBomber__vf6_2` …
- Không có gói C→S nào cho di chuyển (cả trước lẫn sau khi mở): input là client-side tới controller bomber.

Vì sao vẫn đứng: tick `CHiddenCatchGameStage__vf46` 0x70572A (slot vt 46 của CGameStage) đọc đồng hồ
`this[5342]` = `CGameClock` (vt `vt_CGameClock`, nằm ở `ctx+0x1DF05C`, `sub_6E204D`):
`T[2]` started, `T[3]` cờ, `T[4]` base (timeGetTime), `T[5]` arg start, `T[6..9]` "giữ giá trị", `T[10]` mốc pause,
`T[12]` tổng thời gian pause. `T[2] == 0` → mỗi tick gọi vf161 (vòng chờ, query bomber theo slot); `sub_6E19F6`
(elapsed) ≠ 0 → vf159 (bước mô phỏng).
- **Start**: `sub_6E1AF7(clock, a2)` (T[4]=now−x, T[2]=T[3]=1, xoá pause) — gọi qua `sub_6E2054` (ctx) từ:
  189 mode 1 (`sub_70E357`, vô điều kiện), 189 mode 4 **chỉ nhánh B**, 191 `sub_709A38`, `sub_709D5E`, `sub_70A04B`,
  `sub_70A158`, `sub_70A647`; và qua `sub_5A8DA5` trong 190 khi `this[5417]` ≠ 0 (strategy; trận thường = 0).
- **Pause**: 190 → `sub_6E1B3D` (T[10]=now). **Resume**: `sub_6E1A86` — chỉ từ `sub_70E511`.
- **Phase** `this[5402]` (`sub_70C5DC`): 189 mode 1 → 1; 190 → 2; **189 mode 0 khi phase == 2 → `sub_70E511`**: phase 3,
  vf155(1) (BGM), resume clock, `sub_70319B`, strategy vf16, `sub_6FFCC4`, vf74 → `sub_72EFC3`.
- 189 mode 0 = header chung (`u8 0, u32, u16 map_id`), không có field riêng (map_id → `sub_8EAB95`).

Số liệu runtime (`tools/gdb/phase_clock.gdb`, bp tick lấy mẫu 1/400):
- 189 mode 4 (nhánh A) → 190 → 189 mode 0: PHASE 0→2→3 ✓, pause/resume ✓ (acc=0xFB8 ≈ 4 s) nhưng **T[2]=0, base=0** suốt
  → mô phỏng không chạy, không thấy nhân vật trên map.
- 189 mode 1 → 190 → 189 mode 0: HUD đổi (bộ đếm "00" trên cùng, mũi tên đánh dấu spawn), nhân vật hiện ở spawn góc
  phải-trên và **quay mặt theo ←→↑↓** (`docs/evidence/images/vn/battle_turn_LRUD.png`, `docs/evidence/images/vn/battle_phase3.png`). Không đi được vì ô spawn
  bị chặn (viền/cây/gạch) — cần đặt bom.
- Space (đặt bom) → **crash trong VM**: EIP 0x11FB494, stack `sub_708CC8` → ctx vf8 case 4 (0x6E5F2A) →
  `CGOCAHeroGameBomber__vf6_2` (call tại 0x5B1FB5) → `sub_6EFD24` = site VM **0x6EFD4A** (vmtriage P1). Khối VM này
  đã được viết lại (2026-10-18, `docs/archive/re/VM_6EFD4A_INPUTS.md` §8).

CHƯA KIỂ: ý nghĩa u32 của 189 (this[4887]) và v13/T[5] truyền vào start; mode 1 hợp với trận thường hay chỉ là
"nạp lại round" (thứ tự server gốc chưa biết — đây là chuỗi tối thiểu chạy được).

### 5a-4. Khoá phiên, đồng hồ "GIỚI HẠN", 191 và 146 ✅ IDA (+ runtime 191/146 nhận được) (2026-10-18, Claude Opus 5.5)
- **`this[4887]` = khoá phiên** (ghi bởi 189, 146 `sub_70A647`, 571 `sub_71ADEC`, `sub_70A647`; ctor = 0).
  `sub_6FFFA0` trả nó; `CGameMsgCache` dùng byte thấp làm byte ĐẦU của mọi gói trận C→S (vf7, `sub_6F058C`, bản ghi
  vòng `sub_6EFBCA`); `CHiddenCatchGameStage__vf61` (xử lý gói trận nhận về) bỏ gói nếu byte đầu ≠ khoá.
  Byte thứ hai của các gói đó = **`ctx[490549]` = chỉ số context/lượt** trong danh sách `stage[5409..5410]` (gán ở ctor
  context `sub_6EE9C9` từ `CGameStage__vf152`; lượt đầu = 0) — cũng chính là `a4` của 189 mode 4 (nhánh A < số context).
- **Tham số start đồng hồ** (`sub_6E2054(x)` → `sub_6E1AF7`): `T[5] = x` (thời gian game ban đầu; `sub_6E2096` = T[5] +
  elapsed), `T[4] = timeGetTime() − ping/2` (`sub_8F28D1(E6092C) >> 1`, bù trễ — client tự tính). 189 mode 1 dùng lại
  T[5] cũ rồi **khôi phục T[4] cũ** (`sub_6E2087(v12)`) → lượt đầu base = 0 (elapsed = uptime). Server không truyền
  base; chỉ 191/146 truyền `x`.
- **"GIỚI HẠN" (HUD)** = thời gian còn lại `sub_6E1F27` = `ctx[490518]` − (elapsed − pause), kẹp ≥ 0.
  `ctx[490518]` chỉ đặt bởi `sub_6E1F0C`: các component chế độ chơi (vf49: 90 s/120 s/180 s/300 s…) và OnEnter
  `sub_71A9EC` **khi `room[84]` ≠ 0** (= `sub_6DD49B`: map thuộc loại đặc biệt trong bảng 231) với giá trị `room[78]`;
  `room[78]` = `1000 × u8` chỉ đọc trong **opcode 117** (`CPrepareStage__On_117`, biến thể bắt đầu trận có blob).
  ⇒ Trận thường qua 501 + Village01: giới hạn 0 → HUD 00:00 là **đúng theo code**, không phải lỗi gói.
- **Opcode 191** (`sub_709A38`, case 171+20): `u32 x` (start đồng hồ), `u32 t1` → this[19736], `u32 t2` → this[19740]
  (ms, **không** ×1000), `u8` → box this+19652; this[19700]=1, clock f3=1, bomber `sub_592ADE(1)` + [vt+424](24, 9860),
  vf155(1) nếu không strategy nào nhận. fakeserver `round_begin()`, env `BEGIN_START` (+`BEGIN_OP=191`). Runtime:
  nhận, không crash, không đổi HUD (đúng như trên).
- **Opcode 146** (đối chiếu IDA §5h — cấu trúc đúng, sửa nghĩa): `u32 x` → **start đồng hồ** (không phải "context"),
  `u32` → **khoá phiên** (không phải round_len), `u32 t1`, `u32 t2` (giây ×1000 → this[4934]/[4935] = 19736/19740),
  `u8` → box; rồi `u32 rank_key` (≠0 → htonl → `sub_9B54FE`), `u16`, `u8` (`sub_9B5989`); 18 × {`u8 on`; on → 3 ×
  (`u32`, `u16`) → `sub_9B5C5A`}. fakeserver `round_result()`, `BEGIN_OP=146`. Runtime: nhận, màn hình tối lại (trạng
  thái kết quả), không crash.
- Opcode 531/532/572 (`sub_70A158`/`sub_70A04B`/`sub_709D5E`) cũng start đồng hồ — chế độ đặc biệt, chưa xem.

### 5a-5. Vòng mô phỏng chạy được: đồng hồ của **context 0** + gói C→S 111 ✅ runtime (2026-10-18 tối, Claude)

**Đính chính §5a-3** ("189 mode 1 start đồng hồ" là chưa đủ): `CGameStage` giữ **danh sách context trận** (vector `stage+0x5484..0x5488`),
context đang chọn ở `stage+0x547C` (`sub_6FFFDB`), chọn bằng `sub_704E16(idx)`. Còn vòng tick (`vf46` 0x70572A) đọc đồng hồ qua con trỏ
**cache** `stage+0x5378`, chỉ được cập nhật trong reset `sub_71A9EC` (gọi ở OnEnter `vf44` và 0x71AD8F) = đồng hồ của context lúc reset.

Đo bằng gdb (`tools/gdb/ctx_select.gdb`, `clock_stop.gdb`):
```
OnEnter: reset (5378 <- clock ctx0), SELECT idx=0 (ctx0)
189 mode 1: SELECT idx=1 (ctx1 mới, ret 0x70C6E3) -> START clock ctx1 (sub_70E357)
191 (nếu gửi trước 189 mode 0): START clock ctx1
đổi phase (ret 0x70C663): SELECT idx=0 -> quay về ctx0 — đồng hồ ctx0 CHƯA start -> tick chỉ chạy vf161 (chờ), vf159 không bao giờ chạy
```
⇒ **Chuỗi server chạy được:** `501 → 189 mode 1 → 190 → 189 mode 0 → 191` (191 *sau* 189 mode 0, khi đã về ctx0). fakeserver/server Rust: `POST_PLAY_START=2`.
Kết quả chạy thật (`logs/xvfb/postplay`): `CGameStage::vf159` (bước mô phỏng) chạy mỗi tick, HUD "GIỚI HẠN" đếm, nhân vật **đi được** (`logs/xvfb/walk2/step_*.png`), và client gửi **C→S 111**. Bom **chưa nổ** (bóng nước vẫn nằm
đó sau ~20 s): `CGOCAHeroGameBomber__vf6_2` chỉ tạo bom cục bộ (`sub_5B0053`) khi `bomber vf35` (box +184) ≠ 0; ngược lại (người chơi mạng) chỉ ghi
bản ghi 111 — nổ do server điều khiển (ứng viên: S→C **151** `CGameStage__On_151` → `ctx sub_6E9994(id, 0, u32)` → `sub_7324C0` = nổ bom có id = box bom +168). Thứ tự gói của server gốc vẫn CHƯA BIẾT — đây là chuỗi tối thiểu làm mô phỏng chạy.

**Flush:** `CGameStage::vf159` → `sub_6FFE4C` → `CGameMsgCache::vf3(0)` → `vf4`,`vf5`(slot) → nếu `count[slot] != 0` → `vf8(slot)` (0x6F06ED): mỗi bản ghi
một gói 111, rồi `count[slot] = 0`. (`vf3(1)` cũng được gọi trong handler gói 552 và lúc dựng stage.)

**C→S 111 — đặt bom** (bản ghi 68 B của bảng vòng 13/17; ghi bởi `sub_6EFD24` khối VM 0x6EFD4A hoặc `sub_6EFEE3` native):
```
u32 rec+0x08  KHOÁ PHIÊN (sub_6EFBCA → sub_6FFFA0 = this[4887], = u32 của gói 189; §5a-4)
u8  rec+0x00  chỉ số context (arg_0 = ctx[490549])   u8 slot
u32 rec+0x0C  (arg_8 = box bom +280)      u8 rec+0x10 hàng (y)               u8 rec+0x14 cột (x)
u8  rec+0x30  (arg_C = sub_59119D bom)    u8 rec+0x34  u8 rec+0x38  u8 rec+0x3C   (arg_20/24/28)
u8  rec+0x18  (arg_18 = bomber vf31 = **tầm nổ**, 1 → 2 sau ItemFire)      u32 rec+0x1C (arg_1C = box bom +192)   u8 rec+0x40 cờ (arg_2C)
if (cờ & 3): u8 rec+0x20, u8 rec+0x24, u8 rec+0x28, u16 rec+0x2C        (chỉ đường sub_6EFEE3 16 tham số)
```
Ví dụ thật (slot 0, ô cột 14 hàng 1; server gửi khoá 10000 trong 189): `khoá=10000, 0, 0, 0, 1, 14, 1, 0, 0, 0, 1, 1, 0`. Ý nghĩa từng field ngoài hàng/cột: CHƯA KIỂM.

#### 5a-5b. Vòng đời bom: C→S 111 → S→C 152 (gán id + đặt) → S→C 151 (nổ) ✅ runtime (2026-10-18 đêm)
- `CGOCAHeroGameBomber__vf6_2` (Space): nếu `bomber vf35` = box `bomber+0x6AC` ≠ 0 → bom **cục bộ** (`sub_5B0053`); người chơi thường = 0 (box chỉ đặt bởi
  chế độ SnowBear/SnowBall/IamBoss, `sub_599C68`) → lấy bom trong pool (`CGOGameBomb`, id box +168 = -1, số thứ tự box +280) và chỉ gửi **111**. **Server quyết định.**
- **S→C 152** (`CGameStage__On_152` 0x7064AC) → ctx `sub_6E9AF9` → `CBombOwner vf20(desc)` (0x578BA3): tìm bom trong pool có số thứ tự == `desc+0` và id == -1,
  `sub_591209(bom, desc+12)` gán **id**, rồi `CGOGameBomb vf37(desc)` khởi tạo. Thứ tự trường (từ disassembly; desc = biến cục bộ `ebp-0xE4`):
  ```
  u8  ctx (phải == chỉ số context, = rec+0x00 của 111)   u8 slot   u16 a3 (-> desc+176)
  u32 desc+0  số thứ tự bom (= rec+0x0C của 111)         u32 a4, u32 a5 (owner vf23 — rỗng với bomber thường)
  u8  desc+24 (bom +428 = rec y)   u8 desc+36 (bom +440 = rec x)   u8 desc+48 (bom +180)   u32 desc+60 (bom +192 = rec r1c)
  u8  desc+72 cờ — nếu (cờ & 3): u8, u8, u8 (tham số vf33 của bom)
  u16 desc+12 = ID BOM MỚI        u16 desc+96        u8 desc+156 (sức nổ, = rec+0x30)        u64 desc+168
  u8  desc+108 loại (bom +328 = rec+0x34)   u8 desc+120 (bom +340 = rec+0x38)   [u8 desc+132 nếu loại 1 hoặc (4 & desc+120==4); u8 desc+144 nếu loại 2]
  ```
  Giá trị desc+48/+96/+168 hiện đoán (0 / r18); cần đối chiếu nếu có log server thật.
- **S→C 151** (`CGameStage__On_151` 0x70205B): `u8 n`, n × {`u16 id`, `u8 kind`, kind == 0: `u32 arg`} → ctx `sub_6E9994(id, kind, arg)`: tìm bom theo id (box +168);
  kind 0 → `sub_7324C0` (**nổ**), kind 1 → đẩy bom sang ô khác. Ý nghĩa `arg` CHƯA KIỂM (thử 2).
- **Kiểm chứng** (`logs/xvfb/boom2`, `docs/evidence/images/vn/battle_bomb_explode.png`, gdb `tools/gdb/bomb152.gdb`): Space → 111 → server 152 (id 1) → `OWNER_VF20`, `SETID id=1`
  → 2.5 s sau server 151 {1, 0, 2} → `sub_7324C0` → **bom nổ, nhân vật bị nhốt trong bong bóng ("HELP!") rồi chết**.
  fakeserver: `BOMB_PLACE=1` (+ `BOMB_ECHO=2.5 BOMB_IDS=2`); server Rust: `BOMB_FUSE` (mặc định 2.5 s; gửi 152 rồi 151 cho đúng id, test `server/tests/bomb.rs`).
  Thời gian nổ thật của game gốc và cách server tính vùng nổ/hạ gục: CHƯA BIẾT (client tự vẽ vụ nổ khi nhận 151).
- Đã loại trên đường tìm: bảng factory gói (`dword_EA78E8`/`dword_E64B20`, lúc chạy chỉ opcode 70), `CNetSyncMan` (đồng bộ NPC), `CGameStage::vf61` 0x70922B
  (gói trận u16 subtype + byte khoá, có kiểm độ dài — nghi kênh UDP), `CSessionStage__On_143` (16 × 2 byte trạng thái ô).

#### 5a-5c. Sau khi bị bom nổ trúng (đang làm, 2026-10-18)
Quan sát (`logs/xvfb/boom2`, `logs/xvfb/rust_boom`): ngay sau 151 client gửi **C→S 120**; ~6.3 s sau (nhân vật chết) gửi **C→S 162** rồi **120** lần nữa.
Server giả chưa trả gì → nhân vật nằm đó, ván không kết thúc.
- **C→S 120 = sự kiện của bomber** (gói nhóm `sub_88851B(120)`), phần đầu do `CGOCAHeroGameBomber__vf112` (0x5AF165) ghi, rồi người gọi ghi phần riêng:
  ```
  u16 op 120 | u8 SUBTYPE | u8 cờ (bomber vf) | u16 x px (cột*40) | u16 y px (hàng*40+39) | u32 thời gian (sub_8F2ADB) | phần riêng theo subtype
  ```
  Subtype quan sát/đọc được (vf112 gọi qua `(*this+448)(this, pkt, subtype)`):
  - **11** — `CGOCAHeroGameBomber__vf121` trạng thái 3 (**bị nhốt** trong bong bóng): + blob (`sub_9C86D5/9C86E9`, thông tin người/bom gây ra). Gửi ngay sau 151.
  - **13** — `CGameStage__vf150` 0x7172E7 (**chết**, ~6 s sau khi bị nhốt): + blob (`sub_9C8CDA`). (gọi từ `sub_7172A7` ← tick 0x718D51.)
  - 17 — vf121 trạng thái 0: + u32 1 hoặc 2; 26 — vf121 trạng thái 2: không thêm. CHƯA KIỂM ý nghĩa.
  Ví dụ: `subtype 11, cờ 0, x 560, y 79, t 5349, blob 14 B` rồi `subtype 13, …, blob 11 B`.
- 162 có nhiều nơi dựng (`CGOCAHeroGameBomber__vf6_1` 0x5A3681: `u16, u16 0, u16 box+1394, u16 box+1397`; `CGOGameBomb__vf45`; `CHiddenCatchPlayMap`…). CHƯA KIỂM nơi nào gửi.

**Kết thúc lượt bằng 146 (thí nghiệm `DEATH_END=2`, `logs/xvfb/end1`, `docs/evidence/images/vn/battle_round146.png`):** 2 s sau 120/13 server gửi **146** (`round_result(key=10000,
slot 0 có điểm)`) → client **sang lượt mới**: nhân vật hồi sinh ở ô xuất phát, HUD "GIỚI HẠN" đặt lại (01:55 rồi đếm lùi), client gửi C→S 180. Tức 146 = "kết quả lượt +
bắt đầu lượt kế" (đúng §5a-4: 146 cũng start đồng hồ). Kết thúc cả trận / về phòng chờ: gói khác (ứng viên 571 `sub_71ADEC`, 117) — CHƯA LÀM.

**Kết thúc trận → về phòng chờ: S→C 173** ✅ runtime (`MATCH_END=3`, `logs/xvfb/over1`): `CGameStage__OnPacket` case 153 (+20 = 173) → `vf149` (0x7132xx):
`u8` → `sub_8E75D2`; đặt lại 16 slot; `u8 có danh sách`; nếu ≠ 0: `u32`, `u16 n`, n × {`u8` + blob}, rồi n × {trạng thái slot/người chơi …}; cuối cùng
`ChangeStage(12)` (hoặc 11 khi `sub_8DACD5()`). Gói tối thiểu `{u8 0, u8 0}` → client về **phòng chờ** (`dne:12`), gửi lại 337 → server trả 103/106/113 như lúc vào phòng;
với `AUTO_START` server lại gửi 501 → **trận mới**. Vòng lặp đầy đủ sảnh → phòng → trận → chết → phòng → trận chạy được (fakeserver + server Rust `MATCH_END`, mặc định 3 s).
Danh sách người chơi/kết quả trong 173 khi ≠ 0: CHƯA KIỂM.

#### 5a-5d. Gạch vỡ và vật phẩm (đang làm, 2026-10-18)
- **Gạch vỡ do client tự tính** (`logs/xvfb/brick1`): bom nổ cạnh gạch → gạch biến mất, client báo **C→S 135** (`sub_706FC0`, gọi từ `sub_5BF711`
  "vỡ gạch"): `u8 y(hàng)`, `u8 x(cột)`, `u8 slot`, `u16 -1`. Ví dụ `{1, 13, 0, 0xFFFF}` = gạch ở hàng 1 cột 13.
- **Vật phẩm do server gửi trước khi vào trận — S→C 553** (`CStage__OnPacket` → `sub_6A5A5E`) ✅ runtime (2026-10-18 tối, `logs/xvfb/item_kinds3/4`,
  `docs/evidence/images/vn/battle_item_kinds.png`): một blob, inner **bắt đầu bằng byte tag 0** (raw copy, `sub_A62580` — thiếu thì mọi thứ lệch 1 byte), rồi số little-endian:
  ```
  u8 0 (tag)
  u16 n1, n1 × {u16 index, u8 hàng (y), u8 cột (x), u32 kind}   VẬT PHẨM NẰM TRÊN NỀN  (room ctx +339 -> sub_729F44 vòng 2 -> sub_71FCFE)
  u16 n2, n2 × {u16 index, u8 hàng (y), u8 cột (x), u32 kind}   VẬT PHẨM GIẤU TRONG GẠCH (+343 -> sub_729F44 vòng 1 -> sub_5BF485, chỉ ô CGOTileFixedBox)
  u16 n3, n3 × {u16, u32, u32}                                    CHƯA KIỂM (+347)
  ```
  Gửi **trước 501** (đọc lúc nạp map). `index` < 1000, mỗi vật phẩm một index riêng (`sub_89AEA3` đăng ký `kind` vào ô `index`).
  **`kind` = thứ tự vật phẩm trong `assets/fx/Attr/ItemAttr.bin` mục `Item/*` tính từ 0** (đã thấy trên màn hình 0..12 theo đúng thứ tự):
  0 ItemBomb (bóng nước, +1 bom), 1 ItemFire (thuốc, +tầm nổ), 2 ItemRoller, 3 ItemShoe (giày), 4 ItemPunch, 5 ItemMaxPower, 6 ItemHollow, 7 ItemSkeleton0,
  8 ItemSkeleton1, 9 ItemMoonWalk, 10 ItemSuperMan, 11 ItemGreenDevil, 12 ItemRandomBox (rương) … (danh sách đầy đủ: parse `ItemAttr.bin`, 138 mục).
  Đặt 50 vật phẩm: 50 lần `sub_5B6578`, 50 lần đăng ký; vật phẩm ở ô cây/gạch vẫn vẽ chồng lên (server phải tự chọn ô trống).
- ~~Toạ độ (a,b) = (x,y), kind 1 = bạch tuộc~~ — **SAI**, do gói thiếu byte tag (đọc lệch 1 byte); đúng ở trên: (hàng, cột), kind theo `ItemAttr.bin`.
- **Nhặt vật phẩm = C→S 117 → S→C 158** ✅ runtime (`ITEM_ACK=1`, `logs/xvfb/item5`, `docs/evidence/images/vn/battle_item_picked.png`, gdb `tools/gdb/item158.gdb`):
  bước vào ô → client ghi bản ghi chờ vào bảng 21 của `CGameMsgCache` (`sub_6F2ECE`, gọi từ `sub_5B098A`) và `vf9` (0x6F0874) gửi **117** (gói nhóm):
  ```
  u8 ctx | u8 slot | u8 a (rec+12) | u8 b (rec+16) | u16 index vật phẩm (sub_5B61A9) | u16 kind (sub_5B61B4) — các số 13/256 quan sát được trước đây là do 553 đọc lệch
  u32 thời gian (sub_6EFBB8) | u32 khoá phiên (sub_6EFBCA) | u8 (rec+36)
  ```
  Không có trả lời → vật phẩm hiện lại khi bước ra. Server trả **158** (`CGameStage__On_158` 0x715E90):
  `u8 ok (≠0 mới áp dụng)`, `u8 ctx (== chỉ số context)`, `u8 slot`, `u8`, `u8` (đang gửi lại a/b — đoán), `u16 index`, `u16 kind`, `bool`
  → `sub_6F001E` xoá bản ghi chờ (khớp ctx + index), `sub_706E0A(slot, kind)` áp dụng vật phẩm → **vật phẩm mất hẳn**; sau đó client gửi C→S 162 (CHƯA GIẢI MÃ, nghi
  cập nhật hiệu ứng/túi vật phẩm). Kiểm với server Rust (`logs/xvfb/rust_item`, `docs/evidence/images/vn/battle_item_slot.png`): vật phẩm (đọc lệch, kind thật không rõ) vào **ô VẬT PHẨM (Ctrl) ×01** = loại dùng chủ động.
  Hiệu ứng của từng `kind` lên nhân vật (tốc độ/số bom/tầm nổ): CHƯA KIỂM.
  fakeserver: `ITEM_ACK=1`; server Rust: luôn trả 158 cho 117, `ITEMS="x,y,kind;…"` gửi 553 trước 501 (`server/tests/bomb.rs` khớp byte với fakeserver).

- **Dùng vật phẩm (Ctrl)** ✅ runtime với **kim** (`kind 117` ItemNiddle, `FLOOR_ITEMS="13,0,117"`, `logs/xvfb/needle1`, `docs/evidence/images/vn/battle_item_needle.png`):
  nhặt kim (117 → 158) → kim vào ô VẬT PHẨM (Ctrl) ×01 → tự đặt bom, bị nhốt (120 subtype 11) → bấm Ctrl → **nhân vật thoát bong bóng, sống tiếp**, ô trống.
  Client gửi **C→S 120 subtype 1** (thoát bong bóng, chỉ phần đầu 120) rồi **120 subtype 0** (dùng vật phẩm: `u8 1`, `u16 kind`, `u8 0`, `u32 0`).
  Không cần server trả lời — hiệu ứng do client tự làm (server chỉ cần phát lại cho người chơi khác khi có nhiều người). Hàm dùng vật phẩm: `sub_57762F`.
  Vật phẩm Ctrl khác (khiên `kind 123` 2000 ms, phi tiêu 115, thuốc 119, lò xo 120…): cùng đường, chưa thử từng loại. Bảng `kind`: `docs/specification/gameplay/ITEMS_VN_REFERENCE.md`.

- **Hiệu ứng vật phẩm nhặt được (bị động) — client tự áp dụng sau 158** ✅ (`FLOOR_ITEMS="13,0,1;14,1,0"`, `logs/xvfb/item_use3`, `docs/evidence/images/vn/battle_item_effects.png`):
  nhặt `kind 1` ItemFire rồi `kind 0` ItemBomb (2 lần 117 → 158) → đặt được **2 bom liền nhau** (2 bản ghi 111, số thứ tự bom 0 và 20) và trường `r18` của 111
  (bomber vf31) tăng **1 → 2** = tầm nổ; vụ nổ phá gạch xa hơn. Server không cần gửi gì thêm ngoài 158 (server Rust gán id bom 1, 2 và nổ từng quả).

### 5a-6. Hai người chơi (đang làm, 2026-10-18 tối)
- **Chạy 2 client cùng lúc:** `tools/duo.sh <outdir>` — client A: Xvfb :77, `WINEPREFIX=~/.wine`, `wine/Boom`; client B: Xvfb :78, `~/.wine2`, `wine/Boom2`
  (prefix riêng → wineserver riêng → khoá chỉ-một-bản `Global\FxClient…` không đụng nhau). `CLICKS_A/B`, `KEYS_A/B`, `RUST=1`.
- **Server Rust có phòng chung** (`server/src/hub.rs`): mỗi lần đăng nhập (op 1) cấp id 1000, 1001…; op 6 trên kết nối kênh mang id (tìm u32 trong thân gói);
  mọi client vào phòng (337) đều vào **một phòng**, slot theo thứ tự (slot 0 = chủ phòng); người mới nhận 103/106 của người cũ và ngược lại;
  đủ `PLAYERS` người → 501 (danh sách slot) + chuỗi 189/190/189/191 phát cho cả phòng; 152/151 (bom) và 158 (vật phẩm) phát cho cả phòng.
- **Kết quả** (`logs/duo/t2`, `docs/evidence/images/vn/duo_match_B.png`): cả 2 client vào chung phòng chờ rồi **cùng vào trận**, HUD hiện **2 người chơi**.
  Nhưng mỗi client chỉ thấy nhân vật mình di chuyển, nhân vật kia đứng yên ở ô xuất phát (hai nhân vật cùng xuất hiện ở góc phải trên) ⇒
  **vị trí/di chuyển không đồng bộ qua TCP**. Client có engine UDP riêng (`MTUDPBase`, `MTUDPClient`, `CUDPPeer`, `CUDPPeerServer/Client`, `CUDPPeerMan`,
  chuỗi `ErrNoUDP`, `CheckUDPChecking`) — nghi chủ phòng làm UDP server, người khác nối vào; địa chỉ/cổng lấy từ gói nào: CHƯA RÕ (ứng viên: các u32 trong 103,
  host + 3 cổng của gói 15 — server đang gửi cổng 0).
- **Địa chỉ UDP đi qua TCP** (IDA, 2026-10-18 tối): client mở socket UDP riêng (`ss`: wineserver nghe 0.0.0.0:97xx) và **báo cho server bằng C→S 92**
  (`CGameStage vf161` 0x7027B0, một lần khi vào trận): `u32 ip1, u32 ip2` (`sub_9B5552`, IP LAN, byte theo thứ tự mạng như số BE), `u16 cổng UDP`
  (`sub_9B555E`), `u32` (map), rồi mỗi người chơi trên máy (≤2) `bool ×4`. Server trả bảng peer trong **S→C 146** (`sub_70A647`, ĐÍNH CHÍNH: 146 không chỉ là
  "kết quả lượt" mà là **bắt đầu lượt + bảng UDP**): `u32 rank_key` ≠ 0 → `htonl` → `sub_9B54FE` = **MTUDP HostCreate tới máy chủ relay** (`u16` cổng đi kèm);
  18 slot × {`u8 on`; on → 3 × (`u32 ip`, `u16 cổng`) → `sub_9B5C5A(slot, …)` = tạo peer + HostCreate tới từng địa chỉ}. Gói 15 (đăng nhập) chứa host + 3 cổng
  nhưng client **không** gửi UDP tới đó (thử `UDP_PORTS=7000,7001,7002` + `tools/udp_sniff.py`: 0 gói).
  Server Rust: lưu (ip1, ip2, port) từ 92; khi `PLAYERS` > 1 gửi **146 kèm bảng peer** (127.0.0.1, ip1, ip2 với cổng của từng client) thay cho 191.
  ✅ **Di chuyển đồng bộ hai chiều** (`logs/duo/t3`, `logs/duo/t4`, `docs/evidence/images/vn/duo_sync.png`: 3 ảnh đầu = màn A, 3 ảnh sau = màn B): A sang trái → B thấy; B đi xuống →
  A thấy. Các client nói chuyện **trực tiếp qua UDP (P2P)**, server chỉ cần trao địa chỉ (không relay, `rank_key` = 0).
- Ô xuất phát: cả hai về cùng ô góc phải trên — `pos` trong 501 (đang = slot) hoặc khối slot của 189 mode 1 chưa đúng nghĩa. CHƯA KIỂM.

### 5b. C→S các gói client tự gửi khi chơi
| op | hàm gửi (địa chỉ) | nội dung Encode (thứ tự Encode chính xác) |
|---|---|---|
| 139 | `sub_70E357` 0x70E357 (do On_189 mode 1 dẫn) | `bool 1` |
| 140 | `sub_70DF00` 0x70DF00 / `sub_70DFF9` 0x70DFF9 / `sub_71AD24` 0x71AD24 | `bool 1` |
| 141 | `sub_70E9FC` 0x70E9FC / `sub_70EADB` 0x70EADB — client **tự gửi trong vòng tick** `sub_71B521` (khi hết hạn stamp this[19676] / step theo seed sub_654153(3) % 12 == 0) | `u2 uid` (sub_6E2000), `u2 map_id` (sub_8E9977), `u8 n` (số ô bước, clamp 0..3), n × {`u8`, `u8`} — 2 byte mỗi ô, đọc từ queue di chuyển (sub_6EE7A1 / sub_6ED146) |
| 142 | `sub_71B275` 0x71B275 (vòng tick khi this[21956] != 0, cách nhau ≥ 2000 tick) != 0, thời gian cách nhau ≥ 2000) | `u2 uid`, `u2 map_id`, `u8 nblocks` (clamped 0..10), mỗi block: `u8 ncell`, ncell × {`u8`, `u8`} |
| 143 | `sub_71751A` 0x71751A — (agent đoán *gói đặt bom*; **runtime 2026-10-18: Space trong trận thường gửi 111, không phải 143**, xem §5a-5), caller: `CMapActionBomb__vf4` 0x752B44 (nút/nhấn bom) + `sub_76D342` | `u2 uid`, `u8 count` (= số cell), `u2 x?`, `u2 y?`, `u8 dir?`, `u32`, `u2` , rồi count × {`u8`, `u8`} |
|—| còn lại: các gói phụ: `sub_702FCA` (209), `sub_702DE4` (115), `sub_70307F` (158), `sub_70326A/70330E/7033A3/7034FF` (126), `sub_703825` (175), `sub_706FC0` (135), `sub_70732A` (99), `sub_707F0F` (192), `sub_7083EC` (211), `sub_70A3EC` (132), `sub_70B0B3` (196), `sub_70D9BE` (207), `sub_70DFF9` Ngoài nói trên (140), `sub_70E9FC` tự nói trên (141), `sub_70F090` (174), `sub_7100B2` (197), `sub_716B75` (136), `sub_71751A` bản thân (143), `sub_7190C6` (184), `sub_719A3C` (203), `sub_71B275` nói trên (142), `sub_6F4365` (148), `sub_6FB1FE`/`sub_6FA7AB` (183), `sub_703A3?` (unclear) | (1×) thông tin phụ (CHƯA KIỂ) |

> Sửa (2026-10-17): `sub_70E7D6` **không phải gói C→S** — đó là handler opcode **190 S→C "GO"** (dispatch case 190,
> `CGameStage__OnPacket`), cấu trúc ở §5a-2.

### 5c. Opcode S→C đáng chú ý của CGameStage (ngoài bảng §3; cấu trúc đọc từ decompile, ý nghĩa CHƯA KIỂ)
- **On_106** `sub_7019AE` 0x7019AE: `u8 slot, DecodeBuf 3` → `sub_8E977D(slot, buf)` — cập nhật trạng thái ô/người
  (gọn hơn opcode 103: chỉ 3 byte trạng thái, giống buf3 trong 501).
- **On_147** `CGameStage__On_147` 0x701AB1: `u8 slot` — tra bomber của slot (`sub_6E5C2B`), nếu hợp lệ → lệnh vô hiệu
  `sub_7264DE(1, -1)`; CHƯA KIỂ ngữ nghĩa.
- **On_148/149** `sub_704CD8`/`sub_704D77`: `u8 slot, bool` → `sub_8E99A3(slot)` + object+1528 gọi vf 36/52
  (bật/tắt chức năng trên người chơi; 148 bật, 149 tắt).
- **On_150** `sub_6FFF78` 0x6FFF78: `u8 slot` → `this[slot + 4895] = 1` (đánh dấu slot).
- **On_145** `sub_701A48`: `u8 slot, u16 which(1..25), u32, u32` → `sub_6EB940(slot, which, …)` (thay đổi object trên
  bản đồ, tới 25 kiểu).
- **On_146** `sub_70DE4C`: đầu tiên `sub_70A647(this, pkt)` (CHƯA KIỂ format), sau đó reset "sẵn sàng" của người chơi;
  nếu `sub_9B6F75(4)` → `this[5400] = 1`. có vẻ là chuẩn bị lượt tiếp.
- **On_155** `CGameStage__On_155` 0x701E8B: `u8` (so id của mình `sub_6E2000`), `u8` (nibble cao = kiểu hành động
  >4, nibble thấp = slot), `u16 idx`, `u8`, `u8`, `u8`, `u8` — hiệu ứng/âm thanh (`sub_6734D5` 0x4C/0x9D/0x9F),
  đổi trạng thái theo hành động (`sub_591045/50`).
- **On_157** `CGameStage__On_157` 0x7068DE: `u8 id, u16, u8, u8, u8, u8, u8` → `sub_730EF4`; nếu thất bại →
  `sub_70307F(id, 2, -1)`. CHƯA KIỂ (hiệu ứng/trạng thái ô?).
- **On_158** `CGameStage__On_158` 0x715E90: `u8, u8 id, u8, u8, u8, u16 type (97/99/100/103...), u16 itemId, bool` —
  vật phẩm/experience lớn (currency/item), nhiều nhánh riêng theo type; CHƯA KIỂ chi tiết.
- **On_164** `CGameStage__On_164` 0x701765: `u16 slot, u32`, sau đó hiệu ứng picking + cộng giá trị (gọi `sub_581ED1`
  với 212) — tiền/điểm thưởng; CHƯA KIỂ.
- **On_151** 0x70205B (chung với `CSquareStage__On_477`): `u8 n`, n × {`u16 id`, `u8`, nếu cần `u32`} → `sub_6E9994`
  (thông báo theo id).
- **On_152** 0x7064AC (chung với `On_478`): đầy đủ fields giống 478 → `sub_6E9AF9` (thông báo lớn; id 19 riêng).
- **On_153** 0x70679A: `u8 (≤4), u16 slot, u32, u8` — hiệu ứng đổi (CHƯA KIỂ).
- **On_107** 0x703D01: `u16 a(≤0xF), u8 flag, u16 c(≤0xF), u16` → `sub_8E8C5C/8E8CB1` (flag on/off thuộc tính người
  chơi).
- **On_128** `sub_71458E` 0x71458E: `u8 subtype {1..25}` rồi field theo subtype (thường `u16 id` + số lượng
  u16/u32/u64/u8/str) — cộng/trừ tài nguyên trong trận; sub-icon + string "ZombieSurvivel..." nhiệm vụ subtype 20. CHƯA KIỂ từng subtype.
- **On_25** 0x7127B3: `str` → phân token `sub_70ECEF` → mở cửa sổ/thêm người khác (CHƯA KIỂ).
- **On_81** 0x70C8C1: `u32 slot/uid, str` — hiển thị text trên người chơi.

### 5d. Minimum server để trận chạy — ✅ ĐÃ KIỂM CHỨNG runtime tới bước 4 (2026-10-17)
1. Gửi **231** trước (cấu trúc nhánh "blob có tag" như gói 14, xem §2b) để bảng map của trận được nạp đủ.
2. Khi bấm BẮT ĐẦU (hoặc tự quyết): gửi **501** đúng §4 — `map_name` phải == tên theo `map_id` trong bảng **231**,
   ngược lại: hộp thoại "Invalid map..." và quay về phòng chờ.
   ✅ runtime: fakeserver gửi 501 (AUTO_START) → client vào stage 13, nạp map thành công (qua skip 0x736C27), vẽ map.
3. Client vào stage 13, load map (đặt theme từ **231**), KHÔNG thêm gói gửi lúc vào stage.
4. Bắt đầu lượt: gửi **189** (mode 1/2 như §5a; mode 4 cần cổng a4 == getter(4)) — client trả C→S **139** (mode 1)
   hoặc **140** (mode 2); kèm **190 "GO"** sau đó để set phase 2 (§5a-2) → client hiện overlay THỜI GIAN.
   ✅ runtime: cả 139 (mode 1) và 140 (mode 2) đều quan sát được trong log fakeserver.
5. Đồng bộ di chuyển: client tự gửi **141** (và 142); **KHÔNG có handler S→C tương ứng** trong CGameStage/kế thừa
   (xem §5f) — chuyển động người khác client tự mô phỏng; trạng thái bù/tile qua **356** (§5g) và **176**.
   ⬜ runtime: chưa thấy client gửi 141/142 — nhân vật chưa điều khiển được; nguyên nhân CHƯA KIỂ (nghi thiếu
   state sau 140/190, hoặc cần thêm gói 356/176; đọc `sub_71B521` vòng tick để xem điều kiện kích hoạt 141).
6. Đặt bom: client gửi **143** (`CMapActionBomb__vf4`); server phải phát hiệu ứng (nhiều khả năng qua 158/145/163) — CHƯA KIỂ.
7. Kết thúc lượt: **146** (§5h); 128/204 không phải "kết thúc ván" rõ ràng — CHƯA KIỂ.

### 5f. Kiến trúc fallback OnPacket — object "strategy" `this[5417]` ✅ cấu trúc
Khi opcode không có case riêng, `CGameStage__OnPacket` rơi default và chạy chuỗi sau
(xem `decomp/handlers/CGameStage.c` dòng 331–356):
1. nếu `this[5417] != 0`: `RecvPkt__Rewind` rồi gọi **vf12 (offset 48)** của object đó; trả ≠0 → kết thúc nhánh;
2. `RecvPkt__Rewind` + `sub_8883BE`, rồi với **từng phần tử** list `this[5413]`: gọi vf **offset 64** trên `elem[2]`
   (loại "listener" đăng ký vào trận; chưa truy được class của elem[2] — CHƯA KIỂ);
3. cuối cùng `CSessionStage__vf51` (0x6914CD) → `CChatStage__vf51` (0x689C63) → `CStage__vf51` — **không lần nào
   có case cho 141/142** trong cả chuỗi.

- **`this[5417]` là object "strategy" theo chế độ chơi**, gán trong `CGameStage__vf146` (0x700F05), rồi gọi vf4
  (attach stage) của object:
  - kind (`sub_8E78DB(dword_E60884)`) == 17 → `sub_7E6E28` (new 0xC1C, ctor `sub_7EBA6D`) — nhánh RPG;
  - kind ∈ {18, 21, 25} → `sub_7E6DF2` → `sub_7E9613(1)` = **`CNormalGameStrategy`** (new 0x934,
    vtable `vt_CNormalGameStrategy` 0xC6BD24);
  - `sub_988CCC(sub_6DC339(sub_6DC025(dword_E60930)))` != 0 → `sub_7E6E5C` = `CMonSeason2GameStrategy`
    (new 0x10D0, vtable 0xC6BB3C);
  - map id == 114 → `sub_7E6E90` → `sub_7E6807` = `CConfusedFightGameStrategy` (new 0x968, vtable 0xC6B8AC);
  - còn lại (đường thường) → `sub_7E6DBD` → `sub_7E9613(0)` = `CNormalGameStrategy`.
- **vf12 của các strategy chỉ nhận 2 opcode**:
  - `CNormalGameStrategy/CConfusedFightGameStrategy__vf12` (0x7E8E43): opcode **188** → `sub_7E8BD9`
    (`u8 slot, u16 a, u16 b` — nếu `slot < count` (`sub_6E3D75`), lấy bomber slot (`sub_6E5C2B`), đọc vf40
    (điểm/hạng?) rồi `sub_574296(a, giá trị, b)`); opcode **197** → `sub_7E8C6D` (`u16 a` →
    `sub_574296(a, 0, 0)`); opcode khác → trả 0 (do đó luôn rơi sang bước 2/3).
  - `CHiddenCatchStrategy__vf12`/`vf16` (0x7E6D11/0x7E6D1D, dùng cho CMonSeason2) = no-op, trả 0.
- **Kết luận TODO (a)**: *trong CGameStage (và chuỗi fallback) không tồn tại handler S→C phát lại 141/142 của người
  khác*. Giải thích khả dĩ: chuyển động của người chơi khác client tự mô phỏng theo seed/tick (cùng map + cùng
  seed trong 501/189), còn trạng thái cuối/bù được đồng bộ bằng gói **356** (§5g) và **176** (tt), hoặc chỉ là C→S
  để server đối soát. Đây là suy luận — **CHƯA KIỂ runtime** (cần log bắt tay để xác nhận server có phát lại hay không).

### 5g. Opcode 356 (S→C, `sub_7155DA` 0x7155DA) — blob "sync lượt" ✅ cấu trúc (CHƯA KIỂ runtime)
Guard: feature stub == 0, `this[5400] != 2`, `sub_9B6F75(4) != 0`. Đọc `DecodeBlob`; phần nội dung đúng thứ tự:
 (`OnPkt_multi_344...`), field đọc tuần tự bằng `sub_A62F40` (u8) và `sub_A62F70` (u16):
- `sub_70F346` (0x70F346): `u8 count`, rồi count × {`u8 x_new`, `u8 y_new`, `u8 flag`} — đặt/xoá object trên ô (row,col)
  `flag == 0xFF` → xoá (`sub_729573` rồi vf92(0), vf12(0)); nhánh khác: nếu ô cũ còn object
  (`sub_7292F3(x, y)`) thì gỡ, rồi đặt vào ô mới (`sub_729573`/`sub_7294CA`). CHƯA KIỂ (nghĩa thực của hai nhánh).
- `sub_707A13` (0x707A13): `u16 count`, count × {`u16`, `u16`, `u8 px`, `u8 py`} — mỗi entry gọi `sub_728F9E`
  (40*px, 40*py+39, px, py, x1, x2, 0, str, −1) — spawn/drop vật (cùng mẫu `sub_728F9E` với 303/171).
- `sub_70048C` (0x70048C): `u16 count`, count × {`u16`, `u16`} → `sub_89AF2C(a, b)` (cặp stat, CHƯA KIỂ).
- `sub_707AD3` (0x707AD3): `u16 count` — lặp người 0..count (bomber `sub_6E5C2B(i)`):
  `u8 c`; nếu c≠0 đọc thêm {`u8 v4`, `u8 v18`, `u8 v17`, `u8 v16`}: nếu v4 ≠ 255 → bomber vf424(3, 12686),
  vf328(v4) (đổi đồ/skin lên bomber), cập nhật 4 box qua `sub_89B0D4` theo object tìm được
  (`v8 = sub_89EF9B(v4)`, box 132/144/156/168); `sub_594D51(v18)`, `sub_599557(v17)`, v16≠0 → vf452(0).
  c==0 → `sub_592ADE(0)`. CHƯA KIỂ từng field (nhiều khả năng là thay đổi trang bị/chỉ số của người chơi).
- `sub_7004DA` (0x7004DA): `u8 count`, count × {`u8 a`, `u8 b`} → object theo id `sub_6E2119(b)` rồi
  `sub_5B4AE9(b, a, 0, 0)` (set vị trí(?) trực tiếp, CHƯA KIỂ).
- Sau tất cả: `this[5400] = 2` (chuyển trạng thái vòng).

### 5h. Gói 146 — KẾT THÚC lượt + phần thưởng (TODO b, ✅ cấu trúc; CHƯA KIỂ runtime)
```
On_146 (sub_70DE4C 0x70DE4C) — phần 1 gọi sub_70A647 (0x70A647):
  u32 Block_a             // Decode4; đưa vào context qua sub_6E2054 — CHƯA KIỂ ý nghĩa
  u32 round_len   → this[4887]
  u32 dur1, dur2  → this[4934], this[4935] (×1000 hai cái — ms)
  u8  flag_a      → box (sub_413F32); this[4936] = 0
  // tính "thời gian phòng": this[4913] != 0 ? 30 : *(dword_E60934 + 468824) → sub_6E20AD
  // bật hiệu ứng vf424(24, 11814/11832) trên các object trong context
  // tạo dialog kết quả cỡ malloc 0xE8/0xA0/0xA4/0xB0 theo nhánh feature
  //  (58/win-condition, "BonusMazeMap01/02", "Dialog/DialogBonus"...)
  ---- phần "điểm/BXH": ----
  u32 key         // nếu key != 0: htonl(key) → sub_9B54FE(key, extra)
  u16 extra       // (arg thứ 2 của sub_9B54FE)
  u8  flag_top    // sub_9B5989(flag_top)
  // 18 vòng (i = 0..17; CGameStage dùng 16 slot 0..15 — sao 18? CHƯA KIỂ):
  u8 on                                   // Decode1
  sub_8E99A3(&id, i)                      // id người của slot i
  if on != 0:
      3 × { u32 val,      // Decode4
            u16 st }      // Decode2 (low half; mảng 3 phần tử {u32, u16})
      nếu sub_8EAC0E(i) != 0 → sub_9B5C5A(i, mảng, val, ...)   // lưu BXH/thưởng của slot i (CHƯA KIỂ)
  elif id == 0: sub_70307F(-1, 0, -1)  // "bỏ người" của slot (CHƯA KIỂ)
  sub_9B5998(sub_8E9A6B(0))
  memset(this+4890, 0xFF, 0x10)    // 4×u32 top → = -1 (danh sách top rỗng)
  duyệt this[5413]: gọi vf8 (offset 32) từng elem[2]; nếu không vf8 nào trả ≠0 → vf160 (offset 620) của this
  duyệt this[5413]: gọi vf11 (offset 44); nếu ≠0 tạo dialog tiếp theo (sub_73E525 + sub_6E17DB)
  sub_7074E0(a, b, c, d) với giá trị từ bảng "ItemShield"/"ItemTeamshield" (÷30) — CHƯA KIỂ
---- phần 2 (trong On_146/sub_70DE4C sau sub_70A647) ----
  reset "sẵn sàng" của người chơi (sub_8E99A3 v.v.)
  nếu sub_9B6F75(4) → this[5400] = 1
```
- **Ý nghĩa (suy đoán mạnh)**: 146 = "kết thúc lượt": nhận số liệu (BXH từng slot bằng 3 cặp {u32,u16}), thông số lượt tiếp
  (round_len t1/t2/ms…), bật lại theo ready. Server giả lập cần phần u32/u16 đúng thứ tự. **CHƯA KIỂ runtime**
  (cần log để biết 146 có phải bắt buộc, và ý nghĩa 3 cặp {u32,u16} (có thể là rank/score/time)).
- **On_128** (`sub_71458E`, §5c): vẫn là bảng subtype 1..25 tài nguyên, không thấy subtype riêng "kết thúc ván":
  giữ nguyên như §5c.
- **On_204** (`sub_7071AC` 0x7071AC): `u16 a, u16 b, u8 c, u8 d, u32 e, u32 f, u8 g`; nếu (e == −1 && f == −1)
  → `sub_726107(b, c, d)`; rồi `sub_725E7F(f, e, c, d, a, b, 10, str, −1, g)`. dùng "mẫu hiệu ứng"
  tương tự 303/171 (`sub_725E7F` có vẻ cùng họ `sub_728F9E`) — có thể là hiệu ứng nổ/di chuyển; CHƯA KIỂ,
  không kết luận là kết thúc ván.

### 5e. Bổ sung opcode "sự kiện trong trận" (đọc từ decompile)
- **On_163** 0x716735: bắt đầu bằng 4 field chung, sau đó switch theo subtype:
  `u8 sub, u8 target, u16 idx, u16 arg`; theo `sub` 0..27: 0/3/13/14 → `sub_706BCB`/`sub_706B26`/`sub_71652D`/
  `sub_716692` (mỗi cái đọc thêm blob); 1/8/12 → `sub_70B8C4(...,1/0)`; 2 → `sub_7022EE` (u8,u8,u8 nibble);
  4 → `sub_70213E` (u32); 6 → `sub_702552` (4×u32); 7/9/10/15/16/19 → xử lý riêng từng kiểu; 21/22/23.. đọc thêm
  1× u8/u32/u16. Đây là **bộ event/hiệu ứng trong trận**. CHƯA KIỂ chi tiết từng subtype.
- **On_298** `sub_7035C4`: `u8 type(9/12), u16, u16` — type 9: sinh quái/wave (`u8 dir, u8, u8, u8, u8, u32`, sau đó
  đặt cell theo hàng, `sub_729794`, `sub_5C0138`); type 12: `u16 col, u16 row` + text báo lỗi `CreateBoxMon.destroybox
  colrow(%d,%d) failed`. CHƯA KIỂ ngữ nghĩa đầy đủ (nhưng mode đặc biệt/Zombie).
- **On_303** `sub_70727E`: `u8, u16, u16, u16` → `sub_728F9E(-1,-1,..., 93 hoặc 62, 10, ...)` (spawn box mon/npc).
- **On_160** 0x7164BB: phần đầu giống field `On_158` rồi gọi `sub_6F2964(u8, u16)` và tiếp tục trực tiếp **On_158(pkt, 1)**
  — biến thể OCSP item của 158.
- **On_161** 0x704962: `u8 id, u8 target, u8, u8, u8` → `sub_6F007D` rồi kiểm nếu là mình: gửi hiệu ứng `sub_89B623`;
  thất bại → `sub_70307F(target, 4, -1)`.
- **On_215** `sub_7129C4`: `u16 id, u8` → hộp thoại "Banned..." + `sub_708FFE(30/31, 0)` (bị đuổi khỏi trận).
- **On_216** `sub_70DDB4`: hộp thoại `aHordeeventMove` + ra khỏi trận `sub_708FFE(0, 0)` (CHƯA KIỂ).

### 5i. Opcode 571 (S→C, `sub_71ADEC` 0x71ADEC) — "gán lại bomber cho lượt" ✅ cấu trúc (CHƯA KIỂ runtime)
Gói blob (`DecodeBlob`); field đọc bằng `sub_A62F40` (u8), `sub_A62F70` (u16), `sub_A62FB0` (u32), đúng thứ tự:
```
opcode 571 (S→C):
  u8 kind                 // v31 — ==2: "tạo lại toàn bộ" (nhánh cuối dưới)
  u32 type                // → sub_8E78CB(type) (giống u32 type trong 501)
  u32 round_len           // → this[4887]
  u16 v32                 // (truyền cho sub_71AD24 bên dưới; nhiều khả năng = map id)
  u32 v33                 // (truyền cho sub_71AD24)
  u8 pcount               // → sub_8E9027(pcount)
  if Decode4 != 0:        // sub_A62F10
      u8 a, u8 b, u8 c, u8 d → sub_8E7A56(a, b, c); sub_8E7A65(d)
  // pcount vòng (tương tự phần 3 của 501):
  pcount × {
      u8 x                // đọc qua box (sub_4419FE + sub_414543) = slot
      u8 → sub_8E8EDD(slot, …)
      u8 → sub_8E8EFF(slot, …)
      u8 → sub_8E76A3(slot, x)
      u8 → sub_8E8EBB(slot, y)
      u8 p                // phụ (sub_679047(p) khi người sống)
      u8 q                // (bỏ)
      u32 v15             // → sub_67900E (nếu ≥0) khi người của slot còn
      u32 v17             // → sub_67901A
      u32 v40             // → sub_9B69F7(0 hoặc 1 nếu là "slot của đội mình", v40)
      if u8 != 0:         // có kèm blob con: sub_97EDBC → sub_8ED30A(slot, ...) + sub_474031
  }
  // dọn: nếu this[5395] hợp lệ → vf23/vf(96)/vf0(1) rồi this[5395] = 0; this[5396] vf(mk140); this[5561] tương tự
  if kind == 2:
      lặp i = 0..15: sub_8E9BF6(i)     // dựng mảng 16 slot như OnEnter
      sub_71AD24(v32, v42, v33)        // → C→S gửi 140 {bool 1} (và nội dung từ v32/v33)
```
- Ý nghĩa (suy đoán): gói "chuẩn bị lượt" đầy đủ hơn 189 mode 4 — server gửi sau khi kết lượt trước (146) để
  đặt lại vị trí/join người rồi client lại xác nhận **140**. ⇒ server SESSION nên có cặp: 146 → 571 → 140 C→S →
  189 (mode 2/4) → 140. Hoặc 571 thay cho 189 mode 2/4 tuỳ MODE. **CHƯA KIỂ runtime**.

### TODO phân tích tiếp (cập nhật)
- (b) đã chốt cấu trúc **146** (§5h); cần log runtime để confirm và hiểu 3 cặp {u32,u16}; subtype On_128 chưa gỡ.
- (c) thứ tự bắt đầu lượt: server nhận 139/140 rồi đáp mode 2/4 của 189; thêm suy đoán: **146 → 571 (§5i)
  → 140 C→S → 189**; cả hai CHƯA KIỂ runtime.
- (d) ý nghĩa từng field phần 3 (c/d/e) của 501 vẫn CHƯA KIỂ. Bổ sung: các setter nằm trong nhóm "ghi record"
  giá trị hộp toàn cục — u32 c (sub_67900E → sub_44AD23), u32 d (sub_67901A → sub_413F32), u32 e
  (sub_9B69F7: **chỉ nhận khi ≤ 100** — nhiều khả năng tỉ lệ %), và cùng bộ như phần giữa gói 571.

- (a) **ĐÃ GIẢI**: không có handler 141/142 S→C (và cả 188/197 chỉ qua strategy, xem §5f); Nếu làm server nhiều người thật, nên bắt log kiểm xem server có gửi 176 (định vị lại player theo slot)
  giữa trận không.
