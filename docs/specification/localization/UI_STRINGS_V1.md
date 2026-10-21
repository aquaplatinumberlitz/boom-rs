# Chuỗi và ảnh UI cho luồng V1 (Q-012)

Cập nhật: 2026-10-20 (sửa docs stale: bundle **schema 4** ở §3; golden wrap đã có ở §6).
Trước đó: 2026-10-20 (V1-035 phần 2: chốt font Be Vietnam Pro 1.002 + bundle `assets/fonts/`;
§5.1.1 nguồn/giấy phép/SHA-256, §5.2.1 quy tắc layout của `boom-client::text`).
Trước đó: 2026-10-20 (V1-035 phần 1: xuất chuỗi V1 → `data/l10n/vi-v1.toml` bằng
`tools/l10n_export.py`, loader `boom-content::l10n`; §6.1 có báo cáo khóa thiếu do tool sinh).
Trước đó: 2026-10-20 (V1-032: cột "Trạng thái dùng" ở §3 và mục §3.1). Phục vụ **V1-032**
(UI luồng V1 bằng asset KR) và **V1-035** (Việt hóa UI V1), câu hỏi
[Q-012](../../research/QUESTIONS.md). Baseline khi lập: `main` @ `fefd5c0`, contract `contracts-r3`;
kiểm lại nguồn trên `main` @ `5845f15`, worktree `issue-15`; phần trạng thái dùng kiểm trên
`main` @ `5242e48`, worktree `issue-84` (bundle schema 3). Nguồn chuẩn về scope là
[SCOPE_V1](../../planning/SCOPE_V1.md), về luật trận là [RULES_V1](../gameplay/RULES_V1.md).

Tài liệu này là **danh mục chọn/tham chiếu**, không phải nguồn bản dịch. `catalog.csv` ở
`/home/admin/boom-re-l10n/l10n/` là nguồn chuẩn duy nhất của chuỗi đã Việt hóa; spec này chỉ ghi khóa V1,
locator catalog/ảnh, trạng thái nguồn và yêu cầu hiển thị. Không lưu bản VI độc lập, không viết lại/rút gọn
bản catalog và không tự soạn chuỗi mới trong PR này. Mục chưa có locator phù hợp ghi `unknown` + bước tiếp.

Luồng được xét: **đăng nhập → sảnh → phòng chờ → trận → kết quả → về phòng**. Chỉ liệt kê phần V1 dùng;
mọi thứ ngoài scope V1 (shop, túi đồ, nhiệm vụ, hội, bạn bè, kênh xếp hạng, mode khác, phòng kín, đội,
spectator, quảng trường đi bộ) bị loại, kể cả khi catalog có chuỗi/ảnh cho chúng.

## 1. Nguồn và cách đọc

| Nguồn | Vị trí | Đã dùng để làm gì |
|---|---|---|
| Catalog chuỗi KR | `/home/admin/boom-re-l10n/l10n/catalog.csv` | 25.257 dòng: `translated` 21.421, `reused` 3.813, `review` 23. Mỗi dòng có `id, source, ko, vi_base, vi, status, notes`; `vi` là bản chuẩn, `source` là file/entry gốc. |
| Danh sách review đang mở | `/home/admin/boom-re-l10n/l10n/review_pending.csv` | 23 dòng `review`; không dùng làm text đã chốt. |
| Khóa UI VN cũ (chỉ locator) | `/home/admin/boom-re/server/src/strings.rs` | 148 khóa; 146 khóa khớp `id` catalog, 2 khóa không có (`CannotConnectToShop`, `CheckRentedItem`). Đây là text nhóm dự án VN tự viết, **không phải** nguồn dịch KR và không thay thế catalog. |
| Ảnh có chữ | `/home/admin/boom-re-l10n/l10n/images_todo.csv` | 3.038 ứng viên I16/JPEG; chưa có quyết định duyệt nào. |
| Trạng thái duyệt ảnh | `/home/admin/boom-re-l10n/l10n/.cache/image-review/images_review.csv` | Toàn bộ 3.038 dòng `pending`; chưa có `l10n/images_review.csv` (kết quả thật). |
| Ảnh giải mã | `/home/admin/boom-re-l10n/l10n/.cache/image-review/` | PNG giải mã cục bộ, không commit; dùng để đọc chữ và kích thước. |
| Asset UI V1 đã chọn | [kr_v1_content.json](../../evidence/inventory/kr_v1_content.json) | Chỉ 4 entry UI được chọn: `room_create`, `room_create_dialog`, `ready`, `start`; map/theme/nhân vật/item là asset khác. SHA-256 của các entry UI còn lại nằm ở `[[sprites]].source_sha256` của bundle (`kr_v1_content.json` là output của tool `boom-re/tools/kr_v1_content.py`, không sửa tay ở task này). |
| Bảng asset UI của closure V1 | `crates/boom-assets/src/closure.rs` (`UI_ASSETS`) và mục `[[ui]]` của bundle ([BUNDLES §3.3](../assets/BUNDLES.md)) | Trạng thái dùng của từng phần tử UI của luồng V1 (xem §3); nguồn chuẩn cho cột "Trạng thái dùng". |
| Quy trình duyệt ảnh | [IMAGE_REVIEW](IMAGE_REVIEW.md) | Trạng thái `pending/translated/needs_translation/no_text/check`; không tự đánh dấu đã dịch. |
| Tool xuất chuỗi V1 | `tools/l10n_export.py` | Đọc §2 của tài liệu này + `catalog.csv` → `data/l10n/vi-v1.toml`. `--check` kiểm file đã xuất mà **không cần** repo `boom-re-l10n` (chỉ kiểm schema + đối chiếu §2); có catalog thì so từng byte `text` với `catalog.vi`. |
| File chuỗi đã xuất (V1-035) | `data/l10n/vi-v1.toml` | **Output của tool**, không sửa tay: mỗi khóa V1 có `status`; khóa có text thêm `text` (= `catalog.vi` nguyên văn, chỉ trim biên) và `catalog_id`; khóa chưa có text có `reason` (không có `text`) và `note` (yêu cầu hiển thị của §2). Loader: `boom-content::l10n`. |

Đối chiếu **tên khóa** legacy (chỉ là locator, không lấy text làm bản dịch). Một số khóa V1 trùng tên
khóa trong `strings.rs`, ví dụ: `app.connecting` → `strings.rs:32` (`ConnectingToSvr`), `app.auth_failed`
→ `strings.rs:35` (`ForbiddenAuthenForOnePlayer`), `lobby.chat_tab_all` → `strings.rs:44`
(`LobbySquare_ChatTab_All`), `room.stats_bomb` → `strings.rs:89` (`PREEA`), `room.stats_fire` →
`strings.rs:90` (`PREPOW`), `room.stats_speed` → `strings.rs:91` (`PRESPD`). Danh sách này không đầy đủ;
text chuẩn vẫn lấy từ catalog. Hai khóa không có trong catalog là `CannotConnectToShop` và
`CheckRentedItem`, không thuộc luồng V1.

Cột **Locator** chỉ dùng một trong các dạng:

- `catalog:<id>` — dòng tương ứng trong `catalog.csv`; text hiển thị lấy từ `catalog.vi`.
- `ảnh <pack:entry>` — chữ nằm trong ảnh; phải duyệt ảnh trước khi dùng, hoặc render text từ key catalog.
- `—` — chưa có locator phù hợp; ghi `unknown`, không render cho tới khi bổ sung nguồn.

Cột **Trạng thái nguồn**:

| Nhãn | Nghĩa |
|---|---|
| `catalog-translated` | Có `id` catalog status `translated`; dùng `catalog.vi` nguyên văn; khi render chỉ bỏ khoảng trắng/xuống dòng thừa ở biên, không đổi chữ. |
| `catalog-reused` | Có `id` catalog status `reused`; dùng `catalog.vi` nguyên văn, đọc lại ngữ cảnh. |
| `review` | Có `id` catalog nhưng ngữ cảnh/format cần reviewer chọn; **không** viết lại text, chỉ chọn key hoặc đánh dấu chưa dùng. |
| `image-pending` | Chữ nằm trong ảnh; ảnh đang `pending` trong `images_review.csv`, chưa được dùng làm bản dịch. |
| `unknown` | Chưa có locator catalog/ảnh phù hợp; không render tới khi bổ sung key/asset và duyệt. |

Tổng hợp 110 khóa: **44 dòng có locator catalog** (33 `catalog-translated` + 11 `catalog-reused`),
**15 dòng `review`**, **9 dòng `image-pending`** và **42 dòng `unknown`**. Không có dòng nào tự soạn
bản VI hay yêu cầu sửa bản dịch đã có. Mọi dòng `review`/`unknown` là **đầu việc của `boom-re-l10n`**:
ghi rõ id khóa V1 cần thêm/sửa; repo này không tự viết bản dịch.

## 2. Khóa V1 theo màn hình

### 2.1 Khung ứng dụng, kết nối và lỗi

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `app.loading` | Dòng trạng thái tải | `catalog:Loading` | `catalog-reused` | Lấy `catalog.vi`; giữ dấu `...`. |
| `app.connecting` | Dòng đang kết nối | `catalog:ConnectingToSvr` | `review` | Catalog là câu chờ chung; reviewer chọn key/ngữ cảnh, không tự soạn bản mới. Cần chọn/sửa key ở boom-re-l10n. |
| `app.network_error` | Hộp thoại mất mạng | `catalog:NetworkError` | `catalog-translated` | Lấy `catalog.vi`; wrap tối đa 3 dòng. |
| `app.session_replaced` | Phiên bị thay | `catalog:szConnCloseMsg` | `catalog-translated` | Lấy `catalog.vi`; tối đa 2 dòng. |
| `app.auth_failed` | Đăng nhập thất bại | `catalog:ErrLoginNotRegistered` | `review` | Catalog gắn luồng Nexon; reviewer chọn key phù hợp, không tự soạn. Cần chọn/sửa key ở boom-re-l10n. |
| `app.version_mismatch` | Handshake phiên bản | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `app.version_mismatch`; chưa render. |
| `app.content_mismatch` | Handshake rules/content | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `app.content_mismatch`; chưa render. |
| `app.quit_confirm` | Hộp thoại thoát | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `app.quit_confirm`; chưa render. |
| `app.invalid_map` | Map không hợp lệ | `catalog:InvalidMapReturnToLobby` | `catalog-translated` | Lấy `catalog.vi`. |

### 2.2 Đăng nhập

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `login.title` | Tiêu đề màn hình | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `login.title`; chưa render. |
| `login.account` | Nhãn ô tài khoản | `catalog:NewUserDlgID` | `review` | Catalog chỉ là nhãn ID; reviewer chọn key, không tự soạn. Cần chọn/sửa key ở boom-re-l10n. |
| `login.password` | Nhãn ô mật khẩu | `catalog:Password` | `catalog-translated` | Lấy `catalog.vi`. |
| `login.submit` | Nút đăng nhập | `catalog:DailyMission_Mission_Login` | `review` | Catalog gắn nhiệm vụ ngày; reviewer chọn key. Cần chọn/sửa key ở boom-re-l10n. |
| `login.register` | Nút tạo tài khoản | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `login.register`; chưa render. |
| `login.failed` | Sai tài khoản/mật khẩu | `catalog:ErrLoginNotRegistered` | `review` | Catalog gắn Nexon; reviewer chọn key. Cần chọn/sửa key ở boom-re-l10n. |
| `login.empty` | Thiếu trường bắt buộc | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `login.empty`; chưa render. |
| `login.account_rule` | Quy tắc tên tài khoản | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `login.account_rule`; chưa render. |

### 2.3 Sảnh

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `lobby.title` | Tiêu đề sảnh | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.title`; chưa render. |
| `lobby.channel` | Tên kênh | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.channel`; chưa render. |
| `lobby.channel_desc` | Tooltip kênh | `catalog:ChannelExplain1` | `catalog-translated` | Lấy `catalog.vi`; tooltip. |
| `lobby.room_name` | Cột tên phòng | `catalog:CreateDlgTitle` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.room_host` | Cột chủ phòng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.room_host`; chưa render. |
| `lobby.room_count` | Cột số người | `catalog:BigBattle_Lobby_Desc_UserCount` | `review` | Catalog là format BigBattle; reviewer chọn format, giữ `%d`. Cần chọn/sửa key ở boom-re-l10n. |
| `lobby.room_status` | Cột trạng thái | `catalog:RoomInfoDlgStatus` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.state_waiting` | Trạng thái chờ | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.state_waiting`; chưa render. |
| `lobby.state_playing` | Trạng thái đang chơi | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.state_playing`; chưa render. |
| `lobby.empty` | Danh sách rỗng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.empty`; chưa render. |
| `lobby.create` | Nút tạo phòng | `ảnh FxStage:Lobby/LB_Bt_T_Create-NM` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `lobby.join` | Nút vào phòng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.join`; chưa render. |
| `lobby.refresh` | Nút làm mới | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.refresh`; chưa render. |
| `lobby.back` | Nút quay lại | `catalog:BackButton` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.options` | Nút tùy chọn | `catalog:GBOption` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.help` | Nút trợ giúp | `catalog:GBHelp` | `catalog-reused` | Lấy `catalog.vi`; có thể ẩn nếu chưa có nội dung. |
| `lobby.exit` | Nút thoát game | `catalog:GBExit` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.chat_tab_all` | Tab chat chung | `catalog:LobbySquare_ChatTab_All` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.chat_tab_notice` | Tab thông báo | `catalog:LobbySquare_ChatTab_Notice` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.chat_hint` | Gợi ý ô chat | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.chat_hint`; chưa render. |
| `lobby.chat_send` | Nút gửi chat | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `lobby.chat_send`; chưa render. |
| `lobby.chat_too_long` | Chat quá dài | `catalog:150CharLimit` | `catalog-translated` | Dùng giới hạn 150; đếm grapheme cluster, giữ `%`/`[...]` nếu có. |
| `lobby.join_failed` | Vào phòng thất bại | `catalog:CantJoinRoom` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.room_full` | Phòng đã đủ | `catalog:InvalidMaxUser` | `review` | Catalog nói số người không hợp lệ; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `lobby.room_removed` | Phòng đã bị xóa | `catalog:RoomDestroyed` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.room_info_changed` | Thông tin phòng đổi | `catalog:RoomInfoChanged` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.join_session` | Chat hệ thống có người vào | `catalog:JoinSession` | `catalog-translated` | Giữ `%s`; lấy `catalog.vi`. |
| `lobby.leave_session` | Chat hệ thống có người rời | `catalog:LeaveSession` | `catalog-translated` | Giữ `%s`; lấy `catalog.vi`. |

### 2.4 Phòng chờ

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `room.title_label` | Nhãn tên phòng trong dialog tạo | `catalog:CreateDlgTitle` | `catalog-reused` | Lấy `catalog.vi`. |
| `room.create_title` | Tiêu đề dialog tạo phòng | `ảnh FxStage:Lobby/Dlg_LB-NMCreate` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.create_submit` | Nút tạo | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.create_submit`; chưa render. |
| `common.cancel` | Nút hủy | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `common.cancel`; chưa render. |
| `common.confirm` | Nút đồng ý | `catalog:OptionDlgYes` | `review` | Catalog là 'Có' trong hộp thoại tùy chọn; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `common.close` | Nút đóng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `common.close`; chưa render. |
| `room.info_title` | Tiêu đề thông tin phòng | `ảnh FxMap:Prepare/PRE_Bt_Roominfo`, `ảnh FxMap:Prepare/Dlg_PRE-NMRoominfo` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.host` | Nhãn chủ phòng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.host`; chưa render. |
| `room.players` | Nhãn danh sách người chơi | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.players`; chưa render. |
| `room.status` | Nhãn trạng thái | `catalog:RoomInfoDlgStatus` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.id` | Nhãn ID phòng | `catalog:RoomInfoDlgID` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.map` | Nhãn bản đồ | `ảnh FxMap:Prepare/CAStage` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.map_name` | Tên map V1 | `catalog:map:Village00` | `catalog-translated` | Lấy `catalog.vi`; muốn dùng tên khác phải ghi **cần sửa ở boom-re-l10n** kèm id. |
| `room.character` | Nhãn chọn nhân vật | `ảnh FxMap:Prepare/CAStage` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.character_c00` | Tên hiển thị C00 | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.character_c00`; chưa render. |
| `room.character_c01` | Tên hiển thị C01 | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.character_c01`; chưa render. |
| `room.ready` | Trạng thái/nút sẵn sàng | `ảnh FxMap:Prepare/PRE_Bt_Ready` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog. |
| `room.ready_cancel` | Hủy sẵn sàng | `ảnh FxMap:Prepare/PRE_Bt_Ready` | `image-pending` | Cùng nút ready; duyệt ảnh hoặc render text từ key catalog. |
| `room.start` | Nút bắt đầu | `ảnh FxMap:Prepare/PRE_Bt_Start` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog. |
| `room.waiting_players` | Chờ người chơi | `catalog:BigBattle_Lobby_Desc_Line0` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.min_players` | Chưa đủ người để bắt đầu | `catalog:SmallPlayerNo` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.all_ready` | Chưa ready hết | `catalog:AllMustReady` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.loading` | Loading barrier | `catalog:Loading` | `catalog-reused` | Lấy `catalog.vi`; nhắc 15 giây theo SCOPE_V1 §3 nếu hiển thị đếm ngược. |
| `room.kick` | Nút mời ra | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.kick`; chưa render. |
| `room.kick_confirm` | Xác nhận mời ra | `catalog:SureToBan` | `catalog-translated` | Giữ `%s`; lấy `catalog.vi`. |
| `room.kicked` | Bị chủ phòng mời ra | `catalog:BannedByLeader` | `catalog-translated` | Lấy `catalog.vi`; bỏ khoảng trắng đầu khi render, không đổi chữ. |
| `room.non_host_denied` | Non-host start/kick | `catalog:RoomLeaderOnly` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.name_changed` | Đổi tên phòng | `catalog:RoomNameChanged` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.host_changed` | Chuyển chủ phòng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.host_changed`; chưa render. |
| `room.slot_empty` | Slot trống | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.slot_empty`; chưa render. |
| `room.chat_to_all` | Phạm vi chat | `catalog:ChatToAll` | `catalog-translated` | Lấy `catalog.vi`; V1 FFA không có `<Đồng đội>`. |
| `room.stats_bomb` | Nhãn số bom | `catalog:PREEA` | `review` | Catalog là nhãn chung '개수'; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `room.stats_fire` | Nhãn tầm nổ | `catalog:PREPOW` | `review` | Catalog là nhãn '물줄기'; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `room.stats_speed` | Nhãn tốc độ | `catalog:PRESPD` | `catalog-reused` | Lấy `catalog.vi`. |
| `room.leave` | Nút rời phòng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `room.leave`; chưa render. |
| `room.leave_ready_confirm` | Xác nhận rời khi đang ready | `catalog:WannaExitToLobby` | `catalog-translated` | Lấy `catalog.vi`; reviewer chốt ngữ cảnh V1. |

### 2.5 Trong trận

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `match.timer` | Nhãn đồng hồ | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.timer`; chưa render. |
| `match.timer_format` | Định dạng mm:ss | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.timer_format`; chưa render. |
| `match.go` | Hiệu lệnh bắt đầu | `catalog:TutorialClickStartBackStress1` | `review` | Catalog là nhãn tutorial 'Nút Bắt đầu'; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `match.trapped` | Trạng thái bị nhốt | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.trapped`; chưa render. |
| `match.eliminated` | Trạng thái bị loại | `catalog:BlueMonSay_event4_01` | `review` | Catalog gắn sự kiện quái; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `match.disconnect_warning` | Cảnh báo mất kết nối | `catalog:ErrNetworkError` | `review` | Catalog là thông báo mất mạng chung, không có đếm ngược; reviewer chọn key. Cần chọn/sửa key ở boom-re-l10n. |
| `match.disconnect_eliminated` | Bị loại do mất kết nối | `catalog:ErrDisconnectByServer` | `review` | Catalog là thông báo ngắt kết nối chung; reviewer chọn key/ngữ cảnh. Cần chọn/sửa key ở boom-re-l10n. |
| `match.item_bomb` | Tên item Bomb | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.item_bomb`; chưa render. |
| `match.item_fire` | Tên item Fire | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.item_fire`; chưa render. |
| `match.item_roller` | Tên item Roller | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.item_roller`; chưa render. |
| `match.pickup` | Thông báo nhặt item | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.pickup`; chưa render. |
| `match.result_pending` | Chờ server chốt kết quả | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `match.result_pending`; chưa render. |

### 2.6 Kết quả

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `result.title` | Tiêu đề | `catalog:Olympic_TitleResult` | `catalog-translated` | Lấy `catalog.vi`. |
| `result.win` | Nhãn thắng | `catalog:StateWin` | `catalog-translated` | Lấy `catalog.vi`; ảnh `Win` cần duyệt nếu dùng. |
| `result.lose` | Nhãn thua | `catalog:StateLose` | `catalog-translated` | Lấy `catalog.vi`; ảnh `Lose` cần duyệt nếu dùng. |
| `result.draw` | Nhãn hòa | `ảnh FxStage:Game/Draw` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì dùng key catalog được chốt. |
| `result.rank` | Cột hạng | `catalog:Olympic_TitleRank` | `catalog-translated` | Lấy `catalog.vi`. |
| `result.player` | Cột người chơi | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `result.player`; chưa render. |
| `result.continue` | Nút tiếp tục | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `result.continue`; chưa render. |
| `result.auto_return` | Đếm ngược về phòng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `result.auto_return`; chưa render. |
| `result.winner` | Người thắng | `catalog:BigBattle_Ingame_Winner_Line1` | `review` | Catalog là câu BigBattle, không có `%s`; reviewer chọn key/format. Cần chọn/sửa key ở boom-re-l10n. |
| `result.draw_all` | Hòa khi không còn ai | `catalog:TournamentNoWinner` | `catalog-translated` | Lấy `catalog.vi`; reviewer chốt ngữ cảnh giải đấu. |
| `result.waiting` | Chờ ack người khác | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `result.waiting`; chưa render. |

### 2.7 Tùy chọn

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `options.fullscreen` | Chế độ toàn màn hình | `catalog:OptionDlgFullScreen` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.windowed` | Chế độ cửa sổ | `catalog:OptionDlgWindowMode` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.volume` | Âm lượng chung | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `options.volume`; chưa render. |
| `options.music` | Nhạc nền | `catalog:OptionDlgBGM` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.sfx` | Hiệu ứng âm | `catalog:OptionDlgEffect` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.apply` | Áp dụng | — | `unknown` | Cần sửa ở boom-re-l10n: thêm key/asset cho `options.apply`; chưa render. |

### 2.8 Chuỗi không dùng trong V1

Các nhóm sau có trong catalog nhưng **không** thuộc luồng V1; client phải ẩn, không render, không dịch thêm:
shop/ví (`GoToShop`, `UserMarket`, `MyItem`, `Cash`, `Lucci`…), nhiệm vụ (`Quest`, `BnbNavi_*`), hội/bạn bè
(`GuildTab`, `Messenger`), kênh xếp hạng/giải đấu (`ChannelDesc50…82`, `Tournament*`), mode khác
(`CreateDlgRenew*`, `CreateDlgGameType*`, `GameMode*`), phòng kín/mật khẩu (`CreateDlgPrivate`,
`CreateDlgPassword`, `WrongPassword`), đội/spectator (`ChatToOurTeam`, `EnterObserver`), quảng trường đi bộ
(`LobbySquare_*`), auto-ready (`OptionDlgAutoReady`, `TurnOnAutoReady`, `TurnOffAutoReady`), premium/super
host (`PremiumSlotRoomTicket`, `BannedBySuper`, `ChiefBanned`).

## 3. Ảnh có chữ trong luồng V1

Cột **Trạng thái dùng** là quyết định của dự án cho V1-032, lấy từ bảng asset UI của
closure (`crates/boom-assets/src/closure.rs`) và được ghi vào mục `[[ui]]` của bundle
(schema 4, xem [BUNDLES §3.3](../assets/BUNDLES.md)). Bằng chứng phần client gốc (ảnh có
chữ nằm trong ảnh) và quyết định V1 (vẽ text từ catalog tới khi ảnh được duyệt) ở
[RULES_V1 §12](../gameplay/RULES_V1.md#12-nền-và-ảnh-ui-của-client-v1) (R-090…R-093). Ba trạng thái:

| Trạng thái | Nghĩa | Ảnh trong atlas | Hiển thị được |
|---|---|---|---|
| `selected` | Có trong closure V1 và **không có chữ** cần duyệt | có | có |
| `needs_review_text` | Có trong closure V1 nhưng **có chữ** | có | chỉ khi ảnh được duyệt; chưa duyệt thì client vẽ text từ catalog |
| `excluded` | Không dùng asset này | không | thay bằng text/màu theo cột "Ghi chú review" |

Tên trong ngoặc là **ID phần tử UI** trong bundle (`lobby.create`, `result.win`, …), để
client tra cùng một ID ở bảng `[[ui]]`. Trước V1-032, cột này ghi trạng thái chọn trong
`kr_v1_content.json` (`required`/`selected_resources`): `—` nghĩa là chưa nằm trong
closure V1. Cột **Chữ trên ảnh** là đọc trực tiếp PNG giải mã ngày 2026-10-19 (UI) và
2026-10-20 (các ảnh đọc thêm cho V1-032; bản PNG nằm ở cache review ảnh của
`boom-re-l10n`: `l10n/.cache/image-review/`), chỉ là bằng chứng mô tả, **không phải** bản
dịch do spec này đặt. Số frame trong cột **Kích thước** đếm từ blob sprite I16 trong
bundle, khớp với `frame_count` của cache review ảnh. Tất cả trạng thái duyệt hiện là
`pending`; muốn dùng phải import `images_review.csv` qua quy trình
[IMAGE_REVIEW](IMAGE_REVIEW.md).

Chữ ghi trong cột **Chữ trên ảnh** là chữ **đã đọc trực tiếp** trên PNG của build đó (cache
review ảnh có cả `kr` và `vn`); phần chưa đọc rõ thì ghi rõ là chưa xác nhận. Cột **VN cùng
tên** ghi `diff_bytes` là phép so **byte** của hai build — byte khác **không** chứng minh chữ
khác, nên không dùng phép này để suy ra ngôn ngữ hay nghĩa của chữ.

Ảnh nền đọc ở §3 nằm trong cache review ảnh nhưng là **JPEG** (không phải sprite I16), nên
đã đối chiếu thêm: PNG cache **giống hệt pixel** blob trích từ pack (đo 2026-10-20, đối chiếu
lại 2026-10-21: 6/6 ảnh khớp từng pixel, RGB 800×600, cả KR lẫn VN —
`boom-assets pack-extract /home/admin/boomkr-full <stem> <entry> <out>`
với `/home/admin/boom-re/assets/fx/{LoginStage,Lobby,Prepare}/CAStage.bin`; ảnh lấy theo
`l10n/.cache/image-review/manifest.json`). Vì vậy chữ đọc trên ảnh cache là bằng chứng cho
blob trong pack, không phải cho ảnh của cache.

| Asset (pack:entry) | Kích thước | Trạng thái dùng | Chữ trên ảnh (bằng chứng) | VN cùng tên | Ghi chú review |
|---|---|---|---|---|---|
| `FxDesktop:LoginStage/CAStage` | 800×600 JPEG | `excluded` (`login.background`) | KR: lời thoại tiếng Hàn + biểu tượng "ALL"/"전체이용가" (góc trên phải); VN: logo Boom + dòng bản quyền tiếng Anh ở góc dưới phải (ghi "NexonKorea Corporation"; **không** có chữ Việt) | `diff_bytes` | **Không dùng nguyên trạng**: nền là JPEG nên bộ giải mã bundle không đọc được, và ảnh có bản quyền/lời thoại → V1 dựng nền bằng màu, chữ lấy từ catalog |
| `FxStage:Lobby/CAStage` | 800×600 JPEG | `excluded` (`lobby.background`) | KR: hai tab "대전 모드" / "몬스터 모드" + nhãn "레벨"; VN: "ĐẠI CHIẾN" / "NHIỆM VỤ" + "NGƯỜI CHƠI" | `diff_bytes` | Chữ nằm trong ảnh nền; tab chế độ ngoài scope V1 → V1 dựng nền bằng màu, chữ lấy từ catalog |
| `FxMap:Prepare/CAStage` | 800×600 JPEG | `excluded` (`room.background`) | KR: nhãn "캐릭터선택" (chọn nhân vật), "팀선택" (chọn đội), "맵" (bản đồ) và nút "자동 준비" (auto-ready); VN: "NHÂN VẬT", "CHỌN ĐỔI", "BẢN ĐỒ" và nút "TỰ ĐỒNG" | `diff_bytes` | Auto-ready ngoài scope FFA V1 → V1 dựng nền bằng màu, chữ lấy từ catalog |
| `FxStage:Lobby/LB_Bt_T_Create-NM` | 92×495 (9 frame) | `needs_review_text` (`lobby.create`) | Có chữ Hàn (방만들기); VN: "TẠO PHÒNG" | `diff_bytes` | Ảnh nút có chữ; nếu dùng phải duyệt `translated`, nếu không thì render text từ catalog |
| `FxStage:Lobby/Dlg_LB-NMCreate` | 414×455 | `needs_review_text` (`lobby.create_dialog`) | Tiêu đề "방만들기" nằm trong ảnh; VN: "TẠO PHÒNG" | `diff_bytes` | Tiêu đề dialog nằm trong ảnh; phần thân là ô trống, nhãn do client vẽ |
| `FxMap:Prepare/PRE_Bt_Ready` | 193×342 (6 frame) | `needs_review_text` (`room.ready`) | Có chữ Hàn (준비); VN: "SẴN SÀNG" | `diff_bytes` | Nút ready có chữ; cần duyệt ảnh hoặc render text từ catalog |
| `FxMap:Prepare/PRE_Bt_Start` | 193×342 (6 frame) | `needs_review_text` (`room.start`) | Có chữ Hàn (시작); VN: "BẮT ĐẦU" | `diff_bytes` | Nút start có chữ; cần duyệt ảnh hoặc render text từ catalog |
| `FxMap:Prepare/PRE_Bt_Roominfo` | 100×110 (5 frame) | `excluded` (`room.info_label`) | KR: "방정보변경" (đổi thông tin phòng); VN: "THÔNG TIN" (thông tin) | `diff_bytes` | Ảnh KR/VN khác nghĩa; V1 vẽ nhãn từ catalog, không tự chọn/sửa nhãn |
| `FxMap:Prepare/Dlg_PRE-NMRoominfo` | 414×455 | `needs_review_text` (`room.info_dialog`) | Tiêu đề KR "방정보변경" còn VN "THÔNG TIN" — khác nghĩa | `diff_bytes` | Như trên; nếu dùng thì lấy nhãn từ catalog, không tự chọn/sửa nhãn trong ảnh |
| `FxStage:Lobby/LB_Bt_T_ShowAllRenew` | 99×423 (9 frame) | `excluded` (`lobby.show_all`) | KR: "모든방보기" (xem mọi phòng); VN: "TOÀN BỘ" (tất cả) | `diff_bytes` | VN ngắn hơn nên có thể mất nghĩa "xem mọi phòng"; V1 không cần filter, vẽ nút bằng text từ catalog |
| `FxStage:Lobby/LB_Bt_T_WaitRoomRenew` | 99×423 (9 frame) | `excluded` (`lobby.wait_room`) | KR: "대기방보기" (xem phòng chờ); VN: "PHÒNG CHỜ" (danh từ) | `diff_bytes` | VN không phải hành động "xem phòng chờ"; V1 vẽ nút bằng text từ catalog |
| `FxStage:Lobby/ReadyBox` | 127×176 | `excluded` (`room.ready_indicator`) | KR: "준비중" (đang chuẩn bị); VN: "ĐẠI CHIẾN" | `diff_bytes` | **Sai nghĩa**: không dùng làm indicator ready; V1 vẽ trạng thái ready bằng text và màu, cần asset/key khác ở boom-re-l10n |
| `FxStage:Game/Win` | 243×59 (4 frame) | `needs_review_text` (`result.win`) | Chữ Latin "WIN!!" ghép trong ảnh; VN frame 0 chỉ đọc được "THN", chuỗi frame chưa ghép đủ ⇒ **chưa xác nhận** | `diff_bytes` | Ảnh động nhiều frame; duyệt cả chuỗi frame và xác nhận đủ chữ, không chỉ frame 0 |
| `FxStage:Game/Lose` | 317×59 (5 frame) | `needs_review_text` (`result.lose`) | Chữ Latin "LOSE!!" ghép trong ảnh; VN frame 0 chỉ đọc được "SC" ⇒ **chưa đọc rõ** | `diff_bytes` | Chuỗi frame VN không đọc ra từ rõ ràng; đánh dấu `check`, cần người duyệt xem đủ frame |
| `FxStage:Game/Draw` | 366×59 (5 frame) | `needs_review_text` (`result.draw`) | Chữ Latin "DRAW!!" ghép trong ảnh; VN frame 0 chỉ đọc được "HO" ⇒ **chưa đọc rõ** | `diff_bytes` | Dấu có thể nằm ở frame khác; duyệt cả frame và kiểm dấu |
| `FxStage:Game/ResultInfoBar` | 356×288 (6 ảnh) | `excluded` (`result.info_bar`) | Spritesheet trộn nhãn KR/EN và cột EXP/LUCCI; VN có bản tương ứng | `diff_bytes` | Không dùng cả sheet: V1 không có tiền/EXP và chỉ cần cột `Hạng/Người chơi/Kết quả` → vẽ bằng text từ catalog |
| `FxStage:Game/ResultInfoGood` | 98×144 (4 frame) | `selected` (`result.info_good`) | Chỉ icon bàn tay (ngón cái), không thấy chữ | `no` | Đã đọc trực tiếp PNG giải mã (pack KR 00967, 2026-10-20): chỉ có icon ngón cái, không thấy chữ nên không cần duyệt text |

Các asset đã chọn khác trong `kr_v1_content.json` (`FxMap/Map/Village00`, preview, theme, BGM, Attr, sprite
C00/C01, Bomb/Fire/Roller, Bomb00/BombExplosion001) không nằm trong `images_todo.csv` như ứng viên chữ,
hoặc là sprite không chữ; không cần duyệt text. Nếu màn kết quả dùng art khác `Win/Lose/Draw`, phải bổ sung
asset vào evidence trước khi dùng.

### 3.1 Phần tử UI chưa có asset

Bảng UI của closure còn ghi các phần tử mà V1 **không dùng asset nào**; chúng vẫn có
trạng thái `excluded` kèm lý do, để client không im lặng thiếu ảnh:

| ID | Màn hình | Vai trò | Vì sao không có asset |
|---|---|---|---|
| `login.submit` | login | button | `login.submit` ở §2.2 đã chốt lấy key catalog → V1 vẽ nút bằng text |
| `match.background` | match | background | Nền trận là bản đồ V1: lưới ô và sprite nền đã ở closure (role `map`/`tile`) |
| `match.hud` | match | info | HUD trận (đồng hồ, trạng thái nhốt/chết, tên item) chưa có asset; §2.5 ghi `unknown` → V1 vẽ bằng text |
| `match.navigation` | match | button | Trong trận không có nút điều hướng; đường về phòng mở ở màn kết quả |
| `result.background` | result | background | Chưa có asset nền riêng cho màn kết quả → V1 dùng nền màu, chữ lấy từ catalog |
| `result.continue` | result | button | `result.continue` ở §2.6 là `unknown` → V1 vẽ nút bằng text |

## 4. Đối chiếu 23 mục REVIEW đang mở

`review_pending.csv` có đúng **23** dòng `review`; **không dòng nào** thuộc tập khóa V1 ở §2. Vì vậy V1
không được dùng bất kỳ dòng nào dưới đây làm text đã chốt:

`AIBattleEventNpcRewardDateCount`, `BigMonsterEventNpcRewardDateCount`, `mons2fire01default28`,
`MonSnowBear12default07`, `MonSnowBear13default07`, `MonSnowBear19default06`, `MonSnowBear25default06`,
`MonSnowBear27default07`, `Pet_Say_InGame_046_00_00`, `Pet_Say_InGame_046_00_01`,
`Pet_Say_InGame_046_00_02`, `Pet_Say_Prepare_046_00_00`, `Pet_Say_Prepare_046_01_00`,
`PremiumSlotRoomTicket`, `StageClearEventNpcRewardDateCount`, `NPpen06MonIceA5`,
`q:CAMarbleLarva_UseMsg_ExchangeDeco`, `q:LoginReward_Mon2_Reward09`,
`FxMap:Map/MonS2Fire01:say02#196`, `FxDesktop:NoticeImg/Contract`, `FxDesktop:NoticeImg/Notice2PW`,
`exe:kr-full.exe:0x15e67bc`, `exe:kr-full.exe:0x15e69b4`.

Hai mục gần phòng/đăng nhập là `PremiumSlotRoomTicket` (phòng premium, ngoài V1) và
`q:LoginReward_Mon2_Reward09` (quà đăng nhập event, ngoài V1); không được kéo vào V1 chỉ vì tên gần.

## 5. Font, wrap, scale và focus

### 5.1 Font

| Yêu cầu | Quyết định/đề xuất | Nguồn/trạng thái |
|---|---|---|
| Font chính | **Be Vietnam Pro 1.002** (OFL 1.1), thiết kế cho tiếng Việt; file `assets/fonts/BeVietnamPro-Regular.ttf` | `design` — task #112 (chọn + bundle); nguồn/giấy phép ở §5.1.1 |
| Font đậm | **Be Vietnam Pro 1.002 Bold**, file `assets/fonts/BeVietnamPro-Bold.ttf` | `design` — task #112; cùng nguồn với font chính |
| Fallback Latin | **Noto Sans** phủ Latin Extended Additional | `candidate`; chưa bundle (xem §6) |
| Fallback Hàn | **Noto Sans KR** cho tên riêng/credit giữ chữ Hàn | `candidate`; chưa bundle (xem §6) |
| Không dùng font bitmap cũ | Không dùng `CxUni*.fft`, `CxDefault.fft` | `/home/admin/boom-re-l10n/l10n/README.md` §4 |
| Chuẩn hóa chuỗi | **NFC** trước khi shape; dữ liệu VN cũ có dấu tổ hợp cp1258 | Như trên; bắt buộc về kỹ thuật |
| License | Đã kèm `assets/fonts/OFL.txt` (SIL OFL 1.1) + version + nguồn tải + SHA-256 | `design` — task #112; xem §5.1.1 |
| Fallback chain | `[Be Vietnam Pro, Noto Sans, Noto Sans KR]` | Yêu cầu kỹ thuật; **mới bundle phần đầu** (task #112) |

Font phải phủ tối thiểu: Latin cơ bản, Latin Extended Additional cho `ă â đ ê ô ơ ư` và đủ tổ hợp dấu
(ví dụ `ắ ộ ữ ợ ỹ ặ ễ ỗ`), cùng Hangul cho tên riêng. Không cần emoji trong V1.

#### 5.1.1 Font đã bundle (task #112)

Font OFL **không phải asset game** nên được commit kèm license. Client nạp bằng
`include_bytes!` → không cần file font lúc chạy.

| File | Phiên bản | SHA-256 | Nguồn |
|---|---|---|---|
| `assets/fonts/BeVietnamPro-Regular.ttf` | 1.002 | `cd1ef6e9d7db28ad5cdb88a65ccbe693870e60d340b791f349d248342b4fe4c3` | google/fonts `ofl/bevietnampro/BeVietnamPro-Regular.ttf` @ `1cfe4cb468a703a247ca2d02410f80841b496218` |
| `assets/fonts/BeVietnamPro-Bold.ttf` | 1.002 | `7f738fe5c43c8872807b20e2d30d42163618de8a4daf7f48a939adac32c16847` | google/fonts `ofl/bevietnampro/BeVietnamPro-Bold.ttf` @ `1cfe4cb468a703a247ca2d02410f80841b496218` |
| `assets/fonts/OFL.txt` | OFL 1.1 | `6b7f8f73609a25ea78c891e34cf37b06f8a676b7ea986e941e43b009110f2a85` | google/fonts `ofl/bevietnampro/OFL.txt` @ `1cfe4cb468a703a247ca2d02410f80841b496218` |

Nguồn: <https://github.com/google/fonts/tree/1cfe4cb468a703a247ca2d02410f80841b496218/ofl/bevietnampro>
(commit 2021-08-27 — "Be Vietnam Pro: Version 1.002; ttfautohint (v1.8.3) added (#3771)"); dự án font
gốc: <https://github.com/bettergui/BeVietnamPro>. Bảng đầy đủ ở
[`assets/fonts/README.md`](../../../assets/fonts/README.md).

Giới hạn đã biết của font đã bundle:

- Font **không có** `U+27E8`/`U+27E9` (`⟨`/`⟩`) nên placeholder `⟨khóa⟩` của khóa chưa có
  text hiện **hộp báo thiếu** ở hai dấu ngoặc (client không bỏ qua âm thầm). Muốn hiện
  đúng ngoặc thì cần bundle thêm font fallback (xem §6).
- Font không có Hangul ⇒ tên riêng/credit giữ chữ Hàn cần font fallback.

### 5.2 Wrap, kích thước và placeholder

- Xuống dòng theo **ranh giới từ** (khoảng trắng); không bẻ giữa một âm tiết/tổ hợp dấu; không hyphenation.
- Nhãn nút/label ưu tiên 1 dòng; hộp thoại/tooltip tối đa 2–3 dòng. Nếu text vượt khung, **chỉ điều chỉnh
  hiển thị**: tăng chiều cao/chiều rộng, xuống dòng theo từ, hoặc giảm cỡ font đến ngưỡng tối thiểu đọc được
  (chốt bằng golden, không nhỏ hơn cỡ chữ phụ của profile). **Không rút, cắt, viết lại hay thay đổi nội dung
  catalog.** Nếu text vẫn không vừa sau các điều chỉnh hiển thị, ghi **“cần sửa ở boom-re-l10n”** kèm
  `catalog:<id>` và không render bản tự sửa.
- Chiều cao dòng ≥ 1,4× cỡ chữ để dấu chồng không chạm dòng trên/dưới; kiểm bằng ảnh.
- Giữ nguyên mọi placeholder: `%s`, `%d`, `%u`, `%.1f%%`, `%02d`, `{ComponentType|N}`, `<font …>`, `\r\n`,
  `|`. Không dịch/xóa placeholder; lệch phải hiện cảnh báo ở test.
- Chat: dùng giới hạn `150CharLimit` (`[150자 이내]`) làm nguồn; V1 đếm **150 ký tự Unicode theo grapheme
  cluster** (không cắt dấu), chốt ở spec auth/chat.
- Tên người chơi: chưa chốt độ dài. Phải hiển thị được trong danh sách phòng, slot phòng, HUD và kết quả;
  nếu vượt khung thì tăng chiều rộng/xuống dòng hoặc ghi **“cần sửa ở boom-re-l10n”** cho giới hạn tên; không
  ellipsis/cắt tên và không làm vỡ layout. `unknown` cho tới khi có validation.
- Số (thời gian, số người, thứ hạng) dùng chữ số tabular/không cắt số 0.
- Scale: asset gốc 800×600. V1 phải chạy cửa sổ/toàn màn hình; golden tối thiểu 800×600, 1280×720,
  1920×1080 ở 100%/150%/200% UI scale (nếu có), không clipping.
- Focus/IME: tab order rõ, focus ring thấy được; Enter gửi/submit, Escape đóng dialog; phím WASD không bị
  IME/ô chat nuốt khi ngoài ô chat; gõ Telex/VNI trong ô chat/tên phòng không làm mất dấu.

#### 5.2.1 Quy tắc layout đã hiện thực (task #112)

Client hiện thực phần layout ở `crates/boom-client/src/text.rs` (lớp thuần, đo bằng metric
của file TTF nên test được không cần GPU). Quy tắc đã chốt:

- `\r\n` và `\r` được chuẩn hóa thành `\n` **trước** khi đo; `\n` trong chuỗi là xuống dòng
  cứng, không phải chỗ trắng để wrap.
- Xuống dòng theo ranh giới từ; khoảng trắng cuối dòng bị bỏ (không tính vào bề rộng).
- Từ dài hơn khung thì **cắt theo cụm chữ (grapheme cluster)**, không cắt giữa cụm chữ; không
  rút, không ellipsis, không viết lại nội dung catalog.
- Chiều cao dòng = max(metric font, 1,4× cỡ chữ) theo mục “Chiều cao dòng” ở trên.
- Bề rộng đo bằng metric font; vị trí từng ký tự làm tròn về số nguyên (glyph không bị mờ khi
  phóng) nên mép chữ có thể vượt khung dưới 0,5 px.
- Placeholder `{0}`, `{1}`… được thay bằng tham số khi có; `{N}` không có tham số thì giữ
  nguyên văn. Placeholder khác (`%s`, `{ComponentType|N}`, `<font …>`, `|`) giữ nguyên.
- Khóa chưa có text hiện `⟨khóa⟩` (`boom_content::l10n::placeholder`); client không tự viết chữ.
- Ký tự không có glyph trong font được vẽ thành **hộp báo thiếu**, không bỏ qua âm thầm.

Golden layout (số dòng + bề rộng của 44 chuỗi V1 có text ở 1× và 2×) nằm ở test
`text::tests::golden_layout_matches`; ảnh layout local sinh bằng
`cargo test -p boom-client -- --ignored write_golden_images --nocapture`.

### 5.3 Acceptance cho V1-035

1. Mọi chuỗi hiển thị trong luồng V1 có một dòng ở §2; dòng `unknown` không được render tới khi có
   locator catalog/ảnh được duyệt. Không render chuỗi server gửi dạng free text.
2. Mọi dòng `catalog-translated`/`catalog-reused`/`review` dẫn `id` tồn tại; client lấy text từ
   `catalog.vi`. Spec này không lưu bản dịch riêng và không yêu cầu sửa bản dịch đã có.
3. Ảnh dùng trong luồng có quyết định `translated`/`no_text` trong `images_review.csv` (không còn
   `pending`), hoặc client render text từ key catalog đã chốt.
4. Text không vừa: chỉ tăng chiều cao/chiều rộng, wrap hoặc giảm cỡ font (có ngưỡng tối thiểu đọc được);
   không rút/cắt/ellipsis/viết lại text. Nếu vẫn không vừa, phải có dòng **“cần sửa ở boom-re-l10n”** kèm
   `catalog:<id>`.
5. Golden font/wrap: tối thiểu các cặp dấu `ắ ộ ữ ợ ỹ`, tên người chơi dài, chat 150 ký tự, tooltip 3 dòng,
   800×600 và 1920×1080; không clipping.
6. `cargo xtask ci` xanh; `python3 tools/docs_check.py` xanh.

## 6. Còn thiếu và bước tiếp

Số đếm khóa thiếu text do tool sinh nằm ở [§6.1](#61-báo-cáo-khóa-thiếu-v1-035-phần-1); bảng dưới
đây nêu việc còn phải làm.

| Thiếu | Ảnh hưởng | Bước tiếp |
|---|---|---|
| 42 khóa `unknown` chưa có locator catalog/ảnh | Không render được đúng nguồn | **Cần sửa ở `boom-re-l10n`**: thêm key/asset tương ứng; repo này không tự soạn bản dịch |
| 15 dòng `review` cần chọn lại ngữ cảnh/format | Có thể chọn sai key dù text catalog đúng | **Cần chọn/sửa key ở `boom-re-l10n`**; giữ text catalog, không viết lại |
| 3.038 ảnh vẫn `pending`, chưa có `l10n/images_review.csv` | Không được coi ảnh nào là đã Việt hóa | Reviewer duyệt theo [IMAGE_REVIEW](IMAGE_REVIEW.md), import CSV; bắt đầu từ 8 ảnh UI có chữ của V1-032 (§3) |
| 8 ảnh UI có chữ ở trạng thái `needs_review_text` | Client phải vẽ text từ catalog thay cho ảnh | Duyệt ảnh (`translated`/`no_text`) rồi import CSV; đổi trạng thái trong bảng UI của closure |
| Chưa có font/license trong repo | ~~V1-035 chưa bundle được font~~ | **Đã xong ở task #112**: Be Vietnam Pro 1.002 + `OFL.txt` trong `assets/fonts/`, nguồn/SHA-256 ở §5.1.1 |
| Fallback chain chưa bundle (`Noto Sans`, `Noto Sans KR`) | Placeholder `⟨khóa⟩` hiện hộp báo thiếu ở `⟨`/`⟩`; tên riêng chữ Hàn chưa vẽ được | Bundle thêm font fallback (OFL) hoặc chốt hiển thị thay thế; chưa có task |
| Chưa có golden ảnh theo khung 800×600/1920×1080 | Chưa chứng minh được UI scale ở các độ phân giải | Thuộc phần UI (#103): golden layout đã có ở `boom-client::text`, còn golden theo khung cần màn hình |
| Chưa map C00/C01 → tên hiển thị | Slot phòng/character select không có nhãn đúng | RE/trace hoặc dữ liệu tên nhân vật; ghi claim/unknown trước khi hiển thị |
| Nền login/lobby/prepare là JPEG 800×600 có chữ và nhãn ngoài scope | V1-032 không đưa nền vào atlas: bộ giải mã bundle chỉ đọc sprite I16, và ảnh có tab chế độ/auto-ready | V1 dựng nền bằng màu, chữ lấy từ catalog (đã ghi ở §3); muốn dùng ảnh nền thì cần bộ giải mã JPEG + bản ảnh đã che chữ (quyết định kiến trúc, thuộc integrator) |
| Màn kết quả chỉ có art `Win/Lose/Draw` (đều `needs_review_text`) và icon ngón cái | V1-032 không dead end ở result: cột Hạng/Người chơi/Kết quả vẽ bằng text từ catalog | Duyệt ảnh kết quả; nếu cần art khác thì bổ sung asset vào evidence trước khi dùng |
| Golden focus/scale chưa có | Chưa chứng minh được V1-035 cho focus và scale | Golden layout (wrap ở 1×/2×) đã có ở `boom-client::text` (§5.2.1); còn golden ảnh theo khung và focus → tạo sau khi client có renderer |

Không thêm claim luật chơi mới trong tài liệu này. Các giá trị thời gian nhắc ở §2 (120 s, 10 s, 5 s, 15 s)
đã có trong [RULES_V1](../gameplay/RULES_V1.md) R-010 và [SCOPE_V1](../../planning/SCOPE_V1.md) §3; UI chỉ
hiển thị/đếm, không tự đặt lại.

### 6.1 Báo cáo khóa thiếu (V1-035 phần 1)

`tools/l10n_export.py` đọc §2 của tài liệu này và `catalog.csv`, xuất
`data/l10n/vi-v1.toml` rồi in số đếm. Chạy lại khi §2 hoặc catalog đổi:

```sh
python3 tools/l10n_export.py            # xuất lại file
python3 tools/l10n_export.py --check    # kiểm file đã xuất (không cần repo boom-re-l10n)
```

| Trạng thái §2 | Số khóa | Có `text`? | Mã `reason` khi thiếu |
|---|---:|---|---|
| `catalog-translated` | 33 | có (`catalog.vi` nguyên văn) | — |
| `catalog-reused` | 11 | có (`catalog.vi` nguyên văn) | — |
| `review` | 15 | không | `needs-review` |
| `image-pending` | 9 | không | `image-pending-review` |
| `unknown` | 42 | không | `no-locator` |
| **Tổng** | **110** | **44 có text / 66 chỉ có lý do** | — |

Quy tắc của file xuất (nguồn chuẩn vẫn là `catalog.csv`, không lưu bản VI riêng):

- `text` lấy **nguyên văn** `catalog.vi`, chỉ trim biên; giữ nguyên `%s`, `{0}`,
  `<font …>`, CRLF và `|`.
- Tool **không xuất** nếu placeholder của bản dịch lệch số lượng với bản Hàn `ko`, hoặc
  text chưa ở dạng NFC.
- Khóa chưa có text **không có** trường `text`: client hiện placeholder rõ ràng `⟨khóa⟩`
  (`boom_content::l10n::placeholder`), không tự viết chữ.
- `catalog_sha256` trong file là SHA-256 của `catalog.csv` lúc xuất — dùng để biết file
  xuất từ catalog phiên bản nào (giá trị nằm trong file, không lặp ở tài liệu này).
- Loader `boom-content::l10n` từ chối file sai: khóa trùng, `text` có/không có không
  khớp `status`, thiếu `reason`/`catalog_id`/`image`, placeholder lệch giữa hai khóa dùng
  chung một `catalog_id`, và text có dấu tổ hợp (chưa chuẩn hóa NFC). Kiểm tra NFC **đầy
  đủ** nằm ở tool (`unicodedata`), loader chỉ chặn dấu tổ hợp.
