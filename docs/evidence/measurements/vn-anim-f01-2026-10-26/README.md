# VN F-01 — Walk loop, đổi hướng, Blink (metadata #326)

Video local `/home/admin/boom-team/shots/fidelity/F-01/capture.mkv` (ffv1, 1024x768, không commit).
SHA `b092448b89721364f7fda93b23fcbddd6f53a64cadd822c99d3a194cd64ac026` — cần đối chiếu hash thực tế
trong lần chạy; stream khai60 fps, PTS thực tế ~24,6 fps (663 frame/26,967 s), không dùng nominal.
Capture local, Xvfb/Mesa llvmpipe, fake server VN, prefix/game bản sao `vn-grace-240`.

## Quan sát
- S-01: idle2s; Left/Right/Up/Down mỗi hướng0,6s nghỉ1,5s; đổi hướng liên tục2s (0,25s/key);
  đứng yên8s.
- Walk-left crop 2,05–2,60s có 5 pose lặp; contact local `analysis/idle-contact2.png` cho thấy chuỗi
  5 pose đi theo một chiều rồi lặp, không thấy đảo ngược.
- Sau chuỗi đổi hướng nhanh, actor đứng yên giữ hướng cuối; frame mắt nhắm đầu tiên ở khoảng4,1s
  sau idle (16,9s video), mở lại khoảng5,1s; không thấy BlinkSmall trong8s.
- Đổi hướng giữa lúc đi: frame quanh các mốc 10,2–12,4s không đủ ổn định/đồng nhất để phân biệt
  reset frame0 hay giữ index; không claim.
