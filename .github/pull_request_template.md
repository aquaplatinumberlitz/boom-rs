<!-- Một PR = một task. Điền đủ; reviewer từ chối PR thiếu mục. Quy tắc: AGENTS.md, TEAM_WORKFLOW.md. -->

**Task:** #<!-- issue --> · **Feature ID:** <!-- V1-0xx hoặc Q-0xx / không có --> · **Contract:** contracts-r1

## Thay đổi
<!-- Ngắn gọn: làm gì, vì sao. -->

## Phạm vi file
<!-- Chỉ các file/thư mục task cho phép. Có đụng file dùng chung (state/, event.rs, rules.rs, mode.rs,
Cargo.toml, features.json, execution.json)? Nếu có: lý do + integrator đã đồng ý ở đâu. -->

## Luật và bằng chứng
<!-- Claim RULES_V1 dùng/thêm (R-xxx), nhãn nguồn/trạng thái. Giá trị nào là design tạm? -->

## Kiểm chứng
- [ ] `cargo xtask ci` xanh trên máy (dán dòng cuối)
- [ ] Scenario của feature chuyển `active` và qua; đã thêm ca biên
- [ ] Đổi hành vi core có chủ đích → đã `cargo xtask bless` và giải thích ở đây
- [ ] Docs/nguồn chuẩn cập nhật cùng PR (RULES_V1, QUESTIONS, ARCHITECTURE nếu đổi contract)

## Còn thiếu / unknown
<!-- Điều chưa làm, giả định, câu hỏi cho reviewer. -->
