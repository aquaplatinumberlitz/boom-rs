# boom-server — server authoritative V1

Server của Boom KR viết lại bằng Rust: nhận client qua TCP, chạy trận trên `boom-core`
(server quyết kết quả — [ADR 0005](../../docs/decisions/0005-authority-and-prediction.md)),
giữ tài khoản/phiên trong SQLite ([ADR 0011](../../docs/decisions/0011-identity-persistence-log-v1.md))
và báo sức khỏe runtime qua admin listener loopback.

Quy tắc chung của dự án: [AGENTS.md](../../AGENTS.md). Kiến trúc crate:
[ARCHITECTURE](../../docs/planning/ARCHITECTURE.md).

## Chạy thử nhanh (LAN, 2–4 client)

```bash
# 1. Dựng bundle map (cần pack KR đã giải — xem crates/boom-assets).
#    `--map` nhận TÊN map (V1 chỉ có `Village00`), thư mục pack lấy từ `--pack-dir`
#    hoặc biến môi trường `BOOM_KR_PACK_DIR`.
cargo run -p boom-assets -- bundle --map Village00 --out data/bundle/village00 \
  --pack-dir /duong/dan/pack-kr

# 2. Tạo file cấu hình theo mẫu ở mục "File cấu hình" bên dưới, ví dụ `server.toml`

# 3. Chạy server
cargo run -p boom-server -- serve --config server.toml
```

Server in ra log JSON Lines trên stdout. `boom-server health` hỏi trực tiếp tiến trình
đang chạy (exit `0` = healthy):

```bash
cargo run -p boom-server -- health
```

### Log: vòng đời round, lý do disconnect, lỗi nạp content

Mọi dòng log là một object JSON (ADR 0011 §8.1). Ba nhóm sự kiện đáng chú ý:

| Sự kiện | Khi nào | Trường riêng |
|---|---|---|
| `content.load_ok` / `content.load_fail` | Nạp content thành công / thất bại | `content_sha256` (chỉ ở `load_ok`), `error_code` + `error_message` (đường dẫn file, **không** ghi nội dung file) |
| `round.start` → `round.loading` → `round.playing` → `round.result` → `round.end` | Vòng đời một round: tạo round → từng người báo xong loading → barrier đóng → kết quả → về phòng chờ | `round_id` (ID public **thật**, khớp `RoundStart`/`RoundResult` trên dây), `room_id`, `phase`, `outcome` |
| `player.disconnect` | Người chơi rời phòng/mất kết nối | `reason` (`client_closed`, `timeout_no_input`, `kicked`, `server_shutdown`, `protocol_error`, `auth_failed`), `revoke_reason` (lý do thu hồi phiên trong DB) |

Lỗi nạp content đi qua **log JSON** (không chỉ stderr): logger được dựng trước bước nạp
content nên `content.load_fail` luôn có mã lỗi + đường dẫn file. Log không bao giờ chứa
mật khẩu, PHC string hay session token (ADR 0011 §8.1).

## Lệnh

| Lệnh | Việc |
|---|---|
| `boom-server serve [--config <file>]` | Chạy server thật: nạp content, mở DB, bind listener, nhận client |
| `boom-server health [--offline]` | Sức khỏe runtime (exit 0) hoặc kiểm tra DB khi server không chạy |
| `boom-server account revoke-sessions <username>` | Thu hồi phiên của một tài khoản (đường dự phòng) |
| `boom-server config [--out <file>]` | Xuất/ghi cấu hình hiệu lực |

Mã thoát của `serve`: `0` dừng sạch · `2` lỗi khởi động hoặc lỗi lúc chạy · `3` sai cú pháp
hoặc lỗi cấu hình.

## File cấu hình

Thứ tự áp dụng: **mặc định → file → biến môi trường** (`BOOM_*` thắng file). File được
trỏ bởi `--config <file>` hoặc `BOOM_CONFIG_FILE`.

```toml
[server]
# Admin listener: bắt buộc loopback (ADR 0011 §8.2.1).
admin_addr = "127.0.0.1:17890"
# Thư mục chứa health.json, DB và file khoá instance.
data_dir = "data"
# Địa chỉ game server: một kết nối TCP mỗi client (ADR 0006).
# Cổng 0 ⇒ server tự chọn cổng và ghi địa chỉ thật vào health.json.
listen_addrs = ["0.0.0.0:17889"]
server_version = "0.0.0"
heartbeat_interval_ms = 60000
health_timeout_ms = 3000
# Nội dung trận: rules profile, bundle map, bảng phân loại ô theo build.
bundle_dir = "data/bundle/village00"
rules_path = "data/rules/v1-dev.toml"
map_profile_path = "data/maps/village00-kr.toml"
# Ghi replay mỗi round (V1-026); bỏ trống ⇒ tắt.
replay_dir = "data/replay"
replay_checkpoints = [1, 16, 32, 64, 128, 256]

[identity]
db_path = "data/boom-server.sqlite3"
expected_db_schema_version = 1
# Hạn hoàn tất handshake + auth cho kết nối chưa `AuthOk`.
auth_deadline_ms = 10000
session_ttl_ms = 86400000
session_ping_interval_ms = 15000
session_liveness_timeout_ms = 30000

[auth]
registration_open = true
rate_limit_max_failures = 5
register_rate_limit_max_attempts = 5
rate_limit_window_ms = 300000
rate_limit_max_buckets = 4096
password_min_chars = 8
password_max_bytes = 256
argon2_m_cost = 19456
argon2_t_cost = 2
argon2_p_cost = 1
argon2_output_len = 32
argon2_salt_len = 16
```

### Chỉ số nhân vật: ở rules profile, không ở file cấu hình

Chỉ số C00/C01 là **giá trị luật** (RULES_V1 R-040/R-041) nên nằm trong rules profile
(`rules_path`), không phải trong file cấu hình: mọi người chạy server phải dùng cùng một
nguồn, và `rules_hash` phản ánh chỉ số để handshake bắt được lệch (AGENTS §3). Vì vậy mục
`[characters]` **không còn** trong file cấu hình — file còn mục đó bị từ chối khi đọc, kèm
thông điệp chỉ cách sửa (đọc chỉ số từ rules profile).

Profile phải có đủ mười hai khóa `characters.c00_*` / `characters.c01_*`; thiếu nhân vật
hoặc cấp hiện tại vượt trần thì `serve` báo lỗi rõ và thoát ≠ 0 (không tự đặt mặc định).
Lệnh `boom-server content --rules <file>` cũng kiểm bảng chỉ số này.

| Khóa | Nghĩa | Nguồn |
|---|---|---|
| `characters.c00_speed` / `_speed_max` | Cấp tốc độ hiện tại / trần (`BomberAttr` f2/f3) | RULES_V1 R-040 (`confirmed`), nghĩa trường R-042 (`candidate`) |
| `characters.c00_bombs` / `_bombs_max` | Số bom đặt cùng lúc (f4/f5) | như trên |
| `characters.c00_range` / `_range_max` | Tầm nổ (f6/f7) | như trên |
| `characters.c01_*` | Cùng bộ nghĩa cho C01 | R-041 (`confirmed`) |

Giá trị `5,7 / 1,10 / 1,7` (C00) và `4,8 / 2,7 / 1,9` (C01) là số đọc được trong
`BomberAttr` của pack KR 00967 (RULES_V1 R-040/R-041) và nằm ở
[`data/rules/v1-dev.toml`](../../data/rules/v1-dev.toml). Vì **nghĩa trường** f2–f7 còn là
`candidate` (R-042), giá trị dùng trong profile ghi `status = "candidate"`: dãy số đã đối
chiếu nhưng chưa có bằng chứng rằng f2–f7 mang đúng nghĩa speed/bombs/range (AGENTS §4).
Bảng `BomberAttr` **chưa** được giải vào content — gate G2 của SCOPE_V1 §6 còn mở — nên V1
đọc chỉ số từ rules profile.

## Chạy cùng client

Server đã sẵn sàng cho client LAN: handshake `Hello`/`Welcome`, auth `Auth`/`AuthOk`, phòng
`RoomCommand`/`RoomState`, `LoadingAck`, `Snapshot` (ADR 0009 Addendum A/B).

`boom-client` **có** chế độ mạng: `--connect <địa-chỉ>` kèm `--user`/`--password` (hoặc biến
môi trường `BOOM_USER`/`BOOM_PASSWORD`) và `--register` để tự đăng ký tài khoản mới.

```bash
# Client phải dùng CÙNG rules profile và CÙNG bundle với server:
#  - handshake so `rules_profile_id` + `rules_hash` + `content_sha256`
#    (ADR 0009 §1) ⇒ lệch thì server từ chối trước khi vào sảnh;
#  - `content_sha256` lấy từ trường `content_hash` trong `bundle.toml` của bundle.
CONTENT_SHA=$(sed -n 's/^content_hash = "\(.*\)"/\1/p' data/bundle/village00/bundle.toml)

BOOM_CONTENT_SHA256="$CONTENT_SHA" \
BOOM_USER=player1 BOOM_PASSWORD='matkhau123' \
cargo run -p boom-client -- --connect 127.0.0.1:17889 \
  --bundle data/bundle/village00 --register
```

Giới hạn hiện tại của client (theo đúng contract ở ADR 0009 Addendum B §B.2):

- Chế độ mạng dùng rules profile **`data/rules/v1-dev.toml` nhúng trong binary** (cờ `--rules`
  chỉ áp dụng cho phần offline); server vì vậy phải chạy với đúng profile đó.
- Client lấy `content_sha256` từ biến `BOOM_CONTENT_SHA256`; thiếu biến thì client khai
  toàn 0 và server từ chối handshake (`ContentMismatch`).
- **Slot trận của client là `slot` trong `RoundStart.roster` có `player == account_id`** (server
  bind slot của input từ phiên; `Welcome.player_id` chỉ là định danh kết nối, **không** phải
  slot). Client hiện tại vẫn gửi `Welcome.player_id` trong batch input nên server **bỏ** các
  batch đó và đếm (`input.slot_mismatch`); client sẽ lấy slot từ roster ở issue #114.
- Luồng client↔server thật được kiểm bằng test tích hợp `crates/boom-server/src/runtime.rs`
  (client TCP thật dùng `boom_net::TcpTransport` + DTO của `boom-proto`).

## Dừng sạch

Ctrl-C (SIGINT) hoặc SIGTERM ⇒ server:

1. ngừng nhận kết nối mới;
2. thu hồi mọi phiên của instance này trong DB (lý do `server_shutdown`, ADR 0011 §6.1) —
   người chơi đăng nhập lại được;
3. ghi `health.json` cuối cùng;
4. thoát với mã `0`.

## Kiểm tra

```bash
cargo test -p boom-server          # gồm test tích hợp `serve` (client TCP thật, DB thật)
cargo xtask ci                    # cổng bàn giao đầy đủ
```
