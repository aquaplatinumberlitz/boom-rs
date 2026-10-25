# Fidelity trình diễn — cảnh đối chiếu và bảng nợ

Quy trình: [ADR 0014](../decisions/0014-presentation-fidelity.md). Thứ tự nguồn: RE consumer KR/VN,
rồi đo/quay VN, rồi `design` có ghi nợ. Ảnh/video **chỉ lưu local** dưới
`/home/admin/boom-team/shots/fidelity/<F-id>/`, vì repo PUBLIC.

## 1. Công cụ

**Client VN (bản sao, offline)**
- Script mẫu: `/home/admin/boom-team/shots/320/claude-r1/run.py`. Script tự làm các việc sau:
  - mở Xvfb `-displayfd`, Mesa llvmpipe, bỏ `WAYLAND_DISPLAY`;
  - chạy server giả lập `/home/admin/boom-team/tmp/vn149-factory-server.py`;
  - dùng prefix/game bản sao `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/{prefix,game}`;
  - đăng nhập bằng xdotool, chờ CALog `dne:10…13` và S->C 191;
  - quay `ffmpeg x11grab` 60 fps ffv1, ghi mốc thao tác vào `actions.jsonl`.
- Không dùng `boom-re/tools/duo_vn.sh` / `match_play_vn.sh`: hai script này pkill Xvfb/wine dùng chung.
- Mỗi lần chỉ chạy một bản, vì dùng chung prefix.
- Biến server hay dùng: `BOMB_PLACE=1 BOMB_ECHO=<s>` (bom nổ sau s giây), `FLOOR_ITEMS`,
  `ITEM_ACK`, `DEATH_END`, `MATCH_END`.

**boom-rs**
- `boom-client --bundle <dir> --demo [--demo-death --death-frame N | --death-hold] --snapshot <png>`.
- Cảnh chưa có chế độ chụp (ví dụ giữa lúc bị nhốt): thêm cờ demo theo issue, không sửa tạm rồi bỏ.

**Tách khung và so sánh**
- `ffmpeg -ss <t> -i capture.mkv -vf "fps=10,crop=W:H:X:Y,scale=…:flags=neighbor,tile=10xN"`.
- Đối chiếu từng khung với frame trong bundle (`bundle.toml`: entry + chỉ số frame).

## 2. Cảnh đối chiếu

| Cảnh | Kịch bản (cùng trên VN và boom-rs) | Kiểm tra |
|---|---|---|
| S-01 Đi/đứng | Đứng 2 s; đi Left/Right/Up/Down mỗi hướng 0,6 s, nghỉ 1,5 s; đổi hướng liên tục 2 s; đứng 5 s | Clip/frame theo hướng, frame khi dừng, Blink, nhịp đi |
| S-02 Bom/nổ | Đặt bom, tránh ra, chờ nổ | Nhịp fuse, nổ tâm/nhánh, lửa, thời lượng, chồng lớp |
| S-03 Bị nhốt/chết | Đặt bom đứng yên (tự nhốt), chờ chết | Pha bóng, alpha, HELP!, tư thế trong bóng, vỡ, Die, xác |
| S-04 Cứu/thoát | Hai client: một bị nhốt, một chạm cứu | Hiệu ứng thoát bóng (khi có server hỗ trợ) |
| S-05 Item | `FLOOR_ITEMS` có 3 loại; đi nhặt | Hình item trên nền, hiệu ứng nhặt, âm thanh |
| S-06 Chồng lớp | Đi sau/trước nhà, cây, gạch; đứng trên bom; đứng trong lửa | Thứ tự vẽ theo y chân và loại |
| S-07 Mở/kết thúc lượt | Vào trận đến hết lượt/kết quả | Start, đếm ngược, kết quả, chuyển cảnh |
| S-08 Cảm giác di chuyển | Giữ phím 3 s, rẽ góc, trượt vào làn | Bước vị trí/khung, nội suy, corner assist |
| S-09 UI/UX | Đăng nhập → sảnh → phòng → trận → kết quả | Bố cục, font, nút, hover/click, chuyển cảnh |

## 3. Bảng nợ fidelity

`design` = đang dùng giá trị tự đặt; khi đóng issue thì đổi trạng thái theo claim.

| ID | Hành vi | Hiện tại | Trạng thái | Issue |
|---|---|---|---|---|
| F-01 | Walk lặp/ping-pong, đổi hướng, ngưỡng Blink | Lặp 0..4; dừng giữ frame cuối (R-198) | dừng `vn_runtime` candidate; còn lại `design` | #326 |
| F-02 | Tư thế trong bóng, chữ HELP! | FaintLayer lặp; không có HELP! | `design` | #327 |
| F-03 | Clip Start | Phát một lần theo sổ trình diễn | `design` | #328 |
| F-04 | Nhịp fuse 4 frame | Lặp đều 100 ms | `design` | #329 |
| F-05 | Chia Fire00, dải 28..59, motion blur | `FLAME_FRAMES = 28..=59` | `design` | #330 |
| F-06 | explosion.main | 10 frame một lần tại ô bom | `design` | #331 |
| F-07 | Die và xác | 14 frame, giữ khung cuối có ảnh (R-089) | một phần `design` | #332 |
| F-08 | Item trên nền, hiệu ứng nhặt | 1 frame tĩnh | `design` | #333 |
| F-09 | Chồng lớp đầy đủ | Sort y chân + ORDER (R-199 cho bom/bóng) | một phần `design` | #334 |
| F-10 | Cảm giác di chuyển | 30 Hz + nội suy | `design` | #335 |
| F-11 | UI/UX các màn hình | Theo ảnh VN từng phần | một phần `design` | #336 |
| — | Bóng nhốt: lớn dần 0–2, giữ 6 trong mờ, đục 600 ms, vỡ 3–5 | #325 | `vn_runtime` candidate (video ±33 ms) | #320 |
| — | Nhịp frame chung 100 ms | R-118 | `design` | #311 (đóng) |
