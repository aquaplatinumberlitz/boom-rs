# VN R-192 — BOMB_ARG 1/2/3, motion ordinal lửa (metadata)

Baseline bàn giao `dc5241b4fb8c0203bf89142b033e0c26a82926d3`; branch tạo từ main hiện tại
`d6306d6` sau fetch. Không sửa code/profile/binary/pack/IDA DB.

## Cấu hình chạy
- Prefix/game sao chép `vn-grace-240`, Xvfb `:142`, Mesa llvmpipe, fake server
  `/home/admin/boom-team/tmp/vn240-flame-server.py`.
- S->C 151 do server gửi sau C->S 111: `BOMB_ECHO=8`, `BOMB_PLACE=1`, `BOMB_ARG` lần lượt **1/2/3**.
- Probe GDB R-164/R-195: đọc `selector_motion`/`boxed_motion_28` tại selector draw, một bom/lần.
- Mỗi run: `space`, chờ 151, chờ4 s, snapshot; stop. Không launch thêm sau 3 run.

## Packet thật gửi
| Run | packet 151 | observations SHA-256 |
|---|---|---|
| arg1 | `S->C opcode 151 (probe single id 1, arg 1)` | `6a4b9ebc51c519e497fc0db5e4cf84085b35bfc831f77cff59d131227bf20efb` |
| arg2 | `... id 1, arg 2` | `701803a33007b2d08829b4ec7c7e02acd0d0d163fa479a5d071fa21c5e8de943` |
| arg3 | `... id 1, arg 3` | `0bc368aaeca1836a3d907fcd6f2d727cbf07eb781286e3cd091e2fbbf1f653ec` |

Log local `.re-work/fidelity330-probe/r{1,2,3}/packets.log`, observations cùng thư mục.

## Ordinal thực đọc
- arg1: L15, C0, R45, D60 (counter1); thiếu U.
- arg2: L15, R45, D60 (counter15); thiếu C/U.
- arg3: L15, C0, R45, D60 (counter1); thiếu U.
- **Không** thấy chuỗi power2 `1/16/31/46/61` hay power3 `2/17/32/47/62`.
- Vì vậy `BOMB_ARG` một mình **không** chứng minh là power/state selector; hậu tố `*02/*03` vẫn unknown.
  Counter15 ở arg2 không đủ gán power (counter khác motion; R-164 đã tách).

## Cleanup
Ba harness tự kết thúc; kiểm sau chạy không còn `winedbg`, `vn240-flame-server`, `Xvfb :142`,
`ca_fixed_patched` hay launcher con của task. Không dùng `wineserver -k`/`pkill`; prefix đã nhả.
F-09/map không chạy trong lượt này.
