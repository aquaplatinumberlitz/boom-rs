# 0008 — Mô hình mở rộng: thêm tính năng, mode và event không phải sửa lõi

Status: **Accepted**, 2026-10-19, theo yêu cầu người dùng: "cấu trúc Rust engine mới phải dễ mở rộng,
update tính năng, event sau này". Catalog có 75 mục later và W1–W4 (REWRITE_PLAN), nên đây là ràng buộc
kiến trúc từ đầu, không để tới P9.

## Bối cảnh

Client gốc đã tách theo cách tương tự: loại phòng/map chọn một `CGameStrategy`
(`CNormalGameStrategy`, `CConfusedFightGameStrategy`, `CRPGGameStrategy`, `CMonSeason2GameStrategy`;
`boom-re/docs/oc_e-game.md`), item là bảng dữ liệu `ItemAttr` theo `kind`, map mang bảng rơi riêng.
Ràng buộc của engine mới: determinism (ADR 0004), server authority (ADR 0005), provenance cho mọi luật.

## Quyết định — bảy điểm mở rộng

| # | Muốn thêm | Làm ở đâu | Không phải sửa |
|---|---|---|---|
| 1 | Cơ chế gameplay (bom, lửa, item, nhốt, đẩy…) | Một `System` trong `boom-core/src/systems/`, gắn feature ID | System khác, mode khác |
| 2 | Chế độ chơi | Một type implement `Mode` (thứ tự system + điều kiện kết thúc) + đăng ký trong `modes()` | Mode có sẵn, replay cũ |
| 3 | Thông số, cân bằng | Rule trong profile TOML có claim (RULES_V1) | Code |
| 4 | Event/season/biến thể luật | `RulesOverlay` áp lên profile lúc bắt đầu lượt; định danh profile đổi theo | Profile gốc, code |
| 5 | Phản ứng với chuyện xảy ra trong trận (âm thanh, hiệu ứng, thống kê, nhiệm vụ, thành tựu, phần thưởng) | Consumer của `SimulationEvent` ở ngoài core | Core |
| 6 | Item mới ghép từ hiệu ứng có sẵn | Dữ liệu `data/items/` ánh xạ item theo build → danh sách primitive hiệu ứng | Code (chỉ thêm primitive khi cần hiệu ứng mới) |
| 7 | Tính năng ngoài trận (shop, bạn bè, guild, quest, liveops) | Module backend theo domain sau trait, nhận event miền từ server | Core, mode |

Quy tắc đi kèm:

- **State có kiểu, không động.** Feature mới thêm field/struct có tên vào `SimulationState`; dữ liệu riêng
  của mode nằm trong enum có variant theo mode. Không dùng `Any`/map động — chúng phá hash và serialize ổn định.
  Mọi field mới vào `hash()`.
- **Event là contract công khai.** `EventKind` là `#[non_exhaustive]`; consumer bỏ qua loại không biết.
  Event có ID `(tick, seq)` để consumer dedup khi replay/reconcile; server gắn thêm round ID khi phát
  ra ngoài. Phần thưởng/kinh tế ghi sổ theo idempotency key suy từ round ID + event ID.
- **Thứ tự system là luật.** Đổi thứ tự hoặc thêm system vào mode có sẵn là đổi hành vi: phải có
  boundary scenario và bless golden replay có giải thích.
- **Event vận hành (liveops) là dữ liệu.** File `data/events/*.toml` (khi P9/W4 mở) gồm lịch UTC,
  phạm vi mode/map/kênh, overlay luật, bảng thưởng, cờ tính năng. Server chọn lúc tạo lượt và ghi overlay
  vào replay header, nên replay vẫn tái lập.
- **Phiên bản hóa.** Rules profile, replay, protocol và bundle content đều có schema/revision; thay đổi
  thêm field là additive, đổi nghĩa là tăng revision. Client và server trao đổi revision + cờ tính năng
  trong handshake; tính năng tắt thì UI ẩn, server từ chối lệnh.
- **Mỗi phần mở rộng có test riêng.** System mới: boundary scenario + golden replay. Mode mới: test registry
  + replay. Overlay mới: test nạp/validate. Consumer event: test dedup khi nhận lặp.

## Đã có trong skeleton

`Mode` + `modes()`/`mode_by_id()` + `FfaBattle`; `System` + `systems::MOVEMENT`; `SimulationEvent` có ID,
`EventSink`, `event_digest()` trong golden; `RulesOverlay` + `RulesProfile::with_overlay()`; replay header
nhận `overlays`. Điểm 6 và 7 là thiết kế; hiện thực khi P5 (item) và P7/P9 (backend) mở.

## Phương án đã cân nhắc

| Phương án | Lý do không chọn |
|---|---|
| ECS tổng quát (kiểu Bevy) cho core | Thứ tự hệ thống và hash khó kiểm soát; quá rộng cho lưới 15×13 và ≤ 8 người |
| Script nhúng (Lua/WASM) cho luật | Thêm runtime, khó bảo đảm determinism/hiệu năng; để ngỏ cho nội dung later nếu thật cần |
| Một hàm `step` lớn có `match mode` | Mỗi mode/feature mới sửa cùng một chỗ; xung đột khi nhiều worker làm song song |

## Hệ quả

Worker có thể làm song song theo system/mode/consumer với ranh giới file rõ. Chi phí: phải giữ danh sách
`EventKind` và encoding hash cập nhật cùng feature; reviewer kiểm điều này trong mỗi task.
