# boom-client — V1-033

Client V1 trên macroquad ([ADR 0010](../../docs/decisions/0010-client-stack-choice.md)), phạm vi task
#47: input/scale/focus và scene offline chạy `boom-core` trực tiếp (chưa mạng). Không đọc
pack/`boom-assets`; địa hình và chỉ số chỉ là **placeholder** tới khi có bundle content.

## Chạy

```sh
cargo run -p boom-client
cargo run -p boom-client -- --scale 1
cargo run -p boom-client -- --rebind up=W --rebind bomb=J
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
`SimulationState::step`. Địa hình placeholder 15×13 vẽ theo `CellKind`; chưa có renderer bundle,
animation, audio, mạng hay prediction.

Snapshot xác định (Linux headless):

```sh
./crates/boom-client/run-snapshot.sh local/boom-client-snapshot.png
# tương đương:
cargo run -p boom-client -- --snapshot local/boom-client-snapshot.png --ticks 60 --scale 2
```

Script tự bật Xvfb + Mesa llvmpipe; output nằm trong `local/` (gitignored), không commit asset.

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

## Còn thiếu

- R-080 là `design`; chưa có trace client KR/VN cho thứ tự ưu tiên. Khi có trace, cập nhật claim
  thay vì tự nhận `confirmed`.
- Biến môi trường `BOOM_CLIENT_OPTIONS_PATH` chưa có test tự động: `std::env::set_var` là
  `unsafe` ở edition 2024 mà workspace cấm `unsafe` (`unsafe_code = "forbid"`); đã kiểm tay bằng
  CLI.
- Chưa có UI đổi tùy chọn trong game; `mute`/volume chưa nối vào audio (client chưa có audio).
- Chưa có bundle content: chỉ số/địa hình placeholder, chưa vẽ nhân vật/bom/lửa theo asset.
- Chưa có mạng, prediction, âm thanh; chưa chạy runtime Windows/macOS (CI chỉ build/check).
- Focus dựa vào `WindowMinimized`/key-up của macroquad; X11 đã kiểm, Wayland/focus thật trên macOS/Windows
  cần runner P6 kiểm lại.
