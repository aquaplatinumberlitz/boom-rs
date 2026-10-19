# 0002 — VN và KR là hai nguồn tin cậy ngang nhau

Status: **Accepted**, 2026-10-19, theo chỉ đạo trực tiếp của chủ dự án:
“nguồn VN chính là trust source ngang nguồn KR, cứ lấy dùng”.
Thay phần chính sách VN chỉ hỗ trợ tham chiếu trong [0001](0001-project-baseline.md).

## Quyết định

Dùng trực tiếp kết quả RE/runtime VN đã xác nhận để chốt luật chung và làm đầu vào engine mới.
Không yêu cầu kiểm lại trên KR chỉ vì nguồn là VN. Không hạ độ tin cậy của bằng chứng do region VN.
KR vẫn là target content 00967; giữ nguồn/build/commit để truy vết và xử lý khác biệt thực tế.
Nếu có bằng chứng mâu thuẫn, ghi rõ khác biệt và chọn profile phù hợp; không tự đổi ID/map giữa hai bản.

Đây là chấp nhận nguồn, không biến suy đoán hoặc giá trị do emulator tự đặt thành luật gốc.
Unknown vẫn cần nghiên cứu hoặc quyết định DESIGN; không đổi nhãn quan sát VN thành quan sát runtime KR.
Không thay scope V1, authority server mới hay tiêu chí nghiệm thu tiến độ.

## Áp dụng

Kết quả trận mới được nhập ở [VN battle runtime](../research/findings/VN_BATTLE_RUNTIME_REFERENCE.md).
Cứu/chọc bong bóng và đồng bộ item được dùng ngay làm rule/scenario đầu vào.
Không chọn cách làm cũ “đợi lặp mọi phép thử trên KR”; chỉ kiểm thêm khi có unknown hoặc khác biệt cụ thể.
