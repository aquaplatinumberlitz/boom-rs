# boom-client — V1-033 + UI V1 (V1-032, V1-035)

Client V1 trên macroquad ([ADR 0010](../../docs/decisions/0010-client-stack-choice.md)), phạm vi task
#47: input/scale/focus và scene offline chạy `boom-core` trực tiếp (chưa mạng). Không đọc
pack/`boom-assets`; không có `--bundle` thì địa hình và chỉ số chỉ là **placeholder**.

Ngoài ra client có **chế độ demo UI** (`--ui-demo`): chạy trọn luồng màn hình V1 —
đăng nhập → sảnh → phòng chờ → trận → kết quả → phòng — với nguồn dữ liệu **giả lập**,
không cần server (xem [UI luồng V1](#ui-luồng-v1-v1-032--v1-035)).

## Chạy

```sh
cargo run -p boom-client
cargo run -p boom-client -- --scale 1
cargo run -p boom-client -- --rebind up=W --rebind bomb=J
cargo run -p boom-client -- --bundle /tmp/bundle-village00
```

Mặc định: mũi tên di chuyển, `Space` đặt bom, `S` đổi 1×/2×, `Esc` thoát. `F2` giả lập mất focus
để kiểm nhánh nhả phím. Runtime dùng `InputAdapter` đăng ký subscriber event của macroquad
(`key_down_event`/`key_up_event`/`WindowMinimized`) và đưa event theo **đúng thứ tự** vào tracker;
không suy trạng thái phím từ ảnh chụp cuối frame.

## Input

- Chính sách **R-080** trong [RULES_V1](../../docs/specification/gameplay/RULES_V1.md): hướng =
  **phím hướng được nhấn gần nhất theo thứ tự event** trong các phím còn giữ; bỏ autorepeat;
  release rồi press lại giữa hai frame vẫn tạo cạnh bom mới; mất focus xóa toàn bộ phím đang giữ
  và cạnh chờ. Đây là `design` của dự án, không phải claim `confirmed` KR/VN.
- Event đi qua `InputAdapter` theo thứ tự backend; test dùng đúng adapter/queue này, không chỉ gọi
  `key_down`/`key_up` thủ công.
- Đặt bom là **cạnh nhấn**, không lặp khi giữ phím.
- Rebind hoán đổi nếu phím đã thuộc action khác, để không có hai action cùng một phím.

## Options (`options.toml`)

Tùy chọn người dùng lưu **phía client** ([ADR 0011 §7](../../docs/decisions/0011-identity-persistence-log-v1.md)):
server không biết tùy chọn này và không có bảng DB cho nó. Đây là **design** của dự án, không phải
claim luật chơi nên không sinh mã `R-xxx`; keybind chỉ là ánh xạ hành động → phím, policy thứ tự
phím vẫn là `R-080`.

### Đường dẫn

Thứ tự ưu tiên (cao → thấp):

1. `--options PATH` — đường dẫn tường minh của dòng lệnh.
2. Biến môi trường `BOOM_CLIENT_OPTIONS_PATH` — ghi đè đường dẫn theo OS (chạy thử tay, tách
   profile).
3. Quy ước cấu hình của từng OS, trong thư mục con `boom-rs`:

| OS | Đường dẫn |
|---|---|
| Linux | `$XDG_CONFIG_HOME/boom-rs/options.toml` (mặc định `$HOME/.config/boom-rs/options.toml`) |
| macOS | `$HOME/Library/Application Support/boom-rs/options.toml` |
| Windows | `%APPDATA%\boom-rs\options.toml` |

### Định dạng (schema 1)

```toml
schema_version = 1
master_volume = 80
bgm_volume = 70
sfx_volume = 100
mute = false

[keybinds]
up = "Up"
down = "Down"
left = "Left"
right = "Right"
bomb = "Space"
```

- `schema_version` bắt buộc và phải bằng `1`; mỗi kênh `volume` trong miền 0–100.
- `[keybinds]` là ánh xạ **1–1** hành động → phím: không hai hành động nào dùng chung một phím.
  Bảng **hoán đổi/hoán vị** hợp lệ (đúng thứ `--rebind` ghi ra) được giữ nguyên; hành động bị
  trùng phím thì giữ phím mặc định kèm cảnh báo.

### Khi file hỏng

- Lỗi ở **riêng trường** (thiếu, sai kiểu, ngoài miền) → **chỉ trường đó** về mặc định, file giữ
  nguyên, có cảnh báo.
- File **hỏng** (không phải UTF-8/TOML) hoặc **sai schema** (`schema_version` thiếu/sai) → toàn bộ
  dùng mặc định; file cũ được **đổi tên** thành `options.toml.broken` (bản `.broken` cũ bị **ghi
  đè** — chỉ giữ bản hỏng gần nhất để chẩn đoán) rồi client **ghi lại** file bằng giá trị mặc
  định (ghi atomic: file tạm rồi đổi tên). Lỗi đổi tên/ghi chỉ ra cảnh báo, **không** làm client
  crash; nếu không đổi tên được thì file cũ giữ nguyên.

### Cờ dòng lệnh

`--volume/--bgm-volume/--sfx-volume N` (0–100), `--mute/--no-mute`, `--rebind ACTION=KEY`,
`--options PATH`, `--save-options` (ghi lại tùy chọn hiện tại rồi thoát). Cờ dòng lệnh **đè** lên
giá trị trong file (đổi sau, thắng trước).

## Scene offline

Tick cố định từ `Rules::ticks_per_second`, tích lũy theo thời gian render (tối đa 8 tick/frame).
Mỗi tick dựng `PlayerInput` từ input hiện tại cho player 0, player 1 neutral, gọi
`SimulationState::step`. Địa hình placeholder 15×13 vẽ theo `CellKind`; chưa có animation, audio,
mạng hay prediction.

Snapshot xác định (Linux headless):

```sh
./crates/boom-client/run-snapshot.sh local/boom-client-snapshot.png
# tương đương:
cargo run -p boom-client -- --snapshot local/boom-client-snapshot.png --ticks 60 --scale 2
```

Script tự bật Xvfb + Mesa llvmpipe; output nằm trong `local/` (gitignored), không commit asset.

## Scene từ bundle (V1-030)

`--bundle <dir>` nạp bundle content của `boom-assets bundle` (định dạng ở
[BUNDLES.md](../../docs/specification/assets/BUNDLES.md)):

```sh
BOOM_KR_PACK_DIR=/home/admin/boomkr-full \
  cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/bundle-village00
cargo run -p boom-client -- --bundle /tmp/bundle-village00 \
  --snapshot local/bundle-village00-2x.png --ticks 60 --scale 2
```

- Bundle được kiểm `content_hash` và SHA-256 từng trang atlas; sai hash hoặc thiếu file → báo lỗi
  rõ và thoát (exit 2), không panic.
- Địa hình dựng từ ô của bundle + bảng phân loại ô `data/maps/village00-kr.toml` (một nguồn
  chuẩn: bundle không nhân bản phân loại ô). Spawn lấy từ bundle.
- Tile/vật cản vẽ bằng atlas thật; toạ độ vẽ và z-order tính ở module thuần `scene.rs`
  (test được không cần GPU), phần texture ở `render.rs`.
- **Quy ước trình diễn** (presentation, **không** phải luật chơi — nghĩa `f0..f11` của client gốc còn là
  câu hỏi mở Q-003): quy ước đặt ảnh nằm ở nguồn chuẩn
  [BUNDLES §7](../../docs/specification/assets/BUNDLES.md) (`design`): mép dưới ô là `y = (hàng+1)·40`;
  canvas của frame đặt đáy trùng mép dưới ô, trục X của pivot trùng mép trái ô; vùng ảnh thật canh giữa
  ngang và canh đáy trong canvas.
- Z-order: nền vẽ trước; vật cản sort theo hàng rồi cột nên đối tượng ở hàng dưới che đúng đối
  tượng ở hàng trên; nhân vật (placeholder) chen theo hàng ô đang đứng.
- Nhân vật/bom/lửa vẫn là placeholder: animation là V1-031, cần clip trong bundle.

## UI luồng V1 (V1-032, V1-035)

Module `src/ui/` chia làm hai phần:

- **Phần thuần dữ liệu** (`ui::model`, `ui::layout`, `ui::text`): không đụng macroquad,
  kiểm được **không cần GPU** — đây là phần nghiệm thu chính.
- **Phần vẽ** (`ui::draw`) và **vòng lặp app** (`ui::app`): mỏng, chỉ đưa layout ra màn hình.

### Chạy

```sh
# Chế độ demo UI: dữ liệu giả lập, không cần server.
cargo run -p boom-client -- --ui-demo

# Chụp ảnh một màn (QA ảnh local; ảnh không commit vào repo).
cargo run -p boom-client -- --ui-demo --ui-shot room \
  --snapshot /home/admin/boom-team/shots/ui-room.png --bundle /tmp/bundle-village00
```

Phím trong chế độ demo UI:

| Phím | Việc |
|---|---|
| Ký tự thường | gõ vào ô đang focus (tài khoản, mật khẩu, tên phòng, chat) |
| `Backspace` | xoá ký tự cuối |
| `Tab` / `Shift+Tab` | chuyển focus giữa các ô và nút |
| `↑` / `↓` | chọn dòng trong danh sách (phòng ở sảnh, thành viên ở phòng chờ) |
| `←` / `→` | đổi nhân vật C00 ⇄ C01 (ở phòng chờ) |
| `Enter` | kích hoạt nút đang focus, hoặc gửi nội dung ô đang focus |
| `Esc` | quay lại màn trước (màn đăng nhập: thoát ứng dụng) |

### Màn hình và dữ liệu

`UiModel` không tự biết phòng nào đang mở, ai đã sẵn sàng hay kết quả lượt ra sao: nó hỏi
trait `UiBackend`. Bản offline giả lập là `ui::demo::DemoBackend`; **bản mạng nối vào sau
khi #89/#99 merge** (task riêng) — model và layout không đổi.

Mỗi màn có ít nhất một hành động đưa người chơi ra khỏi màn đó (không dead end): sảnh →
về đăng nhập, phòng → về sảnh, kết quả → về phòng. Màn trận **không** có nút rời trận:
theo [SCOPE_V1 §3](../../docs/planning/SCOPE_V1.md) rời trận chủ động là bị loại, nên V1 chỉ
cho thoát ứng dụng; lượt luôn kết thúc bằng màn kết quả vì server chốt kết quả.

### Chữ (V1-035)

- Chữ lấy từ `data/l10n/vi-v1.toml` (nhúng vào client) qua `boom_content::l10n`.
- Khóa chưa có text → hiện placeholder rõ ràng `⟨khóa⟩`; client **không** tự viết chữ.
- `\r\n` trong chuỗi được chuẩn hóa thành `\n` lúc hiển thị (file nguồn giữ nguyên).
- Font: ưu tiên font hệ thống có dấu tiếng Việt (DejaVu/Noto/Segoe/Arial/Helvetica), ghi đè
  bằng `BOOM_FONT`. Font chuẩn của V1 (Be Vietnam Pro) chưa có trong repo.

### Ảnh UI (V1-032)

Bảng `[[ui]]` của bundle ([BUNDLES §3.3](../../docs/specification/assets/BUNDLES.md)) quy
định trạng thái dùng của từng phần tử UI:

- `selected` (đã duyệt, không có chữ cần duyệt) → client vẽ **ảnh** từ atlas.
- `needs_review_text` / `excluded` → client vẽ **khung màu + text catalog**, **không** hiện
  ảnh (ảnh có chữ chưa được duyệt).

Cột phải của màn hình hiện đúng như vậy: ảnh cho phần tử `selected`, khung màu kèm text
catalog cho phần tử còn lại. Test `ui::tests::only_selected_ui_elements_are_drawn_as_images`
giữ quy tắc này.

## Kiểm

```sh
cargo test -p boom-client
cargo clippy -p boom-client --all-targets -- -D warnings
cargo fmt -p boom-client -- --check
```

Test input không cần cửa sổ: thứ tự event/adapter, autorepeat, release/repress, focus-loss, cạnh bom,
rebind/hoán đổi. Test scene offline dựng `World`/`SimulationState` và chạy tick mà không mở cửa sổ.
Test options: round-trip ghi→đọc, restart giữ volume/keybind, lỗi từng trường, và file hỏng →
`.broken` + file mặc định đọc lại được (kèm ca ghi lại thất bại không crash).
Test scene bundle kiểm toạ độ vẽ theo canvas/pivot, thứ tự z-order theo hàng, địa hình dựng từ
bundle + bảng phân loại ô, và các lỗi nạp bundle (thiếu thư mục, map khác profile) — đều không cần
GPU.
Test UI V1 (không GPU): chuyển màn đủ luồng Login → Sảnh → Phòng → Trận → Kết quả → Phòng,
không màn nào là dead end, text lấy đúng khóa l10n, và phần tử `needs_review_text` không hiện ảnh.

## Còn thiếu

- R-080 là `design`; chưa có trace client KR/VN cho thứ tự ưu tiên. Khi có trace, cập nhật claim
  thay vì tự nhận `confirmed`.
- Biến môi trường `BOOM_CLIENT_OPTIONS_PATH` chưa có test tự động: `std::env::set_var` là
  `unsafe` ở edition 2024 mà workspace cấm `unsafe` (`unsafe_code = "forbid"`); đã kiểm tay bằng
  CLI.
- Chưa có UI đổi tùy chọn trong game; `mute`/volume chưa nối vào audio (client chưa có audio).
- Chưa có clip animation trong bundle: nhân vật/bom/lửa vẫn là placeholder (V1-031).
- Quy ước đặt ảnh là quyết định `design` của client ([BUNDLES §7](../../docs/specification/assets/BUNDLES.md));
  nghĩa `f0..f11` của client gốc (Q-003) chưa đối chiếu, cần trace runtime để chốt.
- Chưa có mạng, prediction, âm thanh; chưa chạy runtime Windows/macOS (CI chỉ build/check).
- Focus dựa vào `WindowMinimized`/key-up của macroquad; X11 đã kiểm, Wayland/focus thật trên macOS/Windows
  cần runner P6 kiểm lại.
- **UI V1 chưa có backend mạng**: `ui::demo::DemoBackend` là dữ liệu giả; bản mạng nối vào sau khi
  #89/#99 merge (task riêng).
- UI V1 chưa có font chuẩn của V1 (Be Vietnam Pro) trong repo và chưa có golden wrap/focus/scale
  (UI_STRINGS_V1 §6): ảnh chụp hiện dùng font hệ thống.
- 66/110 khóa chuỗi V1 chưa có text nên màn hình hiện placeholder `⟨khóa⟩`; phần việc bổ sung khóa
  thuộc `boom-re-l10n` (UI_STRINGS_V1 §6.1), không phải của client.
- 8 ảnh UI có chữ ở trạng thái `needs_review_text` chưa được duyệt nên client vẽ khung màu + text
  catalog; duyệt ảnh theo quy trình [IMAGE_REVIEW](../../docs/specification/localization/IMAGE_REVIEW.md)
  thì màn hình tự hiện ảnh.
