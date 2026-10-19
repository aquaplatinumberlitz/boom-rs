# Chuỗi và ảnh UI cho luồng V1 (Q-012)

Cập nhật: 2026-10-19. Phục vụ **V1-032** (UI luồng V1 bằng asset KR) và **V1-035** (Việt hóa UI V1),
câu hỏi [Q-012](../../research/QUESTIONS.md). Baseline khi lập: `main` @ `fefd5c0`, contract `contracts-r3`;
kiểm lại nguồn trên `main` @ `5845f15`, worktree `issue-15`. Nguồn chuẩn về scope là
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
| Asset UI V1 đã chọn | [kr_v1_content.json](../../evidence/inventory/kr_v1_content.json) | Chỉ 4 entry UI được chọn: `room_create`, `room_create_dialog`, `ready`, `start`; map/theme/nhân vật/item là asset khác. |
| Quy trình duyệt ảnh | [IMAGE_REVIEW](IMAGE_REVIEW.md) | Trạng thái `pending/translated/needs_translation/no_text/check`; không tự đánh dấu đã dịch. |

Cột **Locator** chỉ dùng một trong các dạng:

- `catalog:<id>` — dòng tương ứng trong `catalog.csv`; text hiển thị lấy từ `catalog.vi`.
- `ảnh <pack:entry>` — chữ nằm trong ảnh; phải duyệt ảnh trước khi dùng, hoặc render text từ key catalog.
- `—` — chưa có locator phù hợp; ghi `unknown`, không render cho tới khi bổ sung nguồn.

Cột **Trạng thái nguồn**:

| Nhãn | Nghĩa |
|---|---|
| `catalog-translated` | Có `id` catalog status `translated`; dùng `catalog.vi` nguyên văn (chỉ cắt khoảng trắng/xuống dòng). |
| `catalog-reused` | Có `id` catalog status `reused`; dùng `catalog.vi` nguyên văn, đọc lại ngữ cảnh. |
| `review` | Có `id` catalog nhưng ngữ cảnh/format cần reviewer chọn; **không** viết lại text, chỉ chọn key hoặc đánh dấu chưa dùng. |
| `image-pending` | Chữ nằm trong ảnh; ảnh đang `pending` trong `images_review.csv`, chưa được dùng làm bản dịch. |
| `unknown` | Chưa có locator catalog/ảnh phù hợp; không render tới khi bổ sung key/asset và duyệt. |

Tổng hợp 110 khóa: **44 dòng có locator catalog** (33 `catalog-translated` + 11 `catalog-reused`),
**15 dòng `review`**, **9 dòng `image-pending`** và **42 dòng `unknown`**. Không có dòng nào tự soạn
bản VI hay yêu cầu sửa bản dịch đã có.

## 2. Khóa V1 theo màn hình

### 2.1 Khung ứng dụng, kết nối và lỗi

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `app.loading` | Dòng trạng thái tải | `catalog:Loading` | `catalog-reused` | Lấy `catalog.vi`; giữ dấu `...`. |
| `app.connecting` | Dòng đang kết nối | `catalog:ConnectingToSvr` | `review` | Catalog là câu chờ chung; reviewer chọn key/ngữ cảnh, không tự soạn bản mới. |
| `app.network_error` | Hộp thoại mất mạng | `catalog:NetworkError` | `catalog-translated` | Lấy `catalog.vi`; wrap tối đa 3 dòng. |
| `app.session_replaced` | Phiên bị thay | `catalog:szConnCloseMsg` | `catalog-translated` | Lấy `catalog.vi`; tối đa 2 dòng. |
| `app.auth_failed` | Đăng nhập thất bại | `catalog:ErrLoginNotRegistered` | `review` | Catalog gắn luồng Nexon; reviewer chọn key phù hợp, không tự soạn. |
| `app.version_mismatch` | Handshake phiên bản | — | `unknown` | Chưa có locator catalog/ảnh; bổ sung ở task auth/protocol. |
| `app.content_mismatch` | Handshake rules/content | — | `unknown` | Chưa có locator; bổ sung cùng task auth/protocol. |
| `app.quit_confirm` | Hộp thoại thoát | — | `unknown` | Cần key catalog hoặc asset; không tự soạn. |
| `app.invalid_map` | Map không hợp lệ | `catalog:InvalidMapReturnToLobby` | `catalog-translated` | Lấy `catalog.vi`. |

### 2.2 Đăng nhập

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `login.title` | Tiêu đề màn hình | — | `unknown` | Tên hiển thị do chủ dự án chốt; chưa có key catalog. |
| `login.account` | Nhãn ô tài khoản | `catalog:NewUserDlgID` | `review` | Catalog chỉ là nhãn ID; reviewer chọn key, không tự soạn. |
| `login.password` | Nhãn ô mật khẩu | `catalog:Password` | `catalog-translated` | Lấy `catalog.vi`. |
| `login.submit` | Nút đăng nhập | `catalog:DailyMission_Mission_Login` | `review` | Catalog gắn nhiệm vụ ngày; reviewer chọn key. |
| `login.register` | Nút tạo tài khoản | — | `unknown` | Chưa có key; phụ thuộc quyết định V1-001. |
| `login.failed` | Sai tài khoản/mật khẩu | `catalog:ErrLoginNotRegistered` | `review` | Catalog gắn Nexon; reviewer chọn key. |
| `login.empty` | Thiếu trường bắt buộc | — | `unknown` | Chưa có key; bổ sung khi chốt spec auth. |
| `login.account_rule` | Quy tắc tên tài khoản | — | `unknown` | Chưa có spec; không render tới khi có nguồn. |

### 2.3 Sảnh

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `lobby.title` | Tiêu đề sảnh | — | `unknown` | Chưa có key; chủ dự án chốt tên hiển thị. |
| `lobby.channel` | Tên kênh | — | `unknown` | Chưa có key nhãn; V1 một kênh. |
| `lobby.channel_desc` | Tooltip kênh | `catalog:ChannelExplain1` | `catalog-translated` | Lấy `catalog.vi`; tooltip. |
| `lobby.room_name` | Cột tên phòng | `catalog:CreateDlgTitle` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.room_host` | Cột chủ phòng | — | `unknown` | Chưa có key nhãn. |
| `lobby.room_count` | Cột số người | `catalog:BigBattle_Lobby_Desc_UserCount` | `review` | Catalog là format BigBattle; reviewer chọn format, giữ `%d`. |
| `lobby.room_status` | Cột trạng thái | `catalog:RoomInfoDlgStatus` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.state_waiting` | Trạng thái chờ | — | `unknown` | Chưa có key; không tự soạn. |
| `lobby.state_playing` | Trạng thái đang chơi | — | `unknown` | Chưa có key; không tự soạn. |
| `lobby.empty` | Danh sách rỗng | — | `unknown` | Chưa có key; không tự soạn. |
| `lobby.create` | Nút tạo phòng | `ảnh FxStage:Lobby/LB_Bt_T_Create-NM` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `lobby.join` | Nút vào phòng | — | `unknown` | Chưa có key/asset; không tự soạn. |
| `lobby.refresh` | Nút làm mới | — | `unknown` | Chưa có key/asset; không tự soạn. |
| `lobby.back` | Nút quay lại | `catalog:BackButton` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.options` | Nút tùy chọn | `catalog:GBOption` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.help` | Nút trợ giúp | `catalog:GBHelp` | `catalog-reused` | Lấy `catalog.vi`; có thể ẩn nếu chưa có nội dung. |
| `lobby.exit` | Nút thoát game | `catalog:GBExit` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.chat_tab_all` | Tab chat chung | `catalog:LobbySquare_ChatTab_All` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.chat_tab_notice` | Tab thông báo | `catalog:LobbySquare_ChatTab_Notice` | `catalog-reused` | Lấy `catalog.vi`. |
| `lobby.chat_hint` | Gợi ý ô chat | — | `unknown` | Catalog `Chatting` là nhãn, không phải placeholder; cần key khác. |
| `lobby.chat_send` | Nút gửi chat | — | `unknown` | Chưa có key; có thể dùng Enter, không cần nút. |
| `lobby.chat_too_long` | Chat quá dài | `catalog:150CharLimit` | `catalog-translated` | Dùng giới hạn 150; đếm grapheme cluster, giữ `%`/`[...]` nếu có. |
| `lobby.join_failed` | Vào phòng thất bại | `catalog:CantJoinRoom` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.room_full` | Phòng đã đủ | `catalog:InvalidMaxUser` | `review` | Catalog nói số người không hợp lệ; reviewer chọn key/ngữ cảnh. |
| `lobby.room_removed` | Phòng đã bị xóa | `catalog:RoomDestroyed` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.room_info_changed` | Thông tin phòng đổi | `catalog:RoomInfoChanged` | `catalog-translated` | Lấy `catalog.vi`. |
| `lobby.join_session` | Chat hệ thống có người vào | `catalog:JoinSession` | `catalog-translated` | Giữ `%s`; lấy `catalog.vi`. |
| `lobby.leave_session` | Chat hệ thống có người rời | `catalog:LeaveSession` | `catalog-translated` | Giữ `%s`; lấy `catalog.vi`. |

### 2.4 Phòng chờ

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `room.title_label` | Nhãn tên phòng trong dialog tạo | `catalog:CreateDlgTitle` | `catalog-reused` | Lấy `catalog.vi`. |
| `room.create_title` | Tiêu đề dialog tạo phòng | `ảnh FxStage:Lobby/Dlg_LB-NMCreate` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.create_submit` | Nút tạo | — | `unknown` | Chưa có key/asset; không tự soạn. |
| `common.cancel` | Nút hủy | — | `unknown` | Catalog không có nhãn hủy đơn giản; cần key/asset. |
| `common.confirm` | Nút đồng ý | `catalog:OptionDlgYes` | `review` | Catalog là 'Có' trong hộp thoại tùy chọn; reviewer chọn key/ngữ cảnh. |
| `common.close` | Nút đóng | — | `unknown` | Chưa có key; cần key/asset. |
| `room.info_title` | Tiêu đề thông tin phòng | `ảnh FxMap:Prepare/PRE_Bt_Roominfo`, `ảnh FxMap:Prepare/Dlg_PRE-NMRoominfo` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.host` | Nhãn chủ phòng | — | `unknown` | Chưa có key nhãn. |
| `room.players` | Nhãn danh sách người chơi | — | `unknown` | Chưa có key nhãn. |
| `room.status` | Nhãn trạng thái | `catalog:RoomInfoDlgStatus` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.id` | Nhãn ID phòng | `catalog:RoomInfoDlgID` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.map` | Nhãn bản đồ | `ảnh FxMap:Prepare/CAStage` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.map_name` | Tên map V1 | `catalog:map:Village00` | `catalog-translated` | Lấy `catalog.vi`; reviewer chốt dùng tên catalog hay tên kỹ thuật. |
| `room.character` | Nhãn chọn nhân vật | `ảnh FxMap:Prepare/CAStage` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì render text từ key catalog được chốt. |
| `room.character_c00` | Tên hiển thị C00 | — | `unknown` | Chưa map C00 → tên; cần dữ liệu/claim trước khi render. |
| `room.character_c01` | Tên hiển thị C01 | — | `unknown` | Chưa map C01 → tên; cần dữ liệu/claim trước khi render. |
| `room.ready` | Trạng thái/nút sẵn sàng | `ảnh FxMap:Prepare/PRE_Bt_Ready` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog. |
| `room.ready_cancel` | Hủy sẵn sàng | `ảnh FxMap:Prepare/PRE_Bt_Ready` | `image-pending` | Cùng nút ready; duyệt ảnh hoặc render text từ key catalog. |
| `room.start` | Nút bắt đầu | `ảnh FxMap:Prepare/PRE_Bt_Start` | `image-pending` | Duyệt ảnh `translated`; nếu chưa duyệt thì render text từ key catalog. |
| `room.waiting_players` | Chờ người chơi | `catalog:BigBattle_Lobby_Desc_Line0` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.min_players` | Chưa đủ người để bắt đầu | `catalog:SmallPlayerNo` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.all_ready` | Chưa ready hết | `catalog:AllMustReady` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.loading` | Loading barrier | `catalog:Loading` | `catalog-reused` | Lấy `catalog.vi`; nhắc 15 giây theo SCOPE_V1 §3 nếu hiển thị đếm ngược. |
| `room.kick` | Nút mời ra | — | `unknown` | Chưa có key nhãn; catalog chỉ có help text. |
| `room.kick_confirm` | Xác nhận mời ra | `catalog:SureToBan` | `catalog-translated` | Giữ `%s`; lấy `catalog.vi`. |
| `room.kicked` | Bị chủ phòng mời ra | `catalog:BannedByLeader` | `catalog-translated` | Lấy `catalog.vi`; cắt khoảng trắng đầu. |
| `room.non_host_denied` | Non-host start/kick | `catalog:RoomLeaderOnly` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.name_changed` | Đổi tên phòng | `catalog:RoomNameChanged` | `catalog-translated` | Lấy `catalog.vi`. |
| `room.host_changed` | Chuyển chủ phòng | — | `unknown` | Chưa có key; không tự soạn. |
| `room.slot_empty` | Slot trống | — | `unknown` | Chưa có key; không tự soạn. |
| `room.chat_to_all` | Phạm vi chat | `catalog:ChatToAll` | `catalog-translated` | Lấy `catalog.vi`; V1 FFA không có `<Đồng đội>`. |
| `room.stats_bomb` | Nhãn số bom | `catalog:PREEA` | `review` | Catalog là nhãn chung '개수'; reviewer chọn key/ngữ cảnh. |
| `room.stats_fire` | Nhãn tầm nổ | `catalog:PREPOW` | `review` | Catalog là nhãn '물줄기'; reviewer chọn key/ngữ cảnh. |
| `room.stats_speed` | Nhãn tốc độ | `catalog:PRESPD` | `catalog-reused` | Lấy `catalog.vi`. |
| `room.leave` | Nút rời phòng | — | `unknown` | Catalog `GBExit` là thoát game, không phải rời phòng; cần key khác. |
| `room.leave_ready_confirm` | Xác nhận rời khi đang ready | `catalog:WannaExitToLobby` | `catalog-translated` | Lấy `catalog.vi`; reviewer chốt ngữ cảnh V1. |

### 2.5 Trong trận

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `match.timer` | Nhãn đồng hồ | — | `unknown` | Chưa có key nhãn; giá trị 120 s lấy từ R-010, UI chỉ hiển thị. |
| `match.timer_format` | Định dạng mm:ss | — | `unknown` | Định dạng kỹ thuật, chưa có key catalog; không tự soạn. |
| `match.go` | Hiệu lệnh bắt đầu | `catalog:TutorialClickStartBackStress1` | `review` | Catalog là nhãn tutorial 'Nút Bắt đầu'; reviewer chọn key/ngữ cảnh. |
| `match.trapped` | Trạng thái bị nhốt | — | `unknown` | Chưa có nhãn trạng thái; R-026/R-036 chỉ là luật. |
| `match.eliminated` | Trạng thái bị loại | `catalog:BlueMonSay_event4_01` | `review` | Catalog gắn sự kiện quái; reviewer chọn key/ngữ cảnh. |
| `match.disconnect_warning` | Cảnh báo mất kết nối | `catalog:ErrNetworkError` | `review` | Catalog là thông báo mất mạng chung, không có đếm ngược; reviewer chọn key. |
| `match.disconnect_eliminated` | Bị loại do mất kết nối | `catalog:ErrDisconnectByServer` | `review` | Catalog là thông báo ngắt kết nối chung; reviewer chọn key/ngữ cảnh. |
| `match.item_bomb` | Tên item Bomb | — | `unknown` | Tên item lấy từ ItemAttr, không phải catalog UI; cần locator dữ liệu. |
| `match.item_fire` | Tên item Fire | — | `unknown` | Tên item lấy từ ItemAttr; cần locator dữ liệu. |
| `match.item_roller` | Tên item Roller | — | `unknown` | Tên item lấy từ ItemAttr; cần locator dữ liệu. |
| `match.pickup` | Thông báo nhặt item | — | `unknown` | Chưa có key phù hợp; không tự soạn; giữ 1 `%s` cho tên item nếu có. |
| `match.result_pending` | Chờ server chốt kết quả | — | `unknown` | Chưa có key; không tự soạn. |

### 2.6 Kết quả

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `result.title` | Tiêu đề | `catalog:Olympic_TitleResult` | `catalog-translated` | Lấy `catalog.vi`. |
| `result.win` | Nhãn thắng | `catalog:StateWin` | `catalog-translated` | Lấy `catalog.vi`; ảnh `Win` cần duyệt nếu dùng. |
| `result.lose` | Nhãn thua | `catalog:StateLose` | `catalog-translated` | Lấy `catalog.vi`; ảnh `Lose` cần duyệt nếu dùng. |
| `result.draw` | Nhãn hòa | `ảnh FxStage:Game/Draw` | `image-pending` | Duyệt ảnh; nếu chưa duyệt thì dùng key catalog được chốt. |
| `result.rank` | Cột hạng | `catalog:Olympic_TitleRank` | `catalog-translated` | Lấy `catalog.vi`. |
| `result.player` | Cột người chơi | — | `unknown` | Chưa có key nhãn. |
| `result.continue` | Nút tiếp tục | — | `unknown` | Chưa có key nhãn; không tự soạn. |
| `result.auto_return` | Đếm ngược về phòng | — | `unknown` | Chưa có key; 5 giây là policy SCOPE_V1 §3. |
| `result.winner` | Người thắng | `catalog:BigBattle_Ingame_Winner_Line1` | `review` | Catalog là câu BigBattle, không có `%s`; reviewer chọn key/format. |
| `result.draw_all` | Hòa khi không còn ai | `catalog:TournamentNoWinner` | `catalog-translated` | Lấy `catalog.vi`; reviewer chốt ngữ cảnh giải đấu. |
| `result.waiting` | Chờ ack người khác | — | `unknown` | Chưa có key; không tự soạn. |

### 2.7 Tùy chọn

| Khóa V1 | Phần tử / ngữ cảnh | Locator nguồn | Trạng thái nguồn | Yêu cầu hiển thị |
|---|---|---|---|---|
| `options.fullscreen` | Chế độ toàn màn hình | `catalog:OptionDlgFullScreen` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.windowed` | Chế độ cửa sổ | `catalog:OptionDlgWindowMode` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.volume` | Âm lượng chung | — | `unknown` | Chưa có key nhãn âm lượng chung; cần key/asset. |
| `options.music` | Nhạc nền | `catalog:OptionDlgBGM` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.sfx` | Hiệu ứng âm | `catalog:OptionDlgEffect` | `catalog-translated` | Lấy `catalog.vi`. |
| `options.apply` | Áp dụng | — | `unknown` | Chưa có key nhãn; không tự soạn. |

### 2.8 Chuỗi không dùng trong V1

Các nhóm sau có trong catalog nhưng **không** thuộc luồng V1; client phải ẩn, không render, không dịch thêm:
shop/ví (`GoToShop`, `UserMarket`, `MyItem`, `Cash`, `Lucci`…), nhiệm vụ (`Quest`, `BnbNavi_*`), hội/bạn bè
(`GuildTab`, `Messenger`), kênh xếp hạng/giải đấu (`ChannelDesc50…82`, `Tournament*`), mode khác
(`CreateDlgRenew*`, `CreateDlgGameType*`, `GameMode*`), phòng kín/mật khẩu (`CreateDlgPrivate`,
`CreateDlgPassword`, `WrongPassword`), đội/spectator (`ChatToOurTeam`, `EnterObserver`), quảng trường đi bộ
(`LobbySquare_*`), auto-ready (`OptionDlgAutoReady`, `TurnOnAutoReady`, `TurnOffAutoReady`), premium/super
host (`PremiumSlotRoomTicket`, `BannedBySuper`, `ChiefBanned`).

## 3. Ảnh có chữ trong luồng V1

Cột **Đã chọn?** lấy từ `kr_v1_content.json` (`required`/`selected_resources`). `—` nghĩa là chưa nằm
trong closure V1; muốn dùng phải bổ sung evidence và review. Cột **Chữ trên ảnh** là đọc trực tiếp PNG giải
mã ngày 2026-10-19, chỉ là bằng chứng mô tả, **không phải** bản dịch do spec này đặt. Tất cả trạng thái duyệt
hiện là `pending`; muốn dùng phải import `images_review.csv` qua quy trình [IMAGE_REVIEW](IMAGE_REVIEW.md).

| Asset (pack:entry) | Kích thước | Đã chọn? | Chữ trên ảnh (bằng chứng) | VN cùng tên | Ghi chú review |
|---|---|---:|---|---|---|
| FxDesktop:LoginStage/CAStage | 800×600 JPEG | — | KR: overlay tiếng Hàn + huy hiệu "전체이용가"; VN: logo Boom + dòng "Nexon… Copyright" | `diff_bytes` | **Không dùng nguyên trạng**: có nhãn Nexon và bản quyền cũ; cần nền mới hoặc che chữ |
| FxStage:Lobby/CAStage | 800×600 JPEG | — | KR: hai nhãn tab (đọc được "대전 모드" / "대기 모드"); VN: `ĐẤU CHIẾN` / `NHIỆM VỤ` | `diff_bytes` | Chữ nằm trong ảnh nền; `NHIỆM VỤ` ngoài scope V1, phải che/redraw |
| FxMap:Prepare/CAStage | 800×600 JPEG | — | KR: "캐릭터선택" / "팀선택" / "미니맵" / "자동준비"; VN: `NHÂN VẬT` / `CHỌN ĐỘI` / `BẢN ĐỒ` / `TỰ ĐỘNG` | `diff_bytes` | `CHỌN ĐỘI` ngoài scope FFA V1; phải che/redraw; `TỰ ĐỘNG` là auto-ready ngoài V1 |
| FxStage:Lobby/LB_Bt_T_Create-NM | 92×495 | Có (`room_create`) | KR: `방만들기`; VN: `TẠO PHÒNG` | `diff_bytes` | Ảnh nút có chữ; nếu dùng phải duyệt `translated`, nếu không thì render text từ catalog |
| FxStage:Lobby/Dlg_LB-NMCreate | 414×455 | Có (`room_create_dialog`) | KR: `방만들기`; VN: `TẠO PHÒNG` | `diff_bytes` | Tiêu đề dialog nằm trong ảnh; phần thân là ô trống, nhãn do client vẽ |
| FxMap:Prepare/PRE_Bt_Ready | 193×342 | Có (`ready`) | KR: `준 비`; VN: `SẴN SÀNG` | `diff_bytes` | Nút ready có chữ; cần duyệt ảnh hoặc render text từ catalog |
| FxMap:Prepare/PRE_Bt_Start | 193×342 | Có (`start`) | KR: `시 작`; VN: `BẮT ĐẦU` | `diff_bytes` | Nút start có chữ; cần duyệt ảnh hoặc render text từ catalog |
| FxMap:Prepare/PRE_Bt_Roominfo | 100×110 | — | KR: `방정보 변경`; VN: `THÔNG TIN` | `diff_bytes` | Nghĩa KR là "đổi thông tin phòng", VN là "thông tin"; chọn nhãn nào phải chốt |
| FxMap:Prepare/Dlg_PRE-NMRoominfo | 414×455 | — | KR: `방정보 변경`; VN: `THÔNG TIN` | `diff_bytes` | Như trên; dialog V1 có thể chỉ để xem, không đổi tên |
| FxStage:Lobby/LB_Bt_T_ShowAllRenew | 99×423 | — | KR: `모든방보기`; VN: `TOÀN BỘ` | `diff_bytes` | VN rút gọn mất nghĩa "xem mọi phòng"; V1 có thể không cần filter |
| FxStage:Lobby/LB_Bt_T_WaitRoomRenew | 99×423 | — | KR: `대기방보기`; VN: `PHÒNG CHỜ` | `diff_bytes` | VN là danh từ, không phải hành động "xem phòng chờ" |
| FxStage:Lobby/ReadyBox | 127×176 | — | KR: `준비중`; VN: `ĐẠI CHIẾN` | `diff_bytes` | **Sai nghĩa**: không dùng làm indicator ready; cần ảnh/text khác |
| FxStage:Game/Win | 243×59 (nhiều frame) | — | KR: `WIN!!!`; VN: các frame đọc được `TH` + `Ắ` + `N` + `!!` (chưa thấy `G` trong bộ frame giải mã) | `diff_bytes` | Ảnh động nhiều frame; duyệt cả chuỗi frame và xác nhận đủ chữ, không chỉ frame 0 |
| FxStage:Game/Lose | 317×59 (nhiều frame) | — | KR: `LOSE!!!`; VN: frame đọc được `SC` / `A` / `N` / `U!!` / `O` | `diff_bytes` | Chuỗi frame VN không đọc ra từ rõ ràng; đánh dấu `check`, cần người duyệt xem đủ frame |
| FxStage:Game/Draw | 366×59 (nhiều frame) | — | KR: `DRAW!!!`; VN: `HO` + `A` + `!!` → `HÒA!!` | `diff_bytes` | Dấu `Ò` có thể nằm ở frame khác; duyệt cả frame và kiểm dấu |
| FxStage:Game/ResultInfoBar | 356×288 (6 ảnh) | — | Spritesheet trộn nhãn KR/EN: `ID`, `RANK`, `KILL`, `EXP`, `LUCCI+`, và các biến thể tiếng Hàn (마을/아이템/음표) | `diff_bytes` | Không dùng cả sheet; V1 chỉ cần cột `Hạng/Người chơi/Kết quả`; nếu dùng phải chọn frame + dịch |
| FxStage:Game/ResultInfoGood | 98×144 | — | Chỉ mũi tên, không thấy chữ | `no` | Có thể `no_text`; vẫn cần người duyệt xác nhận |

Các asset đã chọn khác trong `kr_v1_content.json` (`FxMap/Map/Village00`, preview, theme, BGM, Attr, sprite
C00/C01, Bomb/Fire/Roller, Bomb00/BombExplosion001) không nằm trong `images_todo.csv` như ứng viên chữ,
hoặc là sprite không chữ; không cần duyệt text. Nếu màn kết quả dùng art khác `Win/Lose/Draw`, phải bổ sung
asset vào evidence trước khi dùng.

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
| Font chính | **Be Vietnam Pro** (OFL 1.1), thiết kế cho tiếng Việt | Đề xuất ở `/home/admin/boom-re-l10n/l10n/README.md` §4; chưa có file trong repo |
| Fallback Latin | **Noto Sans** phủ Latin Extended Additional | Như trên |
| Fallback Hàn | **Noto Sans KR** cho tên riêng/credit giữ chữ Hàn | Như trên |
| Không dùng font bitmap cũ | Không dùng `CxUni*.fft`, `CxDefault.fft` | `/home/admin/boom-re-l10n/l10n/README.md` §4 |
| Chuẩn hóa chuỗi | **NFC** trước khi shape; dữ liệu VN cũ có dấu tổ hợp cp1258 | Như trên; bắt buộc về kỹ thuật |
| License | Phải kèm file license, version và nguồn tải trước khi bundle | `unknown`; chưa có trong repo, chưa được coi là đã chốt |
| Fallback chain | `[Be Vietnam Pro, Noto Sans, Noto Sans KR]` | Yêu cầu kỹ thuật; chưa bundle |

Font phải phủ tối thiểu: Latin cơ bản, Latin Extended Additional cho `ă â đ ê ô ơ ư` và đủ tổ hợp dấu
(ví dụ `ắ ộ ữ ợ ỹ ặ ễ ỗ`), cùng Hangul cho tên riêng. Không cần emoji trong V1.

### 5.2 Wrap, kích thước và placeholder

- Xuống dòng theo **ranh giới từ** (khoảng trắng); không bẻ giữa một âm tiết/tổ hợp dấu; không hyphenation.
- Nhãn nút/label ưu tiên 1 dòng; hộp thoại/tooltip tối đa 2–3 dòng; nếu vượt thì tăng chiều cao hoặc
  rút chữ, **không cắt chữ**. Đây là yêu cầu hiển thị, phải kiểm bằng golden.
- Chiều cao dòng ≥ 1,4× cỡ chữ để dấu chồng không chạm dòng trên/dưới; kiểm bằng ảnh.
- Giữ nguyên mọi placeholder: `%s`, `%d`, `%u`, `%.1f%%`, `%02d`, `{ComponentType|N}`, `<font …>`, `\r\n`,
  `|`. Không dịch/xóa placeholder; lệch phải hiện cảnh báo ở test.
- Chat: dùng giới hạn `150CharLimit` (`[150자 이내]`) làm nguồn; V1 đếm **150 ký tự Unicode theo grapheme
  cluster** (không cắt dấu), chốt ở spec auth/chat.
- Tên người chơi: chưa chốt độ dài. Phải hiển thị được trong danh sách phòng, slot phòng, HUD và kết quả;
  nếu vượt khung thì ellipsis + tooltip, không làm vỡ layout. `unknown` cho tới khi có validation.
- Số (thời gian, số người, thứ hạng) dùng chữ số tabular/không cắt số 0.
- Scale: asset gốc 800×600. V1 phải chạy cửa sổ/toàn màn hình; golden tối thiểu 800×600, 1280×720,
  1920×1080 ở 100%/150%/200% UI scale (nếu có), không clipping.
- Focus/IME: tab order rõ, focus ring thấy được; Enter gửi/submit, Escape đóng dialog; phím WASD không bị
  IME/ô chat nuốt khi ngoài ô chat; gõ Telex/VNI trong ô chat/tên phòng không làm mất dấu.

### 5.3 Acceptance cho V1-035

1. Mọi chuỗi hiển thị trong luồng V1 có một dòng ở §2; dòng `unknown` không được render tới khi có
   locator catalog/ảnh được duyệt. Không render chuỗi server gửi dạng free text.
2. Mọi dòng `catalog-translated`/`catalog-reused`/`review` dẫn `id` tồn tại; client lấy text từ
   `catalog.vi`. Spec này không lưu bản dịch riêng và không yêu cầu sửa bản dịch đã có.
3. Ảnh dùng trong luồng có quyết định `translated`/`no_text` trong `images_review.csv` (không còn
   `pending`), hoặc client render text từ key catalog đã chốt.
4. Golden font/wrap: tối thiểu các cặp dấu `ắ ộ ữ ợ ỹ`, tên người chơi dài, chat 150 ký tự, tooltip 3 dòng,
   800×600 và 1920×1080; không clipping.
5. `cargo xtask ci` xanh; `python3 tools/docs_check.py` xanh.

## 6. Còn thiếu và bước tiếp

| Thiếu | Ảnh hưởng | Bước tiếp |
|---|---|---|
| 42 khóa `unknown` chưa có locator catalog/ảnh | Không render được đúng nguồn | Bổ sung `id` catalog hoặc asset đã duyệt ở repo l10n; không tự soạn bản dịch trong task này |
| 14 dòng `review` cần chọn lại ngữ cảnh/format | Có thể chọn sai key dù text catalog đúng | Reviewer quyết định key/format; giữ text catalog, không viết lại |
| 3.038 ảnh vẫn `pending`, chưa có `l10n/images_review.csv` | Không được coi ảnh nào là đã Việt hóa | Reviewer duyệt theo [IMAGE_REVIEW](IMAGE_REVIEW.md), import CSV; bắt đầu từ 4 asset đã chọn |
| Chưa có font/license trong repo | V1-035 chưa bundle được font | Chọn version, thêm license/attribution, golden glyph; ngoài phạm vi docs này |
| Chưa map C00/C01 → tên hiển thị | Slot phòng/character select không có nhãn đúng | RE/trace hoặc dữ liệu tên nhân vật; ghi claim/unknown trước khi hiển thị |
| Màn kết quả chưa có asset trong closure | V1-032 có thể dead end ở result | Bổ sung asset/art vào evidence, hoặc thiết kế UI kết quả bằng text + asset đã có |
| Nền login/lobby/prepare có chữ và nhãn ngoài scope | Dùng nguyên trạng sẽ hiện Nexon/nhiệm vụ/đội | Che/redraw hoặc chọn asset khác; cần review ảnh |
| Chưa có golden wrap/scale/focus | Không chứng minh được V1-035 | Tạo golden ảnh + test glyph sau khi client có renderer |

Không thêm claim luật chơi mới trong tài liệu này. Các giá trị thời gian nhắc ở §2 (120 s, 10 s, 5 s, 15 s)
đã có trong [RULES_V1](../gameplay/RULES_V1.md) R-010 và [SCOPE_V1](../../planning/SCOPE_V1.md) §3; UI chỉ
hiển thị/đếm, không tự đặt lại.
