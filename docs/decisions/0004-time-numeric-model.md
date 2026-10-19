# 0004 — Mô hình thời gian và số học của core: fixed step + số nguyên

Status: **Accepted (mặc định có thể đảo)**, 2026-10-19. Giải quyết phần mô hình của Q-004;
phần hiệu chuẩn số (tick gốc, speed) vẫn mở. Cho phép walking skeleton trước P3.

## Bối cảnh

SCOPE_V1 nghiệm thu #8 đòi replay cùng state hash trên Windows/Linux/macOS, và server authoritative
cần mô phỏng tái lập để reconcile. Float khác nhau giữa compiler/CPU/flag là rủi ro trực tiếp cho
yêu cầu đó. Client VN dùng đồng hồ `timeGetTime` và bước mô phỏng `CGameStage::vf159` mỗi tick
([VN runtime](../research/findings/VN_BATTLE_RUNTIME_REFERENCE.md)); tần số tick gốc chưa đo.
Để ngỏ mô hình tới khi đo xong sẽ chặn mọi task core.

## Quyết định

1. **Fixed step.** `boom-core` chỉ tiến theo `Tick` (u32). Tần số `sim.ticks_per_second` là tham số
   của rules profile, không phải hằng số trong code. Giá trị dev hiện tại là `design`.
2. **Số nguyên.** Tọa độ là `Mpx` (i32, 1/1000 pixel; một ô 40 px = 40 000 mpx). Tốc độ là mpx/tick.
   Thời lượng trong profile ghi bằng ms, đổi sang tick bằng làm tròn lên (`ceil(ms·hz/1000)`), một lần khi nạp.
   Core cấm `f32/f64` (`clippy::float_arithmetic` deny) và mọi phép chia phải nêu chiều làm tròn.
3. **Thứ tự xác định.** Core không dùng `HashMap/HashSet`, wall clock (`Instant`, `SystemTime`) hay IO;
   thực thi bằng `crates/boom-core/clippy.toml`. Duyệt entity theo ID tăng dần.
4. **RNG riêng.** SplitMix64 trong core, lấy mẫu không lệch bằng rejection; trạng thái RNG nằm trong
   `SimulationState` và được hash.
5. **State hash** `fnv1a64-v0` trên encoding chuẩn (little-endian, thứ tự field cố định); tên thuật toán
   ghi trong replay header để đổi được về sau mà không phát lại sai âm thầm.

## Điều kiện đảo/điều chỉnh

- Q-004 đo được tick gốc và cách tích lũy chuyển động → chỉnh `ticks_per_second`/công thức, không đổi mô hình.
- Nếu chứng minh được chuyển động gốc dùng float và sai khác vượt sai số chấp nhận ở boundary scenario
  (rẽ góc, chạm tường) → ADR mới, giữ số nguyên với hằng số hiệu chuẩn hoặc fixed-point độ phân giải khác.
- Nếu `fnv1a64` gây va chạm hash trong fixture thật → đổi thuật toán, tăng phiên bản.

## Hệ quả

Walking skeleton (`boom-core`, `boom-content`, `boom-assets`, `boom-replay`, `xtask`) được mở ngay để
chứng minh boundary, nạp profile và replay hash; chưa nhận feature ID nào là done.
Chuyển động trong skeleton (căn trục khi rẽ, va chạm ô) là `design` tạm, ghi trong RULES_V1.
