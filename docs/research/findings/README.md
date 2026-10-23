# Findings — kết quả kỹ thuật hiện có

Các tài liệu dưới chủ yếu mô tả **client VN** và hạ tầng tham chiếu. Mỗi phép áp dụng sang KR phải
ghi provenance; VN là [trusted source ngang KR](../../decisions/0002-vn-kr-trusted-sources.md), dùng trực tiếp cho luật chung.

| Tài liệu | Dùng khi |
|---|---|
| [VN_BATTLE_RUNTIME_REFERENCE](VN_BATTLE_RUNTIME_REFERENCE.md) | Luật cứu/chọc, đồng bộ item, chín map, room type/RPG và tác động tới task hiện hành |
| [VN_LEGACY_SERVER_REFERENCE](VN_LEGACY_SERVER_REFERENCE.md) | Server giả lập VN: phần client-derived dùng trực tiếp, phần emulator tự đặt, kỹ thuật tận dụng |
| [VN_ENGINE_DIFF](VN_ENGINE_DIFF.md) | Đối chiếu từng cơ chế V1 với server giả lập VN, nhãn nguồn và scenario pending cho khác biệt đo được |
| [VN_FEATURES_REFERENCE](VN_FEATURES_REFERENCE.md) | Tra chức năng/opcode VN; không phải catalog KR |
| [VN_RUNTIME_REFERENCE](VN_RUNTIME_REFERENCE.md) | Tra cách chạy Wine/WinBoat và giới hạn snapshot cũ |
| [VN_DEVIRT_STATUS](VN_DEVIRT_STATUS.md) | Tra các site/patch và giới hạn RE VN |
| [DEVIRT_METHOD](DEVIRT_METHOD.md) | Tra phương pháp điều tra khối VM |
| [VM_METHOD](VM_METHOD.md) | Tra hạ tầng snapshot/emulation |
| [WEB_RULES_NAMU](WEB_RULES_NAMU.md) | Luật chơi/thông số trích từ namu (item, mode, monster, EXP, map, strategy) — candidate, chưa đối chiếu build |
| [OSS_REIMPLEMENTATIONS](OSS_REIMPLEMENTATIONS.md) | Hằng số gameplay từ bản remake mã nguồn mở (fuse/trap/tick/speed px/s/drop weight) — candidate |
| [KR_RE_MOVEMENT](KR_RE_MOVEMENT.md) | RE tĩnh KR 00967: movement, hộp bàn chân 40×40, neo toạ độ, trợ góc, cách đối chiếu VN↔KR (R-119…R-125). Phạm vi lớp: consumer đã trace là `CGOPet`/AI; R-119…R-122 confirmed chỉ cho `CMovable`, bomber là candidate/chưa chứng minh (R-134/R-135, PR #185 pending). **R-154**: predicate `CGOBomb` +156 `B37DB0` (`0` nếu `+328`∈{1,2} hoặc flag `+152`=0); `MoveSimple` consumer pet/AI, còn OR `EA0FD0`/gate +164; 4/14 caller owner dataflow descriptor/effect; chưa thấy collision trực tiếp, 10 unknown; player grace/timer unknown |
| [KR_RE_TRAPPED](KR_RE_TRAPPED.md) | RE tĩnh KR 00967: số state/chuyển state và phạm vi lớp `CGOPet`/bomber của chuỗi cổng–mover, nhóm state khoá, chuỗi chết (R-126…R-131); nhãn `3` = "bị nhốt" của pet là candidate |

Nghiên cứu KR chuyên sâu hiện ở `/home/admin/boomkr-re/docs/`; xem `kr.md`, `kr-content.md`,
`kr-data.md`. Đây là repo tham chiếu ngoài worktree; không copy code hoặc coi mọi kết luận cũ đã được audit.
Manifest/probe trong [evidence](../../evidence/README.md) là dữ liệu đã quét trực tiếp cho baseline hiện tại.

Đường dẫn `tools/`, `server/`, `logs/`, `win/` trong các findings VN là của repo `/home/admin/boom-re`
(hoặc `boom-re-l10n`), không phải repo này.

Kết quả mới gắn với ID trong [QUESTIONS](../QUESTIONS.md), có nguồn, kết luận và giới hạn.
Chỉ thêm một file khi chủ đề độc lập; không sinh file tổng kết theo agent/phiên.
| [KR_RE_TRAPPED_SPEED](KR_RE_TRAPPED_SPEED.md) | Truy vết `(dir,len)` khi bị nhốt: đường đã trace thuộc `CGOPet`/AI; R-136 nối dispatch dir→field `+1440`→reader `0xB40D60`; R-137 direct caller setter là wrapper NPC, `B479E0` lớp sử dụng `unknown`; R-138 opcode 183→`CGOGhost`→setter `0xB334C0` (packet→setter), input dir→position player `unknown` (R-134…R-138) |
| [KR_RE_UI](KR_RE_UI.md) | **R-152**: hitbox 6 ô room list (`549180`, bảng `19D8A58`, công thức left/top/width/height, UI px); PrepareStage `6F47C0` Ready `512,494`/key `PRE_Bt_Ready`; native/scale/focus, login/result, C00/C01→tên còn blocker |
| [KR_RE_FLAME_CHAIN](KR_RE_FLAME_CHAIN.md) | RE tĩnh KR 00967: client tự tính vùng nổ (`0x00E9C520`, ô chặn thuộc vùng và tia không xuyên), dây chuyền (`0x00E9CDC0`), ô nổ là message id 27, thời gian lửa = số frame animation (`GetAnimCount`), không có damage timer ms; R-139…R-141, §17 |
| [KR_RE_BOMBERATTR](KR_RE_BOMBERATTR.md) | **R-150 blocker** BomberAttr; **R-153** ItemAttr loader `1196A70`: item id `+84`/lookup `1197900`, group stride `0xF4`, bảng 0x8000 fill group index; **chưa thấy reader trong phạm vi literal**, 7 xref/7 thân đã đọc (1192700 đủ 227 dòng); 3 caller còn lại chỉ id/model/ability, chưa thấy row data; client drop vs server placement unknown |
| [KR_RE_TRAP_TIMER](KR_RE_TRAP_TIMER.md) | RE tĩnh KR 00967: bộ đếm bomber `+0x2748` giảm 1/update, `≤0` → slot 202 `0xB7A760` gửi C→S packet 167; status packet 184 mang byte timer; seed/entry nằm ngoài code native (thunk VM `0xB534D0 → 0x02012376`); R-200…R-202 |
| [KR_RE_RENDER](KR_RE_RENDER.md) | RE tĩnh VN→KR render: accessor `CDataI16Alpha` (vf8 trả f4..f11; vf9/10 rộng/cao; vf11/12/13=f0/f1/f2; vf14 ghi f0/f1), neo tile `x=cột·40`, `y=hàng·40+39`; tên lớp `LayerStatic`/`LayerAdjust` + loader ghép `LayerStatic` (VN/KR), cặp sprite theo object chưa chứng minh; **chưa có consumer f8–f11/thứ tự/clip/alpha** (R-209…R-211, blocker) |
| [KR_RE_AUDIO](KR_RE_AUDIO.md) | RE tĩnh KR 00967: consumer âm thanh theo event — bảng 649 entry id→`Wave/*`, dispatcher `0xD1CD10`, nổ 7/8 (+343 tuyết), nhốt 77/37 (+444), chết 42, item 68/69, trap 174; biến thể round-end/start `rand()%N+base`; BGM record map → `Mp3/%s` (Village id 48); pool voice dedup, không ưu tiên (R-203…R-208) |
| [KR_RE_ANIM_STATE](KR_RE_ANIM_STATE.md) | **R-151**: `BA3530` đã thấy nhánh so `[record+4]`, reset, tiến/giữ-cuối/quay-đầu (mã 1/2/3/4/5); blocker còn đơn vị/cadence/getter, state→nhãn, hướng lửa (R-141 giữ frame cho lửa) |
