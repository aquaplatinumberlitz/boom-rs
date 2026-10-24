# Luật trận và runtime VN được chấp nhận cho bản viết lại

Ngày nhập: 2026-10-19. [Chính sách trusted sources](../../decisions/0002-vn-kr-trusted-sources.md):
VN được tin cậy ngang KR, dùng ngay cho luật chung; không cần chờ tái kiểm trên KR.

Nguồn: repo `/home/admin/boom-re`, commit `4ccc8904cd907ce627f8a2cf6cf84bed190fd00d`,
`docs/oc_e-game.md` mục “Cứu đồng đội / chọc vỡ bong bóng”, “Vật phẩm khi 2 người”,
“Nhiều map và loại phòng”. Đây là kết quả runtime do phiên Opus thực hiện; lượt nhập docs này
đọc artifact và lịch sử commit, không tự chạy lại client.
Ảnh và script gốc lưu trong Git tại các commit bên dưới; không phụ thuộc việc đã push.
Tra lại bằng `git -C /home/admin/boom-re show <commit>:<path>`.

| Claim | Kết quả được dùng | Bằng chứng / commit | Giới hạn còn lại |
|---|---|---|---|
| VN-B01 | Chạm đồng đội bị nhốt thì cứu; chạm địch bị nhốt thì địch chết | `34e0910`: `docs/rescue_freed.png`, `docs/rescue_enemy_pop.png`; runtime 2 client `logs/duo/r4`, `r5` | Thời lượng trapped/fuse chính xác và xử lý đồng thời chưa chốt |
| VN-B02 | Legacy client tự báo C→S120/12 cứu, /13 chết, server không cần đáp | `34e0910`, `docs/oc_e-game.md` | Chưa rõ đuôi packet có actor cứu/chọc; BẮT/CỨU kết quả vẫn00 |
| VN-B03 | Item nền, item lộ sau phá gạch và pickup hiển thị đồng bộ hai máy | `0cce915`: `docs/duo_items_A_view.png`, `docs/duo_items_brick.png`; `logs/duo/i1` | Chưa thử hai người cùng nhặt; emulator hiện xác nhận mọi117, chưa chống trùng |
| VN-B04 | Chạy được Village01, Ice01, Desert01, Tomb01, Factory01, Patrit01, Mine01, Ocean01, Pharaoh01 | `e0f9a49`: `docs/map_*.png`, `tools/map_loop.sh` | Mỗi map thử một người vào trận/đặt bom/bị nhốt; chưa test toàn map hay đủ mode |
| VN-B05 | Loại phòng14/16/22/32/33 vào trận thường;17 có UI RPG và gửi120/21 khi trúng bom | `e0f9a49`: `docs/roomtype17_rpg.png`, `docs/roomtype32.png`, `tools/roomtype_loop.sh` | Chạy một người Village01; semantics mất HP của21 chưa chắc, server RPG chưa làm |
| VN-B06 | C→S88 đầu khi vào phòng là lựa chọn tự gửi; không được ghi đè map host | `0e71e8a`: thay đổi server và docs | Bài học lifecycle, không bắt giao thức mới sao chép opcode cũ |

## Grace của owner sau đặt bom — confirmed owner-only runtime 2026-10-24

Theo **quyết định user 2026-10-24**, kết quả đo động VN dùng được làm nguồn luật gần ngang tĩnh KR;
không bắt re-equivalence. Đây là **confirmed owner-only physical observation**, chỉ owner-only; chưa
non-owner/TTL. Nguồn: hai run host private (#240) tại `.re-work/vn-grace-240/host-run-{2,3}/`, raw local; evidence
công khai [`docs/evidence/measurements/vn-player-grace-2026-10-24/README.md`](../../evidence/measurements/vn-player-grace-2026-10-24/README.md) (PR #242).

| Case | Fixture / quan sát | Kết quả owner-only |
|---|---|---|
| run2 | VN `VBNB_116`, `ca_fixed_patched.exe` SHA `bc875ba5…b1`, `Village01` SHA `1ca7e03e…1b28`, spawn `(14,0)`; fakeserver gửi 152/151 theo timer, **không tự mô phỏng collision/damage**; fuse60 emu_choice | Space→152 tại `(14,0)`; owner rời Left sang `(13,0)`, Right 0.45 s vẫn ở `(13,0)` face right; run2 stop trước nổ, không 151 |
| run3 | cùng fixture; đứng yên trên bomb **5 s** sau 152 rồi mới Left/Right | 5 s không làm mất quyền rời bomb; Left→`(13,0)`, Right vẫn `(13,0)`; S151 10:42:07.367 → client C→S120/11 10:42:07.389 (trap confirmed) → 120/13 10:42:13.751 (death); server không ép trap; S151 gửi ids0..7 nhưng chỉ 1 bom được đặt |

- Đây là **owner-only**, hai run; không suy TTL ms, không suy grace toàn hệ thống/all non-owner.
  Không claim timer ≤/>5 s: chưa đo thời điểm nhận 152/tick.
- Fixture rebuild có **devirt gameplay reconstruction** (bomb msg-cache `0x6EFD4A`, CBombOwner
  `0x5788C9` experiment, random guards), không chỉ offline/localhost. Đã đối chiếu 25 patch +10 IP
  +15 devirt sites; không manifest nên semantic equivalence chưa chứng minh → provenance ghi để truy vết,
  không hạ cấp theo quyết định user; không tuyên bố vanilla equivalence.
- Bomb cell `(14,0)` từ packet 111; screen chỉ approximate (owner visual bounds x≈760→719, y center
  ≈62, feet≈78; không phải native coords). Không dùng hold 0.45 s/5 s làm timer; `monotonic_ns`
  trong supervisor chỉ là command clock, không phải fuse.
- Map VN `Village01` W15×H13; `(13,0)` tile1005/obj996 floor; col12 tree, row1col13 brick. Không suy
  spawn `(13,1)` của KR.
- Run3 có trap/death sau 151, nhưng owner đã rời bomb trước đó; không phải bằng chứng timing grace.
- Gap: non-owner đứng chồng ô lúc 152 chưa đo. Không dùng getter KR `0xB335C0/0xB335E0` cho VN khi
  chưa xác minh; `tools/gdb/spawn.gdb` chỉ in spawn, không phải player; chấp nhận screen approx +
  packet cell `(14,0)`.

## Cửa sổ sát thương lửa — confirmed owner-only 2026-10-24

**Quyết định user 2026-10-24:** kết quả đo động client VN dùng được làm nguồn luật, gần ngang phân
tích tĩnh KR; không cần chờ tái kiểm/re-equivalence. Phần dưới là **confirmed owner-only physical
observation** theo nghĩa đó, không phải claim toàn hệ thống.

Proper Wine `winedbg --gdb` (i386), byte gate 16B `6a50…8b83` @`0x7324c0`; entry/return `0x6e9aee`
trước Left. Owner-only, map Ocean01, một case mỗi mốc:

| Case | Tọa độ native tại capture | N/frame | N0 sau RET | frame18 start | Kết quả |
|---|---|---|---:|---:|---|
| good200 | x50,y39 (sample) | 1 / 14 | 531.084441 ms | 504.432797 ms | trapped |
| phase12 | x50,y39 | 1 / 18 | 535.176555 ms | 492.810326 ms | trapped |
| phase13 | x50,y39 | 0 / 19 | 561.943179 ms | 530.836641 ms | alive, no trap/no120 |

- **Frame 18 harm / frame 19 expired** ⇒ cửa sổ tồn tại thật owner-only, không chỉ animation. Native XY
  decode+checksum đối chiếu getter `0x58D56D` EAX=40 lúc spawn `(40,39)`; **không dùng getter KR**.
- Animation lifetime/frame scalar ≠ damage TTL; R-158 chọn540ms cho V1 tại30Hz và tính đủ số tick; đây là hiệu chuẩn, không phải hằng timer native.
- Evidence/CSV: [`vn-flame-window-2026-10-24`](../../evidence/measurements/vn-flame-window-2026-10-24/README.md),
  [samples.csv](../../evidence/measurements/vn-flame-window-2026-10-24/samples.csv).
- Giới hạn: owner-only; chưa non-owner, chưa repeated, chưa exact TTL/range; fixture provenance đã biết.

## Đưa vào công việc hiện hành

- **V1-020 / Q-006:** luật chạm địch làm vỡ bong bóng được chốt từ VN-B01; không RE lại chỉ vì khác region.
  Đồng đội cứu nhau thuộc backlog Battle đội, không tự đưa vào V1 FFA.
- **V1-016 / Q-007:** dùng VN-B03 làm scenario đồng bộ hiện/lộ/nhặt item. Vẫn phải chốt drop weight,
  cap và race pickup; không lấy emulator hiện tại làm thiết kế chống trùng.
- **V1-024/027:** server mới tự quyết damage/rescue/death/pickup, kiểm command; hành vi client tự báo
  của legacy là nguồn hiểu cơ chế, không chuyển quyền quyết định sang client mới.
- **V1-030:** tái dùng hiểu biết loader/theme từ chín map; vẫn giữ KR Village00 đã chọn.
  Index map VN không phải ID map KR; map VN chạy được không tự hoàn thành content closure Village00.
- **Later RPG:** giữ boundary mode/strategy và state HP/EXP/skill riêng; chưa có luật RPG đầy đủ.
- **Q-009/Q-013:** dùng main mới làm mốc khảo sát legacy; không chép server snapshot cũ từ l10n đè main.

## Hạ tầng và unknown

Server thử đã sửa kết thúc phòng một đội: chỉ hết khi cả đội chết; đây là sửa emulator,
không phải bằng chứng riêng về thuật toán server gốc. Bộ đếm bắt/cứu và actor attribution còn thiếu.

Batch thử từng gặp server cũ giữ cổng sau timeout và Wine kẹt; lỗi đó có thể làm sai map/kết luận crash.
Fixture sau này cần lưu PID/port, commit/config, kiểm server đúng lần chạy và cleanup theo phiên.
Không chạy `wineserver -k` hoặc kill tiến trình chung trong phiên có người khác đang dùng Wine.
Log runtime trong `logs/` là dữ liệu local có thể không đi cùng Git; ảnh/script/docs đã commit là
artifact bền vững, chưa thay thế trace đầy đủ cho phép đo timing.

## Movement/trap: checkpoint bounded24/10 (#146/#147)

Nguồn VN fixture `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1` như
[provenance owner-grace](../../evidence/measurements/vn-player-grace-2026-10-24/README.md).
Các tọa độ/thời gian quan sát trong đúng run/cấu hình là `vn_runtime` / confirmed; nối cơ chế/level
vẫn candidate/unknown. Phân tích cạnh máy là `vn_static` trong fixture cùng SHA. Không đổi luật
chỉ từ các cạnh bên dưới.

### Vận tốc một cấu hình, chưa biết speed level

Phân tích lại nativeXY đã có của `host-flame-coordinate-good200`, `phase12`, `phase13`; không
chạy lại client. `begin_ns` đầu→cuối, y giữ nguyên, dx liên tiếp±5, chọn cửa sổ theo command sau
khi thu dữ liệu. Độ phân giải bị giới hạn bởi sampler nominal5ms; số lẻ không phải độ chính xácµs.

| Run/đoạn | dx(px) | Thời lượng(ms) | Trung bình(px/s) |
|---|---:|---:|---:|
| good200 trái | -35 | 207,548528 | 168,635 |
| good200 phải | 75 | 448,227236 | 167,326 |
| phase12 trái | -35 | 212,266142 | 164,887 |
| phase12 phải | 75 | 431,524711 | 173,802 |
| phase13 trái | -35 | 209,681500 | 166,920 |
| phase13 phải | 75 | 441,022350 | 170,059 |
| phase13 trái trước trap | -60 | 350,343348 | 171,261 |

Giữ toàn bộ khoảng164,887–173,802px/s; đoạn phase12-phải có interval5,201ms chưa rõ nguyên nhân,
không tự loại như lỗi clock. Không suy C00/C01, native speed-level, cap, hoặc server30Hz từ bảng.
Raw `position-samples.jsonl` SHA: good200`fee84a6eebe499606fbcf299fd91a6ffe0324d54e5eaf04e5fddcfa755fb0c92`,
phase12`5348c898063de1cd783335637460fd7fe4f64e20a208c50ef3fcde67720f111f`,
phase13`931667bd018af9f9f14a38e95433b68b881fcb083769c4aca8f84668f6b7b58a`.
Script tái tính/raw giữ trong clone `re-kr/.re-work/speed-146/`; flame evidence đã lưu provenance của các run.

Actor vtableC664C4 slot21/+0x54 (84byte)→5B1472; wrapper gọi5A6244@5B1646. Cache wrapper SHA
`c2559f3d25fd11c9592fac362bfad365781b28c7b0d3f812b7c77e11ef685858`.
Base cache `5A6244–5A7005` SHA`54c558a6a728dc5de6f3d97b341249536c4e36a0d0ee58e42fd91a3cda599a46`
là **range chưa trọn hàm** (hàm còn đi tiếp), semantics chỉ đọc một phần; ±4 ở path target/current
chưa nối với5px của actor đo. Không nâng thành speed consumer hoặc đổi24px/s/level của profile.

### Timer nhốt: sender chưa nối được seed/expiry

- `900462` là notifier so getter5A6128 với arg rồi có thể emit0xB; chưa phải setter. Getter+2C0
  cho mã0/1/2/default3; không tự đặt tên3=trapped.
- Counter+26E0 giảm ở5A62B1; call+0x54@5A62BF có receiver **memberthis+D18**, chưa resolve target,
  không gán sang hero5B1472. Hero+0x28C→5AF9AB clear counter nhưng packet trực tiếp là162, không120/13.
- Sender5AF165 (đủ range5AF165–5AF1CE) xây packet120 với subtypearg. Stage7172E7 chỉ đọc prefix
  tới717500; site717348 truyền13 vào virtual+0x1C0 trên arg0. **Receiver chưa chứng minh cùng actor**,
  trigger/seed timer chưa nối; chỉ là candidate producer C120/13.

Raw static/cache private ở `re-kr2/.re-work/trap-timer-147/`; không đọc lại15literalhit/VM cũ.
Bước tiếp #147: receiver/trigger của site717348 và watch seed có đối chứng; không lấy khoảng packet
6359ms hay counter162 làm timer nhốt. #146 cần level/setter nối đúng actor trước đo mới.
