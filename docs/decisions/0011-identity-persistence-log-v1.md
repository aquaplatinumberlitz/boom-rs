# 0011 — Identity, persistence và log/vận hành V1 (Q-011)

Status: **Proposed (chờ review)**, 2026-10-19. Trả lời phần còn mở của Q-011 và làm đầu ra cho
V1-001, V1-028, V1-029. **Task:** GitHub issue #54, nhánh `issue-54`, baseline `main @ 2af995d`.
Owner: backend. Reviewer: integrator (Claude) + QA. **Không viết code trong task này**; ADR chỉ chốt
quyết định, vị trí code và thứ tự task.

> Index `docs/decisions/README.md` và dòng "Persistence" ở
> [ARCHITECTURE §8](../planning/ARCHITECTURE.md#8-quyết-định-còn-mở) do integrator cập nhật khi
> duyệt ADR này (index ADR thuộc integrator theo AGENTS §5). ADR 0009 cũng cần một addendum cho
> message auth — xem §11.

Toàn bộ quyết định dưới đây là **DESIGN của dự án** (SCOPE_V1 §1: giới hạn V1 là policy dự án, không
phải tuyên bố server KR gốc làm vậy). Vì identity/persistence **không phải luật trận**, ADR này
**không sinh mã claim `R-xxx`** trong [RULES_V1](../specification/gameplay/RULES_V1.md); nguồn web/OSS
dùng ở §12 chỉ là căn cứ cho lựa chọn thư viện và tham số, trạng thái `candidate`/`design`.

## 1. Bối cảnh

V1 là bản chơi thử desktop: đăng nhập tài khoản dự án → một kênh/sảnh danh sách → phòng → trận
([SCOPE_V1 §1](../planning/SCOPE_V1.md#1-sản-phẩm-v1)). Ba feature cần identity và vận hành:

| Feature | Trách nhiệm server (registry) | Acceptance (registry) |
|---|---|---|
| V1-001 | Lưu identity/password hash; xác thực session; phiên thứ hai bị từ chối | Tạo/đăng nhập tài khoản test; thông tin sai bị từ chối; một phiên hoạt động mỗi tài khoản |
| V1-028 | Lưu account/session policy; không lưu economy | Restart server giữ identity; restart client giữ volume/keybind; room/trận đang chạy không cần phục hồi |
| V1-029 | Config endpoint, health và log cấu trúc | Có version/rules hash, round ID, lý do disconnect, lỗi load; không log password/session token |

Ràng buộc đã có, không mở lại ở đây:

- Một tài khoản có **một phiên hoạt động**; đăng nhập phiên thứ hai **bị từ chối rõ ràng**
  ([SCOPE_V1 §3](../planning/SCOPE_V1.md#3-policy-phòng-và-vòng-dời)). Reconnect ứng dụng cần **auth lại**.
- V1 **không thương mại**, không ví/tiền thưởng/shop/economy; **không** lưu economy, không ledger
  ([SCOPE_V1 §1](../planning/SCOPE_V1.md#1-sản-phẩm-v1), [REWRITE_PLAN](../planning/REWRITE_PLAN.md#server-thiếu-nguồn-khôi-phục-hoặc-thiết-kế-lại)).
- Room/trận **ephemeral**: restart server không cần phục hồi phòng hay trận đang chạy
  ([SCOPE_V1 §1](../planning/SCOPE_V1.md#1-sản-phẩm-v1), [ARCHITECTURE §8](../planning/ARCHITECTURE.md#8-quyết-định-còn-mở)).
- Transport là **TCP sau một trait**; **TLS/lớp mã hóa tương đương quyết ở task backend**
  ([ADR 0006](0006-transport-default.md)). ADR 0009 đã chốt envelope, handshake và codec nhưng
  **không định nghĩa message auth/session** — handshake không thay thế auth/session
  ([ADR 0009](0009-wire-protocol-input-snapshot-codec.md)).
- Server quyết mọi kết quả; client dự đoán chỉ chuyển động của chính mình
  ([ADR 0005](0005-authority-and-prediction.md)). Vì vậy **server phải bind identity từ session**,
  không tin trường account do client khai.
- Mọi contract có revision; code là nguồn chuẩn của kiểu dữ liệu
  ([ARCHITECTURE §5](../planning/ARCHITECTURE.md#5-contracts--contracts-r4)).

`[ARCHITECTURE §8](../planning/ARCHITECTURE.md#8-quyết-định-còn-mở)` ghi persistence là "Mở (Q-011);
SQLite là ứng viên V1" với điều kiện đảo: uniqueness/transaction, migration/backup, test backend thật.
Q-011 ([QUESTIONS](../research/QUESTIONS.md)) ghi rõ phần còn thiếu: credential/session, schema,
migration, log; và câu hỏi mở của V1-028 là "chọn DB tối thiểu qua ADR, **không ép Postgres/SQLite hai
engine**".

## 2. Phạm vi

Trong phạm vi ADR này: cơ sở lưu trữ, schema tối thiểu + migration, hash mật khẩu, session token,
tùy chọn client, log/health, threat case tối thiểu, vị trí code, thứ tự task triển khai.

Ngoài phạm vi (mở task riêng, không ghi ở đây): giao thức message auth chi tiết (ADR 0009 addendum +
`boom-proto` ở P7), TLS, đồng bộ tùy chọn client, khôi phục mật khẩu/email/2FA, quản trị nhiều server,
mã hóa toàn bộ file DB, economy/ledger (backlog sau V1).

## 3. Quyết định 1 — Cơ sở lưu trữ: SQLite

**V1 dùng một file SQLite** (engine `SQLite`, driver `rusqlite`) làm cơ sở lưu trữ server.

Bốn phương án đã cân nhắc:

| | A — SQLite qua `rusqlite` (đồng bộ) | B — PostgreSQL qua `sqlx` | C — file KV (`redb`/`sled`) | D — file JSON/TOML tự quản |
|---|---|---|---|---|
| Hạ tầng | 1 file, không dịch vụ | phải cài/chạy dịch vụ, user/port, backup riêng | 1 file | 1 file |
| Ràng buộc SQL (unique, FK) | có | có | không (chỉ key–value) | không |
| Transaction | có | có | có | không (phải tự khóa file) |
| Migration | script SQL + số phiên bản | có sẵn | thủ công | thủ công |
| Hợp với server Rust | đồng bộ, không cần async runtime | async + tokio, thêm runtime | đồng bộ | tự viết |
| Quy mô V1 (≤4 người/phòng, một tiến trình) | dư sức | quá nặng | đủ nhưng thiếu ràng buộc | thiếu atomicity |

Chọn **A** vì: V1 chỉ lưu identity + session, một tiến trình server, không economy; SQLite cho
transaction và ràng buộc unique **thật** (điều kiện "Uniqueness/transaction" của ARCHITECTURE §8) mà
không phải vận hành dịch vụ CSDL. `rusqlite` có feature `bundled` (dựng SQLite từ mã nguồn) nên CI ba OS
không cần cài `libsqlite3-dev` (nguồn ở §12).

Cấu hình kết nối (mỗi connection, đặt lúc mở):

- `PRAGMA journal_mode = WAL` — WAL ghi commit vào file riêng nên reader không chặn writer; cần ý thức
  checkpoint (nguồn ở §12).
- `PRAGMA foreign_keys = ON` — **mặc định SQLite là OFF**, phải bật tường minh mỗi connection.
- `PRAGMA synchronous = FULL` — lưu lượng ghi identity/session rất nhỏ, nên chọn bền vững hơn tốc độ.
- `PRAGMA busy_timeout` — chờ khi có ghi đồng thời thay vì lỗi `SQLITE_BUSY` ngay.

File DB nằm trong thư mục dữ liệu của server, quyền file `0600` (owner-only) trên POSIX.

**Điều kiện đảo:** cần nhiều tiến trình/máy server, hoặc cần truy vấn phân tích/số liệu lớn → chuyển
sang PostgreSQL (giữ nguyên logic schema và trait, chỉ đổi driver + runner migration), bằng ADR mới.

## 4. Quyết định 2 — Schema tối thiểu và migration

Schema v1 (chỉ hai bảng; **không** có bảng economy, inventory, ledger — ngoài V1):

```sql
CREATE TABLE accounts (
    id             INTEGER PRIMARY KEY,
    username       TEXT    NOT NULL,          -- tên hiển thị, giữ nguyên dấu gõ
    username_key   TEXT    NOT NULL UNIQUE,   -- chuẩn hóa: trim + lowercase, khóa so khớp
    password_hash  TEXT    NOT NULL,          -- PHC string; không bao giờ log
    created_at_ms  INTEGER NOT NULL,
    updated_at_ms  INTEGER NOT NULL,
    disabled       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE sessions (
    id              INTEGER PRIMARY KEY,
    account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash      BLOB    NOT NULL UNIQUE,   -- SHA-256(token thô), không lưu token
    created_at_ms   INTEGER NOT NULL,
    expires_at_ms   INTEGER NOT NULL,
    last_seen_at_ms INTEGER NOT NULL,
    revoked_at_ms   INTEGER                  -- NULL = còn hiệu lực
);

-- Một tài khoản chỉ có một phiên hiệu lực (SCOPE_V1 §3): ràng buộc ở tầng DB, không chỉ ở code.
CREATE UNIQUE INDEX sessions_one_active_per_account
    ON sessions(account_id) WHERE revoked_at_ms IS NULL;
```

Quyết định kèm theo:

- **Unique partial index** là nơi *thực thi* policy "một tài khoản một phiên hoạt động": đăng nhập thứ hai
  phải thất bại ở tầng DB, không phụ thuộc thứ tự kiểm tra trong code. SQLite hỗ trợ partial index
  (`CREATE UNIQUE INDEX ... WHERE`) (nguồn ở §12).
- **Tên đăng nhập**: 3–24 ký tự, chữ/số/`_`/`-`, không phân biệt hoa thường khi so khớp (chuẩn hóa
  `username_key`). Đây là **design** (V1 không tái tạo auth Nexon/NGM).
- **Mật khẩu**: ≥ 8 ký tự, ≤ 256 byte UTF-8; không bắt buộc độ phức tạp; không lưu lịch sử mật khẩu.
- **Mốc thời gian**: `*_ms` là Unix epoch milliseconds (đồng hồ tường của server; `boom-core` không bị
  ảnh hưởng vì ADR 0004 cấm wall clock **trong core**).
- **Xóa tài khoản**: `ON DELETE CASCADE` xóa session; V1 không có soft-delete.

**Migration** (forward-only, không rollback tự động):

- Số phiên bản schema lưu ở `PRAGMA user_version` — SQLite ghi số nguyên ở offset 60 của header và
  **không tự dùng** số này, nên app dùng làm version của mình (nguồn ở §12).
- Danh sách migration là hằng tuple trong code (`(version, sql)`), mỗi bước chạy **trong một
  transaction** và bump `user_version` trong cùng transaction đó → không nửa vời.
- DB có `user_version` **cao hơn** version mà binary hỗ trợ → server **từ chối khởi động** (không đoán,
  không downgrade).
- Bước migration lỗi → rollback transaction, server không khởi động, log `db.migration` với mã lỗi.
- Trước khi chạy migration, sao lưu bằng **backup API của SQLite** (`sqlite3_backup_init/step/finish`)
  hoặc `VACUUM INTO 'file'` (nguồn ở §12); `VACUUM INTO` tạo bản sao tối giản và xóa nội dung đã xóa.
- **Kiểm tra tính toàn vẹn** (`PRAGMA integrity_check`) chạy trong lệnh `health` (§9), không chạy mỗi
  lần khởi động để tránh khởi động chậm.

## 5. Quyết định 3 — Hash mật khẩu: Argon2id

**Thuật toán:** Argon2id, version `0x13` (mã `v=19`) theo [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html).

| Tham số | Giá trị V1 | Ghi chú |
|---|---|---|
| `m_cost` (bộ nhớ) | 19 456 KiB = 19 MiB | `argon2` 0.6.0 `Params::DEFAULT_M_COST` = `19 * 1024` |
| `t_cost` (số vòng) | 2 | `Params::DEFAULT_T_COST` = 2 |
| `p_cost` (song song) | 1 | `Params::DEFAULT_P_COST` = 1 |
| Độ dài tag | 32 byte | `Params::DEFAULT_OUTPUT_LEN` = 32 |
| Salt | 16 byte ngẫu nhiên **mỗi mật khẩu** | RFC 9106 §3.1: "16 bytes is RECOMMENDED" |

Ba tham số đầu **bằng đúng mặc định của crate `argon2` 0.6.0** (RustCrypto), nên code không chứa hằng số
tự đặt; muốn đổi thì đổi tham số của crate và ghi lại trong ADR mới. Vì sao không dùng mức khuyến nghị
mạnh nhất của RFC 9106 (FIRST: `t=1, p=4, m=2 GiB`; SECOND: `t=3, p=4, m=64 MiB`): đó là mức cho server
chạy nhiều core; V1 là bản chơi thử, có thể chạy trên máy cá nhân, và mức mặc định crate đã là
memory-hard với 19 MiB/lần hash. Ngưỡng nâng lên khi có phép đo (xem §11).

Quyết định kèm theo:

- Lưu **PHC string** (`$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`) trong `accounts.password_hash`;
  verify đọc tham số từ chính chuỗi đó, nên không cần bảng tham số riêng.
- **Rehash khi đổi tham số**: verify thành công mà tham số trong PHC khác tham số hiện hành → hash lại
  và `UPDATE` ngay (người dùng không cần đổi mật khẩu).
- **Pepper** (tùy chọn): đọc từ biến môi trường/secret file, không ghi log, không commit. Đổi pepper
  làm hash cũ không verify được → phải rehash toàn bộ hoặc reset mật khẩu; mặc định V1 **không** bật.
- **Thông báo lỗi thống nhất**: sai tên đăng nhập lẫn sai mật khẩu trả cùng một thông báo và cùng
  thời gian xử lý (verify với một hash giả khi tài khoản không tồn tại) → không lộ việc tài khoản có
  thật hay không.
- **Giới hạn tần suất**: theo tài khoản và theo nguồn (IP), số lần sai trong cửa sổ thời gian; vượt ngưỡng
  thì từ chối và log mã `auth.rate_limited`. Ngưỡng là **tham số vận hành trong config**, không phải
  luật chơi.
- Mật khẩu và PHC string **không bao giờ** vào log (§9).

## 6. Quyết định 4 — Session token

- **Sinh**: 32 byte (256 bit) từ RNG của hệ điều hành (`getrandom` qua `rand` 0.10 `OsRng`), mã hóa
  **base64url không padding** (43 ký tự) để gửi trong message mà không cần escape.
- **Lưu**: DB chỉ lưu `SHA-256(token thô)` (`sessions.token_hash`). Không phục hồi được token từ DB;
  không ghi token thô ở bất kỳ đâu ngoài RAM của tiến trình.
- **Truyền**: token là **bearer credential** — client gửi kèm sau bước auth; server bind
  `account_id`/`session_id` từ token, **không** tin `account_id` do client khai (ADR 0005).
- **Hết hạn**: TTL cố định (mặc định 24 giờ, trong config), **không trượt** khi dùng. Hết hạn thì client
  đăng nhập lại — đúng policy "reconnect cần auth lại" của SCOPE_V1 §3.
- **Một phiên mỗi tài khoản**: unique partial index (§4). Đăng nhập khi tài khoản đã có phiên hiệu lực
  bị **từ chối rõ ràng** với mã lý do riêng (`account_has_active_session`), đúng SCOPE_V1 §3 — server
  **không** tự động đuổi phiên cũ.
- **Logout**: đặt `revoked_at_ms`; token bị từ chối ngay lập tức.
- **Gỡ phiên kẹt**: lệnh quản trị `boom-server account revoke-sessions <username>` (xóa mọi phiên hiệu
  lực của tài khoản) — cần cho client kẹt sau crash.
- **Không rò token trong log**: kiểu `Secret<T>` bọc token, `Debug` in ra chuỗi ẩn, không có `Display`,
  không `Clone` nếu không cần. Test tự động quét output log để chắc không có token (§11, T7).
- **Rủi ro đã biết**: ADR 0006 để TLS cho task backend, nên ở V1 token đi trên TCP **chưa mã hóa**. V1
  chỉ dùng cho LAN/máy local. **Trước khi phát hành ngoài mạng tin cậy, bắt buộc có TLS** (điều kiện ở
  §11). Không coi "V1 không thương mại" là lý do để bỏ qua bước này.

## 7. Quyết định 5 — Tùy chọn client: lưu phía client

V1-028 yêu cầu "restart **client** giữ volume/keybind". Quyết định: **tùy chọn client lưu ở máy người
chơi, không lưu ở server**.

- File `options.toml` trong thư mục config của client (đường dẫn theo quy ước từng OS; crate
  `boom-client` chọn cụ thể), nội dung: `schema_version`, `volume` (0–100), `keybinds`
  (ánh xạ hành động → phím).
- Ghi **atomic** (ghi file tạm rồi rename); đọc khi mở app; file hỏng hoặc sai `schema_version` → log
  cảnh báo, dùng mặc định và tạo lại file, **không** crash.
- Server **không** biết tùy chọn này; không thêm bảng DB. Lý do: tùy chọn là theo thiết bị, đồng bộ
  giữa nhiều máy là tính năng khác (ngoài V1, xem §11).
- Tùy chọn không chứa bí mật; keybind là dữ liệu người dùng, không phải dữ liệu nhạy cảm.

## 8. Quyết định 6 — Log cấu trúc, health và config fingerprint

### 8.1 Log

- Định dạng **JSON Lines**: mỗi dòng là một object JSON, ghi ra **stdout** (service manager thu thập;
  V1 không tự quản lý file/rotate).
- Trường bắt buộc (theo acceptance V1-029):

| Trường | Ý nghĩa |
|---|---|
| `ts` | RFC 3339 UTC, mili giây |
| `level` | `debug`/`info`/`warn`/`error` |
| `event` | tên sự kiện ổn định, snake_case |
| `server_version` | build id của server |
| `wire_schema`, `state_schema`, `protocol_limits_version` | định danh giao thức (ADR 0009) |
| `rules_profile_id`, `rules_hash` | định danh luật đang chạy (ADR 0009) |
| `content_sha256` | chỉ ở `server.start` / `content.load_ok`; ở log khác dùng `content_epoch` |
| `account_id`, `session_id`, `player_id`, `room_id`, `round_id` | định danh phía server (không phải token) |
| `reason` | mã lý do disconnect: `client_closed`, `timeout_no_input`, `kicked`, `server_shutdown`, `protocol_error`, `auth_failed`, `duplicate_session` |
| `error_code`, `error_message` | lỗi load: mã lỗi + đường dẫn file, **không** ghi nội dung file |

- Tên sự kiện tối thiểu: `server.start`, `server.stop`, `server.heartbeat`, `server.health`,
  `auth.register`, `auth.login_ok`, `auth.login_fail`, `auth.rate_limited`,
  `auth.login_rejected_duplicate_session`, `auth.logout`, `session.expired`, `session.revoked`,
  `room.create`, `room.join`, `room.leave`, `room.kick`, `round.start`, `round.end`,
  `player.disconnect`, `content.load_ok`, `content.load_fail`, `db.migration`, `db.backup`,
  `db.integrity_fail`.
- **Cấm tuyệt đối** trong log: mật khẩu, PHC string, session token, header `Authorization`, và mọi
  payload có thể chứa chúng. Cơ chế: type `Secret<T>` (§6) + test tự động (T7).
- `server.heartbeat` mỗi 60 giây (config) với các trường định danh → theo dõi được bằng log, V1 không
  cần endpoint metrics riêng.

### 8.2 Health

Lệnh CLI `boom-server health` in **một object JSON** rồi thoát với mã **0** (ok) / **1** (lỗi):

`db_ok`, `schema_version` (`user_version` thực tế), `expected_schema_version`, `integrity_check`,
`rules_profile_id`, `rules_hash`, `content_sha256`, `uptime_ms`, `accounts`, `active_sessions`,
`listen_addrs`, `last_error_ts`.

V1 không mở HTTP: server đã dùng TCP thuần ([ADR 0006](0006-transport-default.md)); một lệnh CLI
kiểm tra được bằng script và trả mã thoát là đủ cho vận hành bản chơi thử, không thêm bề mặt HTTP.

### 8.3 "Config endpoint"

Registry ghi server chịu trách nhiệm "Config endpoint, health và log cấu trúc". Ở V1, **ngón tay cấu hình
không nhạy cảm** được công bố ở hai chỗ: trong `Welcome` của handshake (ADR 0009) và trong output của
`health`:

`server_version`, `wire_schema`, `state_schema`, `rules_profile_id`, `rules_hash`, `content_sha256`,
`protocol_limits_version`.

Không công bố đường dẫn file DB, địa chỉ bind ngoài loopback, cấu hình rate limit hay bất kỳ bí mật nào.
`Welcome` hiện **chưa** có các trường này (ADR 0009 §2 chốt nội dung `Welcome`); vì `boom-proto` chưa
được viết, có thể bổ sung **trước khi** implement — nhưng sửa ADR 0009 là việc của integrator
(§11, T5).

## 9. Quyết định 7 — Threat case tối thiểu

| # | Nguy cơ | Chống bằng gì | Test nghiệm thu |
|---|---|---|---|
| 1 | Dò mật khẩu (brute force/credential stuffing) | Argon2id memory-hard + giới hạn tần suất theo tài khoản/IP | T7 |
| 2 | Dò tài khoản tồn tại (enumeration) | Thông báo lỗi thống nhất + verify bằng hash giả khi không có tài khoản | T3/T7 |
| 3 | Rò token qua log | `Secret<T>` + `Debug` ẩn + DB chỉ lưu SHA-256 + test quét log | T7 |
| 4 | Rò token qua mạng (TCP chưa mã hóa) | **Giới hạn đã biết**: V1 LAN/local; TLS bắt buộc trước phát hành ngoài mạng tin cậy | §11, T5 |
| 5 | Cố định session (fixation) | Token sinh mới mỗi lần đăng nhập; phiên cũ bị revoke | T4 |
| 6 | SQL injection | Chỉ prepared statement với tham số bind (`?1`), không nối chuỗi vào SQL | review + T2 |
| 7 | Bị lấy file DB | Quyền file `0600`; trong file chỉ có hash mật khẩu và hash token | T2 |
| 8 | Hai lần đăng nhập cùng lúc (race) | Unique partial index + transaction ⇒ chỉ một phiên thắng | T4 |
| 9 | Client tự khai `account_id` | Server bind identity từ session, bỏ qua mọi trường identity của client (ADR 0005) | T4/T5 |
| 10 | Crash giữa lúc ghi | Transaction + WAL + `synchronous = FULL` | T2/T4 |
| 11 | Migration hỏng / mất dữ liệu | Sao lưu trước migration; từ chối khởi động khi version lạ; `integrity_check` trong `health` | T2 |
| 12 | Tạo tài khoản hàng loạt (spam) | Cờ `registration_open` trong config + giới hạn tần suất theo nguồn | T3 |

## 10. Quyết định 8 — Vị trí code và dependency

**Không tạo crate mới ở V1** ([ARCHITECTURE §1](../planning/ARCHITECTURE.md#1-repo-và-crate): crate
`[dự kiến]` chỉ được tạo khi task của pha đó mở). Persistence đặt trong `boom-server`, tách module:

| Module trong `crates/boom-server/src/` | Nội dung |
|---|---|
| `identity/mod.rs` | trait `AccountStore` + kiểu dữ liệu domain (không rò SQL ra ngoài) |
| `identity/sqlite.rs` | hiện thực `rusqlite`, schema, migration runner, backup |
| `identity/secret.rs` | `Secret<T>` (Debug ẩn) dùng cho token và PHC string |
| `auth.rs` | hash mật khẩu, xác minh, phát token, chính sách rate limit |
| `observability.rs` | log JSON, tên sự kiện, heartbeat, output `health` |
| `config.rs` | **đã có** từ task #46 (issue-46) — chỉ bổ sung trường (đường dẫn DB, TTL session, ngưỡng rate limit, `registration_open`), không đổi cấu trúc |

Dependency mới (đề xuất, để integrator thêm vào `[workspace.dependencies]` của `Cargo.toml` gốc — file
này thuộc integrator theo AGENTS §5):

| Crate | Phiên bản đã tra | Feature | Vì sao |
|---|---|---|---|
| `rusqlite` | 0.40.2 | `bundled` | Driver SQLite đồng bộ; `bundled` dựng SQLite từ mã nguồn ⇒ CI ba OS không cần lib hệ thống |
| `argon2` | 0.6.0 | mặc định (`password-hash`, `getrandom`) | Argon2id theo RFC 9106; xuất PHC string |
| `rand` | 0.10.3 | mặc định | `OsRng` sinh token 256 bit |
| `serde_json` | — | mặc định | Log JSON Lines và output `health` |

`toml` đã có trong workspace — dùng cho `options.toml` của client (§7).

**Test dùng backend thật**: test của `identity` chạy SQLite thật trên file tạm (`tempfile`), **không**
mock; đây là điều kiện "test backend thật" của ARCHITECTURE §8.

## 11. Quyết định 9 — Thứ tự task triển khai

| T | Việc | Feature | Phụ thuộc | Nghiệm thu |
|---|---|---|---|---|
| T1 | Log JSON + `server.heartbeat` + lệnh `health` + ngón tay cấu hình | V1-029 | `config.rs` (#46) | Log có đủ trường bắt buộc; `health` trả 0/1 đúng; không rò secret |
| T2 | `identity`: schema v1 + migration runner + `AccountStore` + backup + `integrity_check` | V1-028 | T1 | Migration chạy trên file thật; version lạ bị từ chối; test SQLite thật, không mock |
| T3 | Đăng ký/đăng nhập: Argon2id, PHC, thông báo lỗi thống nhất, rate limit | V1-001 | T2 | Sai thông tin bị từ chối; hash lưu đúng định dạng PHC; không lộ enumeration |
| T4 | Session: phát/xác minh/hạn/revoke token; một phiên mỗi tài khoản; `revoke-sessions` | V1-001 | T3 | Đăng nhập thứ hai bị từ chối; restart server giữ identity; token không có trong log |
| T5 | Message auth trên wire (`Auth`/`AuthOk`/`AuthFail`/`Logout`) + `Welcome` có ngón tay cấu hình | V1-001 | ADR 0009 addendum (integrator) | Handshake + auth chạy end-to-end trên `boom-proto`/`boom-net` (P7) |
| T6 | Tùy chọn client: `options.toml` (volume/keybind), ghi atomic | V1-028 | — | Restart client giữ tùy chọn; file hỏng → mặc định, không crash |
| T7 | Test threat case §9 + runbook vận hành | V1-001, V1-029 | T1–T6 | Mỗi dòng §9 có test tương ứng; runbook nêu cách backup/khôi phục |

### Ánh xạ acceptance → task

| Acceptance (registry) | Task phụ trách |
|---|---|
| V1-001: tạo/đăng nhập tài khoản test | T3 |
| V1-001: thông tin sai bị từ chối | T3 (+T7) |
| V1-001: một phiên hoạt động mỗi tài khoản | T4 |
| V1-028: restart server giữ identity | T2 + T4 |
| V1-028: restart client giữ volume/keybind | T6 |
| V1-028: room/trận đang chạy không cần phục hồi | Không cần task — policy SCOPE_V1 (ephemeral) |
| V1-029: version/rules hash, round ID, lý do disconnect, lỗi load | T1 (+T4 cho round/disconnect) |
| V1-029: không log password/session token | T1 + T4 + T7 |

## 12. Nguồn tham chiếu

Tất cả truy cập ngày **2026-10-19**. Đây là nguồn cho **lựa chọn thiết kế**, trạng thái
`candidate`/`design` theo AGENTS §4; không phải claim luật chơi nên không sinh mã `R-xxx`.

| Tài liệu | URL | Phiên bản/ngày | Dùng cho |
|---|---|---|---|
| RFC 9106 — Argon2 | https://www.rfc-editor.org/rfc/rfc9106.html | 2021-09 (Informational) | Argon2id bắt buộc hỗ trợ (§3.1); salt 16 byte RECOMMENDED; bộ tham số khuyến nghị §4/§7.4 |
| SQLite — Write-Ahead Logging | https://www.sqlite.org/wal.html | tài liệu hiện hành | Cơ chế WAL, checkpoint, reader không chặn writer (§3) |
| SQLite — PRAGMA | https://www.sqlite.org/pragma.html#pragma_user_version | tài liệu hiện hành | `user_version` = số nguyên ở offset 60 header, SQLite không tự dùng (§4); `foreign_keys` mặc định **OFF** từ 3.6.19 (§3) |
| SQLite — Partial Indexes | https://www.sqlite.org/partialindex.html | tài liệu hiện hành | `CREATE UNIQUE INDEX ... WHERE` cho một phiên mỗi tài khoản (§4) |
| SQLite — Backup API | https://www.sqlite.org/backup.html | tài liệu hiện hành | `sqlite3_backup_init/step/finish` cho sao lưu nóng (§4) |
| SQLite — VACUUM | https://www.sqlite.org/lang_vacuum.html | tài liệu hiện hành | `VACUUM INTO 'file'` tạo bản sao tối giản, xóa nội dung đã xóa (§4) |
| `rusqlite` | https://docs.rs/rusqlite/0.40.2/rusqlite/ | 0.40.2 (2026-10-12) | Driver SQLite đồng bộ, `Connection`, `Transaction`, `Savepoint`; feature `bundled` (https://docs.rs/crate/rusqlite/0.40.2/features) |
| `argon2` | https://docs.rs/argon2/0.6.0/argon2/ | 0.6.0 (2026-08-27) | Argon2id mặc định, xuất PHC string; `Params::DEFAULT_M_COST = 19 * 1024`, `DEFAULT_T_COST = 2`, `DEFAULT_P_COST = 1` (https://docs.rs/argon2/0.6.0/src/argon2/params.rs.html) |
| `rand` | https://docs.rs/rand/0.10.3/rand/ | 0.10.3 (2026-09-20) | `OsRng` — RNG của hệ điều hành để sinh token |
| `getrandom` | https://docs.rs/getrandom/0.4.3/getrandom/ | 0.4.3 (2026-09-09) | Nguồn entropy mà `rand`/`argon2` dùng để sinh salt |
| `sqlx` | https://docs.rs/sqlx/0.9.0/sqlx/ | 0.9.0 (2026-09-30) | Phương án B bị loá (PostgreSQL async + tokio) — §3 |

## 13. Điều kiện đảo

| Điều kiện | Hành động |
|---|---|
| Cần nhiều tiến trình/máy server, hoặc DB > vài trăm MB | Chuyển sang PostgreSQL bằng ADR mới; giữ logic schema và trait `AccountStore` |
| Phép đo thời gian hash cho thấy `m_cost`/`t_cost` không đủ (hoặc quá nặng cho máy yếu) | Nâng/hạ tham số Argon2id + ADR mới; rehash dần khi người dùng đăng nhập |
| Muốn đồng bộ tùy chọn client giữa nhiều máy | Thêm bảng `account_options` + API đồng bộ; ADR mới (ngoài V1) |
| Cần phát hành ngoài mạng tin cậy | Bắt buộc TLS/lớp mã hóa tương đương (ADR 0006) trước khi phát hành; cân nhắc token ký HMAC bằng khóa server |
| Cần giới hạn tần suất bền qua restart hoặc nhiều instance | Chuyển bộ đếm vào DB |
| Cần mã hóa file DB | Bật `sqlcipher` của `rusqlite` (thêm dependency build) bằng ADR mới |
| Cần gom log tập trung | Giữ JSON Lines (dễ parse); thêm shipper, không đổi định dạng |
| Cần `health` từ xa hoặc metrics | Mở endpoint HTTP/metrics riêng bằng ADR mới; V1 giữ CLI |

## 14. Giới hạn và việc chưa làm

- **Không có TLS ở V1** → token bearer đi trên TCP thưa (§6, §9 dòng 4).
- **Không có khôi phục mật khẩu, email, 2FA, đăng nhập OAuth** → ngoài V1.
- **Không đồng bộ tùy chọn client** giữa thiết bị.
- **SQLite không mã hóa file**; V1 không dùng SQLCipher.
- **Bộ đếm rate limit nằm trong RAM tiến trình** → mất khi restart server (chấp nhận V1; xem §13).
- **Không có trace server gốc cho auth/session**: toàn bộ ADR là DESIGN. Mọi khác biệt với hệ thống
  Nexon/NGM cũ là **có chủ ý** — V1 không port auth Nexon/NGM (SCOPE_V1 §8).
- **Số trong ADR này là tham số vận hành/hiệu năng, không phải luật chơi**: TTL session, ngưỡng rate
  limit, TTL heartbeat, kích thước token, tham số Argon2. Chúng nằm trong config, không nằm trong
  `Rules`, và không sinh claim `R-xxx`.
- **Chưa làm**: code, migration script, message auth, TLS, endpoint health từ xa, backup tự động theo lịch.

## 15. Việc cần integrator xử lý khi duyệt ADR này

1. Thêm hàng 0011 vào `docs/decisions/README.md` (index ADR thuộc integrator, AGENTS §5) và đổi trạng
   thái ADR này từ `Proposed` sang `Accepted (mặc định có thể đảo)`.
2. Cập nhật dòng "Persistence" ở [ARCHITECTURE §8](../planning/ARCHITECTURE.md#8-quyết-định-còn-mở)
   (đã sửa trong PR này, cần đồng bộ khi review).
3. Cập nhật Q-011 trong [QUESTIONS](../research/QUESTIONS.md) (đã sửa trong PR này, trạng thái
   `proposed-close`; đóng `done` sau khi review + task T1–T7 có bằng chứng).
4. **Addendum ADR 0009**: thêm kind message auth (`Auth`/`AuthOk`/`AuthFail`/`Logout`) và trường ngón tay
   cấu hình trong `Welcome` — cần trước T5.
5. Thêm `rusqlite`, `argon2`, `rand`, `serde_json` vào `[workspace.dependencies]` của `Cargo.toml` gốc
   (Cargo.toml gốc thuộc integrator, AGENTS §5).
6. Mở các task triển khai T1–T7 theo §11 (mỗi task một issue, một owner).
