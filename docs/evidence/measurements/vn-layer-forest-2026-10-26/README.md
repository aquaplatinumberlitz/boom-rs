# VN F-09a — Forest01 layer probe (metadata #357)

Parent task branch: `da6c1cb`; baseline lúc nhận issue: `e6e3d239532e42c3ea72b59007a6f6dfeb0aff98`
(mốc tiếp quản `dc5241b4fb8c0203bf89142b033e0c26a82926d3`). Không sửa code/profile/FIDELITY/ledger.

## Command/config
- Prefix/game sao chép `vn-grace-240`; Xvfb `-displayfd` + Mesa llvmpipe; Wine offline.
- Server: `ALL_MAPS=1`, `BOOM_DIR=/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/game`,
  `MAP_INDEX=1`, `ROOM_TYPE=19`; server `BATTLE_MAPS[1]` theo `all_battle_maps()` là
  `Forest01/Forest/Forest01.map`.
- Video `capture.mkv` 1024x768, 640 frame, PTS0–21,967 s (~29,13 fps; nominal60), ffv1;
  SHA `e2df6afba1c32d0c3767e1b665ff583c05174c9febc397cbcf6e573c3ffc0f2c`.
- Packet `S->C opcode 113` lúc 10:29:48 trong `packets.log`; body
  `4b0096ca539f7f05903af701594cf6fd39`.
- Raw/chỉ ảnh local `/home/admin/boom-team/shots/fidelity/F-09/forest01/` (không commit);
  contact `/home/admin/boom-team/shots/fidelity/F-09/forest01/analysis/contact.png`
  SHA `040c6b852140d812e2d7d5161a3dc2410f6e4db45c7a056546291b4454cbed22`.

## Kết quả/quan sát
- Video 0,5–15 s: nhân vật đi Up/Down lặp lại trong khu **Factory-style**, không thấy cây/nhà cao
  ở đường đi; không có cặp ảnh trước/sau một vật cản cao.
- Vì vậy **không có overlap actor–vật cản** để đo foot-y/layer; không kết luận nhân vật trước/sau.
- Map identity client chưa xác nhận: server gửi index1 (Forest01) nhưng ảnh không có vật cản cao
  đặc trưng; cần kiểm map entry/loader trước khi dùng cảnh này. Không đổi map lần nữa.

## Cleanup
Harness tự kết thúc; kiểm không còn `fidelity357-forest`, `ca_fixed_patched`, `winedbg`,
`vn149-factory-server` hay Xvfb do task tạo. Các Xvfb cũ (81–88/94/95/124) không phải task này và
không bị kill. Không dùng wineserver-k/pkill.
