//! Phòng chờ và vòng đời lượt — V1-004…V1-010, V1-022, V1-023 (phần policy).
//!
//! Crate thuần như `boom-core`: nhận lệnh đã xác thực danh tính + thời gian logic từ server,
//! trả về state mới và event; không socket, không đồng hồ thật, không lưu trữ. Server (P7) bọc nó.
//! Policy nguồn: SCOPE_V1 §3 (DESIGN của dự án); hành vi client gốc tham khảo RULES_V1 §7.
//!
//! Chưa triển khai: owner của task backend thiết kế API (lệnh, event, lỗi) trong crate này,
//! ghi vào ARCHITECTURE §5.4 qua integrator, và phủ các kịch bản SCOPE_V1 §7 bằng test.
