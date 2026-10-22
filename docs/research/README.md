# Tài liệu nghiên cứu Boom KR

**Quy tắc bắt buộc:** [AGENTS.md](../../AGENTS.md). Nguồn chuẩn claim: [RULES_V1](../specification/gameplay/RULES_V1.md);
sổ sai khác có chủ ý: [SCOPE_V1 §10](../planning/SCOPE_V1.md#10-sai-khác-có-chủ-ý-so-với-client-gốc).

| Đọc | Mục đích |
|---|---|
| [QUESTIONS](QUESTIONS.md) | Câu hỏi mở: còn thiếu gì, ai phụ trách, cách đóng |
| [ORACLE_REPORT](ORACLE_REPORT.md) | Báo cáo đối chiếu oracle KR/VN ↔ engine (V1-037) |
| [findings](findings/README.md) | Kết quả RE tìm được: chỉ số, địa chỉ, giới hạn công cụ |
| [sources](sources/README.md) | Sổ nguồn web Hàn–Trung: URL, tiêu đề, ngày truy cập, region/build |

## Báo cáo oracle

[ORACLE_REPORT](ORACLE_REPORT.md) là file **sinh tự động** từ `fixtures/oracle/*.toml`
(`cargo xtask oracle`); không sửa tay. Mỗi file oracle khai nguồn gốc, claim, bằng chứng, đại
lượng, đơn vị, kỳ vọng và dung sai — thiếu thì validator báo lỗi. Sai khác engine so với nguồn
gốc mà **chưa** có mục ở SCOPE_V1 §10 là lỗi của `cargo xtask ci`; sai khác đã ghi nhận và
kỳ vọng chưa có số chỉ được báo trong báo cáo.

Báo cáo là **đối chiếu**, không phải bằng chứng mới: nó không nâng tầng bằng chứng của claim và
không thay thế kịch bản nghiệm thu trong `fixtures/scenarios/`.
