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
  - quay `ffmpeg x11grab` ffv1, ghi mốc thao tác vào `actions.jsonl`. Stream khai 60 fps nhưng thực tế
    lấy mẫu khoảng **30 fps** (capture 26/10: 890 frame / 29,98 s); tính sai số theo PTS thực tế
    (`ffprobe -show_entries frame=best_effort_timestamp_time`), không theo fps khai báo.
- Không dùng `boom-re/tools/duo_vn.sh` / `match_play_vn.sh`: hai script này pkill Xvfb/wine dùng chung.
- Mỗi lần chỉ chạy một bản, vì dùng chung prefix.
- Biến server hay dùng: `BOMB_PLACE=1 BOMB_ECHO=<s>` (bom nổ sau s giây), `FLOOR_ITEMS`,
  `ITEM_ACK`, `DEATH_END`, `MATCH_END`.

**boom-rs**
- `boom-client --bundle <dir> --demo [--demo-death --death-frame N | --death-hold] --snapshot <png>`.
- Giữa lúc bị nhốt: `--demo-death --trap-at <tick>` (#340, chờ tích hợp). Cảnh khác chưa có chế độ chụp
  thì thêm cờ demo theo issue, không sửa tạm rồi bỏ.

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

`design` = đang dùng giá trị tự đặt; khi đóng issue thì đổi trạng thái theo claim. Cột "Hiện tại" ghi rõ
PR nào đổi hành vi; khi PR đó chưa merge thì hành vi trên main vẫn là bản cũ.

| ID | Hành vi | Hiện tại | Trạng thái | Issue |
|---|---|---|---|---|
| F-01 | Walk lặp/ping-pong, đổi hướng, ngưỡng Blink | Lặp 0..4. Khi dừng: main cũ dùng `idle.blink`; #325 đổi sang giữ frame cuối Walk theo hướng | dừng: `vn_runtime` candidate theo R-198 (#323, chờ tích hợp) — giữ hướng thấy ở 4 hướng ≥1,5 s, khớp frame cuối **chỉ đối chiếu ở Left**; còn lại `design` | #326 |
| F-02 | Tư thế trong bóng, chữ HELP! | FaintLayer lặp, chưa có thân. HELP! (`Bomb/HELP!`, 4 frame) lặp 100 ms, căn giữa trên bóng tròn, nằm trên cùng (#346) | thân: chưa định vị. HELP: entry/vị trí `vn_runtime` candidate; nhịp `design` | #327 |
| F-03 | Clip Start | Phát một lần theo sổ trình diễn | `design` | #328 |
| F-04 | Nhịp fuse 4 frame | Lặp đều 100 ms | `design` | #329 |
| F-05 | Fire00 theo vai trò ô và power; motion blur | Nhánh `feat/flame-odt`: 75 state ODT, tâm/C và L/U/R/D theo bom nguồn, tier1..10 (power>10 dùng10); chuỗi một lần từ tick nổ, giữ cuối tới core tắt lửa. Main chưa merge vẫn dải28..59. Đối chiếu local `/tmp/boom-flame-evidence/` (S-02) | Chuỗi `kr_static` confirmed R-251; đơn vị1/30s confirmed R-222; chọn tier candidate R-192/R-195. Motion blur và power2–3 runtime còn thiếu (Q-003/Q-019) | #330 |
| F-06 | explosion.main | 10 frame một lần tại ô bom | `design` | #331 |
| F-07 | Die và xác | 14 frame, giữ khung cuối có ảnh (R-089) | một phần `design` | #332 |
| F-08 | Item trên nền, hiệu ứng nhặt | 1 frame tĩnh | `design` | #333 |
| F-09 | Chồng lớp đầy đủ | Sort y chân + ORDER (R-199 cho bom/bóng) | một phần `design` | #334 |
| F-10 | Cảm giác di chuyển | 30 Hz + nội suy | `design` | #335 |
| F-11 | UI/UX các màn hình | Nhánh `feat/ui-vn-layout`: login R-227, lobby R-228, room R-229 theo visual rect VN 800×600; chưa tích hợp. Chi tiết nợ bên dưới | bố cục `vn_runtime` / `candidate`; phần chưa có asset/font/interaction là `design` | #336 |
| F-12 | Event→SFX và lịch Ready/Start | Nhánh `feat/audio-r191`: nổ id7/8 xen kẽ <200 ms; Ready18/Start21 thay Count1..3; mapping đã khớp giữ entry. Ready đầu warmup/Start hết warmup theo core V1 | Mapping `kr_static` candidate R-191; trapped giữ confirmed R-204. Lịch warmup R-113 và voice/coalesce/priority R-083 là design; native Stop7/8, guard remap và audible runtime còn Q-015 | #363 |
| — | Bóng nhốt | Main cũ: lớn dần 0–2 rồi lặp 3–8 (sai). #325: lớn dần 0–2, giữ 6 trong mờ, đục 600 ms, vỡ 3–5 | `vn_runtime` candidate (video ~30 fps, ±33 ms; claim ở #324 chờ tích hợp) | #320 |
| — | Gốc pha Walk khi đổi hướng (phát lại từ đầu) và gốc pha bom (lúc đặt) | #353: reset khi đổi state/hướng | `design` (R-252) | #326 |
| — | Nhịp frame chung 100 ms | R-118 | `design` | #311 (đóng) |

### F-11 — bố cục login/sảnh/phòng (nhánh `feat/ui-vn-layout`)

Baseline code `7dbfd432bd70a3ddabbbb9b1c5b2c5b8b5d0d036`; owner Codex, reviewer Claude.
Nguồn: R-227 (§71 RULES_V1, metadata `vn-ui-login-2026-10-26` đã trên main);
R-228/R-229 (§72–73 và metadata `vn-ui-lobby-room-2026-10-26` đã tích hợp qua
[PR #359](https://github.com/aquaplatinumberlitz/boom-rs/pull/359).
Các visual rect làm tròn về pixel logic; sai số nguồn ±0,78 px login, ±1,6 px lobby/room.
Visual rect không chứng minh hitbox, font, hover, ready/start policy hoặc chuyển cảnh.

- Login: panel `(284,414,243,130)`, field `(402,461,92,20)` và `(402,486,92,20)`,
  bốn vị trí nút đáy y555. Submit hiển thị disabled/xám khi password rỗng; backend/auth không đổi.
- Lobby: hồ sơ trái; sáu thẻ 2×3, chuyển trang hiển thị theo phòng đang chọn bằng Up/Down;
  hàng nút trên, chat `(227,410,561,148)`, nav y563. Chat sảnh chưa có backend ở V1:
  giữ panel trống, không tạo tin giả. Tên phòng mới và refresh giữ thao tác hiện có trong dải giữa.
- Room: bốn slot ngang, chat `(23,273,453,234)`, vùng nhân vật/map bên phải,
  start `(509,509,203,63)`. Metadata vùng slots cao273 giao nhau với chat bắt đầu y273;
  phần thẻ thực dùng y94–253 theo ảnh, chia nội bộ là `design`. Nav room đặt y574 để
  tránh đè start; đây là `design`, không lấy nav lobby y563 áp lên room.
- Ảnh nút chỉ lấy `selected` của bundle (UI_STRINGS_V1/IMAGE_REVIEW): `lobby.create`,
  `room.ready`, `room.start`, đặt ngay trong nút. Hủy ready dùng text catalog; disabled dùng
  khung xám; không suy frame hover/disabled từ số frame. Dialog chỉ hiển thị khi có luồng dialog,
  không vẽ ảnh dialog thường trực ở sidebar. Không đổi pack/asset/manifest.
- **Nợ `design`:** nền/khung màu, font Be Vietnam Pro, orb, avatar/thumbnail phòng,
  portrait người chơi và lưới nhân vật/map, badge, filter và nav chưa có ảnh phù hợp trong closure:
  chỉ khung giữ chỗ, không tự chế art. Ô tìm mật khẩu để trống vì chưa có key catalog/asset V1
  được duyệt; không thêm reset-password, 1P/2P, shop, nhiệm vụ hay policy ready/auth từ ảnh.
  Nhãn tiếp tục dùng catalog hiện có, chưa khớp toàn bộ chữ VN quan sát.
- Chủ nợ Codex/Claude; bước tiếp ở #336/Q-021: bổ sung locator/asset và key được duyệt cho
  các khung trống, đo font/focus/hover/chuyển cảnh; HUD trận/kết quả giữ trong F-11, chưa nghiệm thu.
  D-04/D-05 giữ nguyên; không nâng trạng thái claim/gate/feature theo kết quả regression.

Ảnh nguồn local: `/home/admin/boom-team/shots/fidelity/F-11/login/01-login-initial.png`,
`lobby-vn-1024x768.png`, `room-vn-1024x768.png`. Sandbox của lượt sửa không ghi được
`/home/admin/boom-team/shots/fidelity/F-11/ours/`. Đã chạy `--ui-shot login|lobby|room`
và ghép cạnh VN trong bộ nhớ (800×600 mỗi bên, VN chia1,28), không lưu media ở đường dẫn khác.
Xvfb cần ép Mesa (`__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json`,
`LIBGL_ALWAYS_SOFTWARE=1`); driver NVIDIA mặc định làm Xvfb crash. Bố cục mới có đúng vùng/thẻ,
art thiếu vẫn là khung, font/nhãn còn khác VN như nợ trên; ảnh local cần reviewer lưu tại đường dẫn
được chỉ định trước nghiệm thu fidelity. Không commit ảnh và không tuyên bố đã đạt fidelity chỉ từ CI.
