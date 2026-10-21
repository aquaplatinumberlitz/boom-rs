# Specification — nguồn dùng cho thiết kế và triển khai

Catalog: [FEATURE_CATALOG](FEATURE_CATALOG.md), [features.json](features.json) nguồn sửa tay,
[features.csv](features.csv) export. Scope hiện hành: [SCOPE_V1](../planning/SCOPE_V1.md).
Trạng thái done/chưa done và tỷ lệ so với catalog: [PROGRESS](../planning/PROGRESS.md), generated từ
implementation/tracking trong JSON nguồn. `done/37` cho V1, `done/112` cho baseline catalog; hai drop loại riêng.

| Chủ đề | Tài liệu | Mức sử dụng |
|---|---|---|
| Luật trận V1 | [RULES_V1](gameplay/RULES_V1.md) | Claim có nguồn/trạng thái; giá trị ở `data/rules/` |
| Pack/sprite | [PACKS](assets/PACKS.md) | RE VN, dùng trực tiếp (ADR 0002); khác biệt KR ghi theo build |
| Map | [MAPS](assets/MAPS.md) | VN + đối chiếu KR (header +4 byte, R-003) |
| Âm thanh | [AUDIO_V1](assets/AUDIO_V1.md) | Bảng event→entry, hash FxSound và policy chồng tiếng V1-034; mapping SFX là design; entry/mapping BGM confirmed R-081, loop là design |
| Item | [ITEMS_VN_REFERENCE](gameplay/ITEMS_VN_REFERENCE.md) | Bảng ItemAttr VN; claim dùng cho V1 ở RULES_V1 §5 |
| Protocol cũ | [LEGACY_VN_PROTOCOL](networking/LEGACY_VN_PROTOCOL.md) | Dùng oracle/tham chiếu, không phải protocol bản Rust mới |
| Replay | [REPLAY_V1](networking/REPLAY_V1.md) | Header content/rules hash, map ASCII/pack, golden V1-026 phần 1 |
| QA mạng | [QA_NETWORK](QA_NETWORK.md) | Hồ sơ mạng fixture V1-027 (RTT/jitter/loss/trùng/đảo thứ tự), cách đo độ trễ và kiểm hội tụ state |
| Phát hành | [RELEASE](RELEASE.md) | Bố cục gói, lệnh dựng bundle, lỗi thiếu file, yêu cầu hệ thống đã biết/chưa đo (V1-036 phần 1) |
| Việt hóa — chuỗi UI V1 | [UI_STRINGS_V1](localization/UI_STRINGS_V1.md) | Khóa UI V1 → locator catalog/ảnh, trạng thái nguồn; không lưu bản dịch; 23 text pending không dùng làm text đã chốt |
| Việt hóa — catalog/ảnh | `/home/admin/boom-re-l10n/l10n/README.md`, [IMAGE_REVIEW](localization/IMAGE_REVIEW.md) | Catalog text/ảnh và quyết định review; giữ 23 text pending |

Chưa có gameplay spec hoàn chỉnh hoặc schema mạng v2 đã chốt. Các phần còn thiếu nằm trong
[QUESTIONS](../research/QUESTIONS.md); interface dự kiến nằm trong ARCHITECTURE, không giả làm code đã chạy.
Kết quả đã được xác nhận mới nhập vào spec; lịch sử quá trình thử nghiệm nằm trong archive.
