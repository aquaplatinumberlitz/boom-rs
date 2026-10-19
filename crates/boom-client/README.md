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
để kiểm nhánh nhả phím. Runtime cũng đăng ký subscriber `WindowMinimized` của macroquad để gọi
`focus_lost` ngay cả khi backend không bắn key-up (macroquad vẫn tự xóa phím đang giữ).

## Input

- Quy ước V1-033 (design, chưa có claim RULES/SCOPE): hướng = **phím hướng được nhấn gần nhất**
  trong các phím còn giữ; nhả phím đó thì quay về phím còn giữ gần nhất kế tiếp. Đây là policy
  input/UI, không phải giá trị luật trong `Rules`; V1-011 còn mở phần ưu tiên và cần chốt claim.
- Đặt bom là **cạnh nhấn**, không lặp khi giữ phím.
- Mất focus/backend báo không còn phím down → xóa toàn bộ phím đang giữ, input về neutral.
- Rebind hoán đổi nếu phím đã thuộc action khác, để không có hai action cùng một phím.

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

Test input không cần cửa sổ: thứ tự ưu tiên, nhả phím, focus-loss, cạnh bom, rebind/hoán đổi.
Test scene offline dựng `World`/`SimulationState` và chạy tick mà không mở cửa sổ.

## Còn thiếu

- Ưu tiên giữ nhiều phím mới là design; cần claim RULES/SCOPE khi V1-011 chốt.
- Chưa có bundle content: chỉ số/địa hình placeholder, chưa vẽ nhân vật/bom/lửa theo asset.
- Chưa có mạng, prediction, âm thanh; chưa chạy runtime Windows/macOS (CI chỉ build/check).
- Focus dựa vào `WindowMinimized`/key-up của macroquad; X11 đã kiểm, Wayland/focus thật trên macOS/Windows
  cần runner P6 kiểm lại.
