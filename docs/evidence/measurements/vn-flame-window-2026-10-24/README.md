# VN flame window — đo runtime offline (confirmed owner-only boundary; TTL còn mở)

Task #240 (FLAME), dsh2; đo ngày 2026-10-24. **Quyết định user 2026-10-24**: kết quả đo động client VN
dùng được làm nguồn luật, gần ngang phân tích tĩnh KR; không bắt re-equivalence. Vì vậy quan sát
owner-only dưới đây là **confirmed physical observation** theo quyết định user, không hạ cấp chỉ vì
VN/devirt đã biết. TTL/non-owner/repeated vẫn chưa đo và không được suy.

## Điều kiện đo
- Client VN offline `ca_fixed_patched.exe` (clone1 `.re-work/vn-grace-240/game/`), SHA-256
  `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`.
- Map Ocean01 index 672 chọn từ pack (map authentic; **chọn map/server là `emu_choice`**); spawn slot0
  thực tế `(1,0)`; owner `(2,0)` lúc nổ; bom `(0,0)`.
- Server private patch `/home/admin/boom-team/tmp/vn240-flame-server.py`: đúng **một** `151` cho id1 sau 8 s
  (fuse 8 s = `emu_choice`), **không** gửi collision/damage.
- Namespace loopback-only + PID namespace; game/prefix clone1; raw/ảnh giữ local, không public.

## Provenance fixture (đã biết, không hạ cấp theo quyết định user)
`patch-audit.md` (clone1): original `13a0a33c…`, `ca_fixed` `c88a23c8…`, devirt `f7e89f8d…`, fixture
`bc875ba5…`; 25 patch offline/localhost + 10 IP `127.0.0.1` byte-proven. **Nhưng** `.devirt` thay đổi
đường gameplay (CGameMsgCache ring setter, `CGameStage__OnPacket` 552, `CGameStage::Init`, map load,
random guards, **`CBombOwner__vf33` 0x5788C9 EXPERIMENT plain skip**); không có manifest/checksum tracked;
**semantic equivalence chưa chứng minh**; theo quyết định user, provenance này được ghi nhưng không
hạ cấp quan sát (không tuyên bố vanilla equivalence).

## Phương pháp
- Mốc: `packets.jsonl` proxy CLOCK_MONOTONIC (ns); lịch input/snapshot `supervisor.log` cùng host.
- Snapshot thực tế `SNAP_BEGIN→END` ~157–180 ms (visible-early 164.9 ms; late-700 157.3 ms); ảnh không có
  timestamp từng frame. `09` chụp trước input; `10/11` sau input.
- Decode `120` (codec `boomproto`, XOR size keys): `u8 subtype, u8 flag, u16 x, u16 y, u32 t`;
  `t` là counter theo run, không phải ms.

## Số đo (sau khi raw đóng)
| Run | Snapshot trước input | Mốc gửi Left (host) | `120/11` | `120/13` | x,y | t (11→13) | report−input |
|---|---:|---:|---:|---:|---|---:|---:|
| late-050 | — | ~+50.3 ms (ENTER_INPUT) | **+209.0 ms** | +6571.4 | 50,39 | 2973→3185 | 158.7 ms |
| late-300 | — | +300.5 ms (ENTER_INPUT) | **+474.0 ms** | +6842.0 | 50,39 | 7539→7751 | 173.5 ms |
| visible-early | +30.4…+195.3 ms | +195.4 ms (KEYDOWN) | **+374.9 ms** | +6719.4 | 50,39 | 1763→1975 | 179.5 ms |
| late-700 | +700.7…+858.0 ms | +858.1 ms (KEYDOWN) | **không có** | — | — | — | — |

- Số `120` lấy từ dòng `C->S opcode 120 body` trong `packets.jsonl` (cùng gốc clock Codex kiểm độc lập);
  làm tròn 0.1 ms. `x,y,t` và Δt là **counter candidate** (t theo run); Δt=212 ở ba run trap
  ⇒ ~29.93–30.04 ms/tick.
- `late-050/300` chỉ có **ENTER_INPUT marker** (log trước `xdotool`, chưa có dòng KEYDOWN): mốc Left
  xấp xỉ; với `visible-early/late-700` mốc là KEYDOWN thật. Đây chỉ là **delay host-marker→packet
  report**, không phải thời điểm vào ô hay TTL.

## Xác nhận native boundary — proper Wine, owner-only (2026-10-24)

Proper `winedbg --gdb` (i386), byte gate 16B `6a50…8b83` @`0x7324c0`; entry/return `0x6e9aee` xảy ra
**trước** Left. Ba case owner-only:

| Case | Trap native | N / frame | N0 sau RET | frame18 start | first frame0 | Kết quả |
|---|---|---|---:|---:|---:|---|
| good200 | x50,y39 | 1 / 18 | 531.084441 ms | 504.432797 ms | 525.731787 ms | trapped |
| phase12 | x50,y39 | 1 / 18 | 535.176555 ms | 492.810326 ms | 529.390210 ms | trapped |
| phase13 | x50,y39 | 0 / 19 | 561.943179 ms | 530.836641 ms | 556.467914 ms | **alive**, no trap/no120 |

- phase12: native callback trap = RET+496.023519 ms, first x50 sample = RET+498.156211 ms, N1 frame18;
  phase13: frame18 x55 @+530.836641 ms, frame19 x50 @+561.943179 ms, alive; good200 frame18 start
  +504.432797 ms. `first frame0` scalar trong bảng/CSV là mốc capture native, không phải sample row.
- **Frame 18 harm, frame 19 expired** ⇒ đây là **cửa sổ tồn tại thật owner-only**, không chỉ animation;
  phase12 trap callback có flame object `435156080` (`0x19EFF470`) và frame18; phase13 N0/frame19,
  ảnh alive.
- Native XY decode + checksum, đối chiếu getter `0x58D56D` EAX=40 lúc spawn `(40,39)`; **không dùng
  getter KR**. Trap callback phase12 mang cùng flame object.
- `N0`/frame18/frame19 scalar trong [samples.csv](samples.csv); creation thực nằm trong `ENTRY..RET`,
  **không** có mốc cố định 500 ms. Animation lifetime **không** đồng nhất damage TTL; mapping field ms
  sang Rust (`tick+flame_ticks-2`, active/clear) đang review root/dsh1, **không** ghi engine 500.
- Giới hạn: owner-only, một map/fixture; chưa non-owner, chưa repeated, chưa exact TTL/range.

## Quan sát ảnh từng run
- visible-early: `09-flame-before-input` — owner vẫn `(2,0)`, **chưa trapped**; không nhận ra art lửa rõ
  (bom `(0,0)` bị cây decorative che). `10-late-enter` — trapped.
- late-700: `09-before-input` — owner `(2,0)`, chưa trapped; `10-late-enter` — ảnh sau cho ở `(1,0)`, **alive**
  (thời điểm vào ô chưa đo); `11-after-enter` — vẫn **alive**, không `120/11`.
- late-050/300: `09-late-enter` — trapped; `10-after-enter` — vẫn trapped. Không nhận dạng được art lửa
  trong ảnh nên **không** kết luận “lửa đã hết”.
- Ảnh sau chỉ cho thấy trapped/report **không lặp trong cửa sổ quan sát**; **không** suy được hit-once vs
  repeated khi đã trapped (trapped có thể chặn report/damage follow-up).

## Kết luận partial (đợt trước proper Wine — giữ lịch sử)
- **Không** đặt upper/lower bound native TTL/cửa sổ từ input/report. Bảng trên chỉ là delay quan sát.
- Ba run trap sau khi vào ô; late-700 **gửi lệnh Left** ở +858.1 ms (KEYDOWN), ảnh sau cho ở `(1,0)`
  vẫn alive, không `120/11`; **thời điểm vào ô chưa đo**. Đây là **quan sát**, không suy bound/TTL.
- Đợt proper Wine xác nhận client xử lý nổ **xong trước Left**; owner ngoài không trap lúc nổ, vào ô
  sau return thì trap ⇒ case này không phải one-shot áp dụng lúc nổ cho owner ngoài. Vẫn **không** suy
  TTL/non-owner; boundary frame18/19 ở mục xác nhận phía trên.
- Không suy fire 500 ms/2500 ms.

## Refinement module-return (2026-10-24, thất bại — không claim receipt)

- Gate byte trước attach: namespace riêng `host-flame-module-return`; đọc `/proc/<pid>/mem` tại `0x7324c0`
  phải khớp đúng 16 byte file `6a50b8c396b600e813902a008bd98b83`; 0 hoặc >1 match thì dừng.
  `module-selection.json`: **đúng 1 match**, actual PE **pid 86**,
  `Z:\home\admin\boom-rs-dsh\re-kr\.re-work\vn-grace-240\game\ca_fixed_patched.exe`,
  `matched: true`; PE base `0x400000`, DLL flags `0x8000` (no ASLR).
- GDB lần mới **hit entry** `0x7324c0`: `explosion-entry.json` `monotonic_ns=767680172516908`,
  `return=0x6e9aee`. Sau đó GDB dừng **SIGSEGV tại `0x7324c0`**, inferior `process 86` detached;
  **không có** `explosion-return.json`; supervisor `ERROR RuntimeError('no client explosion return')`.
- Không phân biệt được ngoại lệ Wine bình thường vs debugger interference/crash; **không claim**
  crash do GDB, không claim receipt/return. Không gửi Left sau hook; không đếm case 5 trap hay damage nào.
- GDB log có architecture mismatch: target `i386:x64-32` nhưng GDB chọn `i386:x86-64`, DLL i386
  “not compatible” — context debugger chưa đúng Wine32/WOW64.
- Bước tiếp: debugger đúng context Wine32/WOW64 (`winedbg --gdb` hoặc xác minh signal forwarding);
  không claim đã fix; không chạy runtime này tiếp.

Raw SHA-256 (sau closed):

| File | SHA-256 |
|---|---|
| module-selection.json | `8f301f70ed9568c4ed0e0313547ebab0a15ed763a0062eefa8f59d051644da17` |
| explosion-entry.json | `737e6f5bb762cc92c7e04fb3961d6a55b124df6df99fb846bd821e8f97faf648` |
| gdb.log | `fe28a13fbb680e550b327258550fe0938300894bed88bbcfef5ca3358ceb8414` |
| supervisor.log | `2391358ce78c97ec3f6d03b087e27a6ec8c6b05bcd82905e358a0c0f96ee0a08` |

## Probe GDB (thất bại, không claim handler)
`host-flame-handler-return/gdb.log`: breakpoint `0x7324c0` →
`Cannot insert breakpoint -14. Cannot access memory at address 0x7324c0`;
`probe.gdb:24: Error in sourced command file: Command aborted.`;
`[Inferior 1 (process 8) detached]`. **Không hit entry/return**; địa chỉ/WOW64 chưa xác minh; Codex đã
stop namespace, không retry. **Không** claim handler/`151`-return/receipt.

## Bước tiếp (không loop)
Root/dsh1 đang review mapping field ms sang Rust (`tick+flame_ticks-2`, active/clear) và calibration
profile; cần non-owner/TTL trước khi chốt luật. Không chạy thêm probe/runtime trong task này; không claim
500/540 là hằng số engine.

## Raw và SHA-256 (sau đóng)
| File (clone1 `.re-work/vn-grace-240/`) | SHA-256 |
|---|---|
| `host-flame-late-050/packets.jsonl` | `1bc88ed06375d7103e37f4508b0d6897115dd57a461e9b64df38babbf796e7c2` |
| `host-flame-late-050/server.log` | `74d60a53aeb6d7b9c6af7d0c40d4d89d84989358a9e8bc599aadc4f0315857ce` |
| `host-flame-late-050/supervisor.log` | `6164f9a7877329515b7d817f411385344d68338b1790e0f362a1880e9287b0b1` |
| `host-flame-late-050/08-outside-at-explosion.png` | `541818316d0f9a9ce4dde56a6054720c9d8f4f7031e331ac3837a54b8807299e` |
| `host-flame-late-050/09-late-enter.png` | `4dd6ea26bdeaacb748d92c8e44f72121c0ec838eaa5311317ea2f96a267999b1` |
| `host-flame-late-050/10-after-enter.png` | `faf964c448a121f25e5819bb8d22a24c8bddfafa5610077c91532c39012a2a36` |
| `host-flame-late-300/packets.jsonl` | `6554c47c8ec10a0a2484e76dbd3105248a3e70cbc1a332f9ae8e1c36332efb0e` |
| `host-flame-late-300/server.log` | `b9147a2914cf0e64e9a9bc630cc786b693ca0ebdebe941c80dcb175787d78f87` |
| `host-flame-late-300/supervisor.log` | `c092e9c5a34515d1293a2e49ff5c0d8c6f71c82522f7e15d13fa709b6bdcf2c3` |
| `host-flame-late-300/08-outside-at-explosion.png` | `5653632f6c2af3ebc748c6ba96ff84468f1bc43b566c32b1323a1aa82d3e5aa4` |
| `host-flame-late-300/09-late-enter.png` | `e65f46e223749cbb180464e2192f7119ca549852175e3ea25425b8bb55af66bf` |
| `host-flame-late-300/10-after-enter.png` | `89f0149852c3379fe45fc274280804dd33c227d8db70c5fefd76c741b2caab47` |
| `host-flame-visible-early/packets.jsonl` | `1b3831d8abc04af2d0866ccb5a49967b9a7939ce23d48052b0b1975bb8e922fe` |
| `host-flame-visible-early/server.log` | `78145d6ed1531900fac7b71d4841206aa49e9fd118633309bb39d5ba6fcefd34` |
| `host-flame-visible-early/supervisor.log` | `4a73355eb3a63d7c34b648842e9962c24010eca5146a60afa083939cfa412b29` |
| `host-flame-visible-early/09-flame-before-input.png` | `2141cd3a87f434d1fa48813e1c01babf0cf4bee2f0a8f64fc710770cc697018c` |
| `host-flame-visible-early/10-late-enter.png` | `f14cf39e4bbe730ddf5e563ccb98b27b58fd1e2959410be51feb26a353497dcf` |
| `host-flame-visible-early/11-after-enter.png` | `6f887f613cae2d7c72ef839ad015649c6d71fba53ef39bb0ad60e717088c30e1` |
| `host-flame-late-700/packets.jsonl` | `5efb9f8547fda64574f44fe939ba21f51d549fcd9690aca1e15b3e5d605d31e7` |
| `host-flame-late-700/server.log` | `00166a85ead3f7fd7e51a25cfff943bef458f8841d9bf64da2153cd44ff2514a` |
| `host-flame-late-700/supervisor.log` | `25af357f5192ed6657a3ca144646b3ff9371a239deba449d9411ee3a3885f5dd` |
| `host-flame-late-700/09-before-input.png` | `479809d9f05b4a27b1a0db8f85bca239aed2e9f558349ee7c42049c111a1eba8` |
| `host-flame-late-700/10-late-enter.png` | `db042eb42fa27e9ede53ae9ad3c8a93bae249b91c26f286e12ad43d3dcb6b1ff` |
| `host-flame-late-700/11-after-enter.png` | `fb8ad5db26d9c67c06c8698db7da85504499bfdd78ab863d5ece69c980f56975` |
| `host-flame-phase12/proxy-context.json` | `4052d93a92bde7af45e77c9ecd21621d8606bcb15dc088ce27c8a2753df8cf45` |
| `host-flame-phase12/explosion-entry.json` | `92099aab2b66b38e9efd7b258811ad5a73dd817ad01b875af714b755aa278aaa` |
| `host-flame-phase12/explosion-return.json` | `2588cea423884e9eedc33df9c90b77950d190dd4c15a67a122351ac1930ab407` |
| `host-flame-phase12/packets.jsonl` | `6395fd9e9c35ab8831bfd451a46d2aff98408972fc958c8b37464931f52952fb` |
| `host-flame-phase12/supervisor.log` | `380ea36df0650dd30bd0bae46baf374310eec8ecfbb0f951f25176ea6799a7d1` |
| `host-flame-phase12/flame-target.json` | `2153736595ec12c054071b5388e906874e4737bfc13902517789ef9acece3dd5` |
| `host-flame-phase12/hero-target.json` | `0e91d24d4377b4262ba909a81e14c604ee011df7519833165eb65f5d770ad05d` |
| `host-flame-phase12/native-trap.jsonl` | `45abd3d116bdc10bf671bba01c2483f33263393541b5f0cb5ff8b196b6458ed7` |
| `host-flame-phase12/position-samples.jsonl` | `5348c898063de1cd783335637460fd7fe4f64e20a208c50ef3fcde67720f111f` |
| `host-flame-phase13/proxy-context.json` | `a8d7f80af9e02e50b4bb1d865797bec39bce1f008c09172ff6b0c5e3482328e3` |
| `host-flame-phase13/explosion-entry.json` | `4c3c46a05395d14ff2a991141d87e24484d7598baa37a5c43326e06008a211fa` |
| `host-flame-phase13/explosion-return.json` | `457b79513c0053629e36591836fe5c50c9e28fe385666edd675e326aca641a95` |
| `host-flame-phase13/packets.jsonl` | `69cb06b7300f8fe83679a4433f94aaff022cc112615d4af4393084dac5a4416e` |
| `host-flame-phase13/supervisor.log` | `525f2eaf678b7ecd59d1ab1827297d0699ab4f7516dfa10602bf20bd0aabcc8a` |
| `host-flame-phase13/flame-target.json` | `12bd970fe7cde29076e2a4cc180aa2ce4927f898f4d9e0c9fdb0765e5721bcbe` |
| `host-flame-phase13/hero-target.json` | `f75d157d6d22a204f22c0d5b2a4146a14e008e24542821cd857fb6447c07ee0c` |
| `host-flame-phase13/position-samples.jsonl` | `931667bd018af9f9f14a38e95433b68b881fcb083769c4aca8f84668f6b7b58a` |

## Giới hạn
- Không có clock hook client-receive; input time là lúc supervisor gửi phím, không phải lúc vào ô.
- Ảnh không timestamp từng frame; capture interval thực tế ~157–180 ms; `(0,0)` bị cây che.
- Fixture có devirt gameplay + experiment skip; provenance ghi để truy vết, **không** hạ cấp theo
  quyết định user; không tuyên bố vanilla equivalence.
- Một client/map; fuse/S151 server giả `emu_choice`; không suy KR `confirmed`, không đổi RULES/profile.

Index chung: [docs/evidence/README.md](../../README.md) mục measurements (không sửa file index).
Canonical `VN_BATTLE_RUNTIME_REFERENCE` chờ PR242 merge/owner; chưa link để tránh sửa file dsh1.
