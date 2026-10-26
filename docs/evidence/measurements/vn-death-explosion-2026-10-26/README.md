# VN F-06/F-07 — explosion/main và death hold (metadata)

Video local `/home/admin/boom-team/shots/320/claude-r1/capture.mkv` (ffv1, 1024x768, không commit);
SHA `e3610325129d9ee3bb0425a98cf2af8a7d454fde6c1da01c3e9f2002b5a329a5`. PTS thực tế ~29,65 fps
(890 frame/29,98 s), stream khai60. Ảnh crop local `F-07/analysis/death-contact.png`.

## F-06
- R-084/ODT: chỉ một clip `explosion.main` 10 frame/8 ảnh, không tách tâm/nhánh; `Bomb/Explosion` không có
  clip hướng riêng. Video có hiệu ứng nước/lửa ở ô bên trái và ô phía trên ô bom; không phân biệt được instance `explosion.main` riêng trên từng nhánh hay chỉ hiệu ứng tổng, nên không kết luận “không có nhánh”.
- Chưa chứng minh effect được tạo cho từng ô lửa hay chỉ ô bom; `CGOBombExplosionEffect` ctor có tên
  `BombExplosion%03d` nhưng chưa nối call site/vị trí. Thời lượng 10 frame vẫn là **design 100 ms/frame**,
  video không đủ mẫu để chốt.

## F-07
- Death bắt đầu khoảng19,2 s; xác còn thấy tới khoảng21,0 s; **không còn** ở21,5/22,0 s. Không thấy
  mờ dần; biến mất sau khoảng1,8–2,3 s. 14 frame bundle; chưa đo pattern lặp frame/duration.
- V1 đang giữ `last_visible_frame` vô hạn; video không ủng hộ giữ vĩnh viễn.
