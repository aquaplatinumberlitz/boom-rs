# Specification — nguồn dùng cho thiết kế và triển khai

Catalog: [FEATURE_CATALOG](FEATURE_CATALOG.md), [features.json](features.json) nguồn sửa tay,
[features.csv](features.csv) export. Scope hiện hành: [SCOPE_V1](../planning/SCOPE_V1.md).
Trạng thái done/chưa done và tỷ lệ so với catalog: [PROGRESS](../planning/PROGRESS.md), generated từ
implementation/tracking trong JSON nguồn. `done/37` cho V1, `done/112` cho baseline catalog; hai drop loại riêng.

| Chủ đề | Tài liệu | Mức sử dụng |
|---|---|---|
| Pack/sprite | [PACKS](assets/PACKS.md) | RE VN tham chiếu; xác nhận lại semantics khi áp KR |
| Map | [MAPS](assets/MAPS.md) | Chủ yếu VN; probe Village00 KR có ghi chú offset riêng |
| Item | [ITEMS_VN_REFERENCE](gameplay/ITEMS_VN_REFERENCE.md) | Bảng VN, không tự đổi thành rules KR |
| Protocol cũ | [LEGACY_VN_PROTOCOL](networking/LEGACY_VN_PROTOCOL.md) | Dùng oracle/tham chiếu, không phải protocol bản Rust mới |
| Việt hóa | [l10n/README](../../l10n/README.md), [IMAGE_REVIEW](localization/IMAGE_REVIEW.md) | Catalog text/ảnh và quyết định review; giữ 23 text pending |

Chưa có gameplay spec KR hoàn chỉnh hoặc schema mạng v2 đã chốt. Các phần còn thiếu nằm trong
[QUESTIONS](../research/QUESTIONS.md); interface dự kiến nằm trong ARCHITECTURE, không giả làm code đã chạy.
Kết quả đã được xác nhận mới nhập vào spec; lịch sử quá trình thử nghiệm nằm trong archive.
