# Tìm bố cục UI trong pack — F-11d (2026-10-26)

Bằng chứng tĩnh; không chạy VN/Wine.

## Phạm vi đã tìm

- Baseline main `9e282fc1e9ef85948ae7cf40fd73a40c60f751d0`.
- KR pack `/home/admin/boom-re/boomKR` (split `FxDesktop`, `FxStage`, `FxContents`, `FxAsset`, `fx`).
- VN pack `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/game` (merged `fx`).
- Công cụ: `boom-assets pack-list <pack> <stem> --build kr|vn`; ODT local
  `/home/admin/boom-team/shots/fidelity/odt/odt_anims.json`; code parser
  `crates/boom-assets/src/closure.rs`.
- Bộ lọc tên: `layout|scene|control|interface|gameui|login|lobby|room|result|dialog|stage|prepare`.
- Tìm thêm chuỗi `x,y,w,h`/control trong payload các entry tên UI; không thấy header
  layout/control có toạ độ.

## Kết quả

| nguồn | số entry/khối | thấy gì | kết luận |
|---|---:|---|---|
| KR `pack-list` 5 pack UI | 8.325 tên | `LoginStage/*`, `Lobby/*`, `Dialog/*`, `Game/*Result*`, `CAStage`, `PRE_Bt_Ready`... đều là sprite/JPEG/PNG/effect | **không có entry layout/scene/control**; tên `Dialog` chỉ là ảnh dialog |
| VN `fx` | 13.449 tên | cùng họ tên `LoginStage/LO_*`, `LobbySquare/RoomUI_*`, `Game/CP_Result_*`, `Dialog/*` | **không có entry layout/scene/control** |
| ODT `ObjectTable` | 7.867 khối | nhãn state animation (`Lobby/...`, `Effect/...`, `SelectServer/...`); không có khối Login/Lobby/Room/Result layout | ODT là bảng animation, không mang x/y/w/h control |
| `fx/ObjectTable/ObjectTable` | bảng tên/entry | tên sprite/animation; parser `closure.rs` đọc name/value, không có control layout | không phải bảng bố cục UI |

Ví dụ tên tìm được (đều là asset, không phải control):
`LoginStage/CAStage`, `LoginStage/LO_BtLogOk`, `LobbySquare/RoomUI_Bt_CreateRoom`,
`Game/CP_Result_1p`, `Dialog/DlgPcRoom`. Không entry nào có `layout`, `scene`, `control`,
`interface`, `gameui` hoặc payload x/y/w/h.

## Đối chiếu với consumer đã có

- KR `docs/research/findings/KR_RE_UI.md` R-152: bảng 6 rect room-list `dword_19D8A58`
  `(20,90)…` và PrepareStage `512,494`/`PRE_Bt_Ready` nằm trong **executable**, không nằm trong pack.
- Vì vậy R-227…R-230 (login/lobby/room/HUD) vẫn là số đo ảnh; không thay bằng dữ liệu pack.
- Không suy bố cục VN từ KR; VN consumer layout chưa được đọc trong lượt này.

## Giới hạn

- Đây là tìm theo tên/entry/ODT/parser; payload nhị phân opaque có thể chứa dữ liệu không đặt tên.
  Không kết luận tuyệt đối “không tồn tại”; chỉ kết luận **không thấy layout/control data trong phạm vi
  tên/entry/ODT đã kiểm**.
- Không mở binary/pack khác, không chạy client, không sửa code/profile/asset.
