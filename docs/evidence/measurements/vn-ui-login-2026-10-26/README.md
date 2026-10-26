# Đo màn đăng nhập VN — F-11a (2026-10-26)

Bằng chứng local; repo PUBLIC không chứa ảnh game.

## Nguồn, lệnh và tỉ lệ

- Issue #354; owner dsh2; baseline main `d979af2f3e7bf5fc3e4347c6285d549ee7a6fc0f`.
- Client VN: `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/game/ca_fixed_patched.exe`,
  SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`.
- Xvfb riêng 1024×768x24 (`-displayfd`), Wine prefix `vn-grace-240/prefix`; server local
  `/home/admin/boom-team/tmp/vn149-factory-server.py`. Lệnh:
  `wine Z:\home\admin\boom-rs-dsh\re-kr\.re-work\vn-grace-240\game\ca_fixed_patched.exe`.
- Ảnh chụp bằng `import -window root`; không dùng `wineserver -k`/pkill. Nhóm tiến trình riêng
  đã được SIGTERM/SIGKILL và kiểm lại; không mở sảnh/phòng/trận.
- Lần đầu chỉ thấy splash; đúng một lần thử lại chờ 30 s. Không cần chạy VN lại cho lần sửa này.
- Nội dung game **800×600** phóng đúng **1,28×** lên ảnh 1024×768
  (1024/800 = 768/600 = 1,28). Toạ độ logic = pixel raw / 1,28.

## Ảnh local

| file | SHA-256 | ghi chú |
|---|---|---|
| `01-login-initial.png` | `e5c2e9ac27088ffecb4a0cfde4e338bc397e9a748d7f03e1152a49740b08922a` | account đã có `tester` từ prefix; password rỗng; submit xám |
| `02-account-focus.png` | `1b174a1bd9d8e60c88832abb82ee3e9da6aa0296023be3df1da32de7aa7d7952` | click account; không thấy viền focus khác |
| `03-account-typed.png` | `f7567cccd15efeeb533cf5a9a316f081c9710927d38e6721f2f5f08ca0d8fea2` | account `tester` |
| `04-password-focus.png` | `555e3c588d199cfe657ffb79e6e3b5359ebae175f134751bddcb587128c3c867` | caret password |
| `05-password-typed.png` | `2f0e76ac9d423c1c56f39c1cac65d00bcd79d57dcb793473ff60932035d63541` | password mask; submit xanh |
| `06-submit-hover.png` | `2fed9e9012d3666a059c0daefa5bd00c259c24a28069d7311cb1bc937cb70f70` | trỏ vào submit; không thấy đổi màu hover |
| `rust-login-current-800x600.png` | `1923643118316a8244faf8e6bc4fd325ad2bb82c83f0ea4b38b7aab2f9044447` | boom-rs hiện hành, 800×600 |

Ảnh `boom-rs-login-current-800x600.png` cũ (SHA `df11a055…`) được giữ local dưới tên
`boom-rs-login-historical-placeholder.png`; **không** dùng làm bằng chứng current vì có placeholder
`⟨login.*⟩` và vị trí field cũ.

## Đo field VN — ROI và loại hộp

ROI account: vùng trắng/gray trong panel, hàng `y=590..615`, cột `x=505..645`.
ROI password: `y=622..647`, cùng cột. Đo **visual box**, không phải hitbox:

| hộp | raw (x0,y0)–(x1,y1), exclusive | raw w×h | logic (x,y,w,h) | ghi chú |
|---|---:|---:|---:|---|
| account **outer** (gồm viền xám 1 px) | (515,590)–(633,615) | 118×25 | (402,34; 460,94; 92,19; 19,53) | dùng cho so layout |
| account **inner** (trắng) | (516,590)–(632,614) | 116×24 | (403,13; 460,94; 90,63; 18,75) | không phải hitbox |
| password **outer** | (515,622)–(633,647) | 118×25 | (402,34; 485,94; 92,19; 19,53) | |
| password **inner** | (516,622)–(632,646) | 116×24 | (403,13; 485,94; 90,63; 18,75) | |

Sai số thật: **±1 raw px** cho mép do viền/antialias ⇒ **±0,78 logic px**; bảng trên làm tròn
2 chữ số. Ảnh chỉ chứng minh visual rect; không suy hitbox từ ảnh.

## Các phần tử khác (visual, xấp xỉ)

| phần tử | raw (x,y,w,h) | logic (x,y,w,h) | ghi chú |
|---|---:|---:|---|
| panel `NGƯỜI CHƠI 1` | 364,530,311,166 | 284,38;414,06;242,97;129,69 | viền cam |
| 1P orb | 251,549,99,101 | 196,09;428,91;77,34;78,91 | xanh |
| 2P orb | 675,549,95,101 | 527,34;428,91;74,22;78,91 | xanh lá |
| TÌM MK | 235,711,136,45 | 183,59;555,47;106,25;35,16 | nút xanh |
| ĐĂNG KÝ | 374,711,136,45 | 292,19;555,47;106,25;35,16 | |
| ĐĂNG NHẬP | 514,711,136,45 | 401,56;555,47;106,25;35,16 | xám→xanh |
| THOÁT RA | 653,711,136,45 | 510,16;555,47;106,25;35,16 | |

Nhãn quan sát: `NGƯỜI CHƠI 1`, `NHÂN VẬT`, `MẬT KHẨU`, `TÌM MK`, `ĐĂNG KÝ`, `ĐĂNG NHẬP`,
`THOÁT RA`, `1P`, `2P`. Không đo font family/size/glyph; chỉ ghi chữ hiển thị.

## boom-rs hiện hành

- Ảnh `rust-login-current-800x600.png` chụp từ build hiện hành:
  `xvfb-run -a -s '-screen 0 1024x768x24' <bin>/boom-client --scale 1 --ui-shot login --snapshot <png>`
  (binary nêu dưới).
  Binary SHA `c4d76534dd5b15f2c8218777b3b51d90a423d39a6489cc07f537f81e0000f0fa`;
  text/l10n và rules profile nhúng mặc định; không truyền `--bundle`/`--rules` riêng.
- Source `ui/layout.rs::login_layout` (800×600): title `(24,12,752,36)`, account
  `(24,130,400,44)`, password `(24,206,400,44)`, buttons y=496 `140×40` gap8
  (`LoginSubmit`, `LoginRegister`, `Quit`), background bên phải `(636,104,156)`.
- Khác biệt so với VN: panel/orb/4 nút và field giữa màn ở VN; boom-rs field góc trái và 3 nút.
  VN submit xám trước khi nhập password, xanh sau; boom-rs luôn enabled.
  `1P`, `2P`, `TÌM MK` **chỉ là phần tử quan sát**, chưa phải acceptance/auth policy; việc đưa
  điều khiển mới vào V1 là quyết định scope riêng.

## Giới hạn

- Một capture VN/một prefix; account bị prefix điền `tester`; không có KR, không vào sảnh.
- Focus/hover yếu: không thấy viền focus hay đổi màu hover rõ; chỉ caret/chữ và disabled→enabled.
  Không kết luận style vắng mặt ngoài pixel đã chụp.
- D-04/D-05 (nền JPEG/ảnh có chữ) vẫn là khác biệt đã chấp thuận.
