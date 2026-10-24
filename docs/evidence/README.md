# Evidence — dữ liệu có thể đối chiếu

| Artifact | Nội dung và giới hạn |
|---|---|
| [measurements/vn-item-effects-2026-10-24/README.md](measurements/vn-item-effects-2026-10-24/README.md) | VN offline: Bomb1→2, Firefieldr18 1→2; Factory01 trần hữu hiệu10 qua3 lượt/đặt lại sau151;210rawhash. Stat/pool/KRidentity vàRoller còn thiếu |
| [measurements/vn-trapped-input-2026-10-24/README.md](measurements/vn-trapped-input-2026-10-24/README.md) | Hai capture VN cùngactor xác nhận trapped vẫn đi chậm theo phím ngang;17px/s là calibration DESIGN, bombgate/hìnhhọc bubble chưa biết |
| [kr_feature_inventory.json](inventory/kr_feature_inventory.json) | CA.exe + 13 IDX/IDD hash, 51.467 entry; không xác nhận gameplay |
| [kr_feature_candidates.csv](inventory/kr_feature_candidates.csv) | 46 nhóm matcher, số khớp và ví dụ; asset signal, không runtime |
| [kr_asset_groups.csv](inventory/kr_asset_groups.csv) | Toàn bộ nhóm tên tài nguyên cấp đầu |
| [kr_scope_groups.csv](inventory/kr_scope_groups.csv) | Export disposition của 46 nhóm từ registry nguồn |
| [kr_v1_content.json](inventory/kr_v1_content.json) | 103 payload được fingerprint và probe Village00; chưa đóng đủ dependency |
| `images/vn/` | Ảnh client VN được chuyển nguyên file từ docs root |
| [measurements/vn-timing-2026-10-19/README.md](measurements/vn-timing-2026-10-19/README.md) | Hồ sơ đo tick/tốc độ/timer nhốt client VN; log trích + script phân tích, không có asset game |
| [measurements/kr-static-trapped-2026-10-21/README.md](measurements/kr-static-trapped-2026-10-21/README.md) | Định danh build + bảng VA/RVA + evidence_id của phép đọc tĩnh trapped KR (R-126…R-131); metadata, không có binary hay decompile |
| [measurements/kr-static-flame-2026-10-21/README.md](measurements/kr-static-flame-2026-10-21/README.md) | Nổ/lửa + frame KR: **R-141** xác nhận lửa đếm frame (`B711C0`/GetAnimCount); **R-151** thấy nhánh `BA3530` so `[record+4]`/reset/tiến/giữ-cuối/quay-đầu; blocker còn đơn vị/cadence/getter, state→nhãn, hướng lửa |
| [measurements/kr-static-ui-2026-10-23/README.md](measurements/kr-static-ui-2026-10-23/README.md) | UI V1 R-152: hitbox 6 ô room list (`549180`/bảng `19D8A58`, UI px); PrepareStage `6F47C0` Ready `512,494` + key `PRE_Bt_Ready`; native/scale/focus/C00-C01 name còn blocker |
| [measurements/kr-static-2026-10-21/README.md](measurements/kr-static-2026-10-21/README.md) | Định danh build KR/VN và bảng VA/RVA của phép đọc tĩnh movement/va chạm (R-119…R-125; R-154 predicate `B37DB0`/`+328`/`+152` (consumer MoveSimple pet/AI; 4/14 owner caller đã đọc (descriptor/effect; chưa thấy collision trực tiếp; 10 unknown); player unknown)); metadata, không có binary hay decompile. Phạm vi lớp: consumer đã trace là `CGOPet`/AI (R-134/R-135); R-119…R-122 `confirmed` chỉ cho `CMovable`, bomber là `candidate`/chưa chứng minh |
| [measurements/kr-static-item-2026-10-21/README.md](measurements/kr-static-item-2026-10-21/README.md) | Điều tra item/cap KR: bộ nạp `ItemAttr` + cấu trúc bảng; **R-150 blocker** BomberAttr (loader f0..f10/kẹp f5≤10, không consumer f2–f7, E9E660 đã đọc hết, không thấy cộng stat; Roller≠Shoe); **R-153** loader `1196A70`, item id `+84`/lookup `1197900`, bảng group `0xF4` + bảng 0x8000; **chưa thấy reader trong phạm vi literal** (7 xref/7 thân đã đọc; 1192700 đủ 227 dòng; 3 caller còn lại chỉ id/model/ability) |
| [measurements/kr-static-trapped-speed-2026-10-22/README.md](measurements/kr-static-trapped-speed-2026-10-22/README.md) | Truy vết `(dir,len)` khi bị nhốt: `CGOPet`; R-136 dispatch dir→sub-object `+1412`→reader; R-137 direct caller setter là wrapper NPC; `B479E0` lớp sử dụng `unknown`; quét 5 site slot 95 + packet VN 176/KR 183 dùng slot `+116`, input dir→position player `unknown`; R-138 packet KR 183→`CGOGhost`→`0xB334C0` (40 px), C→S 148/149 candidate; bảng kiểm, `evidence_id`, giới hạn quét cục bộ |
| [measurements/kr-static-flame-chain-2026-10-22/README.md](measurements/kr-static-flame-chain-2026-10-22/README.md) | Đọc tĩnh nổ lan/chặn/dây chuyền KR: `0x00E9C520`/`0x00E9CDC0`, message id 27 cho từng ô, thời gian lửa = số frame `GetAnimCount`; claim R-139…R-141; bảng VA/RVA, `evidence_id`, giới hạn (chưa có handler gói nổ/damage window) |
| [measurements/kr-static-trap-timer-2026-10-23/README.md](measurements/kr-static-trap-timer-2026-10-23/README.md) | Bộ đếm bomber `+0x2748` (predicate `0x006833D0`, update `0x00B44000`, slot 202 `0x00B7A760` → packet 167, status 184); quét 10 site displacement, thunk VM `0x00B534D0 → 0x02012376`; claim R-200…R-202; bảng VA/RVA, `evidence_id`, giới hạn |
| [measurements/kr-static-render-2026-10-23/README.md](measurements/kr-static-render-2026-10-23/README.md) | Render VN→KR: accessor `CDataI16Alpha` (vf8 f4..f11, vf9/10 rect, vf11/12/13 f0/f1/f2, vf14 ghi f0/f1), neo tile `x=40·cột`/`y=40·hàng+39`, tên lớp `LayerStatic`/`LayerAdjust` + loader ghép `LayerStatic` (cặp theo object chưa chứng minh); quét consumer f8–f11 âm tính có giới hạn; **không có `evidence_id`** (REA timeout VN); claim R-209…R-211 |
| [measurements/kr-static-audio-2026-10-23/README.md](measurements/kr-static-audio-2026-10-23/README.md) | Consumer âm thanh KR: bảng 649 entry `0x19E40D8` (id→`Wave/*`), dispatcher `0xD1CD10`, biến thể `rand()` (18/21/43/47/64/78/128/175), nổ 7/8/343, nhốt 77/37/444, chết 42, item 68/69/174, BGM `0xE71D70` (Village id 48), pool voice `0x14BF4D0`; claim R-203…R-208; `evidence_id`, bảng VA/RVA, giới hạn |
| [measurements/vn-flame-window-2026-10-24/README.md](measurements/vn-flame-window-2026-10-24/README.md) | VN runtime FLAME phần **partial**: 4 case candidate (`late-050/300`, `visible-early`, `late-700`), bảng delay host-marker→`120` report (không TTL/one-shot/window); GDB handler fail, fixture devirt candidate |
| [measurements/rust-client-ux-2026-10-24/README.md](measurements/rust-client-ux-2026-10-24/README.md) | Kiểm Rust DESIGN: UI nearest1×/2×, NVIDIA/fullscreen, focus thật, rebind, BGM/SFX/volume/mute; giới hạn WM restore/đa OS/phần cứng, không oracle VN/KR |
| [measurements/vn-trap-timer-2026-10-24/README.md](measurements/vn-trap-timer-2026-10-24/README.md) | Native VN receiver/bubble counter200, ba baseline6332–6355ms, early resets giữ nguyên; calibration6350ms/191tickRust là DESIGN, raw hashes/negative/limits |
| [DOCS_MIGRATION.json](DOCS_MIGRATION.json) | Bản đồ đường dẫn cũ→mới, không phải bằng chứng luật chơi |

Không commit client/pack/binary/database hoặc cache ảnh lớn vào repo chỉ để tạo evidence.
Trace/capture mới phải có build/hash, điều kiện đầu, cấu hình server giả lập và giới hạn phép đo.
Tách dữ liệu nguồn, phân tích và kết luận; fixture sinh từ implementation không được gọi là oracle gốc.

Tái tạo từ gốc worktree:

```bash
# Tool đọc pack ở repo l10n (ADR 0003); ghi kết quả vào repo này
cd /home/admin/boom-re-l10n
python3 tools/kr_feature_inventory.py /home/admin/boom-re/boomKR --out /home/admin/boom-rs/docs/evidence/inventory --hash-payloads
python3 tools/kr_v1_content.py /home/admin/boom-re/boomKR /home/admin/boom-rs/docs/evidence/inventory/kr_v1_content.json
cd /home/admin/boom-rs
python3 tools/kr_catalog.py
python3 tools/kr_catalog.py --check
```

Quét inventory không ghi đè scope nguồn ở `docs/specification/features.json`.
