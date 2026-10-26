# VN HUD và kết quả — F-11c (2026-10-26)

Bằng chứng local; repo PUBLIC không chứa ảnh game. Một capture HUD; màn kết quả chưa quan sát được.

## Nguồn/lệnh/tỉ lệ

- Issue #336 F-11c; baseline main `f393d782b3d0cbc8fd59b72e4c7b10b355ba5eb5`.
- VN client `ca_fixed_patched.exe` SHA `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`;
  prefix `vn-grace-240`; Xvfb riêng 1024×768x24; server local `vn149-factory-server.py`
  (`AUTO_START=5`, `DEATH_END=1`, `BOMB_PLACE=1`, `BOMB_ECHO=2`).
- `CALog` `dne:13` match; HUD chụp bằng `import -window root`. Game 800×600 phóng **1,28×**.
- Không `wineserver -k`/pkill; nhóm tiến trình riêng đã dừng. Không vào màn khác sau HUD.
- Kết quả: đã thử đặt bom/đi vào vùng nổ và chờ `DEATH_END=1`; **không có C->S 120 subtype13/146**,
  màn kết quả không xuất hiện; chỉ thấy round mới/`START!!`. Không dùng ảnh match làm ảnh kết quả.

## Ảnh local

| file | SHA-256 | ghi chú |
|---|---|---|
| `hud-vn-1024x768.png` | `90b5c96749e186614bdd318ddaf24d92b8e52b508b4a1b3d5913d0c03c52f705` | match stage13, HUD đầy đủ |
| `hud-bomb-vn-1024x768.png` | `5b3504a9c2fb84a8721de809bf8318bdef55bcad6032d4821465644f4b2a2179` | đặt bom, timer 00:00 |
| `rust-hud-current-600x560.png` | `8219edb7ed1c14232ce5c03fd6a4af92dcb8070ed4fa68b02a99b469a329ec5b` | `--demo --snapshot` 600×560, chỉ map |
| `rust-match-ui-current-800x600.png` | `edb3da562c280e3983265117558e4510a4f3d33ab9ce6fa626329bf0d98a7039` | `--ui-shot match`, header loading |
| `rust-result-current-800x600.png` | `9fea159ce737a466983aae2ad321905f06a6468e3d3b8a4412f20d4982535194` | `--ui-shot result` |

Rust binary SHA `5bcd2162049ea1e3a1f4cae3edcb3300b804f4f7a3f11295d0d582bdc57f124f`,
`xvfb-run -a -s '-screen 0 1024x768x24' … --scale 1 --demo --snapshot` / `--ui-shot match|result`;
l10n/rules nhúng, không `--bundle`.

## HUD VN — quan sát (raw 1024×768, logic = raw/1,28)

| vùng | raw xấp xỉ | logic | ghi chú |
|---|---:|---:|---|
| thanh điểm trên-trái | 18,8,86,28 | 14,1;6,3;67,2;21,9 | khiên + `01` |
| panel phải | 820,44,188,684 | 640,6;34,4;146,9;534,4 | avatar/timer/player/items/THOÁT RA |
| timer | 828,92,168,28 | 646,9;71,9;131,3;21,9 | `GIỚI HẠN 02:00` |
| player row | 828,126,168,48 | 646,9;98,4;131,3;37,5 | avatar đỏ, `tester` |
| item slots phải | 828,220,168,360 | 646,9;171,9;131,3;281,3 | 6 ô trống |
| `THOÁT RA` | 835,704,160,44 | 652,3;550,0;125,0;34,4 | nút cuối panel |
| `CÁ NHÂN` + 5 ô | 202,720,410,44 | 157,8;562,5;320,3;34,4 | thanh đáy giữa |
| map | 18,42,768,666 | 14,1;32,8;600,0;520,3 | vùng chơi |

Nhãn: `GIỚI HẠN`, `VẬT PHẨM`, `THOÁT RA`, `CÁ NHÂN`; điểm chỉ thấy `01` (không đủ kết luận công thức điểm).
Không đo font family/size; icon chỉ mô tả vị trí.

## HUD boom-rs hiện hành

- `--demo --snapshot` xuất **600×560** chỉ có map, không có HUD người chơi: nhánh offline
  truyền `SceneHud { player: None, font: None }`.
- `--ui-shot match` xuất 800×600: header `Làng O0`, `Đang tải... 0`, nút `Thoát game`; không có
  timer/avatar/item/`CÁ NHÂN`.
- Vì vậy HUD VN và boom-rs **chưa so được cùng cảnh**; đây là gap renderer/HUD, không phải D-04/D-05.

## Kết quả

- VN: chưa có màn kết quả; `DEATH_END=1` không nhận death packet, ảnh sau đó là round mới/`START!!`,
  không phải result. Không claim bố cục result VN.
- boom-rs `--ui-shot result` 800×600: title `Kết quả`, dòng `Thắng`, bảng `Thứ hạng/Người chơi/Kết quả`
  (`demo` 1 Thắng, `demo-2` Thua), hai nút `Về phòng`, `Thoát game`. Không có ảnh VN để so.

## Giới hạn

- Một capture HUD; VN result **unknown** do thiếu death/round-result. Toạ độ HUD là region ±2 raw px,
  không phải hitbox. Không suy score/timer policy. D-04/D-05 giữ nguyên.
