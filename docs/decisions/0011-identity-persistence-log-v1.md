# 0011 — Identity, persistence và log/vận hành V1 (Q-011)

Status: **Accepted (mặc định có thể đảo)**, 2026-10-19; review #57 (Codex), addendum ADR 0009 A. Trả lời phần còn mở của Q-011 và làm đầu ra cho
V1-001, V1-028, V1-029. **Task:** GitHub issue #54, nhánh `issue-54`, baseline `main @ 2af995d`.
Owner: backend. Reviewer: integrator (Claude) + QA. **Không viết code trong task này**; ADR chỉ chốt
quyết định, vị trí code và thứ tự task.

**Sửa lần 2 (2026-10-20) theo review PR #57** (`Codex/review_identity57`, head `51b00a8`):

| Finding | Sửa ở đâu |
|---|---|
| P2 — phiên cũ không được giải phóng khi mất kết nối/restart → tài khoản bị khóa tới TTL 24 h | §4 (schema + khóa một instance), §6.1 (vòng đời phiên: bind instance/connection, điểm giải phóng, thứ tự atomic), §9 dòng 13–14, §11 ca `T4-A1…T4-A7` |
| P2 — `health` không đọc được trạng thái tiến trình server đang chạy | §8.2 (tách health runtime qua admin IPC loopback khỏi offline check; exit code 0/1/2/3; artifact `health.json` chỉ để chẩn đoán), §9 dòng 15–16, §11 ca `T1-H1…T1-H7` |
| Ghi chú — `rand` 0.10.3 không còn `OsRng` | §6 (dùng `getrandom` 0.4.3 + xử lý lỗi entropy, không fallback RNG yếu), §10 (bỏ `rand`, thêm `getrandom`/`fs4`), §12 (nguồn + ghi chú) |

> Index `docs/decisions/README.md` và dòng "Persistence" ở
> [ARCHITECTURE §8](../planning/ARCHITECTURE.md#8-quyết-định-còn-mở) do integrator cập nhật khi
> duyệt ADR này (index ADR thuộc integrator theo AGENTS §5). ADR 0009 cũng cần một addendum cho
> message auth **và** kind health — xem §15.

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
  Ở đây "phiên hoạt động" được hiểu là **phiên còn sống** (connection còn sống), không phải "hàng còn nằm
  trong DB" — ranh giới này và điểm giải phóng phiên ở §6.1; SCOPE_V1 §3 **không** yêu cầu người chơi
  nhờ admin khi mất kết nối.
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

Trong phạm vi ADR này: cơ sở lưu trữ, schema tối thiểu + migration, hash mật khẩu, session token và
**vòng đời phiên** (gắn instance/connection, điểm giải phóng, thứ tự atomic), tùy chọn client,
log/health (**runtime** qua admin IPC **và** offline check tách riêng), threat case tối thiểu, vị trí
code, thứ tự task triển khai.

Ngoài phạm vi (mở task riêng, không ghi ở đây): giao thức message auth chi tiết và kind
`HealthRequest`/`HealthResponse` (ADR 0009 addendum + `boom-proto` ở P7), TLS, đồng bộ tùy chọn client,
khôi phục mật khẩu/email/2FA, quản trị nhiều server, mã hóa toàn bộ file DB, economy/ledger
(backlog sau V1).

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

**File bản sao** (backup thủ công và backup trước migration) cũng owner-only `0600` trên POSIX, và
quyền đó phải được đặt **ngay từ lúc tạo file** — trước khi SQLite ghi dữ liệu vào. Lý do: bản sao
chứa `accounts.password_hash` và `sessions.token_hash`; nếu để `VACUUM INTO` tự tạo file thì quyền
là mặc định của tiến trình (`0666 & ~umask` ⇒ `0644` dưới umask `022`), tức group/other đọc được
bản sao trong khi file DB gốc đã ở `0600`. Bản sao đích đã tồn tại thì **không ghi đè**.

**Giới hạn Windows:** Windows không có quyền POSIX `0600`. File (DB và bản sao) được tạo với quyền
mặc định của tiến trình, việc bảo vệ dựa vào **ACL của thư mục dữ liệu** (thư mục phải owner-only).
Biện pháp `0600` ở §3 và threat §9 dòng 7 áp dụng cho POSIX; trên Windows đây là **giới hạn đã
biết**, chấp nhận được ở V1.

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
    instance_id     TEXT    NOT NULL,          -- boot id của tiến trình sở hữu phiên (§6.1)
    connection_id   TEXT,                     -- connection đã auth; NULL sau khi connection chết
    created_at_ms   INTEGER NOT NULL,
    expires_at_ms   INTEGER NOT NULL,
    last_seen_at_ms INTEGER NOT NULL,
    revoked_at_ms   INTEGER,                  -- NULL = phiên còn giữ chỗ
    revoke_reason   TEXT                      -- mã lý do giải phóng (§6.1)
);

-- Một tài khoản chỉ có một phiên hiệu lực (SCOPE_V1 §3): ràng buộc ở tầng DB, không chỉ ở code.
CREATE UNIQUE INDEX sessions_one_active_per_account
    ON sessions(account_id) WHERE revoked_at_ms IS NULL;
```

Quyết định kèm theo:

- **Unique partial index** là nơi *thực thi* policy "một tài khoản một phiên hoạt động": đăng nhập thứ hai
  phải thất bại ở tầng DB, không phụ thuộc thứ tự kiểm tra trong code. SQLite hỗ trợ partial index
  (`CREATE UNIQUE INDEX ... WHERE`) (nguồn ở §12).
- **Định nghĩa "phiên giữ chỗ"** (đúng thứ unique partial index thực thi): `revoked_at_ms IS NULL`.
  Một phiên chỉ **sống** khi nó giữ chỗ **và** `expires_at_ms > now` **và** nó gắn với connection còn
  sống của instance đang chạy (§6.1). Vì index chỉ nhìn `revoked_at_ms IS NULL`, mọi phiên không còn
  sống **phải được đánh dấu `revoked_at_ms`** — nếu không, phiên cũ vẫn chặn đăng nhập mới. Đây là
  **điều kiện của index**, không phải tối ưu hiệu năng: nó là cơ chế giải phóng phiên khi mất kết nối
  hoặc restart server (§6.1).
- **`instance_id` / `connection_id`**: `instance_id` là boot id của tiến trình server, `connection_id` là
  connection đã auth (NULL sau khi connection đó chết). Hai cột này cho phép **biết phiên đang thuộc
  instance/connection nào** để giải phóng đúng phiên mà không đụng phiên mới (§6.1).
- **Một instance server cho một file DB**: V1 chạy **một** tiến trình server trên một file DB. Server
  giữ khóa advisory độc quyền trên file `<data_dir>/boom-server.lock` (`flock`/`LockFileEx` qua crate
  `fs4`, §10); không khoá được → **từ chối khởi động** (exit code 2, log `server.start_blocked` reason
  `db_locked`). Không có khóa này thì bước "giải phóng phiên của instance cũ lúc boot" (§6.1) sẽ
  giải phóng nhầm phiên của instance đang chạy.
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
- **Kiểm tra tính toàn vẹn** (`PRAGMA integrity_check`) chạy trong lệnh `health` (§8.2), không chạy mỗi
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

- **Sinh**: 32 byte (256 bit) từ RNG của hệ điều hành: `getrandom` 0.4.3 (`getrandom::fill`), mã hóa
  **base64url không padding** (43 ký tự) để gửi trong message mà không cần escape. **Không dùng `rand`**:
  `rand` 0.10.3 **không còn** `OsRng` (chỉ có `rand::rngs::SysRng` qua trait `TryRng` — xem ghi chú ở
  §12), còn `getrandom::fill` làm đúng việc cần làm với ít gián tiếp hơn.
- **Lỗi entropy thì không có fallback yếu**: `getrandom::fill` trả `Err(getrandom::Error)` → **không**
  dùng PRNG yếu (`rand::rng()`, `SmallRng`, RNG tự viết), **không** dùng lại token/salt cũ, **không**
  hạ xuống token dễ đoán. Server trả thông báo lỗi thống nhất (không tiết lộ chi tiết) và log
  `auth.rng_unavailable` kèm mã lỗi; đăng ký/đăng nhập thất bại và **không** tạo session. `getrandom`
  tự ghi rõ "prioritize failure over returning known insecure random bytes" (§12). Salt Argon2 sinh
  từ cùng nguồn: `password_hash::try_generate_salt()` (feature `getrandom`); lỗi thì xử lý y hệt.
- **Lưu**: DB chỉ lưu `SHA-256(token thô)` (`sessions.token_hash`). Không phục hồi được token từ DB;
  không ghi token thô ở bất kỳ đâu ngoài RAM của tiến trình.
- **Truyền**: token là **bearer credential** — client gửi kèm sau bước auth; server bind
  `account_id`/`session_id` từ token, **không** tin `account_id` do client khai (ADR 0005).
- **Hết hạn**: TTL cố định (mặc định 24 giờ, trong config), **không trượt** khi dùng. Hết hạn thì client
  đăng nhập lại — đúng policy "reconnect cần auth lại" của SCOPE_V1 §3. Phiên hết hạn được đánh dấu
  `revoked_at_ms` (reason `expired`) theo §4/§6.1 để index "một phiên hiệu lực" không chặn đăng nhập
  mới. **TTL không phải hàng rào cuối cho việc khóa tài khoản**: phiên được giải phóng ngay khi mất
  kết nối hoặc restart server (§6.1).
- **Một phiên mỗi tài khoản**: unique partial index (§4). Đăng nhập khi tài khoản đã có phiên **còn
  sống** bị **từ chối rõ ràng** với mã lý do riêng (`account_has_active_session`), đúng SCOPE_V1 §3 —
  server **không** tự động đuổi phiên cũ. Phiên đã được giải phóng theo §6.1 thì không còn là phiên
  hiệu lực, nên đăng nhập mới được chấp nhận.
- **Logout**: đặt `revoked_at_ms` (reason `logout`); token bị từ chối ngay lập tức.
- **Gỡ phiên kẹt**: lệnh quản trị `boom-server account revoke-sessions <username>` (xóa mọi phiên hiệu
  lực của tài khoản) — đường **dự phòng**, không phải đường chính; người chơi bị đứt mạng hoặc server
  restart tự vào lại được (§6.1) mà không cần admin.
- **Không rò token trong log**: kiểu `Secret<T>` bọc token, `Debug` in ra chuỗi ẩn, không có `Display`,
  không `Clone` nếu không cần. Test tự động quét output log để chắc không có token (§11, T7).
- **Rủi ro đã biết**: ADR 0006 để TLS cho task backend, nên ở V1 token đi trên TCP **chưa mã hóa**. V1
  chỉ dùng cho LAN/máy local. **Trước khi phát hành ngoài mạng tin cậy, bắt buộc có TLS** (điều kiện ở
  §11). Không coi "V1 không thương mại" là lý do để bỏ qua bước này.

### 6.1 Vòng đời phiên: gắn vào instance/connection và giải phóng khi mất kết nối

V1 có **một** tiến trình server cho một file DB (§4). Mỗi phiên gắn với **instance** (tiến trình) và
**connection** (kết nối TCP đã auth) đã sinh ra nó. Mục tiêu: **tài khoản không bao giờ bị khóa vì phiên
cũ còn nằm trong DB** — mất kết nối hoặc restart server thì đăng nhập lại được, không cần admin
(SCOPE_V1 §3 yêu cầu reconnect phải auth lại, không yêu cầu người chơi nhờ admin).

**Thuật ngữ:**

| Từ | Nghĩa |
|---|---|
| `instance_id` (boot id) | 16 byte ngẫu nhiên từ OS RNG, mã hex, sinh **một lần** khi tiến trình bắt đầu; ghi trong log `server.start` và trong artifact `health.json` |
| `connection_id` | Định danh connection đã auth (số tăng dần trong tiến trình); NULL trong DB sau khi connection chết |
| Phiên **giữ chỗ** | `revoked_at_ms IS NULL` — đúng thứ unique partial index thực thi (§4) |
| Phiên **sống** | Giữ chỗ **và** `expires_at_ms > now` **và** gắn với connection còn sống của instance đang chạy |

**Khi nào phiên được giải phóng** (`revoked_at_ms` được đặt, `revoke_reason` ghi lý do):

| Tình huống | Thời điểm giải phóng | `revoke_reason` |
|---|---|---|
| Client đăng xuất (`Logout`) | ngay khi nhận message | `logout` |
| Connection chết **đã xác nhận** (EOF, reset, lỗi ghi) | ngay khi transport báo lỗi | `disconnect` |
| Connection im lặng (không trả lời `Ping`) | sau `SESSION_LIVENESS_TIMEOUT_MS` | `timeout_no_input` |
| Hết TTL | job quét định kỳ, hoặc ngay khi đăng nhập lại tài khoản đó | `expired` |
| **Server khởi động (boot mới)** | ngay lúc mở DB, **trước** khi nhận auth | `server_restart` |
| Server tắt có trật tự (SIGTERM) | trước khi thoát | `server_shutdown` |
| Admin `revoke-sessions` | khi lệnh chạy | `admin_revoke` |

**Phát hiện connection chết.** Không dựa vào việc client đóng socket: TCP không báo "peer đã chết" khi
mạng chết im lặng. Server dùng `Ping`/`Pong` đã có sẵn trong ADR 0009 (kind `0x000E`/`0x000F`): gửi
`Ping` mỗi `SESSION_PING_INTERVAL_MS` (config, mặc định 15 000 ms) cho connection đã auth; nếu quá
`SESSION_LIVENESS_TIMEOUT_MS` (config, mặc định 30 000 ms) không có `Pong` → coi connection chết →
giải phóng phiên, log `player.disconnect` reason `timeout_no_input`. TCP keepalive của OS **không**
là nguồn tín hiệu (mặc định OS thường tính bằng giờ, không kiểm soát được).

⇒ **Trần thời gian một tài khoản bị khóa** = `SESSION_LIVENESS_TIMEOUT_MS` + độ trễ xử lý, **không** phải
TTL 24 giờ. Ngoại lệ duy nhất: nếu server **không khởi động lại** sau khi chết, phiên của instance chết
vẫn giữ chỗ cho tới lần boot kế tiếp (xem §14).

**Thứ tự xử lý atomic** (phần dễ sai nhất — đây là quy tắc bắt buộc):

1. **Mọi đường teardown của connection chỉ revoke đúng `session_id` mà connection đó đã auth**, với
   điều kiện `WHERE id = ? AND revoked_at_ms IS NULL`. **Không** dùng `WHERE account_id = ?` ở đường
   này: nếu không, race "connection cũ chết *sau* khi connection mới đã đăng nhập" sẽ revoke nhầm
   phiên mới. Điều kiện `revoked_at_ms IS NULL` làm thao tác idempotent và không chạm phiên mới (id
   khác).
2. **Đăng nhập chạy trong một transaction** (`BEGIN IMMEDIATE`): (a) dọn phiên đã hết TTL **của tài
   khoản đó** (`expires_at_ms <= now`), (b) `INSERT` phiên mới kèm `instance_id`/`connection_id` hiện
   tại. Nếu (b) vi phạm unique partial index → rollback → trả `account_has_active_session`. Vì (a) và (b)
   cùng transaction, không có cửa sổ mà phiên cũ chặn phiên mới, và không có cửa sổ mà cleanup của
   phiên cũ chạm phiên vừa tạo.
3. **Cleanup lúc boot** lọc theo instance: `WHERE revoked_at_ms IS NULL AND instance_id <> <instance_id
   hiện tại>` — **không** lọc theo `account_id`, để không đụng phiên của instance đang chạy (chỉ chạy
   được vì đã có khóa file §4). Bước này chạy **trước** khi server nhận auth.
4. **Job quét định kỳ** dọn theo `expires_at_ms` và theo `instance_id` đã chết; không bao giờ quét
   theo `account_id` trần.

**Quy tắc "một phiên mỗi tài khoản" được giữ nguyên.** SCOPE_V1 §3 vẫn đúng: nếu phiên đầu **còn
sống** (connection còn sống), đăng nhập thứ hai bị **từ chối rõ ràng** với mã
`account_has_active_session`; server không tự động đuổi phiên cũ. Khác biệt là **ranh giới** của
"còn sống": trước đây ranh giới là TTL 24 giờ, nay là connection/instance. Nhờ vậy người chơi bị đứt
mạng hoặc server restart **không** cần admin để vào lại.

**Vì sao không "tự đuổi phiên cũ khi có phiên mới"**: đó là chính sách khác (đuổi phiên), SCOPE_V1 §3
chọn "từ chối", và nó cũng che được trường hợp hai người dùng chung một tài khoản.

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
| `instance_id` | boot id của tiến trình server (§6.1) — dùng để đối chiếu log giữa các lần chạy |
| `revoke_reason` | mã lý do giải phóng phiên: `logout`, `disconnect`, `timeout_no_input`, `expired`, `server_restart`, `server_shutdown`, `admin_revoke` (§6.1) |
| `reason` | mã lý do disconnect: `client_closed`, `timeout_no_input`, `kicked`, `server_shutdown`, `protocol_error`, `auth_failed`, `duplicate_session` |
| `error_code`, `error_message` | lỗi load: mã lỗi + đường dẫn file, **không** ghi nội dung file |

- Tên sự kiện tối thiểu: `server.start`, `server.stop`, `server.heartbeat`, `server.health`,
  `server.start_blocked`, `server.health_rejected`, `auth.register`, `auth.login_ok`, `auth.login_fail`,
  `auth.rate_limited`, `auth.login_rejected_duplicate_session`, `auth.rng_unavailable`, `auth.logout`,
  `session.expired`, `session.revoked`, `room.create`, `room.join`, `room.leave`, `room.kick`,
  `round.start`, `round.end`, `player.disconnect`, `content.load_ok`, `content.load_fail`,
  `db.migration`, `db.backup`, `db.integrity_fail`.
- **Cấm tuyệt đối** trong log: mật khẩu, PHC string, session token, header `Authorization`, và mọi
  payload có thể chứa chúng. Cơ chế: type `Secret<T>` (§6) + test tự động (T7).
- `server.heartbeat` mỗi 60 giây (config) với các trường định danh → theo dõi được bằng log, V1 không
  cần endpoint metrics riêng.

### 8.2 Health: runtime qua admin IPC, kiểm offline tách riêng

Có **hai** loại kiểm tra, **không trộn** với nhau:

| Lệnh | Cần server đang chạy? | Trả lời câu hỏi nào | Exit code |
|---|---|---|---|
| `boom-server health` | **có** | server còn sống, đã bind, đã nạp content, DB còn ổn? | 0 / 1 / 2 / 3 |
| `boom-server health --offline` | không | file DB + config có đọc được, schema đúng, integrity OK? | 0 / 1 / 3 |

**Vì sao phải tách**: đọc được file DB/config **không** chứng minh server còn sống — lệnh offline chỉ
nói về dữ liệu và **không** được báo là "server khỏe"; ngược lại, lệnh runtime **không** được suy ra
từ việc đọc DB. Thiết kế cũ (chỉ đọc DB + stdout) không có đường nào đọc trạng thái **tiến trình
server đang chạy**, nên có thể báo OK khi server đã chết; §8.2.1 chốt lại đường đó.

#### 8.2.1 Health runtime — admin command qua transport hiện có

- Server mở thêm một **admin listener** chỉ bind **loopback** (`127.0.0.1` / `::1`). Địa chỉ nằm trong
  config (`admin_addr`, mặc định `127.0.0.1:17890`); nếu cổng đó bận, server dùng cổng trống và ghi
  địa chỉ thật vào artifact `health.json` (§8.2.3) + log `server.start`. **Không mở HTTP**, không mở
  bề mặt web nào.
- Admin listener nói **cùng envelope/kind** của `boom-proto` (ADR 0009) và chỉ nhận **một** kind:
  `HealthRequest` (kind mới `0x0050`, C→S) → trả `HealthResponse` (kind mới `0x0051`, S→C). Mọi kind
  khác → đóng kết nối, log `server.health_rejected`. Hai kind này phải được thêm bằng **addendum ADR
  0009** (§15) — thêm kind là **additive**, **không** đổi `wire_schema` (ADR 0009 §1).
- `HealthResponse` mang payload **postcard** (đúng codec ADR 0009): `status`, `uptime_ms`,
  `started_at_ms`, `instance_id`, `listen_addrs`, `admin_addr`, `last_error_ts`, `db_ok`,
  `schema_version`, `expected_schema_version`, `integrity_check`, `rules_profile_id`, `rules_hash`,
  `content_sha256`, `accounts`, `active_sessions`, `problems`. CLI **serialize ra JSON** ở biên CLI
  (dây vẫn là postcard): JSON Lines cho log (§8.1), JSON cho stdout của lệnh `health`.
- Không cần auth/session trên admin port: nó chỉ bind loopback và chỉ trả **số liệu vận hành** (không
  có bí mật, không có danh sách tài khoản, không có token). Mọi người dùng cùng máy đều đọc được →
  ghi ở §14 là giới hạn đã biết. Nếu server bind game port ra ngoài loopback (LAN), admin port **vẫn**
  loopback.
- `uptime_ms` / `listen_addrs` / `last_error_ts` lấy **từ tiến trình server**, không phải từ CLI — đó
  chính là phần thiết kế cũ thiếu.
- **CLI tìm địa chỉ admin như thế nào**: đọc `health.json` (ưu tiên — server có thể đã dùng cổng trống),
  không có thì dùng `admin_addr` trong config. Không đọc được config ⇒ exit `3` (lỗi CLI).

**Exit code của `boom-server health`** (nghĩa rõ ràng, dùng được trong script/CI):

| Exit | Nghĩa | Khi nào |
|---:|---|---|
| `0` | healthy | server trả lời trong thời hạn, `status = "healthy"`, `problems` rỗng |
| `1` | server trả lời nhưng **không** healthy | `status = "unhealthy"`: `db_ok = false`, `integrity_check` lỗi, `schema_version` lệch, content chưa nạp, `problems` không rỗng |
| `2` | **không liên lạc được** server | server chưa chạy/đã dừng (connection refused), hoặc **đang treo** (quá `HEALTH_TIMEOUT_MS`, mặc định 3 000 ms), hoặc admin port sai trong config |
| `3` | lỗi phía CLI | config đọc lỗi, tham số sai, artifact `health.json` hỏng mà không xác định được admin port |

Kết quả **luôn** là một object JSON trên stdout (kể cả khi exit ≠ 0), trừ lỗi CLI (exit 3) thì in lỗi
ra stderr. Giá trị của `status`: `healthy`, `unhealthy`, `unreachable`, `timeout`.

#### 8.2.2 Health offline

`boom-server health --offline` **không** kết nối server: mở file DB ở chế độ chỉ đọc, chạy
`PRAGMA user_version` + `PRAGMA integrity_check`, đọc config, rồi in JSON với `"scope": "offline"` và
`"runtime": "not_checked"`. Exit `0` (mọi thứ ổn) / `1` (DB lỗi hoặc schema lệch) / `3` (lỗi CLI).
Kết quả này **không** được diễn giải là "server khỏe" — đó là lý do nó có nhãn `scope` riêng.

#### 8.2.3 Artifact `health.json` — để chẩn đoán, không phải nguồn sự thật

Server ghi `<data_dir>/health.json` (**ghi atomic**: file tạm rồi rename) khi khởi động và mỗi
`HEARTBEAT_INTERVAL_MS` (60 000 ms). Nội dung: `instance_id`, `pid`, `started_at_ms`, `updated_at_ms`,
`listen_addrs`, `admin_addr`, `status`.

- **Artifact không bao giờ dùng để kết luận "healthy"** — nó không chứng minh tiến trình còn đáp ứng.
  Nó chỉ phân biệt các trạng thái khi IPC thất bại:
  - file không có → server chưa từng chạy với thư mục dữ liệu này (hoặc đã bị xoá);
  - file cũ hơn `HEARTBEAT_INTERVAL_MS × 2` → heartbeat không cập nhật: server **đã chết** hoặc **đang
    treo**;
  - file cũ mà `pid` còn sống → nhiều khả năng server **treo** (còn giữ khoá file DB nhưng không
    heartbeat);
  - file mới mà IPC vẫn lỗi → server có vẻ sống nhưng admin port không trả lời (sai cấu hình / bị chặn).
- Vì `pid` có thể được tái sử dụng, `instance_id` là định danh đáng tin hơn `pid`; cả hai **chỉ để
  chẩn đoán**.

**Không mở HTTP**: V1 đã dùng TCP thuần ([ADR 0006](0006-transport-default.md)); admin command đi trên
transport hiện có, trả lời được bằng script và có mã thoát rõ ràng — không cần bề mặt HTTP.

### 8.3 "Config endpoint"

Registry ghi server chịu trách nhiệm "Config endpoint, health và log cấu trúc". Ở V1, **ngón tay cấu hình
không nhạy cảm** được công bố ở hai chỗ: trong `Welcome` của handshake (ADR 0009) và trong output của
`health`:

`server_version`, `wire_schema`, `state_schema`, `rules_profile_id`, `rules_hash`, `content_sha256`,
`protocol_limits_version`.

Không công bố đường dẫn file DB, địa chỉ bind ngoài loopback, cấu hình rate limit hay bất kỳ bí mật nào
— đó là quy tắc cho `Welcome` (gửi cho **mọi** client). `HealthResponse` (§8.2.1) chỉ trả cho client
**loopback** nên được phép kèm `listen_addrs`/`admin_addr`; vẫn không kèm đường dẫn file DB hay bí mật
nào. `Welcome` hiện **chưa** có các trường ngón tay cấu hình (ADR 0009 §2 chốt nội dung `Welcome`); vì
`boom-proto` chưa được viết, có thể bổ sung **trước khi** implement — nhưng sửa ADR 0009 là việc của
integrator (§11, T5).

## 9. Quyết định 7 — Threat case tối thiểu

| # | Nguy cơ | Chống bằng gì | Test nghiệm thu |
|---|---|---|---|
| 1 | Dò mật khẩu (brute force/credential stuffing) | Argon2id memory-hard + giới hạn tần suất theo tài khoản/IP | T7 |
| 2 | Dò tài khoản tồn tại (enumeration) | Thông báo lỗi thống nhất + verify bằng hash giả khi không có tài khoản | T3/T7 |
| 3 | Rò token qua log | `Secret<T>` + `Debug` ẩn + DB chỉ lưu SHA-256 + test quét log | T7 |
| 4 | Rò token qua mạng (TCP chưa mã hóa) | **Giới hạn đã biết**: V1 LAN/local; TLS bắt buộc trước phát hành ngoài mạng tin cậy | §11, T5 |
| 5 | Cố định session (fixation) | Token **mới** sinh mỗi lần đăng nhập (token cũ không tái dùng); phiên cũ chỉ bị revoke theo vòng đời §6.1, không đuổi phiên còn sống | T4 (`T4-A1`) |
| 6 | SQL injection | Chỉ prepared statement với tham số bind (`?1`), không nối chuỗi vào SQL | review + T2 |
| 7 | Bị lấy file DB (hoặc file bản sao của nó) | Quyền file `0600` cho **cả** DB và bản sao, đặt ngay từ lúc tạo file; trong file chỉ có hash mật khẩu và hash token | T2 |
| 8 | Hai lần đăng nhập cùng lúc (race) | Unique partial index + transaction ⇒ chỉ một phiên thắng | T4 (`T4-A4`) |
| 9 | Client tự khai `account_id` | Server bind identity từ session, bỏ qua mọi trường identity của client (ADR 0005) | T4/T5 |
| 10 | Crash giữa lúc ghi | Transaction + WAL + `synchronous = FULL` | T2/T4 |
| 11 | Migration hỏng / mất dữ liệu | Sao lưu trước migration; từ chối khởi động khi version lạ; `integrity_check` trong `health` | T2 |
| 12 | Tạo tài khoản hàng loạt (spam) | Cờ `registration_open` trong config + giới hạn tần suất theo nguồn | T3 |
| 13 | **Phiên giữ chỗ vô hạn** sau đứt mạng/crash → tài khoản bị khóa tới TTL 24 h, cần admin | Phiên gắn `instance_id`+`connection_id`; giải phóng khi disconnect/timeout/boot/shutdown; liveness bằng `Ping`/`Pong` (§6.1) | T4 (`T4-A1`…`T4-A3`, `T4-A7`) |
| 14 | **Cleanup phiên cũ revoke nhầm phiên mới** (race teardown vs đăng nhập lại) | Revoke theo `session_id` + điều kiện `revoked_at_ms IS NULL`; đăng nhập trong một transaction; cleanup lúc boot lọc theo `instance_id` (§6.1) | T4 (`T4-A5`) |
| 15 | **`health` báo OK khi server đã chết/treo** | Health runtime qua admin IPC loopback (nguồn sự thật là tiến trình server) + tách offline check + exit code 2 khi không liên lạc được (§8.2) | T1 (`T1-H1`…`T1-H5`) |
| 16 | Lộ trạng thái server qua mạng | Admin listener chỉ bind loopback, chỉ nhận `HealthRequest`, không mở HTTP (§8.2.1) | T1 (`T1-H6`, `T1-H7`) |

## 10. Quyết định 8 — Vị trí code và dependency

**Không tạo crate mới ở V1** ([ARCHITECTURE §1](../planning/ARCHITECTURE.md#1-repo-và-crate): crate
`[dự kiến]` chỉ được tạo khi task của pha đó mở). Persistence đặt trong `boom-server`, tách module:

| Module trong `crates/boom-server/src/` | Nội dung |
|---|---|
| `identity/mod.rs` | trait `AccountStore` + kiểu dữ liệu domain (không rò SQL ra ngoài) |
| `identity/sqlite.rs` | hiện thực `rusqlite`, schema, migration runner, backup |
| `identity/secret.rs` | `Secret<T>` (Debug ẩn) dùng cho token và PHC string |
| `identity/session.rs` | vòng đời phiên: bind `instance_id`/`connection_id`, revoke theo `session_id`, quét hết TTL, boot cleanup (§6.1) |
| `auth.rs` | hash mật khẩu, xác minh, phát token, chính sách rate limit, giải phóng phiên theo `Ping`/`Pong` |
| `admin.rs` | admin listener loopback: `HealthRequest`/`HealthResponse`, ghi artifact `health.json` (§8.2) |
| `observability.rs` | log JSON, tên sự kiện, heartbeat, output `health` |
| `config.rs` | **đã có** từ task #46 (issue-46) — chỉ bổ sung trường (đường dẫn DB, TTL session, ngưỡng rate limit, `registration_open`, `admin_addr`, `SESSION_PING_INTERVAL_MS`, `SESSION_LIVENESS_TIMEOUT_MS`, `HEALTH_TIMEOUT_MS`, `HEARTBEAT_INTERVAL_MS`), không đổi cấu trúc |

Dependency mới (đề xuất, để integrator thêm vào `[workspace.dependencies]` của `Cargo.toml` gốc — file
này thuộc integrator theo AGENTS §5):

| Crate | Phiên bản đã tra | Feature | Vì sao |
|---|---|---|---|
| `rusqlite` | 0.40.2 | `bundled` | Driver SQLite đồng bộ; `bundled` dựng SQLite từ mã nguồn ⇒ CI ba OS không cần lib hệ thống |
| `argon2` | 0.6.0 | mặc định (`alloc`, `password-hash`, `getrandom`) | Argon2id theo RFC 9106; xuất PHC string; salt từ cùng nguồn entropy của OS |
| `getrandom` | 0.4.3 | mặc định | Sinh token 256 bit **trực tiếp** từ RNG của hệ điều hành (`getrandom::fill`); lỗi entropy → từ chối, **không** fallback RNG yếu (§6) |
| `fs4` | 1.1.0 | mặc định (`sync`) | `FileExt::try_lock` — khóa advisory độc quyền file `<data_dir>/boom-server.lock` để chỉ **một** instance server trên một file DB (§4, §6.1) |
| `serde_json` | 1.0.154 | mặc định | Log JSON Lines và output JSON của lệnh `health` |

**Không dùng `rand`**: `rand` 0.10.3 **không còn** `OsRng` (chỉ có `rand::rngs::SysRng` qua trait
`TryRng` — xem ghi chú ở §12), và `getrandom::fill` làm đúng việc cần làm với ít gián tiếp hơn.

`toml` đã có trong workspace — dùng cho `options.toml` của client (§7).

**Test dùng backend thật**: test của `identity` chạy SQLite thật trên file tạm (`tempfile`), **không**
mock; đây là điều kiện "test backend thật" của ARCHITECTURE §8.

## 11. Quyết định 9 — Thứ tự task triển khai

| T | Việc | Feature | Phụ thuộc | Nghiệm thu |
|---|---|---|---|---|
| T1 | Log JSON + `server.heartbeat` + admin listener loopback + lệnh `health` (runtime + `--offline`) + artifact `health.json` + ngón tay cấu hình | V1-029 | `config.rs` (#46), addendum ADR 0009 (kind `HealthRequest`/`HealthResponse`) | Log có đủ trường bắt buộc; `health` trả 0/1/2/3 đúng nghĩa §8.2; phân biệt được server chạy/dừng/treo; không rò secret |
| T2 | `identity`: schema v1 + migration runner + `AccountStore` + backup + `integrity_check` | V1-028 | T1 | Migration chạy trên file thật; version lạ bị từ chối; test SQLite thật, không mock |
| T3 | Đăng ký/đăng nhập: Argon2id, PHC, thông báo lỗi thống nhất, rate limit, xử lý lỗi entropy | V1-001 | T2 | Sai thông tin bị từ chối; hash lưu đúng định dạng PHC; không lộ enumeration; lỗi OS RNG ⇒ từ chối, không phát token yếu |
| T4 | Session: phát/xác minh/hạn/revoke token; bind `instance_id`/`connection_id`; giải phóng khi disconnect/timeout/restart; một phiên mỗi tài khoản; `revoke-sessions` | V1-001 | T3 | Đăng nhập thứ hai **khi phiên đầu còn sống** bị từ chối; disconnect/restart ⇒ đăng nhập lại được; restart server giữ identity; token không có trong log |
| T5 | Message auth trên wire (`Auth`/`AuthOk`/`AuthFail`/`Logout`) + `Welcome` có ngón tay cấu hình | V1-001 | ADR 0009 addendum (integrator) | Handshake + auth chạy end-to-end trên `boom-proto`/`boom-net` (P7) |
| T6 | Tùy chọn client: `options.toml` (volume/keybind), ghi atomic | V1-028 | — | Restart client giữ tùy chọn; file hỏng → mặc định, không crash |
| T7 | Test threat case §9 + runbook vận hành | V1-001, V1-029 | T1–T6 | Mỗi dòng §9 có test tương ứng; runbook nêu cách backup/khôi phục |

### Ca nghiệm thu chi tiết (ID để reviewer đối chiếu)

Task này **không viết code**; các ca dưới là **acceptance bắt buộc** cho T1/T4, viết ở đây để mỗi
finding review có ca test tương ứng và không bị hiểu lầm là đã làm.

**Session (T4)**

| ID | Ca | Kết quả mong đợi |
|---|---|---|
| `T4-A1` | Disconnect **đã xác nhận**: client đóng socket | `revoked_at_ms` được đặt, `revoke_reason = "disconnect"`; đăng nhập lại bằng credential đúng **thành công**, **không** cần admin |
| `T4-A2` | **Restart server** (instance cũ chết) | Lúc boot, mọi phiên `revoked_at_ms IS NULL` của instance cũ bị revoke với `revoke_reason = "server_restart"`; đăng nhập lại **thành công**; `accounts`/`password_hash` còn nguyên (restart server giữ identity) |
| `T4-A3` | **Mất mạng im lặng** (client treo, không FIN/RST) | Quá `SESSION_LIVENESS_TIMEOUT_MS` không có `Pong` ⇒ phiên bị giải phóng với `revoke_reason = "timeout_no_input"`; đăng nhập lại được. Trần thời gian khóa = timeout này, **không** phải TTL 24 h |
| `T4-A4` | **Hai lần đăng nhập đồng thời** (race) | Chỉ một phiên thắng; lần còn lại nhận `account_has_active_session`; trong DB chỉ có một hàng `revoked_at_ms IS NULL` cho tài khoản đó |
| `T4-A5` | **Race teardown**: connection cũ chết *sau* khi connection mới đã đăng nhập thành công | Phiên mới **vẫn** hiệu lực (`revoked_at_ms IS NULL`) — đường teardown của connection cũ không revoke được phiên mới (regression cho lỗi "revoke theo `account_id`") |
| `T4-A6` | Phiên còn sống, đăng nhập thứ hai | Bị từ chối với mã `account_has_active_session` (giữ nguyên policy SCOPE_V1 §3) |
| `T4-A7` | Khởi động server thứ hai khi instance cũ còn giữ khoá file | Instance mới **từ chối khởi động**, exit code 2, log `server.start_blocked` reason `db_locked`; phiên của instance cũ **không** bị đụng |

**Health (T1)**

| ID | Ca | Kết quả mong đợi |
|---|---|---|
| `T1-H1` | Server đang chạy, healthy | `boom-server health` → exit `0`; JSON có `uptime_ms > 0`, `listen_addrs` khớp cấu hình, `last_error_ts` null, `status = "healthy"` |
| `T1-H2` | Server đang chạy nhưng DB lỗi (ví dụ `integrity_check` fail) | Exit `1`; `status = "unhealthy"`, `problems` không rỗng; **vẫn** có `uptime_ms`/`listen_addrs` (runtime info dùng được) |
| `T1-H3` | Server **đã dừng** | Exit `2`, `status = "unreachable"`; CLI **không** báo OK |
| `T1-H4` | Server **treo** (vẫn accept TCP nhưng không trả lời) | Exit `2` sau `HEALTH_TIMEOUT_MS`; CLI không treo; artifact `health.json` cũ + `pid` còn sống ⇒ chẩn đoán "treo" |
| `T1-H5` | `boom-server health --offline` khi server không chạy | Exit `0`/`1` theo DB+config; JSON có `scope = "offline"` và `runtime = "not_checked"`; **không** được hiểu là "server khỏe" |
| `T1-H6` | `HealthRequest` từ địa chỉ **không** phải loopback | Bị từ chối (đóng kết nối, log `server.health_rejected`); admin listener không bind ngoài loopback |
| `T1-H7` | Kiểm tra bề mặt | Không có cổng HTTP nào; admin đi qua envelope ADR 0009 (`0x0050`/`0x0051`) |

### Ánh xạ acceptance → task

| Acceptance (registry) | Task phụ trách |
|---|---|
| V1-001: tạo/đăng nhập tài khoản test | T3 |
| V1-001: thông tin sai bị từ chối | T3 (+T7) |
| V1-001: một phiên hoạt động mỗi tài khoản | T4 (`T4-A4`, `T4-A6`) |
| V1-028: restart server giữ identity | T2 + T4 (`T4-A2`) |
| V1-028: restart client giữ volume/keybind | T6 |
| V1-028: room/trận đang chạy không cần phục hồi | Không cần task — policy SCOPE_V1 (ephemeral) |
| V1-029: version/rules hash, round ID, lý do disconnect, lỗi load | T1 (+T4 cho round/disconnect) |
| V1-029: không log password/session token | T1 + T4 + T7 |

## 12. Nguồn tham chiếu

Tất cả truy cập ngày **2026-10-19**; các dòng đánh dấu † được **tra lại ngày 2026-10-20** (đợt sửa
lần 2 theo review PR #57). Đây là nguồn cho **lựa chọn thiết kế**, trạng thái
`candidate`/`design` theo AGENTS §4; không phải claim luật chơi nên không sinh mã `R-xxx`.

| Tài liệu | URL | Phiên bản/ngày | Dùng cho |
|---|---|---|---|
| RFC 9106 — Argon2 | https://www.rfc-editor.org/rfc/rfc9106.html | 2021-09 (Informational) | Argon2id bắt buộc hỗ trợ (§3.1); salt 16 byte RECOMMENDED; bộ tham số khuyến nghị §4/§7.4 |
| SQLite — Write-Ahead Logging | https://www.sqlite.org/wal.html | tài liệu hiện hành | Cơ chế WAL, checkpoint, reader không chặn writer (§3) |
| SQLite — PRAGMA | https://www.sqlite.org/pragma.html#pragma_user_version | tài liệu hiện hành | `user_version` = số nguyên ở offset 60 header, SQLite không tự dùng (§4); `foreign_keys` mặc định **OFF** từ 3.6.19 (§3) |
| SQLite — Partial Indexes | https://www.sqlite.org/partialindex.html | tài liệu hiện hành | `CREATE UNIQUE INDEX ... WHERE` cho một phiên mỗi tài khoản (§4) |
| SQLite — Backup API | https://www.sqlite.org/backup.html | tài liệu hiện hành | `sqlite3_backup_init/step/finish` cho sao lưu nóng (§4) |
| SQLite — VACUUM | https://www.sqlite.org/lang_vacuum.html | tài liệu hiện hành | `VACUUM INTO 'file'` tạo bản sao tối giản, xóa nội dung đã xóa (§4) |
| `rusqlite` † | https://docs.rs/rusqlite/0.40.2/rusqlite/ | 0.40.2 | Driver SQLite đồng bộ, `Connection`, `Transaction`, `Savepoint`; feature `bundled` (https://docs.rs/crate/rusqlite/0.40.2/features) |
| `argon2` † | https://docs.rs/argon2/0.6.0/argon2/ | 0.6.0 | Argon2id mặc định, xuất PHC string; `Params::DEFAULT_M_COST = 19 * 1024`, `DEFAULT_T_COST = 2`, `DEFAULT_P_COST = 1` (https://docs.rs/argon2/0.6.0/src/argon2/params.rs.html); feature mặc định gồm `alloc`, `password-hash`, `getrandom` |
| `password-hash` † | https://docs.rs/password-hash/0.6.1/password_hash/ | 0.6.1 | `try_generate_salt()` — salt Argon2 từ cùng nguồn entropy của OS (§5, §6). `argon2` 0.6.0 kéo `password-hash` 0.6.1 (0.6.0 đã bị yanked) |
| `getrandom` † | https://docs.rs/getrandom/0.4.3/getrandom/ | 0.4.3 | `getrandom::fill` — RNG của hệ điều hành để sinh token 256 bit; lỗi trả `Err`, **không** có fallback "random yếu" (§6) |
| `fs4` † | https://docs.rs/fs4/1.1.0/fs4/ | 1.1.0 | `FileExt::try_lock` — khóa advisory độc quyền file (flock/LockFileEx) cho một instance server trên một file DB (§4, §6.1) |
| `serde_json` † | https://docs.rs/serde_json/1.0.154/serde_json/ | 1.0.154 | Log JSON Lines và output JSON của lệnh `health` |
| `sqlx` | https://docs.rs/sqlx/0.9.0/sqlx/ | 0.9.0 | Phương án B bị loá (PostgreSQL async + tokio) — §3 |

> **Ghi chú về `rand` (sửa lần 2).** Bản `rand` 0.10.3 (2026-09-20) **không còn** `OsRng`: tài liệu
> chính thức của đúng bản đó dùng `rand::rngs::SysRng` (giao diện `TryRng`, lỗi kiểu `SysError`) —
> https://docs.rs/rand/0.10.3/rand/rngs/index.html . Vì vậy ADR này **không** dùng `rand` mà gọi
> `getrandom` 0.4.3 trực tiếp (§6, §10), và ghi rõ xử lý lỗi entropy: lỗi ⇒ từ chối yêu cầu, **không**
> fallback RNG yếu.

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
| Cần `health` từ xa hoặc metrics | Mở endpoint HTTP/metrics riêng bằng ADR mới; V1 giữ admin listener loopback + CLI |
| Cần chạy **nhiều** instance server trên cùng một file DB | Bỏ khóa file, thêm lease phiên theo instance (session lease có hạn) + điều phối; ADR mới |
| Cần phân biệt chính xác "server treo" với "server đã chết" | Thêm `Ping`/`Pong` cho admin port hoặc watchdog ngoài tiến trình; ADR mới |
| Cần giới hạn ai được gọi `health` trên cùng máy | Thêm bearer token admin đọc từ file `0600`; ADR mới |

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
- **Phiên chỉ được giải phóng khi có tiến trình server sống**: nếu server chết và **không** khởi động
  lại, phiên của instance chết vẫn giữ chỗ trong DB cho tới lần boot kế tiếp. Đường ra là **khởi động
  lại server** (lúc boot mọi phiên của instance cũ bị revoke, §6.1) — không cần admin, nhưng vẫn cần
  server chạy. Đây là hệ quả của mô hình "một tiến trình server cho một file DB" (§4).
- **Admin listener không có auth**: mọi người dùng cùng máy đều gọi được `health`. Nội dung chỉ là số
  liệu vận hành (không có bí mật, không có token, không có danh sách tài khoản) và listener chỉ bind
  loopback, nên chấp nhận ở V1; xem §13 nếu cần siết.
- **Admin/health đi trên TCP thuần** (ADR 0006) — chỉ loopback nên rủi ro thấp, nhưng vẫn chưa mã hóa.
- **Chưa làm**: code, migration script, message auth, TLS, endpoint health từ xa, backup tự động theo lịch.

## 15. Việc cần integrator xử lý khi duyệt ADR này

1. Thêm hàng 0011 vào `docs/decisions/README.md` (index ADR thuộc integrator, AGENTS §5) và đổi trạng
   thái ADR này từ `Proposed` sang `Accepted (mặc định có thể đảo)`.
2. Cập nhật dòng "Persistence" ở [ARCHITECTURE §8](../planning/ARCHITECTURE.md#8-quyết-định-còn-mở)
   (đã sửa trong PR này, cần đồng bộ khi review).
3. Cập nhật Q-011 trong [QUESTIONS](../research/QUESTIONS.md) (đã sửa trong PR này, trạng thái
   `proposed-close`; đóng `done` sau khi review + task T1–T7 có bằng chứng).
4. **Addendum ADR 0009** (cần trước T1 và T5): thêm kind message auth (`Auth`/`AuthOk`/`AuthFail`/
   `Logout`) và trường ngón tay cấu hình trong `Welcome`; thêm kind `HealthRequest` (`0x0050`) /
   `HealthResponse` (`0x0051`) cho admin listener (§8.2.1). Thêm kind là **additive**, **không** đổi
   `wire_schema` (ADR 0009 §1).
5. Thêm `rusqlite`, `argon2`, `getrandom`, `fs4`, `serde_json` vào `[workspace.dependencies]` của
   `Cargo.toml` gốc (Cargo.toml gốc thuộc integrator, AGENTS §5). **Không** thêm `rand` (§6, §10).
6. Mở các task triển khai T1–T7 theo §11 (mỗi task một issue, một owner).
