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

Nghiên cứu KR chuyên sâu hiện ở `/home/admin/boomkr-re/docs/`; xem `kr.md`, `kr-content.md`,
`kr-data.md`. Đây là repo tham chiếu ngoài worktree; không copy code hoặc coi mọi kết luận cũ đã được audit.
Manifest/probe trong [evidence](../../evidence/README.md) là dữ liệu đã quét trực tiếp cho baseline hiện tại.

Đường dẫn `tools/`, `server/`, `logs/`, `win/` trong các findings VN là của repo `/home/admin/boom-re`
(hoặc `boom-re-l10n`), không phải repo này.

Kết quả mới gắn với ID trong [QUESTIONS](../QUESTIONS.md), có nguồn, kết luận và giới hạn.
Chỉ thêm một file khi chủ đề độc lập; không sinh file tổng kết theo agent/phiên.
| [KR_RE_TRAPPED_SPEED](KR_RE_TRAPPED_SPEED.md) | Truy vết `(dir,len)` khi bị nhốt: chuỗi movement là của `CGOPet`/AI, không phải bomber (R-134, R-135) |
