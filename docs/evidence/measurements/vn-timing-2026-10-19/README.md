# Đo tick, tốc độ và timer nhốt trên client VN — 2026-10-19

Hồ sơ tái lập cho các claim [R-013, R-026, R-043, R-045](../../../specification/gameplay/RULES_V1.md)
và câu hỏi [Q-004, Q-005, Q-006](../../../research/QUESTIONS.md). Đây là **bản trích text** từ artefact
đo; không có client, pack, ảnh hay asset game trong repo.

## 1. Build và điều kiện đo

| Mục | Giá trị |
|---|---|
| Client | `ca_fixed_patched.exe`, SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, 18.904.064 byte |
| Wine/GDB | `wine-10.0-20260425 (CachyOS)`; GDB 18.1 |
| Hiển thị | Xvfb `1024x768x24`, LLVMpipe/software GL |
| Server | `boom-re/tools/fakeserver.py`; `AUTO_START=5 TURN_START=10 TURN_MODE=1 GO_START=3 PLAY_START=4 POST_PLAY_START=2`; run trap thêm `BOMB_PLACE=1 BOMB_ECHO=2.5 BOMB_IDS=4` |
| Input | `xdotool`, script `run_probe.sh` / `run_trap.sh` / `run_trapt.sh` |
| Artefact gốc | `/home/admin/boom-rs-dsh/issue-11/.tmp-measure/` (ngoài repo, chỉ đọc) |

Chi tiết build trong [build.txt](build.txt).

## 2. Tick — `ctx+0x1DF190`

Bộ đếm bước tăng tại `0x6ED316`; `ctx = *(stage+0x547C)`, `stage = *0xE60924`.
Hai run `probe/sample1.txt` (11,505 s, 24 mẫu) và `probe/sample2.txt` (5,503 s, 12 mẫu), lấy mẫu
0,5 s/lần, không gdb.

| Run | Cửa sổ | Bước | Bước/s | ms/bước |
|---|---|---:|---:|---:|
| sample1 | 0,000–11,505 s | 384 | 33,377 | 29,96 |
| sample2 | 0,000–5,503 s | 184 | 33,436 | 29,91 |

Cửa sổ 2,5 s dao động 32,98–34,00 bước/s do lượng tử mẫu 0,5 s; toàn cửa sổ ổn định 33,38/33,44.
Kết luận: **30 ms/bước** (≈33,3 bước/s), sai số làm tròn ±0,1 ms. `33 Hz` không phải số đo; là lựa chọn
nguyên của profile (R-074, design).

- Dữ liệu: [tick-runs.csv](tick-runs.csv), log gốc [tick-samples.txt](tick-samples.txt).
- Lệnh: `python3 analyze_tick.py`.

## 3. Tốc độ — `bomber+0x4C/+0x58`

Vị trí được đọc qua struct mã hóa; `sampler2.py` kiểm checksum bằng `blob[7:9]`, trong khi log worker
ghi checksum ở `+8`. **Toàn bộ mẫu đã lưu có `xok=False`, `yok=False`** (8/8 file, 0 mẫu hợp lệ).
Vì vậy các giá trị x/y dưới đây chỉ là **quan sát thô**, chưa xác thực:

| Run | Mẫu | 5 px/tick | 6 px/tick | Ghi chú |
|---|---:|---:|---:|---|
| all/c00 | 254 | 6 | 0 | base |
| all/c01 | 249 | 0 | 0 | C01 raw, chưa đối chiếu KR Attr |
| fast/c00 | 556 | 3 | 0 | base |
| fast/c01 | 556 | 3 | 0 | C01 raw |
| item/c00 | 804 | 8 | 0 | base |
| item2/c00 | 949 | 14 | 0 | base |
| item3/c00 | 947 | 8 | 6 | nhặt Roller 22:31:20 (server 117/158), **một run** |
| trapt/c00 | 1164 | 0 | 0 | run trap, không di chuyển |

Kết luận tạm: C00/C01 có quan sát thô 5 px/tick; sau Roller có 6 px/tick ở **một run**. Do checksum
chưa xác thực và chưa lặp cấp 6, R-043/R-045 giữ **candidate**, không suy ra 33,3 px/s confirmed và
không suy cấp 5 VN chỉ từ Attr KR (R-042 vẫn candidate).

- Dữ liệu: [speed-observations.csv](speed-observations.csv), chi tiết item3
  [speed-item3-changes.txt](speed-item3-changes.txt).
- Lệnh: `python3 analyze_speed.py <đường-dẫn-pos.csv>`.

## 4. Timer nhốt — packet `120/11` → `120/13`

Ba lần đo khoảng packet:

| Run | Nguồn | 120/11 | 120/13 | Khoảng |
|---|---|---|---:|---:|
| run3-native | worker log `dsh-2-issue-11.run2159.log` dòng 84 | 21:03:32.227 | 21:03:38.582 | 6,355 s |
| run4-trap | `trap/server.log` dòng 225/241 | 22:34:13.192432 | 22:34:19.556305 | 6,363873 s |
| run5-trapt | `trapt/c00/server.log` dòng 228/240 | 22:37:30.972421 | 22:37:37.330015 | 6,357594 s |

Trung bình 6,3588 s; biên độ 9,0 ms. Đây là khoảng **packet** (gồm xử lý/gửi chưa tách), không phải
timer nội bộ. Run3 không còn raw `server.log`, chỉ còn log worker; run4/run5 có raw log.

GDB run trap đặt breakpoint `0x446430` (`TRAP_ARG`), `0x574802` (`BUBBLE_MAX`) và `0x7172E7`
(`DEATH_PKT_120_13`), nhưng log chỉ có `DEATH_PKT_120_13`, **không có** `TRAP_ARG`/`BUBBLE_MAX`.
Vì vậy không chứng minh được `210 tick`, `~2 tick bù`, hay literal `6300`; xem [gdb-trap.txt](gdb-trap.txt)
và [trap.gdb](trap.gdb).

- Dữ liệu: [trap-packets.csv](trap-packets.csv), trích log [trap-packets.txt](trap-packets.txt).
- Lệnh trích: `grep -nE 'bomber event 120 subtype (11|13)' trap/server.log`.

## 5. Giới hạn

- Tick/speed lấy mẫu 0,5 s (tick) và 5 ms (speed); không đo dưới debugger cho tick/speed.
- Speed: checksum x/y chưa khớp; decoder chưa được xác nhận bằng artefact độc lập; chỉ một run cấp 6.
- Timer: chưa thấy setter/timer trong VM; literal 6300 chưa thấy; `210 tick` chỉ là giả thuyết.
- Server là fakeserver, không phải server gốc; Xvfb/LLVMpipe có thể ảnh hưởng frame nhưng tick là
  bộ đếm nội bộ.
- Không chạy lại Wine/Xvfb trong lần sửa tài liệu này; mọi số lấy từ artefact trên.
