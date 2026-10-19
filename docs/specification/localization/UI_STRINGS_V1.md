# Chuỗi và ảnh UI cho luồng V1 (Q-012)

Cập nhật: 2026-10-19. Phục vụ **V1-032** (UI luồng V1 bằng asset KR) và **V1-035** (Việt hóa UI V1),
câu hỏi [Q-012](../../research/QUESTIONS.md). Baseline khi lập: `main` @ `fefd5c0`, contract `contracts-r3`;
kiểm lại nguồn trên `main` @ `5845f15`, worktree `issue-15`. Nguồn chuẩn về scope là
[SCOPE_V1](../../planning/SCOPE_V1.md), về luật trận là [RULES_V1](../gameplay/RULES_V1.md). Tài liệu này
**không** phải bản chốt cuối: nó là danh mục nguồn để reviewer chốt từng chuỗi/ảnh và để client dựng golden.

Luồng được xét: **đăng nhập → sảnh → phòng chờ → trận → kết quả → về phòng**. Chỉ liệt kê phần V1 dùng;
mọi thứ ngoài scope V1 (shop, túi đồ, nhiệm vụ, hội, bạn bè, kênh xếp hạng, mode khác, phòng kín, đội,
spectator, quảng trường đi bộ) bị loại, kể cả khi catalog có chuỗi/ảnh cho chúng.

## 1. Nguồn và cách đọc

| Nguồn | Vị trí | Đã dùng để làm gì |
|---|---|---|
| Catalog chuỗi KR | `/home/admin/boom-re-l10n/l10n/catalog.csv` | 25.257 dòng: `translated` 21.421, `reused` 3.813, `review` 23. Mỗi dòng có `id, source, ko, vi_base, vi, status, notes`; `source` là file/entry gốc. |
| Danh sách review đang mở | `/home/admin/boom-re-l10n/l10n/review_pending.csv` | 23 dòng `review`; không dùng làm text đã chốt. |
| Chuỗi UI VN cũ (ứng viên) | `/home/admin/boom-re/server/src/strings.rs` | 148 khóa; 146 khóa khớp `id` catalog, 2 khóa không có (`CannotConnectToShop`, `CheckRentedItem`). Đây là text nhóm dự án VN tự viết, **không phải** chuỗi gốc KR. |
| Ảnh có chữ | `/home/admin/boom-re-l10n/l10n/images_todo.csv` | 3.038 ứng viên I16/JPEG; chưa có quyết định duyệt nào. |
| Trạng thái duyệt ảnh | `/home/admin/boom-re-l10n/l10n/.cache/image-review/images_review.csv` | Toàn bộ 3.038 dòng `pending`; chưa có `l10n/images_review.csv` (kết quả thật). |
| Ảnh giải mã | `/home/admin/boom-re-l10n/l10n/.cache/image-review/` | PNG giải mã cục bộ, không commit; dùng để đọc chữ và kích thước. |
| Asset UI V1 đã chọn | [kr_v1_content.json](../../evidence/inventory/kr_v1_content.json) | Chỉ 4 entry UI được chọn: `room_create`, `room_create_dialog`, `ready`, `start`; map/theme/nhân vật/item là asset khác. |
| Quy trình duyệt ảnh | [IMAGE_REVIEW](IMAGE_REVIEW.md) | Trạng thái `pending/translated/needs_translation/no_text/check`; không tự đánh dấu đã dịch. |

Cách đọc cột **Nguồn** trong bảng chuỗi:

- `catalog:<id>` — dòng tương ứng trong `catalog.csv`; `source` của dòng đó là file/entry gốc KR (ví dụ
  `csysstr.ssd`, `FxDesktop/ClientData/ClientString`, `fxdynmapinfo.ssd`).
- `strings.rs:<dòng>` — khóa trong `boom-re/server/src/strings.rs`, chỉ là ứng viên.
- `DESIGN Q-012` — chuỗi mới của dự án, chưa có nguồn KR; phải được reviewer chốt.
- `review` — trạng thái cần reviewer chốt vì nguồn mâu thuẫn/không khớp ngữ cảnh. Đây **không phải**
  nhãn của 23 dòng `review_pending.csv`; 23 dòng đó được liệt kê riêng ở §4 và không được dùng.

Cột **Trạng thái** dùng các nhãn:

| Nhãn | Nghĩa |
|---|---|
| `catalog` | Dùng nguyên văn `vi` của dòng `translated` trong catalog (chỉ cắt khoảng trắng/xuống dòng); vẫn phải QA hiển thị. Nếu VI V1 khác bản catalog thì dòng đó không còn là `catalog`, phải là `review`/`design`. |
| `catalog-reused` | Lấy nguyên văn bản VN (giọng Bắc); phải đọc lại ngữ cảnh, có thể lệch nghĩa KR. |
| `strings.rs` | Chỉ là chuỗi VN cũ; chưa qua review l10n, không được coi là đã chốt. |
| `design` | Chuỗi dự án tự đặt; cần reviewer chốt nội dung và wrap. |
| `review` | Có nguồn catalog nhưng phải reviewer chốt vì mâu thuẫn nguồn/ngữ cảnh, hoặc vì VI V1 đã được sửa/rút gọn khác `catalog.vi`; không dùng làm final. |

## 2. Chuỗi theo màn hình

Tổng hợp 110 khóa trong 7 nhóm màn hình: **22** `catalog`, **9** `catalog-reused`, **8** `review`,
**68** `design`, **3** `unknown`; 43 dòng dẫn ít nhất một `id` catalog (42 id khác nhau) và 6 dòng
dẫn `strings.rs`. Ba khóa `unknown` (`login.account_rule`, `room.character_c00`, `room.character_c01`)
chưa được phép render cho tới khi có nguồn/quyết định.

### 2.1 Khung ứng dụng, kết nối và lỗi

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `app.loading` | Dòng "đang tải" | `catalog:Loading` | 로딩중... → Đang tải... | `catalog-reused`; dùng được, QA hiển thị |
| `app.connecting` | Đang kết nối máy chủ | `catalog:ConnectingToSvr` + `strings.rs:32` | 잠시만 기다려주세요 → Đang kết nối máy chủ... | `review`; catalog dịch "Vui lòng chờ trong giây lát", strings.rs rõ nghĩa hơn — chọn design, cần chốt |
| `app.network_error` | Mất mạng giữa phiên | `catalog:NetworkError` | 네트워크 에러 때문에 더 이상 게임을 진행할 수 없습니다. → Mất kết nối mạng. Không thể tiếp tục trận. | `review`; catalog dài, bản VI V1 rút gọn cho hộp thoại — cần reviewer chốt |
| `app.session_replaced` | Phiên bị chiếm | `catalog:szConnCloseMsg` | 다른 컴퓨터에서 연결을 요청하여 로그아웃 됩니다. → Tài khoản đăng nhập ở nơi khác nên phiên này bị đóng. | `review`; catalog nói "máy khác", bản VI V1 viết lại theo policy một phiên/tài khoản (SCOPE_V1 §3) — cần chốt |
| `app.auth_failed` | Đăng nhập thất bại | `catalog:ForbiddenAuthenForOnePlayer` + `strings.rs:35` | 현재 게임을 하고 계시는 곳은 미가입 PC방입니다. → Không thể đăng nhập. | `review`; bản catalog nói PC bang, không dùng; lấy nghĩa strings.rs, cần chốt |
| `app.version_mismatch` | Handshake phiên bản | `DESIGN Q-012` | — → Phiên bản client không khớp máy chủ. | `design`; V1-002 |
| `app.content_mismatch` | Handshake rules/content | `DESIGN Q-012` | — → Luật chơi hoặc nội dung không khớp máy chủ. | `design`; V1-002 |
| `app.quit_confirm` | Xác nhận thoát | `DESIGN Q-012` | — → Thoát game? | `design`; dùng chung với `common.cancel`/`common.confirm` |
| `app.invalid_map` | Map không hợp lệ | `catalog:InvalidMapReturnToLobby` | 올바른 맵이 아닙니다. 로비로 돌아갑니다. → Bản đồ không hợp lệ. Quay về sảnh. | `catalog`; dùng cho lỗi nội dung, cần QA |

### 2.2 Đăng nhập

Tài khoản V1 là tài khoản dự án mới ([SCOPE_V1](../../planning/SCOPE_V1.md) §1), **không** port auth
Nexon/NGM. Catalog có nhãn/lỗi auth cũ (`NewUserDlgID`, `Password`, `ErrLoginNotRegistered`,
`ConnFailedAtLogin`...) nhưng gắn luồng Nexon và không khớp form V1; vì vậy form dùng `design`, riêng
nhãn mật khẩu lấy `Password`. Reviewer có thể đổi sang chuỗi catalog nếu thấy khớp ngữ cảnh.

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `login.title` | Tiêu đề màn hình | `DESIGN Q-012` | — → Boom (tên tạm) | `design`; chủ dự án phải chốt tên hiển thị, không dùng logo Nexon |
| `login.account` | Nhãn ô tài khoản | `DESIGN Q-012` | — → Tài khoản | `design`; catalog `NewUserDlgID` chỉ là "ID" (Nexon), không dùng làm nhãn |
| `login.password` | Nhãn ô mật khẩu | `catalog:Password` | 비밀번호 → Mật khẩu | `catalog`; dùng được |
| `login.submit` | Nút đăng nhập | `DESIGN Q-012` | — → Đăng nhập | `design`; catalog `DailyMission_Mission_Login` là nhiệm vụ ngày, không dùng |
| `login.register` | Nút tạo tài khoản | `DESIGN Q-012` | — → Tạo tài khoản | `design`; chỉ hiện nếu V1-001 cho tự tạo tài khoản |
| `login.failed` | Sai tài khoản/mật khẩu | `DESIGN Q-012` | — → Sai tài khoản hoặc mật khẩu. | `design`; catalog `ErrLoginNotRegistered`/`AuthenNxPass_Fail` gắn Nexon; `WrongPassword` là phòng kín, không dùng |
| `login.empty` | Thiếu trường bắt buộc | `DESIGN Q-012` | — → Vui lòng nhập tài khoản và mật khẩu. | `design` |
| `login.account_rule` | Quy tắc tên tài khoản | `UNKNOWN` | — → chưa đặt | `unknown`; phải chốt ở spec auth (V1-001) trước khi hiển thị |

### 2.3 Sảnh

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `lobby.title` | Tiêu đề sảnh | `DESIGN Q-012` | — → Sảnh chờ | `design` |
| `lobby.channel` | Tên kênh | `DESIGN Q-012` | — → Kênh 1 | `design`; V1 một kênh (V1-003) |
| `lobby.channel_desc` | Mô tả kênh | `catalog:ChannelExplain1` | 자유채널 입니다. 누구나 입장 가능합니다. → Kênh tự do. Ai cũng vào được. | `catalog`; dùng làm tooltip, cần QA |
| `lobby.room_name` | Cột tên phòng | `DESIGN Q-012` | — → Tên phòng | `design`; catalog `CreateDlgTitle` là nhãn ô nhập, không phải cột |
| `lobby.room_host` | Cột chủ phòng | `DESIGN Q-012` | — → Chủ phòng | `design` |
| `lobby.room_count` | Cột số người | `catalog:BigBattle_Lobby_Desc_UserCount` | 참가인원 %d/%d 명 → %d/%d người | `review`; đảo thứ tự format so với catalog (`Người tham gia %d/%d`) — cần reviewer chốt |
| `lobby.room_status` | Cột trạng thái | `catalog:RoomInfoDlgStatus` | 상태 → Trạng thái | `catalog`; dùng được |
| `lobby.state_waiting` | Trạng thái phòng chờ | `DESIGN Q-012` | — → Đang chờ | `design` |
| `lobby.state_playing` | Trạng thái đang chơi | `DESIGN Q-012` | — → Đang chơi | `design` |
| `lobby.empty` | Danh sách rỗng | `DESIGN Q-012` | — → Chưa có phòng nào. | `design` |
| `lobby.create` | Nút tạo phòng | `DESIGN Q-012` | 방만들기 (ảnh) → Tạo phòng | `design`; ảnh VN đã có chữ `TẠO PHÒNG`, xem §3 |
| `lobby.join` | Nút vào phòng | `DESIGN Q-012` | — → Vào phòng | `design` |
| `lobby.refresh` | Nút làm mới | `DESIGN Q-012` | — → Làm mới | `design` |
| `lobby.back` | Nút quay lại | `catalog:BackButton` | 뒤로 가기 → Quay lại | `catalog-reused`; đọc lại ngữ cảnh |
| `lobby.options` | Nút tùy chọn | `catalog:GBOption` | 옵션 → Tùy chọn | `catalog-reused` |
| `lobby.help` | Nút trợ giúp | `catalog:GBHelp` | 도움말 → Trợ giúp | `catalog-reused`; V1 có thể ẩn nếu chưa có nội dung |
| `lobby.exit` | Nút thoát game | `catalog:GBExit` | 나가기 → Thoát game | `catalog-reused` |
| `lobby.chat_tab_all` | Tab chat chung | `catalog:LobbySquare_ChatTab_All` + `strings.rs:44` | 전  체 → Tất cả | `catalog-reused`; dùng được |
| `lobby.chat_tab_notice` | Tab thông báo | `catalog:LobbySquare_ChatTab_Notice` | 공  지 → Thông báo | `catalog-reused`; dùng được |
| `lobby.chat_hint` | Gợi ý ô chat | `catalog:Chatting` + `DESIGN Q-012` | 대화 하기 → Nhập tin nhắn... | `design`; catalog chỉ là nhãn "trò chuyện", không phải placeholder |
| `lobby.chat_send` | Nút gửi chat | `DESIGN Q-012` | — → Gửi | `design`; hoặc Enter, không cần nút |
| `lobby.chat_too_long` | Chat quá dài | `catalog:150CharLimit` + `DESIGN Q-012` | [150자 이내] → Tin nhắn tối đa 150 ký tự. | `design`; nguồn giới hạn 150 là catalog, cách đếm ký tự phải chốt |
| `lobby.join_failed` | Vào phòng thất bại | `catalog:CantJoinRoom` | 방에 참가 할 수 없습니다. → Không thể vào phòng. | `catalog`; dùng được |
| `lobby.room_full` | Phòng đã đủ | `catalog:InvalidMaxUser` + `DESIGN Q-012` | 방의 유저 수가 잘못되어습니다. 방에 참가할 수 없습니다. → Phòng đã đủ người. | `design`; catalog dài và nói "số người không hợp lệ", cần chốt |
| `lobby.room_removed` | Phòng đã bị xóa | `catalog:RoomDestroyed` | 방이 이미 없어졌습니다. → Phòng đã không còn nữa. | `catalog`; dùng được |
| `lobby.room_info_changed` | Thông tin phòng đổi | `catalog:RoomInfoChanged` | 방정보가 변경되었습니다. → Thông tin phòng đã được thay đổi. | `catalog`; dùng được |
| `lobby.join_session` | Chat hệ thống có người vào | `catalog:JoinSession` | %s님이 참가하셨습니다. → %s đã tham gia. | `catalog`; giữ placeholder `%s` |
| `lobby.leave_session` | Chat hệ thống có người rời | `catalog:LeaveSession` | %s님이 퇴장하셨습니다. → %s đã rời đi. | `catalog`; giữ placeholder `%s` |

### 2.4 Phòng chờ

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `room.title_label` | Nhãn tên phòng trong dialog tạo | `catalog:CreateDlgTitle` | 방제목 → Tên phòng | `catalog-reused`; đọc lại ngữ cảnh |
| `room.create_title` | Tiêu đề dialog tạo phòng | `DESIGN Q-012` | 방만들기 (ảnh) → Tạo phòng | `design`; trùng chữ trên ảnh VN |
| `room.create_submit` | Nút tạo | `DESIGN Q-012` | — → Tạo | `design` |
| `common.cancel` | Nút hủy | `DESIGN Q-012` | — → Hủy | `design`; catalog không có khóa chữ đơn giản |
| `common.confirm` | Nút đồng ý | `DESIGN Q-012` | — → Đồng ý | `design` |
| `common.close` | Nút đóng | `DESIGN Q-012` | — → Đóng | `design` |
| `room.info_title` | Tiêu đề thông tin phòng | `DESIGN Q-012` | 방정보 변경 (ảnh) → Thông tin phòng | `design`; ảnh VN ghi `THÔNG TIN`, khác nghĩa KR, xem §3 |
| `room.host` | Nhãn chủ phòng | `DESIGN Q-012` | — → Chủ phòng | `design` |
| `room.players` | Nhãn danh sách người chơi | `DESIGN Q-012` | — → Người chơi | `design` |
| `room.status` | Nhãn trạng thái | `catalog:RoomInfoDlgStatus` | 상태 → Trạng thái | `catalog`; dùng được |
| `room.id` | Nhãn ID phòng | `catalog:RoomInfoDlgID` | 아이디 → ID | `catalog`; dùng được |
| `room.map` | Nhãn bản đồ | `DESIGN Q-012` | — → Bản đồ | `design`; ảnh VN ghi `BẢN ĐỒ` |
| `room.map_name` | Tên map V1 | `catalog:map:Village00` | 빌리지 00 → Làng 00 | `catalog`; cần reviewer chốt dùng "Làng 00" hay "Village00" |
| `room.character` | Nhãn chọn nhân vật | `DESIGN Q-012` | — → Nhân vật | `design`; ảnh VN ghi `NHÂN VẬT` |
| `room.character_c00` | Tên hiển thị C00 | `UNKNOWN` | — → chưa đặt | `unknown`; phải chốt map C00/C01 → tên hiển thị (Bazzi/Dao?) trước khi hiện |
| `room.character_c01` | Tên hiển thị C01 | `UNKNOWN` | — → chưa đặt | `unknown`; như trên |
| `room.ready` | Trạng thái/nút sẵn sàng | `DESIGN Q-012` | 준 비 (ảnh) → Sẵn sàng | `design`; ảnh VN đã có chữ `SẴN SÀNG` |
| `room.ready_cancel` | Hủy sẵn sàng | `DESIGN Q-012` | — → Hủy sẵn sàng | `design` |
| `room.start` | Nút bắt đầu | `DESIGN Q-012` | 시 작 (ảnh) → Bắt đầu | `design`; ảnh VN đã có chữ `BẮT ĐẦU` |
| `room.waiting_players` | Chờ người chơi | `catalog:BigBattle_Lobby_Desc_Line0` | 다른 참가자를 기다리고 있습니다. → Đang chờ người chơi khác. | `catalog`; dùng được |
| `room.min_players` | Chưa đủ người để bắt đầu | `catalog:SmallPlayerNo` | 플레이어 수가 너무 적어서 게임을 시작할 수 없습니다. → Số người chơi quá ít nên không thể bắt đầu trận. | `catalog`; dùng được |
| `room.all_ready` | Chưa ready hết | `catalog:AllMustReady` | 모든 플레이어가 준비 상태이어야 합니다. → Mọi người chơi phải ở trạng thái sẵn sàng. | `catalog`; dùng được |
| `room.loading` | Loading barrier | `DESIGN Q-012` | — → Đang tải trận... | `design`; nhắc 15 giây theo SCOPE_V1 §3 nếu hiển thị đếm ngược |
| `room.kick` | Nút mời ra | `DESIGN Q-012` | — → Mời ra | `design` |
| `room.kick_confirm` | Xác nhận mời ra | `catalog:SureToBan` | '%s'님을 강퇴시키겠습니까? → Mời '%s' ra khỏi phòng? | `catalog`; giữ `%s`, cần QA dấu nháy |
| `room.kicked` | Bị chủ phòng mời ra | `catalog:BannedByLeader` | 방장에 의해 강퇴 당했습니다. 다른 게임에 참여하세요. → Bạn đã bị chủ phòng mời ra. Hãy tham gia phòng khác. | `catalog`; bản dịch có khoảng trắng đầu, cần cắt |
| `room.non_host_denied` | Non-host start/kick | `catalog:RoomLeaderOnly` | 방장만이 사용할 수 있는 명령입니다. → Đây là lệnh chỉ chủ phòng mới dùng được. | `catalog`; dùng được |
| `room.name_changed` | Đổi tên phòng | `catalog:RoomNameChanged` | 방제목이 변경되었습니다. → Tên phòng đã được thay đổi. | `catalog`; dùng được |
| `room.host_changed` | Chuyển chủ phòng | `DESIGN Q-012` | — → %s đã trở thành chủ phòng. | `design`; giữ `%s` |
| `room.slot_empty` | Slot trống | `DESIGN Q-012` | — → Trống | `design` |
| `room.chat_to_all` | Phạm vi chat | `catalog:ChatToAll` | <모두에게> → <Mọi người> | `catalog`; V1 FFA không có `<Đồng đội>` |
| `room.stats_bomb` | Nhãn số bom | `catalog:PREEA` + `strings.rs:89` | 개수 → Số bom | `review`; catalog dịch "Số lượng" sai ngữ cảnh, strings.rs "Bom"; chọn design `Số bom`, cần chốt |
| `room.stats_fire` | Nhãn tầm nổ | `catalog:PREPOW` + `strings.rs:90` | 물줄기 → Tầm nổ | `review`; catalog "Tia nước", strings.rs "Lực"; chọn design `Tầm nổ`, cần chốt |
| `room.stats_speed` | Nhãn tốc độ | `catalog:PRESPD` + `strings.rs:91` | 속도 → Tốc độ | `catalog-reused`; dùng được |
| `room.leave` | Nút rời phòng | `DESIGN Q-012` | — → Rời phòng | `design` |
| `room.leave_ready_confirm` | Xác nhận rời khi đang ready | `catalog:WannaExitToLobby` | 준비 상태에서 윈도우로 나가면 대기실에서 퇴장하게 됩니다. 이동하시겠습니까? → Nếu thoát khi đang sẵn sàng sẽ bị đưa ra khỏi phòng chờ. Bạn có muốn đi không? | `review`; đã sửa "cửa sổ" thành "phòng" cho đúng V1 — cần reviewer chốt |

### 2.5 Trong trận

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `match.timer` | Nhãn đồng hồ | `DESIGN Q-012` | — → Thời gian | `design`; giá trị 120 s lấy từ R-010, không đặt lại ở UI |
| `match.timer_format` | Định dạng mm:ss | `DESIGN Q-012` | — → `%d:%02d` | `design`; dùng số tabular, không cắt số 0 |
| `match.go` | Hiệu lệnh bắt đầu | `DESIGN Q-012` | — → Bắt đầu! | `design`; R-060 có pha "GO" |
| `match.trapped` | Trạng thái bị nhốt | `DESIGN Q-012` | — → Bị nhốt! | `design`; R-026/R-036 |
| `match.eliminated` | Trạng thái bị loại | `DESIGN Q-012` | — → Đã bị loại. | `design` |
| `match.disconnect_warning` | Cảnh báo mất kết nối | `DESIGN Q-012` | — → Mất kết nối. Bạn sẽ bị loại sau %d giây. | `design`; 10 giây là policy SCOPE_V1 §3 |
| `match.disconnect_eliminated` | Bị loại do mất kết nối | `DESIGN Q-012` | — → Bạn đã bị loại do mất kết nối. | `design` |
| `match.item_bomb` | Tên item Bomb | `DESIGN Q-012` | — → Bom | `design`; tên hiển thị phải khớp ItemAttr khi có |
| `match.item_fire` | Tên item Fire | `DESIGN Q-012` | — → Lửa | `design`; như trên |
| `match.item_roller` | Tên item Roller | `DESIGN Q-012` | — → Tốc độ | `design`; như trên |
| `match.pickup` | Thông báo nhặt item | `DESIGN Q-012` | — → %s đã nhặt %s. | `design`; giữ 2 placeholder, tránh lặp effect/audio |
| `match.result_pending` | Chờ server chốt kết quả | `DESIGN Q-012` | — → Đang chờ kết quả... | `design` |

### 2.6 Kết quả

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `result.title` | Tiêu đề | `DESIGN Q-012` | — → Kết quả | `design` |
| `result.win` | Nhãn thắng | `catalog:StateWin` | 승리 → Thắng | `catalog`; ảnh `Win` VN có chữ Việt, cần đọc đủ frame (xem §3) |
| `result.lose` | Nhãn thua | `catalog:StateLose` | 패배 → Thua | `catalog`; ảnh `Lose` VN chưa đọc rõ, xem §3 |
| `result.draw` | Nhãn hòa | `DESIGN Q-012` | — → Hòa | `design`; catalog không có nhãn "Hòa" ngắn |
| `result.rank` | Cột hạng | `DESIGN Q-012` | — → Hạng | `design`; ảnh result ghi `RANK` |
| `result.player` | Cột người chơi | `DESIGN Q-012` | — → Người chơi | `design` |
| `result.continue` | Nút tiếp tục | `DESIGN Q-012` | — → Tiếp tục | `design` |
| `result.auto_return` | Đếm ngược về phòng | `DESIGN Q-012` | — → Tự về phòng sau %d giây. | `design`; 5 giây là policy SCOPE_V1 §3 |
| `result.winner` | Người thắng | `DESIGN Q-012` | — → %s thắng! | `design`; giữ `%s` |
| `result.draw_all` | Hòa khi không còn ai | `DESIGN Q-012` | — → Hòa — không có người thắng. | `design` |
| `result.waiting` | Chờ ack người khác | `DESIGN Q-012` | — → Đang chờ mọi người... | `design` |

### 2.7 Tùy chọn

| Khóa V1 | Phần tử | Nguồn | KR gốc → VI V1 | Trạng thái |
|---|---|---|---|---|
| `options.fullscreen` | Chế độ toàn màn hình | `DESIGN Q-012` | — → Toàn màn hình | `design`; SCOPE_V1 §7 mục 9 |
| `options.windowed` | Chế độ cửa sổ | `DESIGN Q-012` | — → Cửa sổ | `design` |
| `options.volume` | Âm lượng chung | `DESIGN Q-012` | — → Âm lượng | `design` |
| `options.music` | Nhạc nền | `DESIGN Q-012` | — → Nhạc nền | `design` |
| `options.sfx` | Hiệu ứng âm | `catalog:ChatRenew_ChatInfo06` (chỉ là help text) | 효과음 on/off → Hiệu ứng âm | `design`; không dùng câu help F7 làm nhãn |
| `options.apply` | Áp dụng | `DESIGN Q-012` | — → Áp dụng | `design` |

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
trong closure V1; muốn dùng phải bổ sung evidence và review. Cột **Chữ quan sát** là đọc trực tiếp PNG giải
mã ngày 2026-10-19, chỉ để mô tả, **không phải** quyết định duyệt ảnh. Tất cả trạng thái duyệt hiện là
`pending`; muốn dùng phải import `images_review.csv` qua quy trình [IMAGE_REVIEW](IMAGE_REVIEW.md).

| Asset (pack:entry) | Kích thước | Đã chọn? | Chữ quan sát (KR / VN) | VN cùng tên | Ghi chú review |
|---|---|---:|---|---|---|
| `FxDesktop:LoginStage/CAStage` | 800×600 JPEG | — | KR: overlay tiếng Hàn + huy hiệu "전체이용가"; VN: logo Boom + dòng "Nexon… Copyright" | `diff_bytes` | **Không dùng nguyên trạng**: có nhãn Nexon và bản quyền cũ; cần nền mới hoặc che chữ |
| `FxStage:Lobby/CAStage` | 800×600 JPEG | — | KR: hai nhãn tab (đọc được "대전 모드" / "대기 모드"); VN: `ĐẤU CHIẾN` / `NHIỆM VỤ` | `diff_bytes` | Chữ nằm trong ảnh nền; `NHIỆM VỤ` ngoài scope V1, phải che/redraw |
| `FxMap:Prepare/CAStage` | 800×600 JPEG | — | KR: "캐릭터선택" / "팀선택" / "미니맵" / "자동준비"; VN: `NHÂN VẬT` / `CHỌN ĐỘI` / `BẢN ĐỒ` / `TỰ ĐỘNG` | `diff_bytes` | `CHỌN ĐỘI` ngoài scope FFA V1; phải che/redraw; `TỰ ĐỘNG` là auto-ready ngoài V1 |
| `FxStage:Lobby/LB_Bt_T_Create-NM` | 92×495 | Có (`room_create`) | KR: `방만들기`; VN: `TẠO PHÒNG` | `diff_bytes` | Ảnh nút có chữ; nếu dùng phải duyệt `translated`, nếu không thì render text |
| `FxStage:Lobby/Dlg_LB-NMCreate` | 414×455 | Có (`room_create_dialog`) | KR: `방만들기`; VN: `TẠO PHÒNG` | `diff_bytes` | Tiêu đề dialog nằm trong ảnh; phần thân là ô trống, nhãn do client vẽ |
| `FxMap:Prepare/PRE_Bt_Ready` | 193×342 | Có (`ready`) | KR: `준 비`; VN: `SẴN SÀNG` | `diff_bytes` | Nút ready có chữ; cần duyệt ảnh hoặc render text |
| `FxMap:Prepare/PRE_Bt_Start` | 193×342 | Có (`start`) | KR: `시 작`; VN: `BẮT ĐẦU` | `diff_bytes` | Nút start có chữ; cần duyệt ảnh hoặc render text |
| `FxMap:Prepare/PRE_Bt_Roominfo` | 100×110 | — | KR: `방정보 변경`; VN: `THÔNG TIN` | `diff_bytes` | Nghĩa KR là "đổi thông tin phòng", VN là "thông tin"; chọn nhãn nào phải chốt |
| `FxMap:Prepare/Dlg_PRE-NMRoominfo` | 414×455 | — | KR: `방정보 변경`; VN: `THÔNG TIN` | `diff_bytes` | Như trên; dialog V1 có thể chỉ để xem, không đổi tên |
| `FxStage:Lobby/LB_Bt_T_ShowAllRenew` | 99×423 | — | KR: `모든방보기`; VN: `TOÀN BỘ` | `diff_bytes` | VN rút gọn mất nghĩa "xem mọi phòng"; V1 có thể không cần filter |
| `FxStage:Lobby/LB_Bt_T_WaitRoomRenew` | 99×423 | — | KR: `대기방보기`; VN: `PHÒNG CHỜ` | `diff_bytes` | VN là danh từ, không phải hành động "xem phòng chờ" |
| `FxStage:Lobby/ReadyBox` | 127×176 | — | KR: `준비중`; VN: `ĐẠI CHIẾN` | `diff_bytes` | **Sai nghĩa**: không dùng làm indicator ready; cần ảnh/text khác |
| `FxStage:Game/Win` | 243×59 (nhiều frame) | — | KR: `WIN!!!`; VN: các frame đọc được `TH` + `Ắ` + `N` + `!!` (chưa thấy `G` trong bộ frame giải mã) | `diff_bytes` | Ảnh động nhiều frame; duyệt cả chuỗi frame và xác nhận đủ chữ, không chỉ frame 0 |
| `FxStage:Game/Lose` | 317×59 (nhiều frame) | — | KR: `LOSE!!!`; VN: frame đọc được `SC` / `A` / `N` / `U!!` / `O` | `diff_bytes` | Chuỗi frame VN không đọc ra từ rõ ràng; đánh dấu `check`, cần người duyệt xem đủ frame |
| `FxStage:Game/Draw` | 366×59 (nhiều frame) | — | KR: `DRAW!!!`; VN: `HO` + `A` + `!!` → `HÒA!!` | `diff_bytes` | Dấu `Ò` có thể nằm ở frame khác; duyệt cả frame và kiểm dấu |
| `FxStage:Game/ResultInfoBar` | 356×288×6 | — | Spritesheet trộn nhãn KR/EN: `ID`, `RANK`, `KILL`, `EXP`, `LUCCI+`, và các biến thể tiếng Hàn (마을/아이템/음표) | `diff_bytes` | Không dùng cả sheet; V1 chỉ cần cột `Hạng/Người chơi/Kết quả`; nếu dùng phải chọn frame + dịch |
| `FxStage:Game/ResultInfoGood` | 98×144 | — | Chỉ mũi tên, không thấy chữ | `no` | Có thể `no_text`; vẫn cần người duyệt xác nhận |

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
| Font chính | **Be Vietnam Pro** (OFL 1.1), thiết kế cho tiếng Việt | Đề xuất ở `/home/admin/boom-re-l10n/l10n/README.md` §4; `design`, chưa có file trong repo |
| Fallback Latin | **Noto Sans** phủ Latin Extended Additional | Như trên; `design` |
| Fallback Hàn | **Noto Sans KR** cho tên riêng/credit giữ chữ Hàn | Như trên; `design` |
| Không dùng font bitmap cũ | Không dùng `CxUni*.fft`, `CxDefault.fft` | `/home/admin/boom-re-l10n/l10n/README.md` §4 |
| Chuẩn hóa chuỗi | **NFC** trước khi shape; dữ liệu VN cũ có dấu tổ hợp cp1258 | Như trên; bắt buộc về kỹ thuật |
| License | Phải kèm file license, version và nguồn tải trước khi bundle | `design`/`unknown`; chưa có trong repo, **chưa được coi là đã chốt** |
| Fallback chain | `[Be Vietnam Pro, Noto Sans, Noto Sans KR]` | `design`; thử glyph thiếu và log ở dev |

Font phải phủ tối thiểu: Latin cơ bản, Latin Extended Additional cho `ă â đ ê ô ơ ư` và đủ tổ hợp dấu
(ví dụ `ắ ộ ữ ợ ỹ ặ ễ ỗ`), cùng Hangul cho tên riêng. Không cần emoji trong V1.

### 5.2 Wrap, kích thước và placeholder

- Xuống dòng theo **ranh giới từ** (khoảng trắng); không bẻ giữa một âm tiết/tổ hợp dấu; không hyphenation.
- Nhãn nút/label ưu tiên 1 dòng; hộp thoại/tooltip tối đa 2–3 dòng; nếu vượt thì tăng chiều cao hoặc
  rút chữ, **không cắt chữ**. Đây là `design`, phải kiểm bằng golden.
- Chiều cao dòng ≥ 1,4× cỡ chữ để dấu chồng không chạm dòng trên/dưới; `design`, kiểm bằng ảnh.
- Giữ nguyên mọi placeholder: `%s`, `%d`, `%u`, `%.1f%%`, `%02d`, `{ComponentType|N}`, `<font …>`, `\r\n`,
  `|`. Không dịch/xóa placeholder; lệch phải hiện cảnh báo ở test.
- Chat: dùng giới hạn `150CharLimit` (`[150자 이내]`) làm nguồn; V1 đếm **150 ký tự Unicode theo grapheme
  cluster** (không cắt dấu), chốt ở spec auth/chat. `design`.
- Tên người chơi: chưa chốt độ dài. Phải hiển thị được trong danh sách phòng, slot phòng, HUD và kết quả;
  nếu vượt khung thì ellipsis + tooltip, không làm vỡ layout. `unknown` cho tới khi có validation.
- Số (thời gian, số người, thứ hạng) dùng chữ số tabular/không cắt số 0; `design`.
- Scale: asset gốc 800×600. V1 phải chạy cửa sổ/toàn màn hình; golden tối thiểu 800×600, 1280×720,
  1920×1080 ở 100%/150%/200% UI scale (nếu có), không clipping. `design`.
- Focus/IME: tab order rõ, focus ring thấy được; Enter gửi/submit, Escape đóng dialog; phím WASD không bị
  IME/ô chat nuốt khi ngoài ô chat; gõ Telex/VNI trong ô chat/tên phòng không làm mất dấu. `design`.

### 5.3 Acceptance cho V1-035

1. Mọi chuỗi hiển thị trong luồng V1 có một dòng ở §2 hoặc được thêm vào cùng PR; không render chuỗi
   server gửi dạng free text.
2. Mọi dòng `design`/`strings.rs` đã được reviewer chốt; mọi dòng `review` không xuất hiện.
3. Mọi ảnh dùng trong luồng có quyết định `translated`/`no_text` trong `images_review.csv` (không còn
   `pending`), hoặc client render text thay ảnh.
4. Golden font/wrap: tối thiểu các cặp dấu `ắ ộ ữ ợ ỹ`, tên người chơi dài, chat 150 ký tự, tooltip 3 dòng,
   800×600 và 1920×1080; không clipping.
5. `cargo xtask ci` xanh; `python3 tools/docs_check.py` xanh.

## 6. Còn thiếu và bước tiếp

| Thiếu | Ảnh hưởng | Bước tiếp |
|---|---|---|
| 3.038 ảnh vẫn `pending`, chưa có `l10n/images_review.csv` | Không được coi ảnh nào là đã Việt hóa | Reviewer duyệt theo [IMAGE_REVIEW](IMAGE_REVIEW.md), import CSV; bắt đầu từ 4 asset đã chọn |
| Chưa có font/license trong repo | V1-035 chưa bundle được font | Chọn version, thêm license/attribution, golden glyph; ngoài phạm vi docs này |
| Chưa map C00/C01 → tên hiển thị | Slot phòng/character select không có nhãn đúng | RE/trace hoặc dữ liệu tên nhân vật; catalog `Bazzi`/`Dao` chỉ là ứng viên, ghi claim/unknown trước khi hiển thị |
| Màn kết quả chưa có asset trong closure | V1-032 có thể dead end ở result | Bổ sung asset/art vào evidence, hoặc thiết kế UI kết quả bằng text + asset đã có |
| Nền login/lobby/prepare có chữ và nhãn ngoài scope | Dùng nguyên trạng sẽ hiện Nexon/nhiệm vụ/đội | Che/redraw hoặc chọn asset khác; cần review ảnh |
| Nhãn form login, nút phòng, result là `design` | Chưa có nguồn KR, dễ lệch thuật ngữ | Reviewer chốt bảng §2; thêm vào `tm_id.csv` nếu cần |
| Chưa có golden wrap/scale/focus | Không chứng minh được V1-035 | Tạo golden ảnh + test glyph sau khi client có renderer |

Không thêm claim luật chơi mới trong tài liệu này. Các giá trị thời gian nhắc ở §2 (120 s, 10 s, 5 s, 15 s)
đã có trong [RULES_V1](../gameplay/RULES_V1.md) R-010 và [SCOPE_V1](../../planning/SCOPE_V1.md) §3; UI chỉ
hiển thị/đếm, không tự đặt lại.
