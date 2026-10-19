# boom-client — V1-033

Client V1 trên macroquad ([ADR 0010](../../docs/decisions/0010-client-stack-choice.md)), phạm vi task
#47: input/scale/focus và scene offline chạy `boom-core` trực tiếp (chưa mạng). Không đọc
pack/`boom-assets`; không có `--bundle` thì địa hình và chỉ số chỉ là **placeholder**.

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
- Địa hình dựng từ ô của bundle + bảng phân loại ô `data/maps/village00-kr.toml` (một nguồn chuẩn:
  bundle không nhân bản phân loại ô). Spawn lấy từ bundle.
- Tile/vật cản vẽ bằng atlas thật; toạ độ vẽ và z-order tính ở module thuần `scene.rs`
  (test được không cần GPU), phần texture ở `render.rs`.
- **Quy ước trình diễn** (presentation, không phải luật chơi — nghĩa `f0..f11` của client gốc còn là
  câu hỏi mở Q-003): mép dưới ô là `y = (hàng+1)·40` (khớp neo hiển thị nhân vật R-009); canvas
  của frame đặt đáy trùng mép dưới ô, trục X của pivot trùng mép trái ô; vùng ảnh thật canh giữa
  ngang và canh đáy trong canvas.
- Z-order: nền vẽ trước; vật cản sort theo hàng rồi cột nên đối tượng ở hàng dưới che đúng đối
  tượng ở hàng trên; nhân vật (placeholder) chen theo hàng ô đang đứng.
- Nhân vật/bom/lửa vẫn là placeholder: animation là V1-031, cần clip trong bundle.

## Kiểm

```sh
cargo test -p boom-client
cargo clippy -p boom-client --all-targets -- -D warnings
cargo fmt -p boom-client -- --check
```

Test input không cần cửa sổ: thứ tự event/adapter, autorepeat, release/repress, focus-loss, cạnh bom,
rebind/hoán đổi. Test scene offline dựng `World`/`SimulationState` và chạy tick mà không mở cửa sổ.
Test scene bundle kiểm toạ độ vẽ theo canvas/pivot, thứ tự z-order theo hàng, địa hình dựng từ
bundle + bảng phân loại ô, và các lỗi nạp bundle (thiếu thư mục, map khác profile) — đều không cần
GPU.

## Còn thiếu

- R-080 là `design`; chưa có trace client KR/VN cho thứ tự ưu tiên. Khi có trace, cập nhật claim
  thay vì tự nhận `confirmed`.
- Chưa có clip animation trong bundle: nhân vật/bom/lửa vẫn là placeholder (V1-031).
- Quy ước pivot/canvas là quyết định presentation của client; nghĩa `f0..f11` của client gốc
  (Q-003) chưa đối chiếu, cần trace runtime để chốt.
- Chưa có mạng, prediction, âm thanh; chưa chạy runtime Windows/macOS (CI chỉ build/check).
- Focus dựa vào `WindowMinimized`/key-up của macroquad; X11 đã kiểm, Wayland/focus thật trên macOS/Windows
  cần runner P6 kiểm lại.
