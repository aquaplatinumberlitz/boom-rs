> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Tiến độ & handoff (cập nhật 2026-10-18, cuối ngày)

File này dành cho **người mới vào dự án**: đọc từ trên xuống là biết đang ở đâu, chạy thử thế nào, việc gì còn dở.
Chi tiết kỹ thuật từng bước nằm ở các file khác (link ở từng mục).

---

## 1. Tóm tắt 1 phút

- Game: **Boom Online** (VNG, bản Việt của *Crazy Arcade / BnB*). Server gốc đã chết; ta dịch ngược client để viết
  **server giả lập**: `tools/fakeserver.py` (Python, đặc tả sống) và **server Rust** `server/` (mục 12) — dùng Rust cho chạy thử.
- Client gốc bị pack Themida. Ta dùng **bản dump đã unpack + sửa** (`ca_fixed_patched.exe`) chạy trong **Wine**.
- **Chạy được:** đăng nhập → chọn kênh → sảnh → tạo phòng → phòng chờ → bắt đầu trận → **vào trận (stage 13): vòng mô phỏng chạy, nhân vật
  đi được, đặt bom, client gửi C→S 111 (bản ghi đặt bom)**. Chuỗi server tối thiểu: 501 → 189 mode 1 → 190 → 189 mode 0 → **191** (mục 3.3,
  `POST_PLAY_START`; lý do: `oc_e-game.md` §5a-5). **Bom nổ được** khi server trả 152 (gán id) rồi 151 (nổ) — `oc_e-game.md` §5a-5b, `docs/evidence/images/vn/battle_bomb_explode.png`.
- **Khối VM Themida:** `tools/devirt.py` có **15 khối** (viết lại hoặc skip có căn cứ) — bảng trạng thái từng khối ở
  [docs/research/findings/VN_DEVIRT_STATUS.md](../../research/findings/VN_DEVIRT_STATUS.md#bảng-trạng-thái-khối-vm-nguồn-sự-thật-duy-nhất). Khối đặt bom `0x6EFD4A` và handler gói 552 đã viết lại bằng
  **emulate trên snapshot cũ** (không cần đưa client gốc vào trận). Còn dở: `0x736C27` (nạp map, đang skip), vùng anti-cheat A1xxxx (skip khi chạm).
- Song song có công cụ devirt tổng quát `tools/vmdevirt.py` (bảng handler/ctx/opcode, chưa emit được C) — [docs/research/findings/VM_METHOD.md](../../research/findings/VM_METHOD.md);
  hướng dẫn cho người mới: [docs/research/findings/DEVIRT_METHOD.md](../../research/findings/DEVIRT_METHOD.md).

---

## 2. Bản đồ tiến độ

Game chia thành các "stage" (màn). Client ghi stage hiện tại vào `wine/Boom/CALog.log` dạng `Sg - INT(dne:N)`.

| Stage | Màn | Trạng thái | Ảnh minh hoạ |
|---|---|---|---|
| 1 | Khởi động, intro | ✅ | — |
| 10 | Đăng nhập + chọn kênh (`CSelectServerStage`) | ✅ kênh tự hiện, click vào kênh được | `docs/evidence/images/vn/channel_autoshow.png` |
| 11 | Sảnh quảng trường (`CLobbySquareStage`) | ✅ load xong, nút TẠO PHÒNG chạy | `docs/evidence/images/vn/lobby_loaded.png`, `docs/evidence/images/vn/create_room_dialog.png` |
| 12 | Phòng chờ (`CPrepareStage`) | ✅ có người chơi (trưởng phòng), map Village01, nút BẮT ĐẦU bật | `docs/evidence/images/vn/waiting_room_start.png`, `docs/evidence/images/vn/map_dialog_village.png` |
| 13 | Trận đấu (`CGameStage`) | 🟡 mô phỏng chạy (cần 191 sau 189 mode 0), nhân vật **đi được**, đặt bom → 111 → 152 → 151 → **bom nổ**, bị nhốt/chết (C→S 120) → 146 lượt mới hoặc **173 về phòng chờ**; chưa nhiều người, chưa vật phẩm | `docs/evidence/images/vn/battle_bomb_explode.png` | `docs/evidence/images/vn/battle_phase3.png`, `docs/evidence/images/vn/battle_turn_LRUD.png` |

Những phần **chưa làm** ở các màn đã qua (không chặn đường vào trận):
- Hộp thoại **chọn map** trong phòng liệt kê 0 map (báo "CantDecideMap"). Server đang tự gán map cho phòng.
- Server chỉ khai báo **3 map** (Village01, Forest01, Camp01) — chưa có danh sách đầy đủ, chưa rõ map có khoá theo cấp
  nhân vật không.
- Danh sách người trong sảnh, danh sách phòng: đang gửi rỗng.
- Nhiều người chơi, đội, nhân vật, sẵn sàng: chưa làm (mới 1 người chơi).

---

## 3. Chạy thử từ đầu

### 3.1 Cần có trên máy (không nằm trong git)
Xem bảng "File không có trong git" trong [README](../../../README.md#file-không-có-trong-git-nặng--bản-quyền--đang-nằm-trên-máy).
Tối thiểu: `unpacked_ca.exe`, thư mục `wine/Boom/` (bản copy client), Wine, `Xvfb`, `xdotool`, `import` (ImageMagick), `uv`.

### 3.2 Build bản client chạy được
```sh
cd ~/boom-re
tools/rebuild_wine.sh          # unpacked_ca.exe → fix IAT → devirt → patch → wine/Boom/ca_fixed_patched.exe
```
Phải chạy lại mỗi khi sửa `tools/devirt.py` hoặc `tools/patch_client.py`.

### 3.3 Chạy server giả
```sh
python3 tools/fakeserver.py --log logs/fakeserver_host.log &
```
Biến môi trường (đặt trước lệnh):

| Biến | Mặc định | Ý nghĩa |
|---|---|---|
| `ROOM_TYPE` | 19 | Loại phòng trong gói tạo phòng (83). Loại 19 cho **1 người** bắt đầu được |
| `MAP_INDEX` | 0 | Map của phòng = chỉ số trong `BATTLE_MAPS` (0 Village01, 1 Forest01, 2 Camp01) |
| `AUTO_START` | 0 | > 0: server tự gửi lệnh bắt đầu trận (501) sau N giây kể từ lúc vào phòng chờ |
| `TURN_START` / `TURN_MODE` | 0 / 4 | > 0: gửi 189 (mode `TURN_MODE`) N giây sau 501. **Dùng `TURN_MODE=1`** (mode 1 start đồng hồ trận) |
| `GO_START` | 0 | > 0: gửi 190 "GO" N giây sau 189 |
| `PLAY_START` | 0 | > 0: gửi 189 **mode 0** N giây sau 190 → phase 3, đồng hồ chạy (oc_e-game §5a-3) |
| `BEGIN_START` / `BEGIN_OP` | 0 / 191 | > 0: gửi opcode `BEGIN_OP` (191 round begin, 146 kết quả, **552** gói của `sub_71580C`) N giây sau 190 |
| `POST_PLAY_START` | 0 | > 0: gửi **191** N giây sau 189 mode 0 — start đồng hồ context 0 ⇒ **vòng mô phỏng chạy** (bắt buộc để đi/đặt bom thật) |
| `ITEMS` / `ITEM_ACK` | — / 0 | `ITEMS="x,y,kind;…"`: gửi **553** (vật phẩm trên map) trước 501; `ITEM_ACK=1` (fakeserver; Rust luôn bật): trả **158** cho mỗi C→S 117 (nhặt vật phẩm) |
| `MATCH_END` | 0 (Rust: 3) | > 0: N giây sau C→S 120/13 (chết) gửi **173** → về phòng chờ (ưu tiên hơn `DEATH_END`) |
| `DEATH_END` | 0 | (fakeserver) > 0: N giây sau C→S 120 subtype 13 (chết) gửi **146** → lượt mới |
| `BOMB_PLACE` / `BOMB_ECHO` | 0 / 0 | (fakeserver) `BOMB_PLACE=1`: trả **152** cho mỗi C→S 111 (gán id); `BOMB_ECHO=N`: N giây sau gửi **151** nổ (id 0..`BOMB_IDS`-1). Server Rust: `BOMB_FUSE` (mặc định 2.5) làm cả hai |
| `SERVER_IP` | 127.0.0.1 | (**chỉ server Rust**) địa chỉ client được dặn kết nối ở gói 7 và 15; client trong WinBoat cần `172.18.0.50` |

Chuỗi vào trận chơi được: `AUTO_START=5 TURN_START=10 TURN_MODE=1 GO_START=3 PLAY_START=4 POST_PLAY_START=2`.

Log server (mọi gói vào/ra, đã giải mã) ở `logs/fakeserver_host.log`.

### 3.4 Chạy client tự động (Xvfb, không cần màn hình)
```sh
tools/xvfb_join.sh                       # đăng nhập (tester/1234) → chờ màn kênh → click "Kenh 1" → vào sảnh
# vào sảnh → đóng hướng dẫn → TẠO PHÒNG → XÁC NHẬN → phòng chờ:
CLICKS="975,38,3 385,50,4 437,612,10 512,384,8" tools/xvfb_join.sh
```
- `CLICKS="x,y,giây ..."`: sau khi vào sảnh, click lần lượt từng toạ độ rồi chờ số giây; mỗi click chụp `logs/xvfb/click_<i>.png`.
- Kết quả đúng: `wine/Boom/CALog.log` có lần lượt `dne:10`, `dne:11`, `dne:12`.
- Vào trận + bấm phím + gdb tự động: `tools/match_gdb.sh <file.gdb> <outdir>` (đăng nhập → phòng → 501/189/190/189 → mũi tên + Space×3; ảnh `match_*.png`).
  Ví dụ: `GREP='ARGS|EXIT|REC|CRASH' TURN_MODE=1 PLAY_START=4 tools/match_gdb.sh tools/gdb/msgcache_rec.gdb logs/xvfb/x`.

### 3.4b Client gốc trong WinBoat (chỉ để chụp snapshot VM)
Thư mục `win/`, `tools/battle_drive.ps1`, `tools/snap_retry.sh` — bị Themida giết khi code bị patch, chi tiết và bẫy: [docs/research/findings/VN_RUNTIME_REFERENCE.md](../../research/findings/VN_RUNTIME_REFERENCE.md#client-gốc-trong-winboat-2026-10-18). Không cần cho đường đi chính (emulate trên snapshot cũ đủ cho các khối đã làm).

### 3.5 Debug (gdb)
Các file lệnh gdb trong `tools/gdb/`: `input_gates`/`input_dispatch` (đường phím), `phase_clock` (phase + đồng hồ), `msgcache_*` (khối 0x6EFD4A: vào/ra, bản ghi), `h552` (handler 552), `init9682` (Init(index)), `dump_stage` (dump `CGameStage` để emulate).
`tools/wine_gdb.sh <file_lệnh_gdb>` chạy client dưới `winedbg --gdb` + gdb, tự đăng nhập; `JOIN=1` để click vào kênh,
`CLICKS=` như trên. Kết quả ở `logs/xvfb/gdb.txt`. Dưới gdb mọi thứ chậm ~3 lần. Chi tiết:
[docs/research/findings/VN_RUNTIME_REFERENCE.md](../../research/findings/VN_RUNTIME_REFERENCE.md), memory `reference-winedbg-gdb`.

---

## 4. Luồng gói tin đã làm (theo thứ tự xảy ra)

"C→S" = client gửi lên, "S→C" = server trả về. Hàm tạo gói nằm trong `tools/fakeserver.py`.
Định dạng khung, mã hoá, cách đọc số/chuỗi: [docs/specification/networking/LEGACY_VN_PROTOCOL.md](../../specification/networking/LEGACY_VN_PROTOCOL.md).

| Bước | C→S | S→C (hàm trong fakeserver) | Ghi chú |
|---|---|---|---|
| Kết nối | — | hello (`hello_packet`) | version 116 |
| Chào | 0 | 13, 2, 12, 21, 1, 43 (`login_replies`) | mỗi gói `u8 0` |
| Đăng nhập | 1 | 4 (`login_ok`), 15 (`login_net_info`) | id người chơi = 1000+i, phải khớp gói 103 |
| Vào màn kênh | 40 | 14 (`channel_list`), 16 (`channel_status`) | ô kênh cần kind=18; số người tối đa là u32 |
| Chọn kênh | 5 | 7 (`channel_join_ok`) | trỏ client tới "server kênh" (chính ta) |
| Đăng nhập kênh | 6 | 17 (`channel_enter_ok`) | → stage 11 |
| Vào sảnh | 395 | 466 (`square_info`), 467 (`square_users`), 88 (`room_list`), 234 (`map_checksums`), 231 (`dyn_map_info`) | 88 bắt buộc, thiếu thì nút TẠO PHÒNG không chạy |
| Tạo phòng | 77 | 83 (`create_room_ok`) | → stage 12 |
| Vào phòng chờ | 337 | 103 (`room_player`), 106 (`room_slot_state`), 113 (`room_map`) | trưởng phòng, ô 0, map |
| Bấm BẮT ĐẦU | 97 | 501 (`match_start`) | hoặc server tự gửi 501 (`AUTO_START`) |
| Bắt đầu lượt (server tự) | — | 189 (`round_start`, mode 1/2/4 — `TURN_MODE`) | mode 1 → client trả C→S 139; mode 2 → 140; mode 4 có cổng, sai tham số bị bỏ qua âm thầm |
| "GO" (server tự) | — | 190 (`round_go`, sau 189 — `GO_START`) | client set phase 2 (pause đồng hồ), overlay "THỜI GIAN" |
| Bắt đầu chơi (server tự) | — | 189 mode 0 (`round_start(mode=0)`, `PLAY_START`) | phase 3, đồng hồ chạy tiếp; cần đồng hồ đã start (189 mode 1) |

Chi tiết + địa chỉ hàm client của từng bước: [docs/archive/re/VN_LOBBY_LOG.md](VN_LOBBY_LOG.md) (mục "Cập nhật 5" → "Cập nhật 12");
[docs/archive/re/OC_E-GAME.md](OC_E-GAME.md) §5a/§5a-2 (189/190, đã decompile đủ + kiểm runtime).

---

## 5. Tiến độ vào trận (2026-10-18) — LỊCH SỬ (trạng thái hiện tại ở mục 1 và `docs/research/findings/VN_DEVIRT_STATUS.md`)

> Mục này giữ nhật ký các bước đã làm; các giả thuyết "khối VM do chủ dự án tự xử lý" đã được thay bằng kết quả ở `docs/research/findings/VN_DEVIRT_STATUS.md`.

**Cập nhật 2026-10-18 (Claude Opus 5.5, bước 1 "lần ngược từ triệu chứng" bằng gdb) — chi tiết `oc_e-game.md` §5a-3:**
- **Giả thuyết 1 đúng** (thiếu gói), không phải do skip 0x736C27: phím ←↑→↓/Space đi hết đường input tới controller
  bomber (`sub_708CC8` → `CGameContext` vf8 `sub_6E5E05`), mọi cổng đều qua. Thứ đứng im là **mô phỏng**: đồng hồ trận
  `CGameClock` chưa bao giờ được start (`T[2]=0`) nên tick `vf46` chỉ chạy vòng chờ vf161.
- Đồng hồ start bởi `sub_6E1AF7` ← `sub_6E2054` ← **189 mode 1** (hoặc mode 4 nhánh B, 191…); 190 pause; **189 mode 0**
  (phase 2 → 3) resume. Fakeserver thêm `PLAY_START`. Chuỗi `TURN_MODE=1 … PLAY_START=4` → nhân vật hiện ở spawn và
  quay mặt theo phím (`docs/evidence/images/vn/battle_turn_LRUD.png`); không đi được vì spawn bị chặn — phải đặt bom.
- **Kẹt mới:** Space (đặt bom) → crash trong VM `sub_6EFD24` (site 0x6EFD4A, gọi từ `CGOCAHeroGameBomber__vf6_2`
  0x5B1FB5). Khối VM đã viết lại sau đó (2026-10-18, `docs/archive/re/VM_6EFD4A_INPUTS.md` §8).
- Công cụ: `tools/match_gdb.sh <gdb_file> <outdir>` (full flow + bấm phím) với `tools/gdb/{input_gates,input_dispatch,
  phase_clock}.gdb`.

### Ghi chép 2026-10-17 — đã qua nạp map, còn kẹt điều khiển

Được giao xử lý trước đó ("viết lại khối 0x736C27"). Kết quả:

- Client ổn định qua **điểm crash cũ** (nạp map): khối VM `CGamePlayMap__Load` 0x736C27 tạm đi qua bằng skip
  `jmp 0x736D6C` trong `tools/devirt.py` (thí nghiệm; viết lại thật vẫn cần snapshot + emulate).
- Crash kế tiếp nằm ở **`CBombOwner__vf33`**: site **0x5788C9** (không có trong `themida_entries.json`) — 1 lệnh
  `jmp sub_414543` nguyên bản bị Themida đè bằng `jmp 0x26574543` (ngoài image, SIGSEGV khi chạy). Soi hàm song sinh
  `CBombOwner__vf31` (0x578963, nguyên bản) xác byte gốc → skip `jmp 0x414543`. Bằng chứng gdb + chi tiết:
  `docs/archive/re/VM_TRIAGE_HISTORY.md` §3a. Bằng skip này client đi được hết OnEnter (query bomber theo slot của vf161).
- **Cập nhật (2026-10-17, sau khi chủ dự án tái-build + chạy lại):** run mode 4 trước đó của agent **CÓ trả 140**
  ngay (89 ms) — khẳng định "mode 4 bị cổng từ chối âm thầm" trong commit b60a125 là **sai** (grep nhầm cửa sổ +
  đọc nhầm cấu trúc nhánh). Đã sửa ở §5a. Hai nhánh mode 4 đều có thể trả 140; nhánh B mới có đường hủy.
- **Sau đó: stage 13 nạp map xong, vẽ Village01 + HUD, client sống ổn** (`docs/evidence/images/vn/battle_village01.png`).
- Fakeserver đã gửi được chuỗi trong trận: 501 → **189** (mode 1/2/4, decompile đủ §5a `oc_e-game.md`;
  mode 1 → client trả 139 ✓, mode 2 → trả 140 ✓, mode 4 → trả 140 ✓) → **190 "GO"** (§5a-2; phase 2,
  overlay THỜI GIAN).
- Kẹt hiện tại: **nhân vật không di chuyển** sau chuỗi trên. Input tới game (đăng nhập gõ được).
  **Giữ cả hai giả thuyết** (chưa loại được cái nào):
  1. Thiếu gói/state tiếp theo (356/176 theo §5g, hoặc gói khác);
  2. **Skip 0x736C27 bỏ ~0x136 byte code gốc** của `CGamePlayMap__Load` chưa biết làm gì → môi trường trạng thái
     mà các phần sau coi là "đã có sẵn" không tồn tại. (Skip 0x5788C9 an toàn — khối đúng 1 lệnh jmp, cont khớp
     song sinh, không bỏ sót gì.)
- Việc phân định 2 giả thuyết: snapshot + emulate `0x736C27` (viết lại thật) là đường chắc chắn nhất — nếu khối
  chỉ làm việc nội bộ map-list thì nằm ở giả thuyết (1), và dò tiếp gói.

Ghi chép trước đó về phần crash map: [docs/archive/re/VN_LOBBY_LOG.md](VN_LOBBY_LOG.md) "Cập nhật 12".

---

## 6. Việc tiếp theo (theo thứ tự ưu tiên)

1. ~~Gói C→S 111~~ ✅, ~~bom nổ~~ ✅ (2026-10-18 đêm): 111 → **152** (gán id) → **151** (nổ) — `oc_e-game.md` §5a-5/5a-5b. Tiếp trong trận:
   ~~chết → lượt mới~~ ✅ (client báo C→S 120 subtype 11 bị nhốt / 13 chết; server 146 → lượt mới, hồi sinh — `oc_e-game.md` §5a-5c). ~~Kết thúc trận → phòng chờ~~ ✅ (S→C **173**). Tiếp:
   vật phẩm rơi từ gạch, nhiều người chơi (relay 111/152/151/120 cho mọi client), thời gian nổ thật.
2. ~~Dump `CGamePlayMap`~~ ✅: emulate với object thật — khối `0x736C27` là vòng lặp 0 lần với Village01/phòng 19 (skip tương đương); thử thêm map/loại phòng 20/33 nếu cần.
3. **Vùng anti-cheat A1xxxx** (43 site, skip khi chạm) và các khối còn lại: `0x966FEC` (mồ côi).
4. **Mở đủ map**: thay 3 map tự soạn bằng danh sách 1019 map (server Rust đã có `ALL_MAPS=1`); làm hộp thoại chọn map liệt kê được; xem map có khoá theo cấp không.
5. **Audit tài liệu của agent OpenCode** trước khi dùng (mục 7); kiểm lại `sub_9682DE`/các suy luận cũ nếu cần.
6. **Sảnh/phòng đầy đủ**: danh sách người, danh sách phòng, nhiều người chơi, gói trong trận (di chuyển/bom/kết thúc lượt: `docs/archive/re/OC_E-GAME.md`, `docs/archive/re/OC_C-SQUARE.md`, `docs/archive/re/OC_A-ROOM.md`).
7. **Server Rust hoàn chỉnh** (`server/`, mục 12) dùng `tools/fakeserver.py` + `docs/` làm đặc tả.
8. (Tuỳ chọn) Devirt tổng quát `tools/vmdevirt.py` → emit C (`docs/research/findings/VM_METHOD.md` §8: còn chặn ở ngữ nghĩa 46 opcode + key rolling); chưa cần cho đường đi hiện tại vì
   cách emulate-trên-snapshot đã đủ cho các khối game.

## 7. Tài liệu do agent OpenCode viết

Agent OpenCode (model `opencode/glm-5.3-flash-free`) phân tích tĩnh bằng IDA, mỗi agent một file. Đã merge vào `main`.
**Agent hay sai offset/định dạng — phần chưa audit phải đối chiếu IDA trước khi đưa vào server.**

| File | Nội dung | Audit |
|---|---|---|
| `docs/archive/re/OC_D-LOGIN.md` | gói đăng nhập 4 + 15 từng field | ✅ đã kiểm, đã dùng |
| `docs/archive/re/OC_A-ROOM.md` | phòng chờ: 87/97, 94, 501 | 🟡 mới kiểm định dạng 501 |
| `docs/archive/re/OC_C-SQUARE.md` | sảnh quảng trường: 466–481, 492, 613–625… | ⬜ chưa kiểm |
| `docs/archive/re/OC_E-GAME.md` | màn trận: hàm vào màn, bảng gói, 189/356/146/571, server tối thiểu | 🟡 mới kiểm đường gọi vào màn (khớp backtrace thật) |

Chạy lại agent: `~/boom-re-oc/launch.sh <task>` (mỗi task một worktree `~/boom-re-oc/<task>`, branch `oc/<task>`, đề bài ở
`~/boom-re-oc/prompts/`). Dừng tất cả: `pkill -f '[o]pencode run --standalone'`.
Bài học: dặn agent **ghi file dần và commit sau mỗi 2–3 mục**, nếu không nó dễ chết vì hết token lúc tóm tắt.

---

## 8. Sửa client đang áp dụng

Tất cả nằm trong pipeline `tools/rebuild_wine.sh`, không sửa tay file exe.

| Sửa gì | Ở đâu | Vì sao |
|---|---|---|
| Sửa IAT | `tools/fix_iat.py` | bản dump không có bảng import đúng |
| Thay khối VM bằng code viết tay (`BLOCKS`, 15 khối) | `tools/devirt.py` | VM Themida không chạy được trong bản dump; trạng thái từng khối + quy trình: [docs/research/findings/VN_DEVIRT_STATUS.md](../../research/findings/VN_DEVIRT_STATUS.md) |
| Hook: kênh tự hiện (`HOOKS`, sau `On_16`) | `tools/devirt.py` | gốc phải click mới vẽ lại ô kênh |
| Tắt HackShield, bỏ kiểm patch, đổi IP server | `tools/patch_client.py` | chạy với server giả |
| Khôi phục 10 byte hỏng ở 0x592C09 | `tools/patch_client.py` | bản dump hỏng ở đây → crash khi nạp map sảnh |

---

## 9. Bẫy thường gặp

- `pkill -f ca_fixed_patched` có thể **giết luôn shell** đang chạy lệnh (chuỗi khớp chính nó) → dùng `'[c]a_fixed_patched'`.
  **Dạng bẫy khác (2026-10-17):** cả heredoc/giá trị env chứa chuỗi khớp cũng dính — ví dụ ghi file bằng
  `cat > x <<'EOF' ... pkill -f ca_fixed ... EOF` thì chính lệnh shell ngoài khớp. Nghiêm chút: **ghi file bằng công
  cụ write/edit, chạy file riêng**, không nhúng nội dung pkill vào cmdline.
- Còn tiến trình client cũ thì client mới không chạy (mutex) → các script đã tự pkill trước.
- `/proc/<pid>/mem` không đọc được trong Wine kiểu WoW64 mới → dùng gdb (`tools/wine_gdb.sh`).
- Số trong pseudocode Hex-Rays là **thập phân**.
- Sự kiện bấm nút **không** đi qua `CStage::vf5`, mà qua `CCtrlButton__vf14` → hàm xử lý của cửa sổ cha (`docs/archive/re/VN_LOBBY_LOG.md` "Cập nhật 8").
- Số đọc từ gói bị XOR theo kích thước (u8 0x5A, u16 0xA569, u32 0x96CA5395, u64 0x71B18589) — `obf_body()` lo phần này.
- Commit cũ có trailer "Claude Opus 4.8" có vài lỗi offset đã sửa: xem "Errata audit" trong `docs/archive/re/VN_LOBBY_LOG.md`.

---

## 10. Từ điển nhanh

| Từ | Nghĩa |
|---|---|
| opcode | số đầu gói tin, xác định loại gói; hàm xử lý phía client tên `On_<opcode>` |
| stage | màn chơi (10 kênh, 11 sảnh, 12 phòng chờ, 13 trận) |
| bản dump | `unpacked_ca.exe`: client đã gỡ Themida, chạy được sau khi qua `rebuild_wine.sh` |
| khối VM | đoạn code Themida biến thành bytecode riêng; trong bản dump không chạy được, phải viết lại (`devirt.py`) |
| CALog | `wine/Boom/CALog.log`, log của client; `dne:N` = đã vào stage N |

---

## 11. Nhật ký phiên 2026-10-17 (agent OpenCode — qua nạp map tới ván)

### Chuỗi công việc

1. **Skip thí nghiệm 0x736C27** (`CGamePlayMap__Load`) trong `tools/devirt.py` BLOCKS → build lại + chạy:
   điểm crash cũ (nạp map) qua được, nhưng crash **kế tiếp** ở OnEnter, backtrace trong `logs/xvfb/gdb.txt`
   (EIP 0x26574543 — heap "rác").
2. **Định danh crash thật bằng gdb + static** (bẫy "object rác" bị bác bỏ):
   - `HIT6FFCA9` (bp trước lời gọi ảo trong `sub_6FFC94`): `[stage]` = `vt_CGameStage` **lành** → object stage OK.
   - Crash EIP = **0x26574543**, stack: `CHiddenCatchGameStage__vf161` (0x7027B0, loop query bomber theo slot
     `sub_6E5C2B`) → wrapper `CGOCAHeroGameBomber__vf33_0` (0x5A0274; `[esi]=vt_CGOZzolmanBomber_off510` — bomber
     **lành**) → `CBombOwner__vf33` (0x5788B8) → hết thân, nhảy 0x26574543.
   - Hex-Rays của `CBombOwner__vf33` có sẵn `JUMPOUT(0x26574543)` — **khớp chính xác địa chỉ crash** → là site VM,
     không phải dữ liệu hỏng.
3. **Site 0x5788C9 không có trong `themida_entries.json`**: disasm cho thấy lệnh nguyên bản tại đó (so với hàm song
   sinh `CBombOwner__vf31` 0x578963 *không bị đè*: `...; lea ecx,[esi+0x1C]; pop esi; jmp sub_414543`) là
   `jmp sub_414543` (5 byte) **bị Themida đè bằng** `jmp 0x26574543`. Skip = `jmp 0x414543` (giữ đúng lệnh gốc,
   bỏ qua mutator-in-heap). Chi tiết trong `docs/archive/re/VM_TRIAGE_HISTORY.md` §3a/§5.
4. **Kết quả chạy thật**: LOGIN → kênh → sảnh → phòng → AUTO_START 501 → **dne:13, nạp map xong, vẽ Village01 + HUD,
   client sống ổn** (`docs/evidence/images/vn/battle_village01.png`). Nhân vật đứng ở spawn không di chuyển.
5. **Fakeserver phần trong trận** (`tools/fakeserver.py`):
   - Builders mới: `round_start()` (opcode 189, mode 1/2/4 — field từ decompile đủ, `oc_e-game.md` §5a) và
     `round_go()` (opcode **190 "GO"** = `sub_70E7D6`, set phase `this[5402]=2`; field ở §5a-2).
   - Env: `TURN_START` (giây sau 501), `TURN_MODE`, `TURN_A2/3/4` (giá trị mode 2), `GO_START` (giây sau 189).
   - Thí nghiệm: mode 1 → C→S **139 ✓** (nạp lại round, nhân vật về spawn); mode 2 (1,1,12345) → C→S **140 ✓**
     (screen đen = loading ~5 s, map mới); mode 4 (1,1,1) → C→S **140 ✓ sau 89 ms** — lúc làm chắc chắn rằng
     "bị cổng từ chối" là **sai** (grep `tail -4` bị keep-alive che; đọc nhầm cấu trúc nhánh — hai nhánh mode 4
     đều gửi 140 ở đuôi dùng chung, chỉ nhánh B mới có đường hủy `sub_708FFE(5,0)`). Đã sửa trong `oc_e-game.md` §5a.
     190 Go → overlay "THỜI GIAN" hiện (phase 2 set ✓).
6. **Vẫn kẹt**: nhân vật không di chuyển; HUD timer 00:00/02:00 đứng. Input tới game (đăng nhập gõ được bằng bàn
   phím) → vấn đề là **state client**, không phải infra.

### Bài học / kinh nghiệm

- Crash "EIP rác" trong bản dump **không có nghĩa là object hỏng**: kiểm `[esi]`/vtable trước khi kết luận. Ở đây
  `Hex-Rays JUMPOUT(địa chỉ crash)` luôn là dấu hiệu site VM — grep nó đầu tiên.
- Site VM có thể sống **ngay trong 1 lệnh jmp ngắn** của hàm nguyên bản (mẫu "jmp bị chiếm đoạt"); đặc điểm nhận:
  Hex-Rays JUMPOUT ra ngoài image. Sửa nhanh + đúng: tìm hàm song sinh nguyên bản có cùng đuôi để biết byte gốc.
- Bp gdb dạng script tuần tự (`continue` → inspect → `continue` → bt+quit) đủ dùng khi biết bp hit đúng 1 lần
  trước crash — thông tin `[stage]`, `[esi]` lấy được gọn.
- **grep opcode trong log fakeserver phải dùng pattern cụ thể** ("C->S opcode 140"), không dùng `tail -N` của mọi
  "C->S opcode" — keep-alive (op 15) mỗi 5 s dễ đẩy phản hồi cần tìm ra khỏi cửa sổ (đây là lý do kết luận sai
  "mode 4 im lặng" — đã đính chính 2026-10-17).
- gdb chậm ~2-3× so với driver click tương tác của `wine_gdb.sh` (JOIN/CLICKS dùng thời gian tĩnh) — chờ lâu hơn.
- Luồng click trước đó dính typo tọa độ (344,611 thay 434,611) — luồng "sàn sạch" phải xem ảnh sau từng click.
- Mode 189/190: tất cả giá trị "chưa rõ" dễ gửi 0/1 — mode 2 chạy được với (1,1,12345) nhưng **cần audit** giá trị
  a4/a5/a6 thật (`sub_70A385`) trước khi tin.

### Trạng thái để phiên sau bắt lại

- `tools/devirt.py`: 9 BLOCKS (2 skip thí nghiệm mới: 0x736C27→0x736D6C, 0x5788C9→0x414543) — lần build kế tiếp nên
  giữ và ghi chú như hiện tại.
- `tools/fakeserver.py`: AUTO_START/TURN_*/GO_START hoạt động; đã test với ROOM_TYPE=19, MAP_INDEX=0.
- Ảnh màn trận: `docs/evidence/images/vn/battle_village01.png` (map + HUD), `docs/evidence/images/vn/battle_mode1_spawn.png` (spawn sau mode 1).
- Log phiên: `logs/xvfb/gdb.txt` (bp HIT6FFCA9 + crash), `logs/fakeserver_host.log` (189/190/139/140), ảnh
  `/tmp/opencode/st_*.png` (tạm thời — copy sang docs nếu cần thêm).

---

## 12. Server Rust (`server/`, 2026-10-18)

Khung server thật, port 1:1 từ `tools/fakeserver.py` (đặc tả vẫn là fakeserver + `docs/`).
- `src/proto.rs` codec (CRC32/CRC8, XOR payload, int/chuỗi/blob obf, khung, bảng thay opcode `decomp/net/*.bin`),
  `src/packets.rs` builder S→C (login → kênh → sảnh → phòng → 501/189/190/191/146), `src/maps.rs` bảng 231/234 +
  đọc `fx.idx` (`ALL_MAPS=1`), `src/session.rs` luồng phiên (tokio; cùng biến môi trường với fakeserver), `src/main.rs`.
- Kiểm chứng: `cargo test` — **mọi builder và cả hai chiều đóng khung khớp từng byte với fakeserver** (`tests/golden.txt`
  sinh bởi `python3 tools/gen_golden.py > server/tests/golden.txt`; sinh lại khi sửa fakeserver), danh sách 1019 map khớp.
- Chạy: `cd server && cargo run --release -- --log ../logs/server.log` (env như mục 3.3). **Đã chạy với client gốc trong WinBoat**
  (2026-10-18, `SERVER_IP=172.18.0.50`) tới phòng chờ; và **với bản dump trong Wine trọn một trận** (`RUST=1 tools/match_play.sh …`, `logs/xvfb/rust_boom`):
  đăng nhập → phòng → 501/189/190/189/191 → đi lại → đặt bom → 152 → 151 → bom nổ (`BOMB_FUSE`, mặc định 2.5 s).
- Việc tiếp: parse gói C→S có cấu trúc (77, 97, 139/140, 111/112/145 của `CGameMsgCache`), nhiều người chơi/phòng,
  relay gói trận theo khoá phiên (`oc_e-game.md` §5a-4).

