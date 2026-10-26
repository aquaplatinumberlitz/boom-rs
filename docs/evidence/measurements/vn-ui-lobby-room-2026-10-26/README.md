# VN lobby/room — F-11b (2026-10-26)

Bằng chứng local; repo PUBLIC không chứa ảnh game. Một capture lobby + một capture room,
cùng prefix VN `vn-grace-240`; không vào trận.

## Nguồn, lệnh, tỉ lệ

- Issue #336 (F-11b), owner dsh2; baseline main `f393d782b3d0cbc8fd59b72e4c7b10b355ba5eb5`.
- Client VN: `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/game/ca_fixed_patched.exe`,
  SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`.
- Xvfb riêng 1024×768x24 (`-displayfd`), Wine prefix `vn-grace-240/prefix`, server local
  `/home/admin/boom-team/tmp/vn149-factory-server.py`.
- `wine Z:\home\admin\boom-rs-dsh\re-kr\.re-work\vn-grace-240\game\ca_fixed_patched.exe`;
  login `tester/1234`; `CALog` `dne:11` lobby, `dne:12` room. Ảnh `import -window root`.
- Lần đầu login/lobby lấy được nhưng click room sai (script), một lần retry đúng; không phải
  lỗi môi trường. Không `wineserver -k`/pkill; nhóm tiến trình riêng đã dừng, prefix nhả.
- Nội dung game 800×600 phóng **1,28×** lên 1024×768; toạ độ logic = raw/1,28.
- Không mở sảnh khác/phòng khác/trận/kết quả; không click `BẮT ĐẦU`/`SẴN SÀNG`.

## Ảnh

| file | SHA-256 | ghi chú |
|---|---|---|
| `lobby-vn-1024x768.png` | `4109908ad25eeab7183fe2d0209761a5a64733e55bdb8f4bfa3ae9aaaff3bf7c` | lobby sau đóng tutorial; mock server không cấp chuỗi nên nhiều nhãn là thanh xám |
| `room-vn-1024x768.png` | `3ce029ab9a09a78cdc8673910128046b5d4dacd9a5144295874ec153808269b8` | room host-only, title `DefaultRoomTitle4`, map `Factory01`; không bấm bắt đầu |
| `rust-lobby-current-800x600.png` | `65e55d153c12d29de2267f81b47238396dd1f05c149b9e6122f0519426bff0a0` | boom-rs hiện hành |
| `rust-room-current-800x600.png` | `0bfc24e831423ecbf8d12e33bbf8a0a20826989b8c69ae98c2c4afd73e6c994d` | boom-rs hiện hành |

Rust build: binary SHA `5bcd2162049ea1e3a1f4cae3edcb3300b804f4f7a3f11295d0d582bdc57f124f`,
`xvfb-run -a -s '-screen 0 1024x768x24' <bin>/boom-client --scale 1 --ui-shot lobby|room
--snapshot …`; l10n/rules nhúng mặc định, không `--bundle`.

## Lobby — quan sát

| vùng | VN raw (x,y,w,h) | VN logic | boom-rs hiện hành | kết luận |
|---|---:|---:|---:|---|
| thông tin người chơi | 11,0,212,673 | 8,6;0;165,6;525,8 | không có panel hồ sơ | thiếu vùng hồ sơ/avatar/character |
| lưới phòng | 291,99,717,356 | 227,3;77,3;560,2;278,1 | bảng `(24,104,580,324)` | VN 2 cột×3 thẻ; Rust bảng dòng |
| thẻ phòng | khoảng 291,99,345,97 | 227,3;77,3;269,5;75,8 | dòng bảng | VN có thumbnail + ổ khóa/số người; Rust chỉ text |
| nút tạo/đi tắt | `TẠO PHÒNG` 306,14,166,68; `ĐI TẮT` 487,14,139,68 | 239,1;10,9;129,7;53,1; 380,5;10,9;108,6;53,1 | không có | thiếu nút tạo/đi tắt |
| nút lọc | `TOÀN BỘ` 765,14,117,68; `LUYỆN TẬP` 900,14,~112,68 | 597,7;10,9;91,4;53,1 | không có | thiếu lọc |
| chat | 290,525,718,190 | 226,6;410,2;560,9;148,4 | ô chat dưới bảng, một dòng | VN chat panel lớn + filter/tab |
| nav đáy | 0,720,1024,48 | 0;562,5;800;37,5 | không có nav | thiếu `TÙY CHỌN/SIÊU THỊ/BINGBING/NHIỆM VỤ/CÁ NHÂN` |

Nhãn quan sát: `TẠO PHÒNG`, `ĐI TẮT`, `TOÀN BỘ`, `LUYỆN TẬP`; mock server để nhiều nhãn room/profile
thành thanh xám, nên không dùng chúng làm chuỗi thiết kế. Không thấy trạng thái hover/focus đáng tin.

## Room — quan sát

| vùng | VN raw (x,y,w,h) | VN logic | boom-rs hiện hành | kết luận |
|---|---:|---:|---:|---|
| 4 slot người chơi | 30,120,575,350 | 23,4;93,8;449,2;273,4 | bảng `(24,104,580,324)` | VN slot thẻ; Rust bảng text |
| chat | 30,350,580,300 | 23,4;273,4;453,1;234,4 | ô chat dưới bảng | VN chat lớn; Rust một field |
| bảng bản đồ | 634,14,368,590 | 495,3;10,9;287,5;460,9 | không có | VN có grid/chọn map; Rust không |
| chọn nhân vật | 630,120,368,300 | 492,2;93,8;287,5;234,4 | không có | VN grid nhân vật; Rust không |
| nút bắt đầu | 652,651,260,80 | 509,4;508,6;203,1;62,5 | `Bắt đầu` (172,496,140,40) | VN nút lớn giữa panel; Rust nút hàng dưới |
| nhãn ready | host `TRƯỞNG PHÒNG`; slot trống `SẴN SÀNG` | — | bảng trạng thái `Chưa sẵn sàng/Sẵn sàng` | VN dùng badge slot; Rust dùng cột bảng; chưa bấm ready |

Nhãn quan sát: `THÔNG TIN`, `TRƯỞNG PHÒNG`, `SẴN SÀNG`, `BẮT ĐẦU`, `TỰ ĐỘNG`, `CHỌN BÀN ĐỒ`,
`Factory01`, `DefaultRoomTitle4` (placeholder server, không phải tên thiết kế). Không thấy trạng thái
ready/hover sau click vì không bấm; room chỉ host.

## Giới hạn / phân loại

- `vn_runtime` / **candidate**, một capture/một prefix; toạ độ là visual region ±2 raw px (±1,6 logic px),
  không phải hitbox. Font family/size chưa đo.
- D-04/D-05 (nền JPEG/ảnh có chữ) vẫn là khác biệt đã chấp thuận; các khác biệt bố cục trên **không**
  phải D-04/D-05.
- Không suy auth/ready policy; `1P`/`2P`/item... ngoài phạm vi. Không có KR, không vào trận.
