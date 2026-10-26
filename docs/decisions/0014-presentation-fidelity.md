# 0014 — Fidelity trình diễn: không tự đặt luật phát, đối chiếu với client gốc

Status: **Accepted**, 2026-10-26, quyết định trực tiếp của chủ dự án.
Tác giả: Claude (integrator). Bổ sung [ADR 0013](0013-rule-source-precedence.md) cho phần trình diễn;
không đổi ưu tiên nguồn luật trận.

## Bối cảnh

Issue #320: chủ dự án chơi thử thấy sprite và animation khác client gốc. Có hai lỗi:

- Đường mạng luôn coi nhân vật là đứng yên. Đây là bug code, đã sửa ở #321.
- Bóng nhốt lặp frame 3–8 nên trông như vỡ đi vỡ lại. Lỗi này đáng chú ý vì mapping asset đã có
  đủ: `trapped.bubble` = `Bubble00`, 9 frame, offset đúng.

Nguyên nhân gốc của lỗi bóng:

1. Pack client gốc chỉ cho biết **có những hình nào**. Nó không chứa thứ tự phát, thời lượng,
   đoạn lặp hay pha theo state (R-079). Logic phát nằm trong code consumer của client và chưa được
   RE (Q-003).
2. `Bubble00` gộp ba thứ khác nhau: lớn dần (0–2), vỡ (3–5), bóng tròn và lấp lánh (6–8).
   Nhìn danh sách frame không thể suy ra điều đó.
3. Khi sửa #305, luật "lớn dần 3 frame rồi lặp phần còn lại" được **tự đặt ra** mà không xem
   client gốc. Trạng thái `candidate` của mapping không chặn việc đó.
4. Review và test chỉ kiểm tra code có nhất quán với luật tự đặt hay không. Không có bước so với
   hình ảnh của client gốc.

## Quyết định

Áp dụng cho mọi hành vi **trình diễn**: chọn clip theo state, thứ tự frame, thời lượng, lặp/giữ,
pha, độ đục, vị trí/neo, thứ tự chồng lớp, hiệu ứng, âm thanh theo sự kiện, cảm giác di chuyển,
UI/UX và chuyển cảnh.

1. **Thứ tự nguồn:**
   - Tầng 1: RE consumer phía client KR 00967 (`kr_static`/`kr_runtime`) hoặc VN (`vn_static`).
   - Tầng 2: đo/quay client VN chạy bản sao offline (`vn_runtime`): video, ảnh, gdb đọc bộ đếm.
     Nguồn này được chấp nhận là gần chính xác với KR (ADR 0002).
   - Tầng 3: `design`, chỉ khi tầng 1–2 chưa có.

   Một câu hỏi RE kẹt quá 2 lượt dsh thì chuyển sang tầng 2, không chờ.
2. **Không tự đặt luật trình diễn.** Không được suy luật phát từ tên entry, số frame hay "hợp lý".
   Giá trị tầng 3 phải ghi `design` trong dữ liệu/claim và nằm trong bảng nợ ở
   [FIDELITY](../specification/FIDELITY.md) §3. Không được trình bày như đã khớp bản gốc.
3. **Luật phát là dữ liệu có nguồn.** Gồm pha, dải frame, thời lượng, lặp/giữ, alpha và lớp. Mỗi
   luật có claim R-xxx và trạng thái. Code client đọc theo luật đó. Khi chưa chuyển hết sang dữ
   liệu thì hằng số trong code phải trỏ tới claim.
4. **Cảnh đối chiếu bắt buộc.** PR đổi hình ảnh, chuyển động, hiệu ứng, thời gian hoặc UI phải
   chạy cảnh tương ứng trong [FIDELITY](../specification/FIDELITY.md) §2 trên cả client VN và
   boom-rs, rồi ghi so sánh. Ảnh/video **chỉ lưu local**, vì repo PUBLIC. PR ghi đường dẫn local,
   kết luận và sai lệch còn lại. Reviewer xem ảnh so sánh, không chỉ đọc code.
5. **Test là regression, không phải bằng chứng fidelity.** Test sinh từ implementation không chứng
   minh khớp bản gốc (AGENTS §4).
6. Giữ nghi thức gọn theo quy trình RE 26/10: issue nhỏ, tối đa 2 lượt dsh, một review độc lập
   mỗi PR, và một lần chạy lại nếu môi trường hỏng.

## Phương án đã cân nhắc

- Chỉ RE: chính xác nhất, nhưng consumer animation đã kẹt nhiều ngày (Q-003). Không chọn làm
  đường duy nhất.
- Chỉ quay video: nhanh, nhưng sai số khoảng ±33 ms (capture thực tế ~30 fps dù khai 60) và không thấy logic ẩn. Chỉ dùng làm
  tầng 2.
- Giữ cách cũ (mapping + design tự đặt): đã gây #320. Loại.

## Hệ quả

- Rà lại mọi clip đang phát theo luật tự đặt: F-01…F-11, issue #326–#336.
- Thời gian tới khi một tính năng trình diễn được coi là xong tăng thêm một bước quay/so sánh.
  Đổi lại, sai lệch được phát hiện trước khi chủ dự án chơi thử.
- `presentation.frame_duration_ms = 100` (R-118) vẫn là `design` cho tới khi có nhịp per-clip từ
  tầng 1–2.

## Bằng chứng và giới hạn

- Video VN 26/10: `/home/admin/boom-team/shots/320/claude-r1/capture.mkv` (local), đọc ở R-198/R-199
  (#323) và #324 — hai PR claim đang chờ tích hợp. R-198 khớp frame cuối Walk chỉ đối chiếu ở Left. Thời lượng các pha của bóng có sai số khoảng ±33 ms.
- Công cụ: script quay `/home/admin/boom-team/shots/320/claude-r1/run.py` (Xvfb `-displayfd`,
  server giả lập `vn149-factory-server.py`, prefix bản sao). Không chạy server thật, không bypass
  anti-cheat.

## Điều kiện đảo

Đảo bằng ADR mới nếu chủ dự án chấp nhận lệch bản gốc cho một nhóm tính năng, ví dụ UI hiện đại
hóa có chủ ý. Khi đó nhóm đó ghi `design` có lý do, các nhóm khác giữ ADR này.
