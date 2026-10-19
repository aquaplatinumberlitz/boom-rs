# boom-server — server authoritative V1

Server của Boom KR viết lại bằng Rust: nhận client qua TCP, chạy trận trên `boom-core`
(server quyết kết quả — [ADR 0005](../../docs/decisions/0005-authority-and-prediction.md)),
giữ tài khoản/phiên trong SQLite ([ADR 0011](../../docs/decisions/0011-identity-persistence-log-v1.md))
và báo sức khỏe runtime qua admin listener loopback.

Quy tắc chung của dự án: [AGENTS.md](../../AGENTS.md). Kiến trúc crate:
[ARCHITECTURE](../../docs/planning/ARCHITECTURE.md).

## Chạy thử nhanh (LAN, 2–4 client)

```bash
# 1. Dựng bundle map (cần pack KR đã giải — xem crates/boom-assets)
cargo run -p boom-assets -- bundle --map <đường dẫn Village00.map> --out data/bundle/village00

# 2. Tạo file cấu hình theo mẫu ở mục "File cấu hình" bên dưới, ví dụ `server.toml`

# 3. Chạy server
cargo run -p boom-server -- serve --config server.toml
```

Server in ra log JSON Lines trên stdout. `boom-server health` hỏi trực tiếp tiến trình
đang chạy (exit `0` = healthy):

```bash
cargo run -p boom-server -- health
```

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
session_ttl_ms = 3600000
session_ping_interval_ms = 15000
session_liveness_timeout_ms = 30000

[auth]
registration_open = true
rate_limit_max_failures = 5
rate_limit_max_buckets = 1000
rate_limit_window_ms = 60000
register_rate_limit_max_attempts = 5
password_min_chars = 8
password_max_bytes = 128
argon2_m_cost = 19456
argon2_t_cost = 2
argon2_p_cost = 1
argon2_output_len = 32
argon2_salt_len = 16

# Chỉ số nhân vật C00/C01 — xem mục kế bên.
[characters.c00]
speed_value = 5
speed_max = 7
bombs_value = 1
bombs_max = 10
range_value = 1
range_max = 7

[characters.c01]
speed_value = 4
speed_max = 8
bombs_value = 2
bombs_max = 7
range_value = 1
range_max = 9
```

### Chỉ số nhân vật (`[characters]`)

`serve` **bắt buộc** có mục này: thiếu thì báo lỗi rõ và thoát ≠ 0 (không tự đặt giá trị).
Lệnh `boom-server config` xuất cấu hình hiệu lực **không** có mục `[characters]` (chỉ số là
giá trị luật, không phải tham số vận hành) — thêm mục này vào file trước khi chạy `serve`.

| Trường | Nghĩa | Nguồn |
|---|---|---|
| `speed_value` / `speed_max` | Cấp tốc độ hiện tại / trần (`BomberAttr` f2/f3) | RULES_V1 R-040/R-041 (`confirmed`), nghĩa trường R-042 (`candidate`) |
| `bombs_value` / `bombs_max` | Số bom đặt cùng lúc (f4/f5) | như trên |
| `range_value` / `range_max` | Tầm nổ (f6/f7) | như trên |

Giá trị `5,7 / 1,10 / 1,7` (C00) và `4,8 / 2,7 / 1,9` (C01) là số đọc được trong
`BomberAttr` của pack KR 00967 (RULES_V1 R-040/R-041). Bảng `BomberAttr` **chưa** được giải
vào content — gate G2 của SCOPE_V1 §6 còn mở — nên V1 đọc chỉ số từ file cấu hình thay vì
hard-code trong code. Khi G2 mở, giá trị này chuyển sang `data/rules` + claim trong RULES_V1.

## Chạy cùng client

Server đã sẵn sàng cho client LAN: handshake `Hello`/`Welcome`, auth `Auth`/`AuthOk`, phòng
`RoomCommand`/`RoomState`, `LoadingAck`, `Snapshot` (ADR 0009 Addendum A/B).

**Giới hạn hiện tại:** `boom-client` **chưa** có chế độ mạng (`--connect` chưa tồn tại — client
vẫn chạy mô phỏng offline trong `crates/boom-client`). Luồng client↔server thật hiện được
kiểm bằng test tích hợp `crates/boom-server/src/runtime.rs` (client TCP thật dùng
`boom_net::TcpTransport` + DTO của `boom-proto`). Khi client có `--connect`, chạy:

```bash
boom-client --connect 127.0.0.1:17889
```

Client phải dùng **cùng** rules profile và **cùng** bundle với server: fingerprint
(`rules_profile_id`, `rules_hash`, `content_sha256`) được kiểm ở `Hello`, lệch thì server từ
chối trước khi vào sảnh.

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
