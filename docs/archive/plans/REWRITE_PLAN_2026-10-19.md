> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.
>
> **Bản kế hoạch tổng thay thế trực tiếp:** [docs/planning/REWRITE_PLAN.md](../../planning/REWRITE_PLAN.md).
> Bản mới giữ lộ trình từ planning tới V1/full feature, cập nhật sau audit; cuối bản mới có bảng đối chiếu §0–10.

# Boom Online — Master Plan viết lại engine bằng Rust (giữ ~100% cảm giác chơi)

> Audit source/docs ngày 2026-10-19: [REWRITE_PLAN_AUDIT.md](../reports/REWRITE_PLAN_AUDIT_2026-10-19.md).
> Các vấn đề về oracle KR/VN, timing, nghiệm thu và phạm vi cần được xử lý trước khi chốt lịch triển khai.
> Workflow nhóm/agents và nguồn luật Hàn–Trung: [REWRITE_TEAM_WORKFLOW.md](../../planning/TEAM_WORKFLOW.md).
> **Phạm vi V1 đã chốt**: [KR_RELEASE1.md](../../planning/SCOPE_V1.md), baseline `KR-00967-V1-scope-1`.
> [Catalog KR](../../specification/FEATURE_CATALOG.md): 46 nhóm, 114 mục; 37 V1, 75 later, 2 drop legacy.
> Scope V1 này thay thế các mô tả cũ mâu thuẫn bên dưới; fidelity/ước lượng chưa được coi là đã nghiệm thu.

> Trạng thái: **ĐỀ XUẤT (2026-10-19)**. Bổ sung cho `docs/archive/plans/MASTER_PLAN_2026-10-18.md` (kế hoạch RE/bảo tồn);
> tài liệu này chi tiết hoá nhóm PORT-* thành kế hoạch viết lại cụ thể.
> Quy ước tag giống `docs/archive/plans/MASTER_PLAN_2026-10-18.md`: **[VERIFIED]** đã kiểm trong repo, **[DOC]** theo tài liệu sẵn có,
> **[HYPOTHESIS]** giả thuyết, **[UNKNOWN]** chưa biết. Thêm hai tag cho dữ liệu phía server (§9):
> **[ARCHIVE]** lấy từ tài liệu lưu trữ trên mạng (ghi URL + ngày truy cập + phiên bản game),
> **[DESIGN]** tự thiết kế, không có nguồn gốc (designer duyệt).

## 0. Bối cảnh & quyết định

Engine gốc (Crazy Arcade/BnB, bản VN `VBNB_116`) đã 25 năm tuổi, source và asset gốc phần lớn đã thất lạc.
Định hướng: **RE client còn lại → viết lại engine bằng Rust, dùng asset trích từ client**.

Quyết định đã chốt (2026-10-19):

| Chủ đề | Quyết định |
|---|---|
| Tương thích client gốc | **Không.** Giao thức v2 mới; giao thức gốc chỉ dùng để đối chiếu/replay test |
| Nền tảng | Desktop: Windows / Linux / macOS |
| Hiển thị | **Pixel-perfect**: render đúng độ phân giải gốc vào framebuffer, phóng to nguyên lần; có chế độ "gốc 100%" để so sánh |
| Release 1 (V1) | Battle cá nhân 2–4 người, Village00, C00/C01, Bomb/Fire/Roller; auth dự án → sảnh/phòng → trận → kết quả → vòng tiếp. Theo KR_RELEASE1.md |
| Full feature | Có: port toàn bộ tính năng client cũ theo các đợt W1–W4 (§9) |
| Logic server gốc | **Không còn.** Thiết kế lại: `Attr/*` trong client → tài liệu lưu trữ trên mạng (wiki KR, fan site…) → designer quyết |
| Nhân lực | Đội **3–5 người**, 4 luồng song song |
| Cash/nạp tiền thật | **Để sau.** Chỉ làm tiền ảo trong game |

## 1. Kết luận khả thi

**Khả thi.**

- Asset đã gỡ hết **[DOC]** (`docs/specification/assets/PACKS.md`): 108.726 PNG, 645 ogg, 965 map, `Attr/*`.
- Giao thức và luồng trận đã thông **[DOC]** (`docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `docs/archive/re/OC_E-GAME.md`).
- Client gốc chạy được dưới Wine + gdb, nên có thể **đo trực tiếp** hành vi gốc (`tools/wine_gdb.sh`, `tools/match_gdb.sh`, `tools/gdb/`).

Rào cản thật: **gần như chưa có hằng số gameplay nào được khôi phục**. Thiếu tốc độ, hitbox, fuse bom,
thời gian và cách lan của vụ nổ, timing animation, blend/z-order, tick rate, độ phân giải render (xem §3).
Cảm giác chơi nằm ở chính những con số này.

→ **Nguyên tắc số 1: "Đo, không đoán."**
- Mọi thông số ảnh hưởng cảm giác chơi phải có nguồn từ client gốc (đọc tĩnh trong IDA hoặc đo runtime).
- Mỗi thông số phải có test so sánh tự động.
- "~100%" được định nghĩa bằng số đo (§2), không bằng cảm nhận.

## 2. Fidelity Spec — "giống gốc" nghĩa là gì (đo được)

| Hạng mục | Tiêu chí chấp nhận |
|---|---|
| Mô phỏng (vị trí bomber, bom, nổ, item, bẫy/chết) | Cùng chuỗi input → trace vị trí theo tick **khớp 0 px** với client gốc; sự kiện (nổ, chết, nhặt item) lệch ≤ 1 tick |
| Hình ảnh | Khung hình native-res so pixel với ảnh chụp gốc: ≥ 99,5% pixel trùng trên cảnh tĩnh; đúng frame animation theo tick |
| Âm thanh | Đúng file, đúng tick phát (± 1 tick), đúng âm lượng/kênh/loop, đúng quy tắc chồng tiếng |
| Input | Độ trễ input → chuyển động bằng gốc (đo theo frame); quy tắc ưu tiên phím khi giữ nhiều hướng giống gốc |
| Mạng | LAN: không phân biệt được với offline. Online: client prediction giữ cảm giác ở ping ≤ ~100 ms |
| Playtest | Người chơi cũ chơi A/B mù (gốc và mới) không phân biệt được ở các tình huống chuẩn |

## 3. Đã biết / chưa biết

### 3.1 Đã biết

| Thông tin | Nguồn |
|---|---|
| Ô 40 px (`idiv 0x28`), map thường 15×13; toạ độ gói bomber x = col·40, y = row·40+39 | `docs/specification/assets/MAPS.md`, `docs/archive/re/OC_E-GAME.md` **[DOC]** |
| `.map` v1000/1001 giải gần hết: lưới (tile, obj, overlay), điểm spawn, bảng drop | `docs/specification/assets/MAPS.md`, `decomp/CGamePlayMap__Load.c` **[DOC]** |
| Sprite `CDataI16Alpha` v2: RLE, RGB565 colorkey `0xF81F` hoặc BGR24 + kênh alpha (alpha = 255 − v) | `tools/i16.py`, `docs/specification/assets/PACKS.md` **[DOC]** |
| Frame record 48 byte: chỉ số ảnh, rect nguồn, canvas, pivot, offset (**chưa có thời lượng**) | `docs/specification/assets/MAPS.md` **[DOC]** |
| `CGameClock`: base = `timeGetTime()` − ping/2, có pause/resume; giới hạn trận 90/120/180/300 s theo chế độ | `docs/archive/re/OC_E-GAME.md` **[DOC]** |
| Tick trận: `CGameStage` vf46 `0x70572A` → vf159 `0x71BF9A` mỗi tick | `docs/archive/re/OC_E-GAME.md` **[DOC]** |
| Bị bẫy → chết ~6–6,3 s; tầm nổ = 1, tăng theo ItemFire | `docs/archive/re/OC_E-GAME.md` **[DOC]** |
| Bảng ItemAttr đầy đủ (tên, sprite, trọng số drop, nhóm, tham số); Shield 2000/1000 ms | `docs/specification/gameplay/ITEMS_VN_REFERENCE.md`, `decomp/item_kinds.json` **[DOC]** |
| Âm thanh: Miles (`mss32.dll`, `_AIL_*`), SFX `.ogg`, BGM `.mp3` theo map | `decomp/iat_resolved.json`, `docs/archive/re/OC_E-GAME.md` **[DOC]** |
| Renderer `CRenderD3D9` (d3dx9_30), dự phòng DDraw7/D3D7; font `CxUni*.fft`; video Bink | `rtti.txt`, `ca_strings.txt` **[DOC]** |
| Mô hình lai: client tự quyết di chuyển, phá gạch, bẫy/chết; server quyết bom id, **thời điểm nổ (151)**, item layout, duyệt nhặt item; vị trí đồng bộ P2P UDP | `docs/archive/re/OC_E-GAME.md` **[DOC]** |

### 3.2 Chưa biết **[UNKNOWN]** (đều phải đo ở R1)

| Thông số | Ghi chú |
|---|---|
| Tick rate; vòng lặp fixed-step hay variable-step | Client import `timeGetTime`, `QueryPerformanceCounter`, `Sleep`; có chuỗi `"FPS: %d"` |
| Tốc độ di chuyển theo cấp, gia tốc, cách trượt qua góc ô | Va chạm tính ở client, chưa phân tích (`docs/research/findings/VN_FEATURES_REFERENCE.md` F12) |
| Hitbox bomber, bom, vụ nổ | — |
| Fuse bom | 2,5 s trong `server/src/session.rs` (`BOMB_FUSE`) chỉ là giá trị tạm |
| Thời gian vụ nổ tồn tại, cách lan, nổ dây chuyền, xuyên/chặn | Server gốc tính vùng nổ thế nào chưa rõ |
| Timing animation từng sprite | Trường f2 của frame record chưa rõ |
| Blend mode, z-order/layer | Chỉ biết map v1001 có lớp overlay theo ô |
| Hiệu ứng item chi tiết (tốc độ, kim, khiên…) | Mới chắc kind 0 (+1 bom), kind 1 (+tầm) |
| `BomberAttr.bin` (41 bản ghi × 11 i32) | Có thể là chỉ số gốc của nhân vật, độ tin cậy thấp |
| Độ phân giải render thật | Cửa sổ 800×600 trong guest; chưa log `Init Renderer D3D9 %dx%d` |
| Định dạng font `.fft` | — |

## 4. Chiến lược đo: "Feel Oracle" (WS-ORACLE, làm đầu tiên)

Dựng trên hạ tầng sẵn có: `tools/wine_gdb.sh`, `tools/match_gdb.sh`, `tools/match_play.sh`, `tools/gdb/*.gdb`
(`phase_clock`, `clock_start`, `bomb_path`, `input_dispatch`, `input_gates`, `items`…), `tools/snapdiff.py`,
`tools/xvfb_*.sh`, `tools/duo.sh`.

1. **Tracer.** Script gdb ghi mỗi tick thành JSONL:
   - input;
   - vị trí, hướng, trạng thái bomber (`CGOCAHeroGameBomber`);
   - bom (`CGOGameBomb`), ô nổ (`sub_7324C0`), item;
   - lệnh phát âm thanh (`_AIL_*`);
   - sprite và frame được vẽ, kèm render state D3D9 (`SetRenderState`, `SetTextureStageState`).
2. **Input tất định.** Chuỗi phím có kịch bản (`xdotool`, như `tools/duo.sh`) để tái lập y hệt.
3. **Thư viện trace chuẩn** `tests/oracle/*.jsonl` (đề xuất), mỗi trace kèm ảnh chụp:
   - đi thẳng; rẽ góc; trượt góc;
   - đặt bom, nổ dây chuyền, nổ chặn bởi gạch/khối;
   - từng loại item;
   - bẫy, giải cứu, chết; hết giờ.
4. **Đọc tĩnh song song** trong IDA để lấy hằng số chính xác (tốc độ dạng fixed-point, bảng timing).
   Trace dùng để xác nhận; khối Themida VM thì đo black-box (`docs/research/findings/VM_METHOD.md`).
5. Mỗi hằng số ghi vào **`docs/spec/rules.md`** (đề xuất) kèm địa chỉ hàm và trace chứng minh.
   Không hằng số nào vào `boom-core` mà thiếu nguồn.

Sau R2, `boom-oracle` chạy trên CI: mỗi PR đụng `boom-core`, `boom-render` hoặc `boom-audio` phải replay
toàn bộ trace và so ảnh xanh.

## 5. Kiến trúc Rust

Một Cargo workspace; `server/` hiện tại chuyển thành `crates/boom-server`.

```
crates/boom-core    luật chơi tất định: fixed tick, số nguyên/fixed-point (không float), không IO, replay được
crates/boom-assets  port tools/idf.py + tools/i16.py (+ frame record, .map, Attr/*, .fft); CLI `boom-convert`
crates/boom-render  wgpu: framebuffer native-res → integer scale; sprite batch; colorkey/alpha/blend đúng gốc
crates/boom-audio   kira: SFX ogg + BGM mp3; mô phỏng kênh/ưu tiên/âm lượng của Miles
crates/boom-ui      widget skin bằng sprite gốc (dialog, list, nút, ô nhập); layout data-driven tái tạo màn gốc
crates/boom-proto   giao thức v2 (serde + postcard, có version)
crates/boom-net     client prediction + reconciliation/rollback trên boom-core
crates/boom-server  từ server/ (tokio; hub/room/session tái sử dụng), server-authoritative chạy boom-core
crates/boom-client  winit + wgpu + kira; vòng lặp fixed-tick tách khỏi render
crates/boom-oracle  so trace gốc ↔ boom-core, ảnh ↔ ảnh, sự kiện âm thanh
```

**Vì sao không dùng engine lớn (Bevy…):** để khớp 0 px cần kiểm soát tuyệt đối thứ tự tick, pipeline vẽ và timing.
Stack mỏng (winit/wgpu/kira) ít "phép thuật", dễ tất định, dễ debug so với gốc.

**Hiện đại hoá ở đâu:**
- tách luật / hiển thị / âm thanh / mạng;
- dữ liệu (map, item, UI layout) đọc từ file, không hardcode;
- test tự động bằng oracle;
- build cho 3 hệ điều hành;
- không còn anti-cheat hay packer cũ.

**Netcode:**
- Gốc là mô hình lai: client tự quyết di chuyển, vị trí đồng bộ P2P qua UDP.
- Bản mới là **server-authoritative**: server chạy chính `boom-core` cùng tick; client dự đoán (prediction)
  rồi reconcile hoặc rollback.
- Kết quả: LAN giống hệt offline, chống gian lận tự nhiên, không cần P2P/NAT.
- Giao thức gốc (`server/` + `server/tests/golden.rs`) giữ lại làm công cụ đối chiếu hành vi.

## 6. Lộ trình

Mỗi pha có exit criteria đo được. Ước lượng ghi theo công 1–2 người cho từng pha; đây là con số định hướng, chưa phải cam kết.

Đội 3–5 người chia 4 luồng chạy song song:
- **RE/Oracle**: R1, rồi cấp hằng số và trace cho mọi pha sau;
- **Core/Server**: R2, R4, R5;
- **Client/Render/UI**: R0 (`boom-assets`), R3, R6;
- **Backend/Data**: giao thức v2, persistence, chuẩn bị §9.

| Pha | Mục tiêu | Exit criteria | Ước lượng |
|---|---|---|---|
| **R0** Nền móng | Workspace; `boom-assets` đọc đủ dependency nội dung V1, giữ legacy oracle hoạt động | Bundle V1 resolve đủ; pixel RGBA/metadata khớp nguồn; test server tham chiếu vẫn xanh | Ước lượng cũ 2–3 tuần, cần lập lại sau prototype |
| **R1** Feel Oracle | Tracer + RE hằng số cốt lõi (tick, tốc độ, hitbox, fuse, nổ, animation timing) | ≥ 15 trace chuẩn; `docs/spec/rules.md` có nguồn cho từng hằng số §3.2 cốt lõi | 4–6 tuần (song song R0) |
| **R2** Core | Village00, Battle cá nhân, bom/nổ/gạch/Bomb-Fire-Roller/bẫy-chết | Các rule có provenance, replay deterministic và báo sai khác VN/KR/DESIGN riêng | Ước lượng cũ 3–4 tuần, cần đo lại |
| **R3** Hình + tiếng | Render/audio và animation cho nội dung V1 | Image diff RGBA/ROI và audio event; golden Việt hóa riêng; snapdiff.py không phải tool so ảnh | Ước lượng cũ 4–6 tuần, cần đo lại |
| **R4** Nội dung trận | Chỉ nội dung V1 theo allowlist; các map/item/mode khác thuộc later | Gate G1–G3 trong KR_RELEASE1.md | Cần lập lại từ scope đã thu hẹp |
| **R5** Mạng | Server authoritative, phòng Battle 2–4 người; không auto-match V1 | Convergence, command validation và delay/jitter/loss theo gate G4/G6 | Ước lượng cũ 6–8 tuần, cần đo lại |
| **R6** UI ngoài trận | Auth dự án, một kênh/sảnh danh sách, phòng/chat chờ/kết quả | Luồng V1 hoàn chỉnh, Việt hóa đúng và không dead end | Ước lượng cũ 4–6 tuần, cần đo lại |
| **R7** QA & phát hành | 37 mục V1 qua acceptance, playtest và build 3 OS | Gate G1–G6; sai khác nguyên bản được ghi nhận; chưa tuyên bố parity KR100% | Ước lượng cũ 3–4 tuần, cần đo lại |

**Tổng Release 1: khoảng 6–8 tháng với đội 3–5 người** (khoảng 8–12 tháng nếu chỉ 1–2 người).

Sau Release 1: port full feature theo các đợt W1–W4, xem §9.

## 7. Rủi ro & giảm thiểu

| Rủi ro | Mức | Giảm thiểu |
|---|---|---|
| Timing gốc phụ thuộc FPS (variable-step) → khó khớp tick | Cao | Việc đầu tiên của R1 là xác định mô hình vòng lặp. Nếu gốc là variable-step thì mô phỏng đúng công thức đó ở tần số cố định |
| Hằng số nằm trong khối Themida VM | Trung bình | 14/15 khối game đã xử lý (`docs/research/findings/VN_DEVIRT_STATUS.md`); phần còn lại đo black-box |
| Blend/hiệu ứng D3D9 khó tái tạo | Trung bình | Hook và ghi render state cho từng loại sprite; tái tạo bằng shader wgpu |
| Font `.fft`, video Bink (`nm.bik`) | Thấp–TB | RE format font; chuyển video offline sang định dạng mở |
| Server-authoritative làm lệch cảm giác khi ping cao | Trung bình | Prediction + rollback; đo input latency trong oracle |
| Phạm vi phình to | Cao | Khóa phạm vi Release 1; tính năng mới phải có dòng trong `docs/research/findings/VN_FEATURES_REFERENCE.md` và được duyệt |
| Logic server không RE được → lệch "cảm giác kinh tế" (giá, drop, exp) | Cao | Lấy tối đa từ `Attr/*`; đối chiếu [ARCHIVE]; số [DESIGN] cho playtest với người chơi cũ |
| Nguồn [ARCHIVE] sai hoặc thuộc phiên bản khác (KR/VN, mùa khác) | Trung bình | Luôn ghi phiên bản + ngày; cần ≥ 2 nguồn độc lập hoặc khớp với ảnh chụp/video gốc |
| Pháp lý asset/IP | Cao | Xác nhận bằng văn bản quyền sử dụng/công khai trước khi public repo hoặc phát hành |

## 8. Việc làm ngay

- [ ] **R1-001**: xác định tick rate và mô hình vòng lặp (`CGameStage` vf46 `0x70572A`, vf159 `0x71BF9A`; `timeGetTime`/`Sleep`).
- [ ] **R0-001**: tạo workspace, chuyển `server/` thành `crates/boom-server` (giữ `server/tests/golden.rs`, `server/tests/bomb.rs` xanh).
- [ ] **R0-002**: port `tools/idf.py` + `tools/i16.py` sang `boom-assets`; test hash khớp bản Python.
- [ ] **R1-002**: script gdb tracer bomber + bom; trace đầu tiên "đi thẳng + đặt 1 bom".
- [ ] **FF-001**: mở rộng `tools/feature_stats.py` sinh `docs/spec/feature-catalog.md` (520 opcode → ID tính năng + PORT/REDESIGN/DROP).

## 9. Port full feature (sau Release 1)

### 9.1 Hai loại tính năng

| Loại | Ví dụ | Nguồn sự thật | Chuẩn "giống gốc" |
|---|---|---|---|
| **Phía client** | UI, dialog, hiệu ứng, animation, luật trong trận, chế độ chơi | Client gốc: RE được | Fidelity Spec §2: khớp pixel, khớp tick |
| **Phía server** | giá shop, tỉ lệ drop/gacha, bảng exp, thưởng quest, luật guild, chợ | Server gốc đã mất; client chỉ cho thấy **giao diện và gói tin** | **Giống những gì người chơi nhìn thấy**, không phải giống từng dòng code server |

Nguồn dữ liệu phía server, theo thứ tự ưu tiên:
1. bảng `Attr/*` trong client (ItemAttr, BomberAttr, MapGroup…, `docs/specification/gameplay/ITEMS_VN_REFERENCE.md`, `docs/specification/assets/MAPS.md`) **[DOC]**;
2. tài liệu lưu trữ trên mạng: wiki KR (Namuwiki…), fan site, video/ảnh chụp cũ **[ARCHIVE]**;
3. phần còn thiếu do game designer quyết **[DESIGN]**.

### 9.2 Danh mục tính năng đầy đủ: `docs/spec/feature-catalog.md` (đề xuất)

- Sinh tự động bằng cách mở rộng `tools/feature_stats.py`, đọc:
  - `decomp/opcode_dispatch.json` (520 opcode, 59 dispatcher);
  - `decomp/ida_names.json` (class, dialog);
  - danh sách dialog trong `ca_strings.txt`.
- **Mọi opcode phải gắn với một ID tính năng** (A–H trong `docs/research/findings/VN_FEATURES_REFERENCE.md`) và một quyết định:
  - `PORT`: làm lại giống gốc;
  - `REDESIGN`: giữ tính năng, đổi cơ chế;
  - `DROP`: bỏ, kèm lý do.
- Chỉ số tiến độ: `opcode đã quyết định / 520` và `tính năng PORT xong / tổng`.
- `docs/research/findings/VN_FEATURES_REFERENCE.md` sẽ có thêm cột "Rewrite".

Quyết định sơ bộ:

| Quyết định | Tính năng | Lý do |
|---|---|---|
| DROP | A7 renderer DDraw7/D3D7 | Có renderer wgpu |
| DROP | A3, A14, B13 (anti-cheat, Themida, báo cáo 141/142) | Server-authoritative thay thế |
| DROP | C5 `CLoginStageChina` | Nếu xác nhận không thuộc bản VN |
| REDESIGN | B8, C7 (endpoint launcher/multisvr) | Thay bằng chọn server của giao thức v2 |
| REDESIGN | A8 cửa sổ web trong game (`CWebWnd`) | UI native hoặc mở trình duyệt ngoài |
| REDESIGN | G4 cash/nạp tiền | Để sau; trước mắt chỉ tiền ảo |

### 9.3 Quy trình port một tính năng (Feature Playbook)

Áp dụng giống nhau cho mọi tính năng:

1. **RE client**:
   - đọc handler trong `decomp/handlers/*.c` (ví dụ `CShopStageNew.c`, `CChatStage.c`, `CExchangeStage.c`, `CUserMarketStage.c`);
   - xác định định dạng gói S→C/C→S và các dialog liên quan.
2. **Làm hiện màn hình trên client gốc**:
   - mở rộng `tools/fakeserver.py` để gửi dữ liệu mẫu, cho client gốc hiển thị màn hình hoặc dialog đó;
   - chụp ảnh chuẩn (`tools/snapdiff.py`), dùng làm golden cho UI mới.
3. **Viết spec** `docs/spec/features/<ID>.md`:
   - luồng màn hình, trạng thái, thông điệp;
   - mỗi số liệu có tag `[DOC]`, `[ARCHIVE]` hoặc `[DESIGN]`.
4. **Backend**: model dữ liệu và logic trong `boom-server` (§9.4), thông điệp mới trong `boom-proto` (v2).
5. **UI**: dựng màn trong `boom-ui` bằng skin gốc; so ảnh với golden ở bước 2.
6. **Test**: unit test logic, test tích hợp client ↔ server, golden ảnh. Cập nhật `feature-catalog.md` và `docs/research/findings/VN_FEATURES_REFERENCE.md`.

### 9.4 Backend cần thêm

- **Lưu dữ liệu**: PostgreSQL (production) qua `sqlx`, có migration; SQLite cho dev và test.
- **Tài khoản**: đăng ký, đổi mật khẩu (C6), phiên đăng nhập.
- **Kinh tế ảo**:
  - ví tiền trong game;
  - **sổ cái giao dịch (ledger) bất biến**: mọi mua bán, trao đổi, chợ đều là giao dịch nguyên tử;
  - chống nhân bản đồ (dupe).
- **Dữ liệu cân bằng** (giá, drop, exp, gacha, quest) trong file cấu hình (RON/TOML) có version, không hardcode. Designer chỉnh không cần build lại.
- **Công cụ vận hành (admin)**: tra cứu người chơi, cấp/thu đồ, xem log, khoá tài khoản.
- **Kiểm duyệt**: lọc chat, báo cáo người chơi (D1, D2).

### 9.5 Các đợt sau Release 1 (đội 3–5 người)

| Đợt | Nội dung (ID trong `docs/research/findings/VN_FEATURES_REFERENCE.md`) | Ước lượng |
|---|---|---|
| **W1 Xã hội** | chat phòng/kênh, whisper (D1, D2: `CChatStage` 21, `CChattingStage` 17 opcode); bạn bè (D3); thư (D7); ranking (G8); kick/chủ phòng/tuỳ chọn phòng (E9); sẵn sàng/đội/nhân vật (E12) | 2–3 tháng |
| **W2 Tiến trình & kinh tế** | level/exp (G7); túi đồ/trang bị (G3); trang phục `CharTE`, biểu cảm `EmoTE` (H4, H5); shop + mua đồ (G1, G2: `CShopStageNew`, `CBuyItemListDlg`); quest (G6: `CGOLobbySquareQuestNpc`); trao đổi (D4: `CExchangeStage` 16 opcode); chợ người chơi (D5: `CUserMarketStage` 13 opcode); khoá đồ, mật khẩu cấp 2 (`CSecondaryPwdConfig`, `CChangeItemPasswordDlg`) | 4–5 tháng |
| **W3 Chế độ chơi** | HiddenCatch (F13, E10); SnowBear (F14); SnowBall (F15); MonSeason2 (F16); IamBoss (F17); RPG (F18); Fighter of Legend (F19); Zombie (F20); AI NPC/quái (F21); script map `csys*` (H6). Tất cả theo Fidelity Spec + oracle như trận thường | 4–6 tháng (song song W2) |
| **W4 Live-ops** | guild/clan (D6); event/season (G9); gacha/rương (G5) bằng tiền ảo; công cụ admin đầy đủ | 2–3 tháng |
| sau | cash/nạp tiền thật (G4): workstream riêng về thanh toán và tuân thủ | — |

**Tổng (3–5 người):** Release 1 khoảng 6–8 tháng; full feature thêm khoảng **10–14 tháng** (W2 song song W3).
Tổng cộng khoảng **1,5–2 năm**.

### 9.6 Định nghĩa "full feature xong"

- 520/520 opcode đã có quyết định PORT, REDESIGN hoặc DROP; mọi PORT có test.
- Mọi dialog/màn hình PORT có golden ảnh so với client gốc.
- Mọi số liệu kinh tế có tag nguồn; mọi số [DESIGN] được designer duyệt và đã qua playtest.

## 10. Bản KR: content profile `kr` trên cùng engine

Bản KR (patch Nexon `00967`) là cùng dòng engine, mở rộng nội dung: 13 pack, 51.467 entry, so với 4 pack của VN.
Engine **không fork**:
- `boom-assets` đọc pack bằng khoá tự thân (4 byte đầu `.idx`) cho cả hai bản;
- một bảng *content profile* (`vn`, `kr`) chọn pack, map và chế độ.

Client KR chưa vào được gameplay trong các lần chạy đã ghi (bị chặn ở `ProcessPatch`).
VN là nguồn tham chiếu chạy được, **chưa phải oracle đã xác nhận cho KR**.
Tên class/asset của mode trong VN không chứng minh tương đương hành vi KR; các mode ngoài V1
giữ trong backlog với mức bằng chứng riêng. Profile phải chứa cả khác biệt rules có version.
Chi tiết và việc cần làm (KR-R0-1..4, KR-W3-1/2, KR-UI): `boomkr-re/docs/rust-rewrite.md`.
