# Server giả lập VN (`boom-re/server`) — phần tận dụng được cho engine mới

Khảo sát 2026-10-19 trên `/home/admin/boom-re` main `4ccc890` (`server/src/*.rs`, 2,5k dòng Rust; test
`tests/golden.rs`, `tests/bomb.rs`) cùng `docs/oc_e-game.md`. Theo chỉ đạo người dùng, server này là
nguồn tin cậy cho những gì nó học được từ **client VN** (ADR 0002). Nó không được dựng từ server gốc
(đã thất lạc) mà được viết để client chạy được, nên cần tách hai loại thông tin:

- **Client-derived** (`vn_static`/`vn_runtime`, confirmed): định dạng gói, trường dữ liệu, client tự làm gì,
  chờ server làm gì. Đã kiểm byte-for-byte với fakeserver và chạy runtime 1–2 client.
- **Lựa chọn của emulator** (`emu_choice`, candidate): những giá trị/chính sách tác giả tự đặt cho client chạy
  được (fuse, xếp hạng, thời gian chờ). Dùng làm điểm xuất phát `design`, không phải luật gốc.

## 1. Client-derived — dùng trực tiếp

| Chủ đề | Điều học được | Dùng cho engine mới | Locator |
|---|---|---|---|
| Vòng đời lượt | 553 (item) → 501/117 (vào trận) → 189 nạp lượt → 190 "GO" (đồng hồ dừng) → 191/146 bắt đầu (đồng hồ chạy) → 189 mode 0 chơi → 168 kết quả → 173 về phòng | Pha lượt: Loading → GO → Playing → Result → Room; khớp loading barrier/cửa sổ kết quả của SCOPE_V1 §3 | `session.rs` `match_flow`; oc_e §5a-3 |
| Hết giờ | HUD đếm 120 s; client **không gửi gì** khi về 0, server phải kết thúc | R-010; kết thúc lượt là việc của authority | oc_e "Hết giờ" |
| Trạng thái slot phòng | 106: bit0 đóng, bit1 chủ phòng, bit2–4 trạng thái (1 = ready), 6 bit nhân vật, 5 bit đội; client có 16 slot | DTO phòng: slot cố định có trạng thái đóng/mở; V1 dùng 4, mô hình đủ cho 8/16 về sau | `packets.rs` `room_slot_state` |
| Bắt đầu trận | Chủ phòng bấm START sau khi **client tự kiểm** mọi người ready (`sub_6C869F`); server chỉ bắt đầu | Server mới vẫn phải tự kiểm (authority); khác biệt V1: chủ phòng cũng ready (SCOPE_V1 §3, DESIGN) | `session.rs` op 97 |
| Ready / kick / đóng slot / chọn map | 87 ready; kick hiện "BannedByLeader" rồi về sảnh; 94 đóng/mở slot; 88 chọn map, gói 88 đầu tiên sau khi vào phòng là tự động (VN-B06) | Lệnh phòng tin cậy có dedup; đóng slot và chọn map để later | `session.rs` op 87/94/88/kick |
| Chat | 28 phát lại trong phòng/sảnh | V1-009 chat chờ | `session.rs` op chat |
| Danh sách sảnh | Người dùng 11/trang; phòng có cờ mật khẩu, đang chơi, số người, tối đa, loại phòng | DTO sảnh V1-003; mật khẩu/private là later | `packets.rs` 69, 88 |
| Đặt bom | 111 mang **sức nổ tại lúc đặt** (`power`, `r18`) và serial của client; server cấp **ID bom** (152) | R-032: sức nổ chụp lúc đặt; ID entity do authority cấp | `packets.rs` `BombRecord`, `bomb_placed` |
| Nổ / đẩy bom | 151 `kind 0` nổ, `kind 1` đẩy bom sang ô khác | Event nổ; đẩy/đá bom (later W1) đi qua authority | `packets.rs` `bomb_explode` |
| Item | 553: danh sách item nền + item trong gạch, `index < 1000` mỗi map, `kind` = thứ tự ItemAttr; nhặt 117 → 158 có bản ghi chờ | R-050, R-054; item entity có ID ổn định theo lượt | `packets.rs` `map_items`, `ItemPick` |
| Sự kiện bomber | 120 subtype: 0 dùng item, 1 thoát bong bóng (kim), 11 bị nhốt, 12 cứu, 13 chết, 21 trúng bom (RPG); 17/26 chưa rõ | Danh sách `EventKind` tương lai (ADR 0008): Trapped, Rescued, Died, ItemUsed, BubbleEscaped | `session.rs` op 120; oc_e §5a-5c |
| Kết quả | 168: hạng mỗi slot (≤ 1 = thắng), cột BẮT/CỨU/EXP/VÀNG | DTO kết quả có sẵn chỗ cho thống kê bắt/cứu, EXP, tiền (W2) | `packets.rs` `game_result` |
| Đồng bộ người khác | 146 mang bảng peer UDP (MTUDP), client báo IP/port qua C→S 92 | Legacy đồng bộ di chuyển người khác **P2P**; engine mới thay bằng snapshot server (ADR 0005) | `session.rs` op 92, `match_flow` |
| Shop | Ví cash/gold; 223 = mua thành công; **client tự trừ gold** | W2: server phải sở hữu số dư và ledger, không tin client | `session.rs` shop |
| Map/theme | 1019 entry `Map/*` (VN); theme = thư mục `Theme/*` dài nhất là tiền tố tên sau khi bỏ `NXCL_`/`WS_`/`WinningStreak_`; chạy được 9 map (VN-B04) | Quy tắc tra theme cho content closure (Q-002); `candidate` cho map chưa chạy | `maps.rs` `all_battle_maps` |

## 2. Lựa chọn của emulator — chỉ là candidate

| Giá trị/chính sách | Emulator đặt | Trạng thái trong engine |
|---|---|---|
| Fuse bom | 2,5 s (`BOMB_FUSE`) | R-022 candidate; profile dùng `design` 2,5 s |
| Kết thúc trận | Còn ≤ 1 đội sống → xếp hạng sống = 1, chết = 2; một đội thì đợi cả đội chết | SCOPE_V1 §4 FFA (design) đã tương đương; đội là later |
| Thời gian chờ kết quả | `MATCH_END` 3 s + 5 s trước 173 | SCOPE_V1 dùng 5 s (design) |
| Spawn | Slot i → điểm xuất phát i | SCOPE_V1 dùng hoán vị theo seed (design) |
| Chủ phòng mới | Slot thấp nhất | SCOPE_V1: người vào sớm nhất (design) |
| Phòng | Một phòng chung, tối đa 8 trong danh sách | Không dùng |
| Chuỗi UI tiếng Việt | `strings.rs` ~150 khóa UI do nhóm dự án tự viết (client VN không có) | Ứng viên cho Q-012, kèm review như catalog l10n |

## 3. Kỹ thuật tận dụng được

- **Chặn quyết định trùng:** `match_no` làm bộ đếm lượt; timer/quyết định của lượt cũ bị bỏ. Chính là yêu cầu
  "kết quả commit một lần" và "snapshot trễ của lượt cũ bị bỏ theo round ID" (ARCHITECTURE §6).
- **Golden byte-for-byte:** mọi gói S→C kiểm với fakeserver (`tests/golden.rs`). Áp dụng cho `boom-proto`:
  golden encoding cho từng message và revision.
- **Harness hai client** (`tools/duo.sh`, `match_play_vn.sh`, `duo_measure.sh`): dùng cho phép đo Q-004/Q-005
  (tick, speed-level) trên client VN.
- **Không port:** framing/obfuscation XOR và seq, đăng nhập Nexon, bảng peer UDP — engine mới dùng giao thức
  riêng (ADR 0006).

## 4. Đã nhập vào nguồn chuẩn

- RULES_V1: R-032 (sức nổ chụp lúc đặt), §7 vòng đời/phòng R-060…R-066.
- ADR 0005: legacy đồng bộ người khác bằng P2P.
- QUESTIONS: Q-002 (quy tắc tra theme), Q-012 (chuỗi UI tiếng Việt ứng viên).
