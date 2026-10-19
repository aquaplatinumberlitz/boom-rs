# Phát hành V1 — bố cục gói, dựng bundle, yêu cầu hệ thống

Phạm vi: **V1-036 phần 1** (GitHub issue #102). Nguồn chuẩn về *gói phát hành*; luật chơi vẫn ở
[RULES_V1](gameplay/RULES_V1.md), bố cục engine ở [ARCHITECTURE](../planning/ARCHITECTURE.md),
điều kiện nghiệm thu ở [SCOPE_V1](../planning/SCOPE_V1.md) §6 (gate G6).

> **Phần này chỉ mô tả cách đóng gói và chạy.** Build xanh **không** đồng nghĩa V1 xong:
> gate G6 đòi cả 37 mục V1 đạt acceptance, còn đây mới là phần build 3 OS.

## 1. Đã làm và chưa làm

| Việc | Trạng thái |
|---|---|
| Build `--release` `boom-client`, `boom-server`, `boom-assets` trên Windows/Linux/macOS trong CI | **Đã làm** (workflow `release.yml`) |
| Bố cục gói: `bin/`, `data/rules/`, `data/l10n/`, README chạy nhanh, `LICENSES` | **Đã làm** |
| Không kèm pack/bundle/asset game trong gói | **Đã làm** (người dùng tự dựng bundle) |
| Báo lỗi rõ (đường dẫn + cách sửa) khi thiếu file rules/bundle/l10n hoặc sai hash | **Đã làm** (client + `boom-server content`) |
| Ký số artifact, installer, đo cấu hình máy tối thiểu | **Chưa làm** (ngoài phạm vi issue #102) |

## 2. Bố cục gói phát hành

```text
boom-rs-<phiên bản>-<os>-<arch>/
├── bin/
│   ├── boom-client[.exe]      # client V1 (macroquad, ADR 0010)
│   ├── boom-server[.exe]      # server V1
│   └── boom-assets[.exe]      # công cụ đọc pack KR và dựng bundle
├── data/
│   ├── rules/v1-dev.toml      # rules profile (giá trị luật + claim)
│   └── l10n/vi-v1.toml        # chuỗi UI V1 đã Việt hóa
├── README.md                  # README chạy nhanh (§8)
└── LICENSES                   # crate phụ thuộc + license SPDX
```

Quy tắc:

- **Không** có pack/bundle/atlas/ảnh game trong gói. Người dùng giữ pack của mình và dựng
  bundle bằng `boom-assets` (§4) — repo `boom-rs` cũng không commit asset game.
- `data/rules/` và `data/l10n/` là **file dữ liệu** của bản phát hành: client/server nạp
  từ đĩa khi được trỏ bằng `--rules`/`--l10n`; nếu không trỏ thì dùng bản nhúng trong binary.
- `LICENSES` liệt kê **toàn bộ đồ thị phụ thuộc** (mọi nền tảng), sinh từ `cargo metadata`
  lúc build: tên · phiên bản · license (SPDX) · kho mã nguồn. Đây là bảng SPDX + kho
  mã nguồn, **không** phải toàn văn license.
- `README.md` trong gói được trích nguyên văn từ §8 của tài liệu này (một nguồn chuẩn).

## 3. Workflow CI phát hành

`.github/workflows/release.yml`:

| Mục | Giá trị |
|---|---|
| Chạy tay | `workflow_dispatch` (ô `version`) |
| Chạy tự động | đẩy tag `v*` (phiên bản lấy từ tên tag, bỏ tiền tố `v`) |
| Job `check` | `cargo xtask ci` trên ubuntu (cổng bàn giao [AGENTS §7](../../AGENTS.md)) |
| Job `package` | ma trận 3 OS: `ubuntu-latest` (x86_64), `windows-latest` (x86_64), `macos-latest` (aarch64) |
| Artifact | mỗi OS một artifact `boom-rs-<phiên bản>-<os>-<arch>` (zip cho Windows, tar.gz cho Linux/macOS) |
| Kiểm trong job | `boom-server --help`, `boom-client --help` (exit 0) · `boom-server content` với file trong gói (exit 0) · thiếu file rules → exit khác 0 và thông điệp có đường dẫn |

`workflow_dispatch` trên nhánh để thử: job `package` cần job `check` xanh trước.

## 4. Dựng bundle từ pack của bạn

Client **không** đọc pack: client chỉ tiêu thụ bundle đã validate
([BUNDLES](assets/BUNDLES.md), [ARCHITECTURE](../planning/ARCHITECTURE.md) §7). Dựng bundle:

```bash
bin/boom-assets bundle --map Village00 --out ./bundle --pack-dir /path/to/pack
```

- `--pack-dir` trỏ tới thư mục pack KR đã giải nén; có thể bỏ và đặt biến môi trường
  `BOOM_KR_PACK_DIR` trỏ tới cùng thư mục.
- Lệnh in `content_hash` của bundle: đó là định danh nội dung mà client kiểm khi nạp.
- Thiếu pack hoặc pack sai → lệnh in thư mục đã dùng và **thoát với mã khác 0**; không dựng
  được bundle nửa vời.

## 5. Chạy client và server

```bash
# Kiểm bộ nội dung của gói trước khi chơi (exit 0 = dùng được)
bin/boom-server content --rules data/rules/v1-dev.toml --l10n data/l10n/vi-v1.toml

# Client: dùng rules + chuỗi trong gói, scene từ bundle bạn tự dựng
bin/boom-client --rules data/rules/v1-dev.toml --l10n data/l10n/vi-v1.toml --bundle ./bundle

# Server
bin/boom-server serve
bin/boom-server health
```

Bỏ `--bundle` thì client chạy chế độ offline với địa hình placeholder (không cần pack).

## 6. Lỗi khi thiếu file hoặc sai hash

Cả hai binary đều **không panic**: lỗi in ra **đường dẫn** và **cách sửa**, rồi thoát với
mã khác 0.

| Tình huống | Thông điệp (rút gọn) | Mã thoát |
|---|---|---|
| Thiếu file rules | `không đọc được file rules: <path> …` + `sửa: …` | client: 2 · `boom-server content`: 2 |
| Rules sai cú pháp/schema | `file rules không dùng được: <path> …` + `sửa: …` | 2 |
| Thiếu file chuỗi | `không đọc được file chuỗi: <path> …` + `sửa: …` | 2 |
| Thiếu bundle / metadata / atlas | `không nạp được bundle: <dir>` + `sửa: dựng lại bundle …` | 2 |
| Sai hash nội dung bundle | `content_hash không khớp: metadata ghi <a>, nội dung tính <b>` | 2 |
| Sai hash file atlas | `file <file> không khớp SHA-256: …` | 2 |
| Sai cú pháp dòng lệnh | `tham số … không hỗ trợ` | 3 |

Mã thoát của `boom-server`: `0` thành công · `2` không dùng được (file nội dung thiếu/sai) ·
`3` sai cú pháp hoặc lỗi cấu hình.

## 7. Yêu cầu hệ thống

### Đã biết

| Mục | Yêu cầu | Nguồn |
|---|---|---|
| Nền tảng | Windows x86_64, Linux x86_64, macOS aarch64 | build 3 OS trong `release.yml` |
| Toolchain build | Rust theo `rust-toolchain.toml` (edition 2024) | [ARCHITECTURE](../planning/ARCHITECTURE.md) §3 |
| Client (Linux) | thư viện hệ thống cho macroquad: `libX11`, `libXi`, `libGL`, `libasound` | [ADR 0010](../decisions/0010-client-stack-choice.md), cùng danh sách CI dùng |
| Client (Windows/macOS) | không cần thư viện hệ thống ngoài | build release.yml |
| Server | SQLite `bundled` ⇒ không cần cài libsqlite3 | [ADR 0011](../decisions/0011-identity-persistence-log-v1.md) §10 |
| Dung lượng gói | chưa đo (xem phần chưa đo) | — |

### Chưa đo (phải đo trước khi phát hành)

- Cấu hình máy tối thiểu (CPU/RAM/GPU) — câu hỏi mở của V1-036, chưa có phép đo.
- Kích thước gói và thời gian build trên máy người dùng (chỉ đo trên runner CI).
- Hiệu năng render trên GPU thật; [ADR 0010](../decisions/0010-client-stack-choice.md) §4.5
  ghi "chưa đo GPU thật", mọi phép đo trước đó chạy trên llvmpipe.
- Input-held/focus và phát âm thanh trên soundcard thật (cũng chưa đo theo ADR 0010).

## 8. README chạy nhanh (đóng gói vào gói)

Phần dưới nằm giữa hai marker `QUICKSTART`; workflow `release.yml` **trích nguyên văn** vào
`README.md` của gói. Sửa ở đây thì gói đổi theo — không sửa trực tiếp trong workflow.

<!-- QUICKSTART:START -->
# boom-rs — V1 (phần 1)

Gói phát hành gồm binary (`bin/`), file luật (`data/rules/`), file chuỗi (`data/l10n/`) và
giấy phép crate phụ thuộc (`LICENSES`). **Không** kèm pack/bundle/asset game: bạn tự dựng
bundle từ pack của mình.

## 1. Kiểm tra bộ nội dung

```bash
bin/boom-server content --rules data/rules/v1-dev.toml --l10n data/l10n/vi-v1.toml
```

Mã thoát `0` = nội dung dùng được. Mã `2` = thiếu/sai file; thông điệp in ra **đường dẫn**
và **cách sửa**.

## 2. Dựng bundle (cần pack KR của bạn)

```bash
bin/boom-assets bundle --map Village00 --out ./bundle --pack-dir /path/to/pack
```

`--pack-dir` có thể bỏ nếu đặt biến `BOOM_KR_PACK_DIR` trỏ tới thư mục pack. Lệnh in
`content_hash` — đó là định danh nội dung client kiểm khi nạp.

## 3. Chạy client

```bash
bin/boom-client --rules data/rules/v1-dev.toml --l10n data/l10n/vi-v1.toml --bundle ./bundle
```

Bỏ `--bundle` để chạy chế độ offline với địa hình placeholder (không cần pack).
Bấm `Esc` để thoát, `S` để đổi tỉ lệ 1×/2×.

## 4. Chạy server

```bash
bin/boom-server serve     # chạy server (admin listener + heartbeat)
bin/boom-server health    # kiểm tra runtime
bin/boom-server --help    # danh sách lệnh
```

## 5. Yêu cầu hệ thống

- Windows x86_64, Linux x86_64, macOS aarch64.
- Linux: cần thư viện hệ thống cho client (`libX11`, `libXi`, `libGL`, `libasound`).
- Server không cần cài SQLite riêng (bản `bundled`).
- Cấu hình máy tối thiểu **chưa đo** — xem `docs/specification/RELEASE.md` §7.

## 6. Lỗi thường gặp

| Thông điệp | Cách sửa |
|---|---|
| `không đọc được file rules: <path>` | kiểm tra file trong `data/rules/`; trỏ `--rules` đúng file |
| `không đọc được file chuỗi: <path>` | kiểm tra file trong `data/l10n/`; trỏ `--l10n` đúng file |
| `không nạp được bundle: <dir>` | dựng lại bundle bằng `boom-assets bundle` (mục 2) |
| `content_hash không khớp` | bundle bị sửa/thiếu atlas; dựng lại bundle |
<!-- QUICKSTART:END -->

## 9. Giới hạn của phần này

- Chưa ký số artifact; người dùng phải tự kiểm nguồn gói.
- Chưa có installer; gói là thư mục giải nén được, chạy bằng binary trong `bin/`.
- `LICENSES` là bảng SPDX + kho mã nguồn, không phải toàn văn license.
- Profile rules trong gói là `kr00967-v1-dev` (`release = false`): còn giá trị
  `candidate`/`design` chưa review, nên **chưa** đủ điều kiện gate G6
  ([SCOPE_V1](../planning/SCOPE_V1.md) §6).
- Bố cục gói và bước đóng gói là **design** của dự án; định dạng bundle là
  [design](assets/BUNDLES.md) chưa được chốt hoàn toàn (ARCHITECTURE §8).
