# Kiến thức RE client Boom VN/KR — lưu trữ khi tạm ngưng (2026-10-26)

Dự án boom-rs **tạm ngưng** ngày 2026-10-26 theo quyết định của chủ dự án. Tài liệu này hệ thống hoá kiến
thức, nguồn, công cụ và bài học để dự án RE client KR sau này nối tiếp được. Đây là **chỉ mục tổng hợp**.
Nguồn chuẩn chi tiết vẫn là [RULES_V1](../specification/gameplay/RULES_V1.md) (claim R-xxx),
[findings](../research/findings/README.md), [evidence](../evidence/README.md), [spec](../specification/README.md)
và [ADR](../decisions/README.md).

Asset game **không lưu**. Mục 2 chỉ trỏ tới client gốc VN/KR (đường dẫn máy dự án + hash).
Bản lưu cục bộ kèm script vận hành: `/home/admin/boom-re-archive-2026-10-26/`.

## 1. Kiến thức theo chủ đề
Quy ước:
- `R-xxx` là claim trong `boom-rs/docs/specification/gameplay/RULES_V1.md`, kèm nguồn/trạng thái/locator.
- Địa chỉ không ghi build thì là **KR 00967** khi bắt đầu bằng `0x00…`/6–7 chữ số hex lớn, và là **VN
  VBNB_116** (`ca_fixed_patched.exe`, image base 0x400000) với các địa chỉ 0x4xxxxx–0xAxxxxx. Luôn kiểm
  lại trong claim gốc.
- VN và KR cùng codebase. Chính sách: VN tin cậy ngang KR (ADR 0002). Ưu tiên KR confirmed → VN
  confirmed → BnB OSS → design (ADR 0013). Trình diễn: RE → quay VN → design có nợ (ADR 0014).

### 1. Định dạng dữ liệu
| Chủ đề | Tóm tắt | Nguồn |
|---|---|---|
| Pack `.idx/.idd` | Index tên entry → offset/size/key trong `.idd`; entry có thể mã hoá; KR 13 cặp IDX/IDD, 51.467 entry | `boom-rs/docs/specification/assets/PACKS.md`, `boom-re/docs/assets.md`, evidence `kr_feature_inventory.json` |
| Sprite `CDataI16Alpha` v2 | Header (version, số ảnh, `unk` dword 4) + ảnh RLE alpha + **12 số mỗi frame** (image, offset_x/y, flag, x0,y0,x1,y1, canvas w/h, pivot x/y). Không có thời lượng trong blob | MAPS §3, R-077/R-079, `boom-assets/src/sprite.rs` |
| Lớp nhân vật | Entry `C0x-<State>_ALayerStatic` + `…Adjust` vẽ chồng; loader ghép tên `LayerStatic` | R-209…R-211, KR_RE_RENDER |
| Map | `.map` VN/KR (KR thêm 4 byte header, R-003); ID tile/obj = chỉ số entry `fx/ObjectTable/ObjectTable` | MAPS, `boom-assets/src/legacy_map.rs`, `closure.rs` |
| **ObjectTable (ODT)** | `fx/ObjectTable/ObjectTable`: u32 count + (len, tên cp949, u32 offset)… Sau bảng tên là **khối dữ liệu**. Khối animation (R-094): u32 số state; mỗi state gồm nhãn `"<số> <tên>"`, entry, u32 n, n×(u32 frame, u32 thời lượng). **7.867 khối** trên KR; payload VN và KR giống byte ở các khối đã so | R-094, **R-251**, `boom_assets::closure::{parse_object_table, animation_block}` |
| Thời lượng ODT | **Đơn vị 1/30 s**: consumer `BA3530` so bộ đếm `B33170()` với `[record+4]`; `sub_636620` +1 mỗi lần chạy, nhịp native 30 Hz. `0xFFFFFFFF` = giữ vô hạn; 32767 là số hữu hạn | **R-222/R-223**, R-151, KR_RE_ANIM_STATE |
| Âm thanh | `FxSound`: `Wave/*` (SFX), `Mp3/*` (BGM); KR có bảng 649 id → `Wave/*` ở `0x19E40D8` | KR_RE_AUDIO, R-203…R-208, R-191 |
| Item | `ItemAttr` VN 269 kind (tên/sprite/trọng số); KR loader `1196A70`, stride group `0xF4` | ITEMS_VN_REFERENCE, R-153, `boom-re/docs/items.md` |

#### Chuỗi ODT đáng nhớ (KR, đơn vị 1/30 s)
- `Character/C00_A`:
  - Walk <Dir> `0,1,2,0,3,4` × 3; Stop <Dir> frame 0 × 100; Blink `0:25 1:5 2:5 1:5 2:5 1:15 0:75`;
  - Faint `0..2,1,2…` × 5; Die 44 bước = 92 (≈3,07 s, kết thúc ở frame rỗng 13); Locked In `Die` frame 0/1 × 32767; Start 49 bước = 78.
  - Ngoài ra có Ride/Throw/Slide/Jump/Drill/Hurrah/Release…
- `Bomb/Bubble`: Lock `0,1,2` × 4 rồi frame 6 × ∞; ExpLive `6,7,8` × 4; ExpKill `3,4,5` × 4.
- `Bomb/Help` `0:6 1:3 2:9 3:3`. `Bomb/BasicBomb` `0..3` × 6. `BombExplosion/BombExplosion001` `0..9` × 4.
- `Flame/Flame` 76 state: tâm `FireC01..C10` frame 0..3; nhánh `FireL/U/R/D01..10` bắt đầu frame 4/5/6/7, bước 4, kết thúc 96..99, mỗi state 19 bước × 1. Hậu tố = power của bom là **candidate** (R-192, R-195).
- Item nền `Item/Bomb|Fire|Roller`: không có khối ODT, 1 frame tĩnh (R-226).

### 2. Kiến trúc client (VN/KR)
- Khởi động `CFxApp__vf4` `0x8E6FEE` (VN):
  - `InitHackShield` + BlackCipher;
  - patch server, nạp pack, renderer;
  - stage Login. Log ở `CALog.log`, mốc `dne:10…13` = login/sảnh/phòng/trận.
  - Nguồn: `boom-rs/docs/research/findings/VN_RUNTIME_REFERENCE.md`, `boom-re/docs/runtime.md`.
- **Stage:** 10 login/kênh, 11 sảnh (`LobbySquare`), 12 phòng chờ (`Prepare`), 13 trận, 14 siêu thị.
- **Packet → handler:** 520 opcode / 59 dispatcher (VN; `boom-re/decomp/opcode_dispatch.json`). RTTI: 3.699 vtable, ~20,7k tên.
- **Vẽ (VN):** draw `5AD00C` → selector `5ACC58` → vtable+5C `58D3E9` / bộ đếm `58D517` → baseDraw `58DA94`; `0x7293B4` = tra ô/occlusion; neo tile `x = cột·40`, `y = hàng·40 + 39` (R-210). **Comparator sort object chưa tìm ra** (R-190 unknown).
- **Lửa:** client tự tính vùng nổ (KR `0x00E9C520`), dây chuyền (`0x00E9CDC0`), mỗi ô một message id 27, thời gian lửa = số frame (`GetAnimCount`, R-139…R-141). VN: flame ctor `0x73290A` → `0x731BAF` ghi ordinal state vào `+0x90` (setter `0x5AB812`).
- **Bị nhốt:** KR bộ đếm bomber `+0x2748` giảm 1 mỗi update; ≤0 → slot 202 `0xB7A760` gửi C→S 167; status 184 (R-200…R-202). VN receiver/bubble counter 200; trap → chết ≈ 6,35 s (R-160).
- **UI:** rect nằm trong exe, không trong pack (R-232).
  - VN login ctor `0x0084cd94`: `LO_BtLogNewId (240,556)`, `LO_BtLogOk (350,556)`, `LO_BtLogExit (460,556)`; offset cha +52 suy ra (R-233).
  - KR room-list hitbox `549180` / bảng `19D8A58`; PrepareStage `6F47C0`, Ready `512,494` (R-152).
  - Room/lobby VN chưa thấy constructor (R-234). Game 800×600 (capture ×1,28 lên 1024×768).

### 3. Luật trận và số đo (giá trị chọn trong `boom-rs/data/rules/v1-dev.toml`)
- Tick native 30 Hz (R-013, R-113). Bước vị trí rời 5/6 px mỗi tick, **không nội suy khi vẽ** (R-171, R-176, R-225).
- Tốc độ: **33 px/s mỗi cấp** (R-183/R-184; C00 cấp 5 ≈ 165 px/s, khớp đo 167).
- Hộp chân 40×40, toạ độ neo (R-119…R-125, KR_RE_MOVEMENT). Trợ góc mover VN `59DA6F`: trượt vuông góc theo bước 5/6, lệch tối đa trong (30,35] px (chỉ đo hướng Down; R-157, R-177). V1 hiện 12 px (lệch).
- Bom: fuse/flame/trap thời lượng, đơn vị cộng Bomb/Fire, trọng số rơi item 3650/2800/2000 (R-110…R-118, R-158, R-160). Bom chờ nổ lặp 4 frame × 200 ms (R-221).
- Cứu/chọc bóng do client tự xử lý (C→S 120 subtype 12/13); đồng bộ item 2 người (VN_BATTLE_RUNTIME_REFERENCE).

### 4. Trình diễn (fidelity)
- Nguồn luật phát: **ODT (R-251)**, không phải pack sprite và không phải đoán. Gốc pha khi đổi hướng là design (R-252).
- Vị trí và lớp:
  - bom < nhân vật < bóng (R-199);
  - HELP! căn giữa trên bóng tròn, khe 2 px, nằm trên cùng (R-250);
  - thân nhân vật trong bóng = `Locked In` + overlay Faint (R-220).
- Quan sát video VN: R-193…R-198, R-217…R-219. Capture thực tế ~30 fps dù khai 60.
- Bảng nợ: `boom-rs/docs/specification/FIDELITY.md` §3.

### 5. Âm thanh (R-191, R-203…R-208)
- Dispatcher KR `0xD1CD10`. Id thường dùng:
  - đặt bom 9; nổ 7/8 (xen kẽ trong 200 ms; tuyết 343);
  - nhốt 77 (couple 37, help 444); chết 42;
  - item ăn 68, item lộ 69, dùng trap 174;
  - Ready/Start 18/21; thắng/thua/hoà 175/78/43 (biến thể `rand()%N+base`).
- BGM: `0xE71D70`, tên lấy từ record map (Village id 48). Pool voice `0x14BF4D0` dedup, không ưu tiên.

### 6. Mạng (VN)
- TCP: frame + mã hoá + seq + hello. Server nghe 4 port 3838/4848/5858/6868 (IP hardcode, `CMultiServer` `0x998FD5`).
- Trận: 117 → 189/190/189/191; bom 111 → 152 → 151; chết 120 → 146; kết quả 168 → 173; item 553/158. UDP P2P: C→S 92 → S→C 146 bảng peer.
- Phòng: map 88 → 113 (gói 88 đầu tiên sau khi vào phòng là tự động và giữ map của phòng).
- Tài liệu: `boom-re/docs/protocol.md`, `lobby.md`, `oc_e-game.md`; `boom-rs/docs/specification/networking/LEGACY_VN_PROTOCOL.md`.

### 7. Bảo vệ/Themida (VN)
- Unpack: OEP `0x9E5EFE` → dump `unpacked_ca.exe` → sửa IAT → `ca_fixed_patched.exe`, chạy được trong Wine.
- Patch bỏ HackShield `0x8E701D 74 29 → EB 29`.
- 15 khối VM đã viết lại (`boom-re/tools/devirt.py`); còn `0x966FEC` và 49 site anti-cheat chưa đụng.
- Tài liệu: `boom-re/docs/devirt.md`, `vmdevirt.md`, `boom-rs/docs/research/findings/DEVIRT_METHOD.md`, `VM_METHOD.md`.
- KR: chỉ RE tĩnh (không chạy server thật, không bypass anti-cheat). Thunk VM ví dụ `0xB534D0 → 0x02012376`.

### 8. Câu hỏi còn mở (để dự án KR sau đóng)
- Comparator sort đối tượng khi vẽ (R-190).
- Hậu tố state lửa: power hay khoảng cách (R-192).
- Trợ góc ba hướng còn lại (#369).
- Rect room/lobby/HUD/result trong exe (R-234, R-230, R-231).
- Bomber mover/hitbox thật so với `CMovable`/pet (R-134/R-135).
- BomberAttr consumer (R-150).
- Danh sách đầy đủ: `boom-rs/docs/research/QUESTIONS.md`, issue GitHub, `FIDELITY.md` §3.

## 2. Nguồn: client gốc và cơ sở dữ liệu RE
Không sao chép client, pack hay IDA DB vào bản lưu hoặc repo. Mọi phân tích làm trên **bản sao**; không
sửa binary, pack hay IDA DB gốc.

### Client VN — Boom Online (VNG, nhánh source VBNB_116)
| Thứ | Đường dẫn | SHA-256 / ghi chú |
|---|---|---|
| Thư mục cài gốc | `/home/admin/boom-re/wine/Boom/` | gồm `ca.exe` gốc (Themida), pack `fx.idx/idd`, `charte`, `emote`, `fxasset`… |
| `ca.exe` gốc (Themida) | `wine/Boom/ca.exe` | `bce8644d4e783d1354ab4fc880e1ab9a4ab3325327028f37d99e5439e25f4bbd` |
| Bản unpack đã sửa IAT | `wine/Boom/ca_fixed_patched.exe` | `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1` (fixture các claim VN) |
| IDA DB bản unpack | `/home/admin/boom-re/unpacked_ca.exe.i64` | RTTI/tên ở `boom-re/decomp/ida_names.json` |
| Bản sao chạy runtime (prefix + game) | `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/{prefix,game}` | dùng cho harness quay; không dùng thư mục gốc |
| Prefix Wine khác | `~/.wine-vn`, `~/.wine-vn2` (2 client) | `boom-re/tools/duo*.sh` (cẩn thận: pkill dùng chung) |
| Server cũ | đã chết; IP hardcode `222.255.122.204`, `120.138.77.22–25/83–85`, port 3838/4848/5858/6868 | `VN_RUNTIME_REFERENCE` |

### Client KR — Crazy Arcade/BnB (Nexon, build 00967)
| Thứ | Đường dẫn | SHA-256 / ghi chú |
|---|---|---|
| Thư mục client đầy đủ | `/home/admin/boomkr-full/` | `CA.exe`, 13 cặp `*.idx/*.idd` (FxDesktop, FxCharacter, FxAsset, FxContents, FxStage, FxMap, FxSound, fx, CharTE, EmoTE…), `CAKR.ini` |
| `CA.exe` | `boomkr-full/CA.exe` | `a52545c14b1fea423de655c3163d6360b628d61c1cd472b46fdbc8a9ffe15f4e` (= `kr_feature_inventory.json`) |
| IDA DB KR | `boomkr-full/CA.exe.i64` | dùng qua MCP `ida` hoặc REA; mở bằng bản sao khi cần ghi |
| Bản sao khác | `/home/admin/boom-re/boomKR/` | cùng bộ file, dùng cho so payload (ví dụ `Bomb/HELP!`) |
| Inventory | `boom-rs/docs/evidence/inventory/kr_feature_inventory.json` | hash exe + 51.467 entry; nhóm tên tài nguyên |
| ODT payload KR | `fx/ObjectTable/ObjectTable` | SHA-256 `a68fd5ffcc815b24…`; dump animation cục bộ `/home/admin/boom-team/shots/fidelity/odt/` |

Chính sách KR: **chỉ RE tĩnh/offline**. Không kết nối server thật, không bypass anti-cheat.

### Repo và dữ liệu phụ trợ
- `/home/admin/boom-re-l10n`: catalog chuỗi/ảnh tiếng Việt và review ảnh (`l10n/README.md`).
- `/home/admin/boomkr-re/docs/`: `kr.md`, `kr-content.md`, `kr-data.md` (RE KR cũ, chưa audit).
- Ảnh/video đo đạc (local, không public): `/home/admin/boom-team/shots/` (`320/`, `fidelity/F-01…F-12`, `odt/`).
- Nguồn web/OSS: `boom-rs/docs/research/sources/`, `findings/WEB_RULES_NAMU.md`, `OSS_REIMPLEMENTATIONS.md` (BnB `tpoisonooo/BnB@5c108e8`).

## 3. Công cụ
### A. boom-rs (repo, Rust workspace) — `cargo xtask ci` là cổng kiểm đầy đủ
| Công cụ | Dùng để | Ví dụ |
|---|---|---|
| `boom-assets` (CLI) | Đọc pack KR/VN, xuất sprite, dựng bundle | `boom-assets pack-list <dir> [stem]`, `pack-extract <dir> fx ObjectTable/ObjectTable out.bin`, `sprite-info`, `sprite-png`, `audio --list`, `bundle --map Village00 --out <dir> --pack-dir /home/admin/boomkr-full` |
| `boom_assets::closure` | Parse ObjectTable (bảng tên, khối animation ODT R-094/R-251) | `parse_object_table`, `animation_block(data, table, "Bomb/Bubble")` |
| `boom-client` | Client Rust; chụp ảnh đối chiếu | `--bundle <dir> --demo --snapshot out.png`; `--demo-death --death-frame N`, `--death-hold`, `--trap-at TICK`; `--ui-shot login\|lobby\|room\|result`; `--connect host:port --register --auto-ready` |
| `boom-server` | Server Rust mới (không phải server cũ) | `boom-server serve` (env `BOOM_*`), `content`, `health --offline` |
| `tools/kr_catalog.py` | Catalog/scope KR, sinh export | `python3 tools/kr_catalog.py --check` |
| `tools/docs_check.py` | Kiểm link/docs | `python3 tools/docs_check.py` |
| `tools/check_package.py` | Kiểm gói phát hành | `check_package.py check <dir>` |
| `tools/rules_precedence.py`, `kr_tracking.py` | Thứ tự nguồn luật, tracking feature | test kèm |
| Test pack thật | Hash bundle trên pack KR | `BOOM_KR_PACK_DIR=/home/admin/boomkr-full cargo test -p boom-assets recorded_content_hash` |

### B. boom-re (RE VN) — `/home/admin/boom-re/tools`
- **Unpack/IAT/Themida:** `fix_iat.py`, `themida_scan.py`, `patch_client.py`, `launcher/boomlaunch.c`, `launch.ps1`, `unlicense.exe`.
- **Devirt/VM:** `devirt.py` (15 khối), `vmdevirt.py`, `vmemu.py`, `vmtrace.py`, `verify_emu.py`, `snapdiff.py`, `snap.exe`.
- **IDA:** `ida_export_names.py`, `ida_apply_names.py`.
- **Protocol/server:** `fakeserver.py` (đặc tả, golden test), `boomproto.py`, `gen_golden.py`, `udp_sniff.py`; server Rust giả lập ở `boom-re/server/` (`target/release/boom-server --log <file>`).
- **Asset:** `i16.py` (sprite), `idf.py` (pack).
- **Chạy client:** `run_xvfb.sh`, `xvfb_login*.sh`, `match_play*.sh`, `match_gdb.sh`, `map_loop.sh`, `roomtype_loop.sh`, `duo*.sh`, `wine_gdb.sh`, gdb script `tools/gdb/*.gdb`.
  - ⚠ `duo_vn.sh` và `match_play_vn.sh` pkill Xvfb/wine dùng chung, không dùng khi có người khác chạy.

### C. Harness quay client VN đã kiểm chứng (bản sao cục bộ `/home/admin/boom-re-archive-2026-10-26/tools/`)
- `tools/vn_capture_run.py` (gốc `/home/admin/boom-team/shots/320/claude-r1/run.py`):
  - Xvfb `-displayfd`, Mesa llvmpipe, bỏ `WAYLAND_DISPLAY`;
  - server giả lập `tools/vn149-factory-server.py`;
  - đăng nhập bằng xdotool, chờ `CALog dne:10–13` + S->C 191;
  - quay `ffmpeg x11grab` ffv1, ghi mốc thao tác `actions.jsonl`.
- Biến server: `BOMB_PLACE=1 BOMB_ECHO=<s>`, `FLOOR_ITEMS`, `ITEM_ACK`, `DEATH_END`, `MATCH_END`, `ALL_MAPS=1 BOOM_DIR=<game> MAP_INDEX=<i>`.
  - Với `vn149` thì Forest01 vẫn hiện Factory. Server Rust boom-re thì dừng ở stage 11.
- Video: game 800×600 phóng ×1,28; capture thực tế ~30 fps. Tách khung: `ffmpeg -ss t -i capture.mkv -vf "fps=10,crop=W:H:X:Y,tile=…"`.

### D. Dump ODT (`tools/re/odt_dump.py` trong repo này)
Đọc `ObjectTable.bin` (lấy bằng `boom-assets pack-extract … fx ObjectTable/ObjectTable`) rồi in hoặc ghi JSON mọi khối animation. Kết quả KR: `/home/admin/boom-team/shots/fidelity/odt/odt_anims.json`.

### E. RE tĩnh
- MCP `ida` (IDA Pro), MCP `rea` (REA: decompile/xref/runtime), `objdump`, `strings`. Luôn làm trên bản sao.
- Ghi evidence: địa chỉ + `evidence_id` REA + giới hạn; không commit decompile/binary.

### F. Hạ tầng team
Cục bộ, không public: `/home/admin/boom-team` (giao việc dsh, agent, safe-merge).

### Bẫy đã gặp
- **Target dùng chung:** `/home/admin/boom-rs-wt/.cargo/config.toml` đặt target dùng chung. Build song song giữa worktree làm lẫn crate path, nên build/test dùng `CARGO_TARGET_DIR` riêng.
- **`/tmp` là RAM:** thư mục review 3,5 GB đã từng làm đầy `/tmp`. Xoá khi xong.
- **zsh:** `set -- $x` không tách từ, `c(){}` lỗi do alias; chạy qua `bash -c`.
- **PR/merge:**
  - PR **draft** không merge được (`gh pr ready`);
  - trạng thái mergeable `UNKNOWN` ngay sau một merge thì phải chờ;
  - commit merge sạch nhưng không push thì head không khớp.
- **Codex:** `codex exec` chạy một lượt rồi thoát. Điều phối dài phải lặp hoặc dùng timer.
- **Test chập chờn:** hook pre-push đôi khi lỗi `boom-server runtime::tests::serve_binary_exits_zero_on_sigterm…` khi build song song. Không được dùng `--no-verify`.

## 4. Quy trình và bài học
### Bằng chứng (AGENTS §4, ADR 0002/0013/0014)
- Mỗi claim có **nguồn**:
  - `kr_static`, `kr_runtime`: RE client KR;
  - `vn_static`, `vn_runtime`: RE hoặc đo client VN;
  - `web`, `oss`, `emu_choice`: chỉ là candidate cho tới khi đối chiếu;
  - `design`: tự chọn có lý do.
- Mỗi claim có **trạng thái** `confirmed` / `candidate` / `design` / `unknown`. Không dùng `partial`. `unknown` không mang giá trị.
- **Thứ tự nguồn:** KR confirmed → VN confirmed → BnB OSS → design. Trình diễn: **RE consumer → quay/đo VN → design có ghi nợ**.
- **Không suy diễn quá bằng chứng:**
  - asset tồn tại ≠ tính năng chạy;
  - log server giả lập ≠ luật server gốc;
  - test sinh từ implementation chỉ là regression.
- Locator gọn: địa chỉ/hàm + `evidence_id` REA + giới hạn + hash build. Không commit binary, decompile, ảnh hay asset (repo PUBLIC).
- Video: ghi PTS thực tế (capture ~30 fps dù khai 60), quy đổi pixel game (÷1,28), sai số theo khung kề.

### Bài học lớn
1. **Tra ODT/exe trước khi đo video.** Đã mất nhiều lượt đo video cho animation, trong khi bảng ODT có sẵn chuỗi phát đúng (R-251).
2. **Mapping asset ≠ luật phát.** Lỗi bóng nhốt lặp frame 3–8 (#320) vì đã tự đặt luật "lớn dần rồi lặp". Theo ADR 0014: không tự đặt; luật phải là dữ liệu có claim.
3. **Một câu hỏi RE = một issue nhỏ, tối đa 2 lượt.** Hết lượt thì ghi blocker có phạm vi rồi chuyển sang hướng khác (RE tĩnh, trạng thái design có nợ). 6 issue RE lớn từng kẹt nhiều ngày vì không có điểm dừng.
4. **Chỉ làm câu hỏi đổi được hành vi game.** Câu "không đổi profile" xuống backlog P9 (#312).
5. **UI rect nằm trong exe, không trong pack** (R-232). Tìm theo tên control (`LO_*`, `PRE_Bt_*`) và constructor đẩy x,y.
6. **Runtime VN qua server giả lập có giới hạn:** chưa đổi được map ngoài Factory, chưa ra được màn kết quả, chưa tăng được power bom. Những câu đó cần RE tĩnh.

### Team và agent
- **Vai trò:**
  - Claude = integrator (merge, gỡ xung đột, review PR của Codex);
  - Codex = co-integrator/reviewer (review PR worker, code);
  - dsh1/dsh2 = RE worker (dải claim R-150…199 / R-200…249).
- Không tạo session hay worker mới khi chủ dự án chưa cho phép.
- Giao dsh vào **cùng một session** để tận dụng cache. Mỗi prompt ghi rõ giới hạn lượt, khoá code sẽ đổi và nơi lưu ảnh local.
- **Mỗi PR một review độc lập** đúng head. Commit chỉ merge main/gỡ xung đột thì integrator xác nhận lại head, có ghi chú.
- **Số mục RULES:** chốt số mục cho từng PR trước khi giao. Xung đột cuối file thì giữ cả hai khối, xếp theo số mục.
- **Phiên điều phối Codex phải lặp:** đã có lúc dsh không có việc vì phiên chạy một lượt rồi thoát.

### Gate/tiến độ lúc dừng
- V1 36/37 feature. Gates G1–G5 đạt, G6 (phát hành đa OS) chưa ký. CI hiện chỉ chạy Linux (#341).
- Chi tiết: `boom-rs/docs/planning/PROGRESS.md`, `execution.json`.

