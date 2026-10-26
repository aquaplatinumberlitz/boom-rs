# VN F-03 — Start/countdown (metadata #326)

Video local `/home/admin/boom-team/shots/fidelity/F-03/capture.mkv` (ffv1, 1024x768, 14 s,
không commit); SHA `0e350c8a500e8a6aa90f6724135827f811bfa854d04ad236b523c20991739c5d`
(đối chiếu hash thực tế lần chạy). Stream khai 60 fps, PTS thực tế ~?; ảnh chỉ local.

## Quan sát
- Vào trận (sau stage13) C00 đã được vẽ ngay từ frame đầu; không có khoảng “chưa vẽ nhân vật”.
- Một **bàn tay đỏ lớn** xuất hiện phía trên nhân vật khoảng1,5–3,5 s; overlay **START!!** xuất hiện
  khoảng4,0 s và giữ tới hết capture. Nhân vật phía dưới vẫn đứng; không thấy clip `c00.start.*`
  mười frame tách biệt để gán.
- Khoảng5,0 s nhân vật có silhouette xám dưới mũi tên; chưa đủ để gọi là Start hay shadow.
- Không thấy clip `c00.start.*` 10 frame trước/dưới overlay; không xác định được Start phát lúc nào/bao lâu/lặp. Không coi `PresentationKind::Start` playback là confirmed; chỉ đổi khi có consumer.
