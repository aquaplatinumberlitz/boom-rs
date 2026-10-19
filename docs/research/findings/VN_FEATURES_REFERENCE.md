# Danh sách tính năng client VN — checklist 100%

Mục đích: một danh sách **đầy đủ** để theo dõi tiến độ, thay vì mỗi lần lại hỏi "còn gì nữa".
Mỗi dòng có **bằng chứng** (class/opcode/tài liệu). Dòng nào còn suy luận thì ghi rõ "suy luận"/"CHƯA KIỂM" trong cột bằng chứng.

Cập nhật số liệu tự động: `python3 tools/feature_stats.py` (đọc `decomp/opcode_dispatch.json`,
`decomp/ida_names.json`, `server/src/*.rs`, `tools/fakeserver.py`).

## Quy ước trạng thái

| Ký | Nghĩa |
|---|---|
| ✅ | **kiểm chứng runtime**: client thật chạy qua, có log/ảnh |
| 🟡 | đã đọc code / một phần chạy được, còn thiếu |
| ⬜ | chưa làm |
| ⛔ | bị chặn bởi thứ khác |
| — | không áp dụng (client-side, không cần server) |

Cột **Rust** = server Rust `server/` đã làm phần server của dòng đó chưa (✅ chạy với client thật, 🟡 một phần/giá trị giả, ⬜ chưa, — không cần server).
`tools/fakeserver.py` là đặc tả thử nghiệm: thường có trước Rust (ví dụ 553 vật phẩm hiện chỉ có ở fakeserver).

## Số liệu nền (tự sinh)

| chỉ số | giá trị |
|---|---|
| opcode client biết (từ bảng dispatch tĩnh) | **520** |
| dispatcher (`<Stage>__OnPacket` + component) | **59** |
| handler có tên từ RTTI | 812 / 1113 |
| class C++ có tên | **2305** |
| hàm/vtable đã đặt tên | 20.682 / 3699 vtable |
| opcode server giả đã xử lý | **87 / 520 (16,7%)** (2026-10-18 tối, `feature_stats.py`) |
| khối VM Themida | **64 điểm vào** (`push key; jmp 0x11EE128`): 15 trong code game (**14 xử lý**, còn `0x966FEC` mồ côi) + 49 vùng anti-cheat A1xxxx (0 xử lý, chưa chạm). `devirt.py` có 15 `BLOCKS` = 14 site game + `0x5788C9` (jmp bị chiếm, không phải VM). 73 = số marker SDK, không phải số khối |
| stage đã chơi được | 1 → 13, **trọn 1 ván** (kết thúc → phòng chờ → ván mới) |

Phân bố opcode theo nhóm:

| nhóm | dispatcher | opcode | handler có tên |
|---|---|---|---|
| khác (base `CStage` + component chưa rõ) | 32 | 331 | 327 |
| lobby/room | 10 | 197 | 210 |
| battle | 6 | 134 | 148 |
| social | 4 | 56 | 59 |
| auth/login | 5 | 46 | 46 |
| economy | 2 | 22 | 22 |

---

## A. Hạ tầng & runtime (client-side)

| ID | Tính năng | Bằng chứng | Trạng thái | Tài liệu | Rust |
|---|---|---|---|---|---|
| A1 | unpack Themida + tìm OEP | `unpacked_ca.exe`, OEP `0x9E5EFE` | ✅ | `runtime.md` | — |
| A2 | sửa IAT bản dump | `ca_fixed_patched.exe`, `tools/fix_iat.py` | ✅ | `runtime.md` | — |
| A3 | tắt anti-cheat bằng công tắc game | `Feature_IsDisabled(212)` → 1, 22 chỗ (bản dump); client gốc chỉ chịu được 13 chỗ lúc đăng nhập (`runtime.md` cuối) | ✅ | `runtime.md` | — |
| A4 | phục hồi RTTI → tên class/hàm | 3699 vtable, 20.7k tên | ✅ | `decomp/ida_names.json` | — |
| A5 | bảng dispatch opcode tĩnh | 59 dispatcher / 520 opcode | ✅ | `opcode_dispatch.json` | — |
| A6 | render D3D9 | chạy trong Wine, `CxUni*.fft` | ✅ | `runtime.md` | — |
| A7 | render DDraw7 / D3D7 (fallback) | `CRenderDDraw7` (75 hàm), `CRenderD3D7` (75) | ⬜ | — | — |
| A8 | cửa sổ web trong game | `CWebWnd` (74), `ADAD_BrowserControlPane` (58) | ⬜ | — | — |
| A9 | input bàn phím/chuột | đi được trong trận | ✅ | `progress.md` §5 | — |
| A10 | audio/BGM/SFX | chưa điều tra | ⬜ | — | — |
| A11 | font bitmap `.fft` | `CxUni.fft`, `CxUniBold.fft` | 🟡 | `assets.md` | — |
| A12 | pack `.idx/.idd` | `CharTE.idd` 368 MB, `EmoTE.idd` 275 MB | ✅ | `assets.md` | — |
| A13 | sprite → PNG | `docs/*.png` sinh từ pack | ✅ | `assets.md` | — |
| A14 | devirt khối VM | 15 site game: 14 xử lý (viết lại/skip có căn cứ), `0x966FEC` chưa; 49 site anti-cheat A1xxxx chưa chạm (chưa gây crash) | 🟡 | `devirt.md` (bảng trạng thái) | — |
| A15 | chạy client trên host có GPU | Wine + Mesa llvmpipe / WinBoat | ✅ | `runtime.md` | — |
| A16 | điều khiển guest WinBoat từ host | `tools/qmp_guest.py` | ✅ | `vmdevirt.md` §20 | — |

## B. Mạng & protocol

| ID | Tính năng | Bằng chứng | Trạng thái | Tài liệu | Rust |
|---|---|---|---|---|---|
| B1 | lớp socket (kết nối, đóng, timeout) | kiểm chứng với client thật | ✅ | `protocol.md` | ✅ |
| B2 | frame: header + seq + length | khớp traffic | ✅ | `protocol.md` | ✅ |
| B3 | mã hoá payload | khớp traffic | ✅ | `protocol.md` | ✅ |
| B4 | obfuscate số nguyên (mode 1/2) | khớp traffic | ✅ | `protocol.md` | ✅ |
| B5 | bảng tráo opcode (mode 0) | nghịch đảo nhau, `decomp/net/*.bin` | ✅ | `protocol.md` | ✅ |
| B6 | Hello (packet đầu mọi kết nối) | chạy với client thật | ✅ | `protocol.md` | ✅ |
| B7 | keep-alive opcode 15 | mỗi 5 s | ✅ | `protocol.md` | ✅ |
| B8 | endpoint multisvr / launcher | 6×u32 trong opcode 0 **chưa rõ nghĩa** | 🟡 | `protocol.md` | 🟡 |
| B9 | byte thứ 2 của header khi A=2 | `0xF4`, client không kiểm tra | 🟡 | `protocol.md` | — |
| B10 | gói nhóm C→S (`sub_88851B`: 111, 112, 120, 145…) | 111/120 giải mã được, mỗi bản ghi một gói | 🟡 | `oc_e-game.md` §5a-5 | 🟡 |
| B11 | khoá phiên trận (u32 của 189 → `this[4887]`, byte đầu gói trận) | IDA + runtime (bản ghi 111 mang khoá) | 🟡 | `oc_e-game.md` §5a-4 | 🟡 |
| B12 | kênh UDP P2P giữa các client | client báo địa chỉ bằng C→S 92, server phát bảng peer trong S→C 146 → MTUDP P2P trực tiếp, đồng bộ di chuyển ✅ (`docs/evidence/images/vn/duo_sync.png`); relay (`rank_key`) chưa dùng | ✅ | — | ✅ |
| B13 | báo cáo ngẫu nhiên kiểu anti-cheat C→S 141/142 | `sub_71B521` (tick), không phải di chuyển | 🟡 (server bỏ qua được) | `oc_e-game.md` §5b | — |

## C. Account / auth / session

| ID | Tính năng | Bằng chứng | Trạng thái | Rust |
|---|---|---|---|---|
| C1 | đăng nhập bằng tài khoản | opcode 1 → 4, 15 | ✅ | ✅ |
| C2 | danh sách kênh | opcode 14, stage 10 | ✅ (field chưa rõ) | ✅ |
| C3 | chọn kênh → vào sảnh | stage 10 → 11 | ✅ | ✅ |
| C4 | `CSessionStage` (9 opcode) | dispatcher | 🟡 | ⬜ |
| C5 | `CLoginStageChina` (12 opcode) | dispatcher — có thể của bản KR/TQ | ⬜ | ⬜ |
| C6 | đăng ký / đổi mật khẩu | chưa thấy dispatcher | ⬜ | ⬜ |
| C7 | chọn server (nhiều server) | multisvr endpoint | ⬜ | ⬜ |

## D. Social

| ID | Tính năng | Bằng chứng | Trạng thái | Rust |
|---|---|---|---|---|
| D1 | chat phòng/kênh | `CChatStage` 21 opcode, `CChattingStage` 17 | ⬜ | ⬜ |
| D2 | whisper / chat riêng | trong D1 | ⬜ | ⬜ |
| D3 | danh sách bạn | chưa thấy dispatcher riêng | ⬜ | ⬜ |
| D4 | trao đổi (exchange) | `CExchangeStage` 16 opcode | ⬜ | ⬜ |
| D5 | chợ người chơi (user market) | `CUserMarketStage` 13 opcode | ⬜ | ⬜ |
| D6 | clan/guild | chưa thấy dispatcher | ⬜ | ⬜ |
| D7 | thư/mail | chưa thấy dispatcher | ⬜ | ⬜ |

## E. Lobby & phòng

| ID | Tính năng | Bằng chứng | Trạng thái | Rust |
|---|---|---|---|---|
| E1 | `CLobbyStage` (23 opcode) | dispatcher | 🟡 | ⬜ |
| E2 | sảnh quảng trường `CLobbySquareStage` (49) | load xong, nút TẠO PHÒNG | ✅ | ✅ |
| E3 | `CSquareStage` (41) | 395/466/467 trả lời được | 🟡 | 🟡 |
| E4 | tạo phòng | dialog tạo phòng, map Village01 | ✅ | ✅ |
| E5 | danh sách phòng / vào phòng | S→C 88 (định dạng phòng), C→S 78 (vào phòng) → 83; 2 client: B thấy phòng của A và vào được (`lobby.md` cập nhật 13) | ✅ | ✅ |
| E6 | phòng chờ `CPrepareStage` (36) | người chơi, trưởng phòng, nút BẮT ĐẦU | ✅ | ✅ |
| E7 | chọn map trong phòng | hiện `Village01`; hộp thoại từng trống | 🟡 | 🟡 |
| E8 | bắt đầu trận | 501 → 189 mode 1 → 190 → 189 mode 0 → **191** (191 phải sau 189 mode 0) | ✅ | ✅ |
| E12 | sẵn sàng / đội / chọn nhân vật trong phòng | mới 1 người, chưa làm | ⬜ | ⬜ |
| E9 | kick / đổi chủ phòng / option phòng | trong E6 dispatcher | ⬜ | ⬜ |
| E10 | `CHiddenCatchPrepareStage` (33) | mode riêng, dispatcher | ⬜ | ⬜ |
| E11 | vào kênh (enter channel) | stage 10 → 11 | ✅ | ✅ |

## F. Trận đấu

| ID | Tính năng | Bằng chứng | Trạng thái | Rust |
|---|---|---|---|---|
| F1 | `CGameStage` core (95 opcode) | dispatcher | 🟡 | 🟡 |
| F2 | bắt đầu trận / đồng hồ | 189 mode 1, 190, 189 mode 0, 191 (đồng hồ context 0, `oc_e-game.md` §5a-5) | ✅ | ✅ |
| F3 | di chuyển nhân vật | ←↑→↓ đi được | ✅ | — |
| F4 | đặt bom | Space → C→S 111 | ✅ | ✅ |
| F5 | bom nổ | S→C 152 (gán id) → 151 (nổ); thời điểm nổ do server quyết (đang 2.5 s, **giá trị gốc chưa biết**); vài trường 152 đang đoán | ✅ | ✅ |
| F6 | bị nhốt / chết / lượt mới | bom nổ → C→S 120 subtype 11 (bị nhốt) → ~6 s → 120 subtype 13 (chết) → server 146 → lượt mới, hồi sinh (`5aee4e4`) | ✅ | ✅ |
| F6b | cứu đồng đội đang bị nhốt / kim (`ItemNiddle`) | chưa | ⬜ | ⬜ |
| F7 | kết thúc lượt / điểm | 146 chạy được (`5aee4e4`); **bảng điểm 18 slot gửi giá trị giả, ý nghĩa chưa kiểm** | 🟡 | 🟡 |
| F8 | kết thúc ván → về phòng chờ | S→C 173 `{0,0}` → về phòng chờ → **trận mới** (`b1fc290`); server Rust xử lý C→S 120; danh sách kết quả trong 173 chưa kiểm | ✅ | ✅ |
| F9 | nhiều người chơi trong trận | 2 client cùng phòng + cùng vào trận (`tools/duo.sh`, `hub.rs`); **di chuyển đồng bộ 2 chiều** qua UDP P2P (C→S 92 → S→C 146 bảng peer); bom/chết/vật phẩm nhiều người chưa kiểm | 🟡 | 🟡 |
| F10 | item / power-up rơi | gạch vỡ client tự tính, báo C→S 135; vật phẩm trong gạch do server gửi trước 501 bằng S→C **553** (`oc_e-game.md` §5a-5d); vật phẩm **hiện được** (danh sách 2 của 553, (a,b) = (x,y), kind 1 = bạch tuộc tím); gạch thường không giấu được | 🟡 | 🟡 |
| F10b | nhặt / dùng vật phẩm | nhặt: C→S 117 → server 158; bị động (bom +1, tầm nổ) client tự áp dụng; dùng Ctrl (kim thoát bong bóng) → C→S 120 subtype 1/0, không cần trả lời (`oc_e-game.md` §5a-5d, `docs/specification/gameplay/ITEMS_VN_REFERENCE.md`) | ✅ | ✅ |
| F11 | map load trong trận | `0x736C27` skip; emulate với object map thật = vòng lặp 0 lần (Village01, phòng 19) → tương đương; map/loại phòng khác chưa kiểm | 🟡 | — |
| F12 | va chạm / vật lý | mô phỏng chạy; gạch vỡ, tia nước, nhốt nhân vật đều client tự tính | 🟡 | — |
| F23 | gói 552 (`sub_71580C`) | khối VM đã viết lại, chạy không crash; ý nghĩa gói chưa rõ | 🟡 | 🟡 |
| F13 | mode HiddenCatch | `CHiddenCatchGameStage` (76) + 5 component | ⬜ | ⬜ |
| F14 | mode SnowBear | `CGOSnowBearBomber` (94) | ⬜ | ⬜ |
| F15 | mode SnowBall | `CGOSnowBallBomber` (82) | ⬜ | ⬜ |
| F16 | mode MonSeason2 | `CGOMonSeason2Bomber` (72) | ⬜ | ⬜ |
| F17 | mode IamBoss | `CGOIamBossBomber` (63) | ⬜ | ⬜ |
| F18 | mode RPG | `CGORPGBomber` (58) | ⬜ | ⬜ |
| F19 | Fighter of Legend | `CFighterOfLegendComponent` (13 opcode) | ⬜ | ⬜ |
| F20 | Zombie | `CZombieComponent` (10 opcode) | ⬜ | ⬜ |
| F21 | NPC/monster AI | `CGOAINpcBasicMon` (100), `CGONpcSnowMon` (53) | ⬜ | ⬜ |
| F22 | 1019 map | `server/src/maps.rs`, `ALL_MAPS` | 🟡 | 🟡 |

## G. Kinh tế & tiến trình

| ID | Tính năng | Bằng chứng | Trạng thái | Rust |
|---|---|---|---|---|
| G1 | shop | `CShopStageNew` (14 opcode) | ⬜ | ⬜ |
| G2 | mua đồ | `CBuyItemListDlg` (50 hàm) | ⬜ | ⬜ |
| G3 | túi đồ / trang bị | class `CItem*` | ⬜ | ⬜ |
| G4 | cash / nạp tiền | chưa thấy dispatcher | ⬜ | ⬜ |
| G5 | gacha / rương | chưa thấy dispatcher | ⬜ | ⬜ |
| G6 | quest | `CGOLobbySquareQuestNpc` (74) | ⬜ | ⬜ |
| G7 | level / exp | class `CUser*` | ⬜ | ⬜ |
| G8 | ranking | chưa thấy dispatcher | ⬜ | ⬜ |
| G9 | event / season | `CGOMonSeason2*` | ⬜ | ⬜ |

## H. Dữ liệu & định dạng

| ID | Tính năng | Bằng chứng | Trạng thái | Rust |
|---|---|---|---|---|
| H1 | `.idx/.idd` + mã hoá entry | `assets.md` | ✅ | — |
| H2 | sprite → PNG | ✅ | — |
| H3 | định dạng map | `map.md` (loader chưa tiêu thụ hết chunk) | 🟡 | — |
| H4 | avatar/char TE | `CharTE.idd` | 🟡 | — |
| H5 | emotion TE | `EmoTE.idd` | 🟡 | — |
| H6 | script map (`csys*`) | `csysstr.ssd`, `csysfxmg.ssd` | 🟡 | — |

---

## Ưu tiên cho mục tiêu "chơi lại được"

Thứ tự này suy ra từ chỗ đang kẹt, không phải từ danh sách:

1. ~~F8 kết thúc ván~~ **xong** (`b1fc290`); ~~F6/F7 chết/lượt~~ **xong** (`5aee4e4`).
2. **F9** nhiều người trong trận (mới 1 người) — cần B12 (UDP P2P) + server phát 111/152/151/120 cho mọi client.
3. **F10** vật phẩm rơi — đang làm (553 + 135 đã tìm, còn mã `kind`).
4. **E5/E7** danh sách phòng + chọn map (để chơi map khác Village01).
5. **D1** chat (38 opcode, chưa làm gì).
6. **F11** `0x736C27`: kiểm thêm map/loại phòng khác (20/33) — với Village01/phòng 19 skip đã chứng minh tương đương.
7. Các mode F13–F20: chỉ làm khi map/mode tương ứng được mở.

## Kế hoạch viết lại (Rust)

Port full feature theo đợt W1–W4 và quyết định PORT/REDESIGN/DROP cho từng opcode: `docs/archive/plans/REWRITE_PLAN_2026-10-19.md` §9
(danh mục chi tiết sẽ ở `docs/spec/feature-catalog.md`).

## Cách cập nhật checklist này

```bash
python3 tools/feature_stats.py            # số liệu nền
grep -n "opcode" docs/specification/networking/LEGACY_VN_PROTOCOL.md         # cái gì đã kiểm chứng
git log --oneline -- docs/                # tiến độ theo ngày
```

Quy tắc: **không đổi trạng thái sang ✅ nếu chưa có log/ảnh chứng minh client thật chạy qua.**
