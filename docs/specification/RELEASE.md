# Phát hành V1 — bố cục gói, dựng bundle, yêu cầu hệ thống

Phạm vi: **V1-036 phần 1** (GitHub issue #102) + **V1-036 phần 2** (GitHub issue #119:
license thực tế + font trong gói, kiểm bộ đóng gói) + **V1-036 phần 3** (GitHub issue #167:
smoke cài/chạy gói trên Windows/Linux/macOS trong CI). Nguồn chuẩn về *gói phát hành*; luật
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
| Smoke cài/chạy gói trên 3 OS: **mở gói đã đóng**, chạy binary trong gói, lỗi thiếu pack/rules/l10n báo rõ | **Đã làm** (phần 3, §3.2) |
| Log smoke (version + fingerprint + từng ca) lưu thành artifact CI (**kể cả khi smoke fail**) | **Đã làm** (phần 3, §3.2) |
| Ký số artifact, installer, đo cấu hình máy tối thiểu | **Chưa làm** (ngoài phạm vi issue #102/#167) |
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
| Bước trong job | build release → gom gói (`pack`: `bin/`, `data/`, `assets/fonts/`, README trích từ §8) → `cargo fetch` (nguồn crate **mọi** nền tảng) → sinh `LICENSES` → **kiểm bộ đóng gói** → đóng gói → **mở gói đã đóng + smoke** (§3.2) |
| Artifact | mỗi OS một artifact `boom-rs-<phiên bản>-<os>-<arch>` (zip cho Windows, tar.gz cho Linux/macOS) · một artifact log `…-smoke` (§3.2) |

`workflow_dispatch` trên nhánh để thử: job `package` cần job `check` xanh trước.

Ngoài `release.yml`, `.github/workflows/ci.yml` chạy **mọi PR**:

| Job | Việc |
|---|---|
| `ci` | `cargo xtask ci` + `python tools/check_package.py test` (tự kiểm công cụ đóng gói) trên cả ba OS |
| `smoke` | build debug → `pack` → `smoke` → upload artifact `smoke-log-<os>` (§3.2) |

### Gói QA đã kiểm ngày2026-10-24

Workflow[40069259503](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40069259503) từ
head`28dad286c467a870c3047ff21b26ce82c654f2fc` đã ra ba archive version`qa-20261024-28dad28`.
Mỗi OS đạt10ca smoke từ archive,27hashfile tổng cộng đã đối chiếu sau tải/mở gói; Linux còn kiểm
snapshot2× bằng binary từ archive. [Hash/size/giới hạn](../evidence/measurements/rust-client-ux-2026-10-24/README.md#gói-qa-ba-os-từ-đúng-code-đã-review).
Đây là QA/dev với `release=false`, không tag/Release, chưa thay nghiệm thu V1-036/G6 hoặc minimum hardware.

### Gói QA cho luật hiện hành — task #265

Gói `qa-20261024-28dad28` ở trên là **lịch sử**: profile tại head28dad có trap4000ms, trước PR258
hiệu chuẩn6350ms và PR260 trapped-speed17px/s. Không dùng smoke đó để tuyên bố đã kiểm các luật mới.
Task[#265](https://github.com/aquaplatinumberlitz/boom-rs/issues/265) dựng đúng một workflow QA mới
[40091603701](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40091603701),
version`qa-20261024-641f99a`, từ reviewed head`641f99aa997813eca77ecd1d0ef554b3fd55c272`.
Workflow đã **success4/4job**, tải/mở/check đủ baarchive, đối chiếu27hashfile với log và10smokecase/OS.
Metadata [lịch sử task265](https://github.com/aquaplatinumberlitz/boom-rs/blob/265432d8b5d24df26daea74df8d37855a292a3d8/docs/evidence/measurements/rust-client-ux-2026-10-24/package-current-qa.json)
ghi archive/SHA/fingerprint và profile thực trong gói: trap6350ms, trapped17px/s, flame540ms.
Linux binary từ archive đã mở room2×1600×1200 trên Xvfb/llvmpipe, RGBA khớp ảnh room đã review;
không phải GUIWindows/macOS hoặc minimumhardware. Không tag/Release, mọi giới hạn gate còn nguyên.

### Sửa cửa sổ KWin/X11 — issue270

[Metadata](../evidence/measurements/rust-client-ux-2026-10-24/window-restore.json) ghi baseline đỏ
và bản sửa miniquad EWMH xanh: cửa sổ bình thường trả600×560 sau hai vòng fullscreen; cửa sổ
vốn maximized giữ state/kích thước cũ. RebindF10/F11 và startupfullscreen có đối chứng.
Tại lúc kiểm sửa runtimeLinux, GUIWindows/macOS/WM khác/minimumhardware và sign-offG5 còn thiếu; quyết định G5 về sau ở issue278.
Gói QA40091603701 giữ lịch sử trước patch; gói QA40107097457 sau sửa được kiểm ở issue273 bên dưới.

### Kiểm notice của dependency path sau sửa fullscreen

QA workflow40105903687 tại head2977b0e build/smoke ba OS thành công, nhưng kiểm nội dung độc lập
ở issue273 phát hiện LICENSES bỏ dòng tác giả `@ 2019-2020 Fedor Logachev` của miniquad.
Collector trước đó chỉ nhận `Copyright`/`(c)`/`©`; có tên crate và toàn văn MIT chung chưa chứng minh
notice riêng được giữ. Các archive đó giữ làm bằng chứng trước sửa, chưa đạt acceptance notice273.
Collector bổ sung dạng `@` theo sau bởi năm và giữ nguyên dòng nguồn; regression dùng file
`vendor/miniquad/LICENSE-MIT` thật đã tái hiện đỏ trước sửa/xanh sau sửa. Không đổi source/vendor license,
không coi smoke xanh là chứng minh đầy đủ nội dung LICENSES. Gói sau sửa cần kiểm lại hash/notice riêng.

### Gói QA sau sửa notice/fullscreen — task273

[Workflow40107097457](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40107097457)
version`qa-20261025-95985fe`, reviewed codehead`95985fe941be5199568b012482c8eac88b33629f`,
đã build/check/smoke ba OS thành công. Tải/mở/check đủ ba archive, đối chiếu27hashfile/10smokecaseOS;
LICENSES của từng archive có135crate và notice miniquad thật trong phần bản quyền riêng. Luật/l10n
byte-identical với source, trap6350/trapped17/flame540, vẫnreleasefalse. [Metadata hiện hành](../evidence/measurements/rust-client-ux-2026-10-24/package-current-qa.json)
giữ head/archive/hash/notice và giới hạn; gói401059 trước sửa được giữ làm bằng chứng finding.

Binary Linux lấy từ archive SHA`bc3a1aefd3abde0d4575ff4289e8a71f1fb7a434c9cde1a8a6a47412bcb8e65d`
đã chạy KWin/X11: normal/maximized mỗi loại2vòng fullscreen khôi phục kích thước/state đúng;
rebindF10/F11 không kích hoạt shortcut cửa sổ. Fixture pulse ngắn đầu không thấy fullscreenflag,
giữ raw và chưa xác định nguyên nhân; phương pháp keydown150ms/keyup kiểm focus thật đã đạt.
Không đổi product timeout/điều kiện hoặc lặp CI tới xanh. Không suy GUIWindows/macOS, minimumHW,
physicalaudio hoặc gatepassed. Tại lúc thu QA, V1-036/G2/G3/G5 chưa ký; quyết định G5 về sau ở issue278, không tag/Release.

### Blocker nghiệm thu hiện hành của G2/G3/G5/V1-036

| Phần | Đã có bằng chứng | Còn chặn nghiệm thu / bước cụ thể | Fidelity giữ backlog |
|---|---|---|---|
| G2 movement/stats | Movement/hitbox/assist/time DESIGN có scenario; ownergrace/trapped17; R180/R181 namedrow/consumer và R182 logicalsource12khóa | G2 đã nghiệm thu issue294/296; PR297 chọn DESIGN33 theo nativecurveR183, giữ hitbox13/assist12 và30HzDESIGN. Profile vẫn releasefalse; G2 không nghiệm thu G6/platform/minHW. Loader vẫn reject candidate nếu có. | Nativehurtbox/maxcorner/curve ngoài actor/points4..8, override mọi mode/KRconsumer còn fidelity; không cần toàn bộ6root. |
| G3 bomb/damage/items | Nghiệm thu cơ chế/boundary V1 theo [issue280](https://github.com/aquaplatinumberlitz/boom-rs/issues/280): fuse/chain/flame/trap/cap/drop/clock có nguồn/version và ca biên | Không còn thiếu cơ chế G3 theo SCOPE6 đã review. Việc chốt12giá trị nhân vật/profile phát hành vẫn thuộc G2; không relabel candidate hoặc lấy actorVN làm mọiKR. | Writer/clamp gốc, non-owner/repeatdamage/animationtail và vai trò ItemAttr còn mở; DESIGN đã chấp nhận không gọi là native confirmed. |
| G5 UX/localization | Nghiệm thu functional V1 theo [issue278](https://github.com/aquaplatinumberlitz/boom-rs/issues/278): text/ảnh/font/golden; actual Linux scale/focus/rebind/fullscreen; BGM/SFX/volume/mute đường phát số; reconcile regression | Không còn blocker G5 trong tiêu chí SCOPE6/7.9 đã review. GUI Windows/macOS và minimum hardware chuyển đúng V1-036/G6; không coi smoke CLI là GUI hoặc virtual sink là physical DAC. | Native KR layout/name/voice count/loop, thiết bị âm thanh thật và WM khác chưa được xác nhận; giữ giới hạn, không đổi nhãn native. |
| V1-036/G6 | Archive QA ba OS trước đây đã kiểm; source/luật mới có CI/review | Gói QA mới/fingerprints đã kiểm; còn minimum hardware CPU/RAM/GPU có phép đo và quyết định, GUI platform validation và acceptance profile phát hành; G3/G5 nghiệm thu riêng, V1 vẫn36/37; gate hiện hành xem PROGRESS. | Chỉ số fidelity hoặc sáu issueRE mở không tự là blocker, cũng không tự là nghiệm thu phát hành. |

Profile hiện vẫn `release=false`; ngoại lệ12khóaKRcandidate ở ADR0013 chưa được thay bằng quyết định
phát hành mới. DESIGN đã được review ở từng PR không đồng nghĩa đã sign-off toàn gate. Không thêm
file plan/tổng kết song song; tracking ở execution.json/PROGRESS, câu hỏi native ở QUESTIONS/rootissues.

### Nghiệm thu cơ chế G3 — 2026-10-25

Quyết định tại [issue280](https://github.com/aquaplatinumberlitz/boom-rs/issues/280), theo
[SCOPE_V1 §6](../planning/SCOPE_V1.md). Đây là review cơ chế và ca biên theo lựa chọn luật có nguồn,
không biến regression của implementation thành oracle native.

| Tiêu chí | Claim/profile có nguồn | Nghiệm thu biên đã tích hợp |
|---|---|---|
| Fuse, ray, chain, không nhân đôi drop | R-115 fuse2500 DESIGN; R-033/R-109; profile hiện hành | [chain ba bom](../../fixtures/scenarios/explosion/chain_of_three.toml), [roll một lần](../../fixtures/scenarios/explosion/chain_rolls_once.toml), [gạch chặn](../../fixtures/scenarios/explosion/soft_block_stops_flame.toml) |
| Lửa tính cả tick tạo | R-158 hiệu chuẩn540ms/17tick, không phải native literal | [TTL một tick](../../fixtures/scenarios/explosion/flame_ttl_one_tick0.toml), [hai tick](../../fixtures/scenarios/explosion/flame_ttl_two_ticks.toml), life-time và core current-profile tests |
| Nhốt/chết và di chuyển chậm | R-160/R-162 hiệu chuẩn6350ms/191tick,17px/s | [timeout](../../fixtures/scenarios/life/trap_timeout.toml), [pause warmup](../../fixtures/scenarios/life/trap_timer_pauses_during_warmup.toml), [slow movement](../../fixtures/scenarios/movement/trapped_moves_slowly.toml); core test current profile còn nhốtT+189/chếtT+190 |
| Drop: chance, trọng số, thời điểm hiện | R-112 chance350 DESIGN; số weight3650/2800/2000 đã trích, vai trò native R-052 unknown; áp làm trọng số V1 có chủ ý | [roll không ra](../../fixtures/scenarios/blocks/drop_roll_misses.toml); tests blocks kiểm hit/miss theo profile, trọng số, tỉ lệ trên mẫu lớn và cùng seed |
| Lửa/item và pickup tại trần | R-109 DESIGN; item cũ cháy, item mới rơi còn; item tại cap vẫn mất | [old item/new drop](../../fixtures/scenarios/blocks/flame_burns_old_item_new_drop_lives.toml), [at cap](../../fixtures/scenarios/pickup/at_cap.toml), pickup clamp/no-stack tests |
| Clock/warmup | R-113 warmup3000 DESIGN, không ăn vào thời hạn lượt | [freeze countdown](../../fixtures/scenarios/round/warmup_freezes_round.toml), [không đi/đặt bom](../../fixtures/scenarios/round/warmup_blocks_movement_and_bomb.toml) |

Reviewer độc lập nghiệm thu các cơ chế và lựa chọn DESIGN ở đúng version đã tích hợp. **12 giá trị nhân vật hiện là logicalsource VNconfirmed R182; profile vẫn releasefalse**.
G2 đã nghiệm thu logicalsource và calibration33R184 riêng; G3 không thay nguồn đó thành nativeKR
hoặc nghiệm thu profile phát hành. Cap consume/clamp mechanics không phải xác nhận cap mọi KR; số đo VN chỉ dùng
đúng actor/đại lượng. Không cần đóng toàn bộ sáu root RE để nghiệm thu cơ chế V1, nhưng cũng không
đổi nhãn native/unknown để qua gate. G6/V1-036 còn platform/minimum hardware và quyết định profile.

### 3.1 Công cụ đóng gói (`tools/check_package.py`)

Một script, sáu lệnh (offline, không gọi mạng):

```bash
python3 tools/check_package.py pack --bin-dir target/release --out <gói>   # dựng gói
python3 tools/check_package.py licenses --out <gói>/LICENSES                # sinh LICENSES
python3 tools/check_package.py check <gói>                                 # kiểm bộ đóng gói
python3 tools/check_package.py unpack <archive> --out <thư mục>            # mở gói đã đóng
python3 tools/check_package.py smoke <gói> --log <file>                    # chạy binary trong gói
python3 tools/check_package.py test                                        # tự kiểm script
```

`pack` dựng gói theo đúng bố cục §2: `bin/` (3 binary trong `--bin-dir`), `data/rules/`,
`data/l10n/`, `assets/fonts/` (font OFL) và `README.md` **trích nguyên văn** từ §8 của
tài liệu này. `pack` không kèm `LICENSES` (sinh bằng `licenses`), không kèm
pack/bundle/asset game.

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

Bộ test còn giữ **hợp đồng với workflow** (V1-036 phần 3): JSON của `health --offline`
phải là **object** (array/null/scalar ⇒ báo lỗi), smoke fail vẫn **ghi log**, và bước
upload log trong `ci.yml`/`release.yml` phải chạy **cả khi job fail** (xem §3.2).

### 3.2 Smoke cài/chạy gói trên ba OS (V1-036 phần 3)

Build xanh **không** đồng nghĩa gói chạy được. `smoke` (lệnh `check_package.py smoke`)
chạy **binary thật** lấy từ `bin/` của gói; trong `release.yml` đó là gói **đã giải nén
từ archive đã đóng** (không phải thư mục dựng tạm):

| Ca | Lệnh | Kỳ vọng |
|---|---|---|
| `--help` | `boom-server --help` · `boom-client --help` | exit `0`, in danh sách lệnh |
| Nội dung | `boom-server content --rules <gói>/data/rules/v1-dev.toml --l10n <gói>/data/l10n/vi-v1.toml` | exit `0`, in profile + hash rules + locale chuỗi |
| Vận hành | `boom-server health --offline` | in **JSON là object** có `scope: offline` + fingerprint (`server_version`, `rules_hash`, `content_sha256`); JSON đúng cú pháp nhưng **không phải object** (array/null/scalar) ⇒ **báo lỗi** (không có field nào để kiểm); mã `0` (healthy) hoặc `1` (unhealthy) — gói **không** kèm DB identity, DB do `serve` tạo sau; mã `3` (lỗi cấu hình) là lỗi thật |
| Thiếu rules | `boom-server content --rules <file thiếu>` · `boom-client --rules <file thiếu>` | exit `2`, thông điệp **nêu đường dẫn** và **cách sửa** (`sửa:`) |
| Thiếu chuỗi | `boom-server content --l10n <file thiếu>` | exit `2`, nêu đường dẫn + cách sửa |
| Thiếu bundle (pack) | `boom-server content --bundle <thư mục thiếu>` · `boom-client --bundle <thư mục thiếu>` | exit `2`, nêu thư mục bundle + cách dựng lại từ pack (`boom-assets bundle … --pack-dir …`) |
| Thiếu pack | `boom-assets bundle --map Village00 --out <dir> --pack-dir <thư mục thiếu>` | exit khác `0`, nêu thư mục pack |

Lệnh được chạy trong thư mục tạm và output được giải mã **UTF-8** (không phụ thuộc
encoding console Windows), nên thông điệp tiếng Việt kiểm được giống nhau trên ba OS.

Chạy ở đâu:

- `.github/workflows/ci.yml` — job `smoke`, ma trận 3 OS, chạy **mỗi PR**: build debug →
  `pack` → `smoke` → upload artifact `smoke-log-<os>` (log smoke).
- `.github/workflows/release.yml` — job `package`, sau khi **đóng gói**: `unpack` archive
  thật → `check` (đủ thành phần, không pack/asset game) → `smoke` trên gói đã mở →
  upload artifact `…-smoke` (log smoke).

**Log smoke luôn được upload, kể cả khi smoke fail.** Bước upload dùng điều kiện
`if: ${{ !cancelled() && hashFiles('dist/smoke-log.txt') != '' }}` (điều kiện mặc định của
GitHub là `success()`, nên bước upload sẽ **bị bỏ qua** khi smoke trả mã khác 0 — mất
đúng artifact cần để debug). Job vẫn **đỏ** khi smoke fail: bước smoke không bọc
`continue-on-error`, nên lỗi không bị nuốt. Artifact **gói phát hành** (`Upload artifact`
trong `release.yml`) vẫn theo điều kiện mặc định: gói smoke fail thì không phát hành.

Log smoke (`.txt`) ghi: phiên bản gói, OS, commit git, fingerprint (từ `health --offline`:
`server_version`, `rules_hash`, `content_sha256`, profile rules + hash của **file rules
trong gói**), SHA-256 từng file trong gói, và output + mã thoát của từng ca.

> **Giới hạn:** smoke **không** có pack/bundle thật trong gói (gói cố tình không kèm asset
> game), nên ca "thiếu bundle" chỉ chứng minh đường báo lỗi, chưa chứng minh client đọc
> được bundle hợp lệ. `boom-assets bundle` báo lỗi và nêu thư mục pack nhưng **không** có
> dòng `sửa:` (chỉ client và `boom-server content` có).

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
| Thiếu thư mục pack (dựng bundle) | `<thư mục pack>: lỗi pack: …/FxMap.idx: …` (nêu thư mục pack; **không** có dòng `sửa:`) | `boom-assets bundle`: 1 |
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
| Binary trong gói chạy được trên OS của gói | `boom-server --help`/`content`/`health --offline` và `boom-client --help` chạy từ `bin/` của gói đã mở | smoke 3 OS trong `ci.yml` + `release.yml` (§3.2) |
| Dung lượng gói | Đã đo archive QA ba OS ngày24/10; không phải ước tính minimum disk/RAM | [Metadata QA](../evidence/measurements/rust-client-ux-2026-10-24/package-qa.json) |

### Chưa đo (phải đo trước khi phát hành)

- Cấu hình máy tối thiểu (CPU/RAM/GPU) — câu hỏi mở của V1-036, chưa có phép đo.
- Thời gian build/mở gói trên máy người dùng (build mới đo trên runner CI); dung lượng archive QA đã ghi trong metadata.
- Hiệu năng render/cấu hình tối thiểu trên GPU thật: kiểm vận hành RTX3060 ngày24/10 đã có,
  chưa benchmark FPS/latency hoặc đại diện minimum hardware.
- UX đa OS/WM khác và nghe qua soundcard thật; khôi phục kích thước trên KWin/X11 đã kiểm ở issue270.
  [Hồ sơ UX24/10](../evidence/measurements/rust-client-ux-2026-10-24/README.md) có focus thật,
  integer scale và capture BGM/SFX/volume/mute qua đường phát số; không thay kiểm GUI/hardware còn lại của V1-036/G6; quyết định G5 functional có ở issue278.

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
Bấm `Esc` để thoát, `F10` để đổi tỉ lệ 1×/2×, `F11` để đổi fullscreen/windowed.
Dùng `--scale 1|2` và `--fullscreen|--windowed` để chọn lúc mở. Phím đã rebind cho gameplay
được ưu tiên hơn shortcut; `S` đổi tỉ lệ trong offline khi chưa gán cho gameplay.
Canvas luôn phóng nguyên; màn không đủ2× dùng1× có viền.

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
- Profile rules hiện là `kr00967-v1-dev` (`release = false`): logicalstats R182 dùng nguồn
  VNconfirmed, tốc độ33px/s/point R184 là DESIGN hiệu chuẩn đã review. G1–G5 đã nghiệm thu;
  minimumhardware/platformvalidation/gói cuối/V1-036 vẫn **chưa** đủ điều kiện gate G6
  ([SCOPE_V1](../planning/SCOPE_V1.md) §6).
- Bố cục gói và bước đóng gói là **design** của dự án; định dạng bundle là
  [design](assets/BUNDLES.md) chưa được chốt hoàn toàn (ARCHITECTURE §8).
- Smoke (§3.2) **không** chạy client/server với bundle thật: gói cố tình không kèm
  pack/bundle/asset game, nên ca "thiếu bundle" chỉ chứng minh đường báo lỗi chứ không
  chứng minh client đọc được bundle hợp lệ. Chạy client thật với bundle cần pack KR của
  người dùng (không commit vào repo) — nằm ngoài phạm vi V1-036 phần 3.

## 10. Probe cửa sổ native trên runner — task298

Workflow `release.yml` có input tùy chọn `native_gui=true` (mặc địnhfalse). Chỉ dispatch sau source
được exact-head review/CI theo AGENTS; không push tag/Release. Dùng **binary trong archive đã mở**,
không build thêm bản thử. `tools/gui_probe.py` chạy UI demo không bundle/pack/media ở1x/2x, tối đa
25s mỗi launch, không retry. Sourcehead/version cùng SHA binary/rules/l10n, exit/stderr, PNGsize/SHA
vào `*-gui` JSON artifact; ảnh chỉ ở temp và xóa, không upload ảnh/gameasset. SchemaPNG/CRC/image
stream/size và nonuniformRGB sau giải filter phải hợp lệ, exit0; process lỗi/timeout/khácsize giữFAIL với nguyên nhân chưa phân loại.
Không biến failcontext thành productregression hoặc giả định runner không hỗ trợ trước khi đo.

Linux dùng Xvfb1920×1440 và Mesa software cho cửa sổ X11; không phải minimumGPU/realdisplay.
Windows thử nativeWGL backend hiện có (miniquad thử3.1 rồi2.1, cần context extensions);
macOS dùng backend hiện hành/WindowServer; capability hosted runner được quan sát, không mặc định.
Tool không provisiondriver/DLL hoặc thay đổi hostfocus/audio/security. JobGUI lỗi vẫn giữ archive và
smokelog đã upload trước đó; JSON được upload cảkhi lỗi, workflow có thể FAIL dù packageCLI tốt.
Chỉ kết quả cả2capture PASS mới chứng minh framebuffer kích thước yêu cầu trên đúng runner.

Đây không OSfocus/keyrelease/fullscreen/physicalaudio/FPS/minimumHW hoặc toànG6; framebuffer2x
cũng chưa chứng minh exactnearest pixels. Các proofLinuxKWin/realfocus và những hạn chế đãreview
vẫn giữ riêng. Fullscreen/focus đaOS cần ca kiểm riêng khi đã biết nativecontext hoạt động. Không
coi smoke `--help`, simulationFocusLost hoặc CI xanh là cửa sổ GUI thật. GóiQA95985fe trướcR182/R184
là lịch sử; finalpackage phải dùng stablehead và rules33 cùngversion/hash mới.

### 10.1 Kết quả package33 và ưu tiên Linux hiện hành

[QA40193995532](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40193995532) đã kiểm
3archive/27filehash/30CLIcases và profile33/17/6350/540; Linux cửa sổ800×600/1600×1200 có
PNGnonuniform hợp lệ. macOSpixel-formatnull và WindowsWGL_ARB_pixel_format FAIL giữ riêng,
workflowFAIL không bị giấu. [Metadata/exits/giớihạn](../evidence/measurements/rust-client-ux-2026-10-24/package-current-qa.json).

Theo chủ dự án25/10, **ưu tiên Linux, Windows/macOS đểsau**. Không rerun haiOS hoặc sửaApple
backend làm điều kiện tiếp tụcLinux; task300 được giữ deferred với patch chưamerge. Đối vớiLinux,
chạy/package/runtime và minimumhardware/performance cần đo riêng; Xvfbsoftware/capturePNG không
là GPUminimum/FPS/physicalaudio. ALSAbackgroundthreadpanic trong runner cũng giữmetadata, không
gọi audioPASS. G6/phát hànhfull/V1-036 theo acceptance riêng, không claim37/37 từ ưu tiênLinux.
