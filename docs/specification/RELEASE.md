# Phát hành V1 — bố cục gói, dựng bundle, yêu cầu hệ thống

Phạm vi: **V1-036 phần 1** (GitHub issue #102) + **V1-036 phần 2** (GitHub issue #119:
license thực tế + font trong gói, kiểm bộ đóng gói). Nguồn chuẩn về *gói phát hành*; luật
chơi vẫn ở [RULES_V1](gameplay/RULES_V1.md), bố cục engine ở
[ARCHITECTURE](../planning/ARCHITECTURE.md), điều kiện nghiệm thu ở
[SCOPE_V1](../planning/SCOPE_V1.md) §6 (gate G6).

> **Phần này chỉ mô tả cách đóng gói và chạy.** Build xanh **không** đồng nghĩa V1 xong:
> gate G6 đòi cả 37 mục V1 đạt acceptance, còn đây mới là phần build 3 OS.

## 1. Đã làm và chưa làm

| Việc | Trạng thái |
|---|---|
| Build `--release` `boom-client`, `boom-server`, `boom-assets` trên Windows/Linux/macOS trong CI | **Đã làm** (workflow `release.yml`) |
| Bố cục gói: `bin/`, `data/rules/`, `data/l10n/`, README chạy nhanh, `LICENSES` | **Đã làm** |
| `LICENSES` có **toàn văn** mỗi license SPDX dùng trong đồ thị phụ thuộc + bản quyền từng crate | **Đã làm** (phần 2, §2) |
| Font OFL (`assets/fonts/`) kèm gói, kèm `OFL.txt` | **Đã làm** (phần 2, §2) |
| Script kiểm bộ đóng gói (chặn pack/bundle/asset game, kiểm thành phần bắt buộc) | **Đã làm** (phần 2, §3) |
| Không kèm pack/bundle/asset game trong gói | **Đã làm** (người dùng tự dựng bundle) |
| Báo lỗi rõ (đường dẫn + cách sửa) khi thiếu file rules/bundle/l10n hoặc sai hash | **Đã làm** (client + `boom-server content`) |
| Ký số artifact, installer, đo cấu hình máy tối thiểu | **Chưa làm** (ngoài phạm vi issue #102) |
| License cho **chính crate của dự án** (`boom-*` chưa khai `license` trong `Cargo.toml`) | **Chưa làm** (cần integrator chọn, §9) |

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
├── assets/
│   └── fonts/                 # font OFL dùng cho chữ Việt + OFL.txt (không phải asset game)
├── README.md                  # README chạy nhanh (§8)
└── LICENSES                   # toàn văn license SPDX + bản quyền từng crate
```

Quy tắc:

- **Không** có pack/bundle/atlas/ảnh game trong gói. Người dùng giữ pack của mình và dựng
  bundle bằng `boom-assets` (§4) — repo `boom-rs` cũng không commit asset game.
- `data/rules/` và `data/l10n/` là **file dữ liệu** của bản phát hành: client/server nạp
  từ đĩa khi được trỏ bằng `--rules`/`--l10n`; nếu không trỏ thì dùng bản nhúng trong binary.
- `assets/fonts/` là **font mở** (SIL OFL 1.1, [assets/fonts](../../assets/fonts/README.md)),
  **không** phải asset game: client nhúng font bằng `include_bytes!` nên không cần file lúc
  chạy, nhưng gói phân phối font thì phải kèm `OFL.txt` (đã ở `assets/fonts/OFL.txt`).
- `LICENSES` gồm 4 phần (sinh bởi `tools/check_package.py licenses`):
  1. **Toàn văn** mỗi license SPDX dùng trong đồ thị phụ thuộc (MIT, Apache-2.0, …),
     lấy nguyên văn từ file `LICENSE*`/`COPYING*` trong nguồn crate của cargo registry
     lúc build (không tải mạng, không tự chép văn bản license); file nguồn phải khớp
     **mẫu toàn văn chuẩn** của license đó ở mọi điều khoản (không chỉ tin tên file);
  2. **Bảng crate**: tên · phiên bản · license (SPDX) · license dùng khi phân phối ·
     kho mã nguồn;
  3. **Bản quyền từng crate**: file license crate đó kèm trong nguồn registry + các
     dòng bản quyền trích từ đó (kể cả dòng **không ghi năm**, vd `Copyright (c)
     Microsoft Corporation.`);
  4. **Ghi chú**: crate không kèm file license, biểu thức license dạng cũ `A/B`, vế OR
     được chọn nhánh nào, license có trong biểu thức nhưng không kèm toàn văn.
- Với biểu thức nhiều nhánh (`MIT OR Apache-2.0`), gói phân phối theo **một** nhánh có
  toàn văn trong `LICENSES` (được phép theo điều khoản OR). Biểu thức có `AND` thì mọi
  nhánh đều bắt buộc: thiếu toàn văn thì công cụ **báo lỗi và không sinh gói**.
- `README.md` trong gói được trích nguyên văn từ §8 của tài liệu này (một nguồn chuẩn).

## 3. Workflow CI phát hành

`.github/workflows/release.yml`:

| Mục | Giá trị |
|---|---|
| Chạy tay | `workflow_dispatch` (ô `version`) |
| Chạy tự động | đẩy tag `v*` (phiên bản lấy từ tên tag, bỏ tiền tố `v`) |
| Job `check` | `cargo xtask ci` trên ubuntu (cổng bàn giao [AGENTS §7](../../AGENTS.md)) · `python tools/check_package.py test` (tự kiểm công cụ đóng gói) |
| Job `package` | ma trận 3 OS: `ubuntu-latest` (x86_64), `windows-latest` (x86_64), `macos-latest` (aarch64) |
| Bước trong job | build release → gom gói (`bin/`, `data/`, `assets/fonts/`, README trích từ §8) → `cargo fetch` (nguồn crate **mọi** nền tảng) → sinh `LICENSES` → **kiểm bộ đóng gói** → kiểm binary → đóng gói |
| Artifact | mỗi OS một artifact `boom-rs-<phiên bản>-<os>-<arch>` (zip cho Windows, tar.gz cho Linux/macOS) |
| Kiểm trong job | `boom-server --help`, `boom-client --help` (exit 0) · `boom-server content` với file trong gói (exit 0) · thiếu file rules → exit khác 0 và thông điệp có đường dẫn |

`workflow_dispatch` trên nhánh để thử: job `package` cần job `check` xanh trước.

### 3.1 Công cụ đóng gói (`tools/check_package.py`)

Một script, ba lệnh (offline, không gọi mạng):

```bash
python3 tools/check_package.py licenses --out <gói>/LICENSES   # sinh LICENSES
python3 tools/check_package.py check <gói>                      # kiểm bộ đóng gói
python3 tools/check_package.py test                             # tự kiểm script
```

`check` kiểm:

- **thành phần bắt buộc**: `bin/` (đủ `boom-client`, `boom-server`, `boom-assets`),
  `data/rules/*.toml`, `data/l10n/*.toml`, `assets/fonts/` (kèm `OFL.txt`), `README.md`,
  `LICENSES` (đủ 4 phần ở §2);
- **không có pack/bundle/asset game**: chặn thư mục `bundle/`, `bundles/`, `atlas/`,
  `atlases/`, `pack/`, `packs/`, `sprites/`, `textures/`, `tilesets/`; file `bundle.toml`;
  và các đuôi `.map` `.idx` `.idd` `.pak` `.bin` `.atlas` `.png` `.jpg` `.jpeg` `.bmp` `.gif`
  `.webp` `.tga` `.wav` `.ogg` `.mp3` `.flac` `.opus`.
  Font `.ttf` **không** nằm trong danh sách cấm (OFL, được kèm `OFL.txt`).

`licenses` kiểm nguồn trước khi sinh `LICENSES`:

- **xác minh toàn văn theo nội dung**: một file chỉ là toàn văn license khi nội dung khớp
  **mẫu toàn văn chuẩn** của license đó ở **mọi điều khoản** (đối chiếu từng câu, xem
  `LICENSE_TEMPLATES` trong `tools/check_package.py`; mẫu nằm ở
  `tools/fixtures/licenses/`); so khớp bỏ qua khác biệt xuống dòng, dấu câu, hoa/thường.
  Tên file chỉ dùng để **đối chiếu**, không đủ để kết luận.
  Đối chiếu là **nghiêm, không dùng ngưỡng mờ**: mỗi điều khoản của mẫu là một cụm từ
  **liên tiếp** phải có **trọn vẹn** trong nội dung crate. Vì vậy:
  - file **mất một điều khoản** (vd MIT bỏ câu `shall be included in all copies or
    substantial portions of the Software.`) → **không** còn là toàn văn;
  - file **mất một phần** của điều khoản (vd MIT bỏ `or substantial portions`, còn
    `shall be included in all copies of the Software.`) → **không** còn là toàn văn;
  - file có điều khoản **viết lại** (vd `some copies` thay `all copies or substantial
    portions`) → **không** còn là toàn văn;
  - file chỉ có **tiêu đề mục** (vd Apache-2.0 chỉ có `1. Definitions.` …) → **không**
    phải toàn văn;
  - chỉ **hai khác biệt được phép**, đều phải **liệt kê rõ** trong mẫu:
    (1) **dòng bản quyền** và phần đầu (tên, phiên bản, URL — nằm ngoài phần điều khoản,
    mỗi crate ghi riêng); (2) **tên chủ thể** — cụm từ chung trong điều khoản
    (`the copyright holder`) được crate thay bằng tên thật (vd `Xiph.org Foundation` của
    crate `ogg 0.7.1`); điều khoản viết lại (vd MIT của crate `bytemuck 1.25.2` thêm
    `(including the next paragraph)`) phải khai báo tường minh trong `ClauseVariant`.
    Khác biệt chưa liệt kê ⇒ file **không** được tính là toàn văn và công cụ **báo
    lỗi**;
  - nội dung **thừa** xen giữa các điều khoản vẫn được phép (vd phần `APPENDIX` của
    Apache-2.0, tiêu đề riêng của crate, một file chứa nhiều license): công cụ xác
    minh là file có **đủ** điều khoản, không kiểm tra phần thừa có gây hại không;
- **bác bỏ file không xác minh được**: file rỗng, file bị cắt cụt, file tên khai
  `LICENSE-MIT` nhưng chứa license khác → công cụ in lỗi (nêu crate + tên file) và
  **không sinh gói**; không dùng văn bản tự chép để lấp chỗ trống;
- **giữ dòng bản quyền không ghi năm**: `Copyright (c) Microsoft Corporation.` (của
  `windows-link`/`windows-sys`) vẫn vào PHẦN 3; câu văn trong điều khoản license
  (vd `copyright notice that is included in or attached to the work`) không bị lấy nhầm.

Tự kiểm (`test`) dựng gói mẫu rồi kiểm: gói hợp lệ → đạt; thêm file cấm (atlas PNG,
`.map`, `bundle.toml`) → **bắt được**, nêu đúng đường dẫn; thiếu `LICENSES`/`OFL.txt`/
binary → báo thiếu; toàn văn license đủ điều khoản → nhận đúng; file rỗng/cụt/sai nhãn →
**bị bác bỏ** và báo lỗi; dòng bản quyền không ghi năm → còn trong `LICENSES`. Toàn văn
license dùng làm **mẫu đối chiếu** và dữ liệu vào nằm ở `tools/fixtures/licenses/`
(nguyên văn, có SHA-256 + nguồn trong README của thư mục).

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

Gói phát hành gồm binary (`bin/`), file luật (`data/rules/`), file chuỗi (`data/l10n/`), font
chữ Việt (`assets/fonts/`, kèm `OFL.txt`) và giấy phép crate phụ thuộc (`LICENSES`: toàn văn
license SPDX + bản quyền từng crate). **Không** kèm pack/bundle/asset game: bạn tự dựng
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
- `LICENSES` có **toàn văn** license SPDX của crate phụ thuộc, nhưng toàn văn được lấy
  từ file license của crate nguồn lúc build (không tải mạng): phần đầu/bản quyền có thể
  khác câu chữ so với bản SPDX chuẩn. File nguồn được xác minh bằng cách khớp **mẫu
  toàn văn chuẩn** ở mọi điều khoản (không chỉ tin tên file, không chỉ khớp câu đầu hay
  tiêu đề mục) — đối chiếu **nghiêm**, mỗi điều khoản phải có trọn vẹn, không có ngưỡng
  mờ; chỉ dòng bản quyền và tên chủ thể được phép khác, và phải **liệt kê rõ** trong
  `LICENSE_TEMPLATES` (`slots`/`ClauseVariant`) — crate mới có câu chữ khác thì công cụ
  báo lỗi và không sinh gói. Crate **của chính dự án** (`boom-*`) chưa khai `license` trong
  `Cargo.toml` nên không có trong `LICENSES`; chọn license cho dự án là việc của
  integrator, chưa làm ở V1-036 phần 2.
- Với biểu thức license nhiều nhánh (`OR`), gói phân phối theo một nhánh; nhánh khác (vd
  `LGPL-2.1-or-later` của `r-efi`) không kèm toàn văn trong gói — ghi rõ ở PHẦN 4 của
  `LICENSES`.
- `assets/fonts/` là font OFL do client nhúng sẵn; kèm trong gói để phân phối font hợp lệ
  và để người dùng dùng lại cho giao diện riêng.
- Profile rules trong gói là `kr00967-v1-dev` (`release = false`): còn giá trị
  `candidate`/`design` chưa review, nên **chưa** đủ điều kiện gate G6
  ([SCOPE_V1](../planning/SCOPE_V1.md) §6).
- Bố cục gói và bước đóng gói là **design** của dự án; định dạng bundle là
  [design](assets/BUNDLES.md) chưa được chốt hoàn toàn (ARCHITECTURE §8).
