# 0012 — V1 chọn game rules từ BnB

Status: **Accepted**, 2026-10-21, theo quyết định trực tiếp của chủ dự án:
“Chốt game rules lấy từ https://github.com/tpoisonooo/BnB — cập nhật game rules theo repo, cập nhật lại docs.”
Chủ dự án chỉ định cả trường hợp BnB khác giá trị KR/VN đã confirmed: **profile V1 dùng BnB**.
Tác giả: Codex (co-integrator). Review PR: Claude; Accepted ghi nguồn quyết định, không thay approval PR.

## Nguồn và giới hạn

[BnB](https://github.com/tpoisonooo/BnB/tree/5c108e8ad3ea07684d4666f60e6eec6c2f3e569e),
SHA **`5c108e8ad3ea07684d4666f60e6eec6c2f3e569e`**, đọc lại 2026-10-21 tại
`/home/admin/boom-team/tmp/bnb` (chỉ đọc): `server/game.js`, `server/map.js`, `server/characters.js`;
đọc các nhánh mount/kick/death-loot để xác định ranh giới scope. Đây là remake CN đã cân bằng lại,
không phải client KR 00967 hay server gốc. Không copy code vào `crates/`.
Locator/giá trị chi tiết: [OSS_REIMPLEMENTATIONS](../research/findings/OSS_REIMPLEMENTATIONS.md) §8,
[RULES_V1](../specification/gameplay/RULES_V1.md) §10 (R-090…R-099).
Bản khảo sát 2026-10-19 không ghi SHA nên không chứng minh được diff giữa hai ngày.

## Quyết định

1. Giá trị gameplay V1 design/candidate/unknown chọn theo BnB, ghi `source = "oss"`,
   `status = "design"`, claim lựa chọn mới có SHA/file:dòng và quyết định 2026-10-21.
   Claim OSS cũ vẫn candidate; R-074/R-075/R-076 và cơ chế cũ ghi superseded, không xóa lịch sử.
2. Khi khác KR/VN confirmed, BnB ghi đè **giá trị profile**, không ghi đè bằng chứng gốc hoặc hạ nhãn.
   R-001/R-010/R-013/R-026/R-040/R-041/R-051 giữ nguyên. Có thể quay lại bằng profile/overlay
   theo ADR 0008; không lấy trung bình. Nhân vật chờ #117 merge rồi #133 đưa vào `characters.*`.
3. Khác biệt cơ chế phải có task worker và acceptance, không sửa core trong PR docs/profile này.
   Chọn BnB trong phạm vi gameplay đã mở; không tự mở mount/kick/kim/remote/death loot/map khác/bot.
   V1 chỉ có ba item nên **chuẩn hóa có điều kiện** trọng số BnB 20:28:16, không gọi chúng là
   xác suất tuyệt đối BnB. C00→baobao, C01→haidao là ánh xạ template cân bằng design,
   không đổi tên/asset KR hoặc tuyên bố hai nhân vật là cùng identity.
4. Đổi 33→30 Hz; đổi ms→tick làm tròn lên một lần theo ADR 0004: round 5400, fuse 75,
   flame 15, trap 120, warmup 90 tick. Một cấp speed = 800 Mpx/tick chính xác.
   Skeleton-test/golden giữ profile riêng, không đổi hoặc bless để che sai khác.
   Presentation KR (100 ms/frame, R-085) và policy vận hành/phòng của scope giữ riêng:
   BnB không có timing cho các clip KR và không thay auth/transport/timeout vận hành V1.

## BnB ↔ client gốc

| Tham số | Gốc KR/VN (claim) | BnB / lựa chọn V1 | Chênh lệch |
|---|---|---|---|
| Tile | 40 px confirmed R-001 | 40 px (`map.js:7`) | 0 |
| Tick | 30 ms/bước VN confirmed R-013 (~33,3 Hz) | 30 Hz (`game.js:12`), 33⅓ ms/bước | +3⅓ ms/bước; profile cũ 33 Hz →30 |
| Thời hạn vòng | 120 s VN confirmed R-010 | 180 s (`game.js:18`) | +60 s |
| Trap→chết | Packet VN 6,359 s ±5 ms R-026; timer nội bộ unknown | Timer 4 s (`game.js:17,546–550`) | −2,359 s so packet; không coi hai đại lượng là cùng phép đo; profile cũ 6,3→4 s |
| Speed/cấp | Chưa confirmed; raw 5→6 px/tick candidate R-043 | 24 px/s (`characters.js:15`), profile cũ 33 | −9 px/s so design cũ; không suy chênh lệch so client |
| Fuse/lửa | Fuse unknown R-021; 2,5 s emu candidate R-022 | 2500/500 ms (`game.js:14–16`) | Không có giá trị gốc để trừ |
| Bomb/Fire delta | +1 confirmed R-025 | +1 (`game.js:579–580`) | 0 |
| Roller | +1 level candidate R-045 | +24 px/s = +1 level (`game.js:21,581`) | Xác lập design, không xác nhận checksum VN |
| Drop Bomb:Fire:Roller | 3650:2800:2000 VN confirmed R-051; vai trò bảng unknown | 20:28:16 (`game.js:433–437`) chuẩn hóa trên ba item | Tỉ trọng có điều kiện 43,20/33,14/23,67% →31,25/43,75/25%; không phải xác suất gốc |
| Gạch có item | Unknown R-053 | 35% (`game.js:20,430`) | Không có giá trị gốc để trừ |
| C00 speed/bomb/range min–max | KR f2–f7 `5,7,1,10,1,7` R-040; mapping R-042 candidate | `6–10 / 1–6 / 1–7` (`characters.js:20–21`) | Theo mapping candidate: speed +1/+3, bomb 0/−4, range 0/0 |
| C01 speed/bomb/range min–max | KR f2–f7 `4,8,2,7,1,9` R-041; mapping R-042 candidate | `5–9 / 1–8 / 2–7` (`characters.js:25–26`) | Theo mapping candidate: speed +1/+1, bomb −1/+1, range +1/−2 |
| Hitbox/corner assist | Chưa confirmed | 26×26 px / 12 px (`game.js:31–32`) | Design mới, không suy fidelity |

## Cơ chế khác engine V1 và task triển khai

| Cơ chế | Engine/bằng chứng trước ADR | BnB được chọn / phần giữ riêng V1 | Task |
|---|---|---|---|
| Movement/hitbox/assist | Điểm tâm, căn về tâm trước khi rẽ | Hộp half=13, assist khi chặn trong 12 px (`game.js:173–275`); last-pressed đã tương thích | [#130](https://github.com/aquaplatinumberlitz/boom-rs/issues/130) |
| Bomb pass | Cấp theo cùng ô, mất khi tâm rời ô | Cấp theo overlap hộp, xóa sau hết overlap (`game.js:150–153,392–399`) | [#130](https://github.com/aquaplatinumberlitz/boom-rs/issues/130) |
| Trapped di chuyển | R-036 đứng yên, web R-029 nói đi chậm | BnB **đứng yên**, `tick():631–633`; giữ R-036, không thêm slow move | Không cần task riêng |
| Trap/chọc/thứ tự | Cùng ô; trap mới chưa chết cùng tick; touch ưu tiên timer | Khoảng cách strict <28 px trên mỗi trục; trap mới giảm timer ngay; timeout trước touch (`game.js:546–567,634–638`) | [#132](https://github.com/aquaplatinumberlitz/boom-rs/issues/132) |
| Chain/lửa | R-033 ưu tiên id; flame event theo core hiện tại | Queue FIFO theo insertion/chain; gạch xóa sau toàn queue, lửa mới giảm TTL ngay (`game.js:401–451`). V1 giữ thứ tự entity id ổn định thay JS Map; worker ghi boundary khác biệt | [#132](https://github.com/aquaplatinumberlitz/boom-rs/issues/132) |
| Drop | Pre-roll đầu trận R-067 theo R-050 | Roll khi gạch vỡ, 35%, chỉ ba loại V1 (`game.js:425–445`) | [#131](https://github.com/aquaplatinumberlitz/boom-rs/issues/131) |
| Lửa/item | Lửa không phá item R-068 | Đốt item cũ trước sinh drop mới (`game.js:417–430`) | [#131](https://github.com/aquaplatinumberlitz/boom-rs/issues/131) |
| Pickup tại cap/tranh item | Không nhặt tại cap R-058; id nhỏ thắng | Tại cap vẫn xóa item (`game.js:569–589`); V1 giữ id nhỏ thay insertion order, không thêm eaten/death-loot ngoài scope | [#131](https://github.com/aquaplatinumberlitz/boom-rs/issues/131) |
| Warmup/kết quả | V1 clock sau loading; resolve gameplay trước timeout | Thêm 90 tick warmup đóng băng trước clock; checkOver trước timeout; trapped vẫn alive (`game.js:600–646`). FFA 2–4 người thật, disconnect grace 10 s giữ policy V1, không đổi sang kill ngay của BnB | [#132](https://github.com/aquaplatinumberlitz/boom-rs/issues/132) |
| Chỉ số nhân vật | #117 đang mở, sở hữu loader/profile | Hai template BnB theo R-095 sau khi #117 merge; không sửa file của PR đang mở | [#133](https://github.com/aquaplatinumberlitz/boom-rs/issues/133) |

## Hệ quả và điều kiện đảo

Profile đổi hash, client/server/replay phải cùng profile mới. Giá trị mới có hiệu lực khi loader
đọc profile; các khóa hitbox/assist/touch/warmup mới **chưa được core tiêu thụ** trước task tương ứng.
Chưa thể tuyên bố toàn engine theo BnB hoặc feature done. Không sửa tracking/progress trong ADR này.
Bằng chứng client thật vẫn là nguồn chuẩn cho fidelity gốc; quyết định này chọn một biến thể gameplay V1.
Có thể đảo bằng ADR mới và overlay/profile mang giá trị gốc sau khi chủ dự án chốt;
giữ các scenario BnB để không âm thầm đổi hành vi. Không chọn cách tiếp tục để unknown chặn V1,
hoặc gọi OSS là confirmed: cả hai không đáp ứng quyết định chủ dự án và quy tắc nguồn.
