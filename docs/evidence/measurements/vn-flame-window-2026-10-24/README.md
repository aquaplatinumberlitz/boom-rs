# VN flame window — đo runtime offline (PARTIAL, chưa kết luận một lần vs cửa sổ)

Trạng thái: **partial**, PR metadata (không claim luật). Task #240 (FLAME), dsh2; đo ngày 2026-10-24.
VN là nguồn tin cậy theo AGENTS §4; đây là build VN đo được, không phải vùng untrusted. Tuy nhiên
fixture có devirt/patch nên hành vi dưới đây vẫn là **candidate**, không phải luật KR `confirmed`.

## Điều kiện đo
- Client VN offline `ca_fixed_patched.exe` (clone1 `.re-work/vn-grace-240/game/`), SHA-256
  `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`.
- Map Ocean01 index 672 chọn từ pack (map authentic; **chọn map/server là `emu_choice`**); spawn slot0
  thực tế `(1,0)`; owner `(2,0)` lúc nổ; bom `(0,0)`.
- Server private patch `/home/admin/boom-team/tmp/vn240-flame-server.py`: đúng **một** `151` cho id1 sau 8 s
  (fuse 8 s = `emu_choice`), **không** gửi collision/damage.
- Namespace loopback-only + PID namespace; game/prefix clone1; raw/ảnh giữ local, không public.

## Provenance fixture (candidate, chưa đủ)
`patch-audit.md` (clone1): original `13a0a33c…`, `ca_fixed` `c88a23c8…`, devirt `f7e89f8d…`, fixture
`bc875ba5…`; 25 patch offline/localhost + 10 IP `127.0.0.1` byte-proven. **Nhưng** `.devirt` thay đổi
đường gameplay (CGameMsgCache ring setter, `CGameStage__OnPacket` 552, `CGameStage::Init`, map load,
random guards, **`CBombOwner__vf33` 0x5788C9 EXPERIMENT plain skip**); không có manifest/checksum tracked;
**semantic equivalence chưa chứng minh** ⇒ mọi quan sát là **candidate**.

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

## Quan sát ảnh từng run
- visible-early: `09-flame-before-input` — owner vẫn `(2,0)`, **chưa trapped**; không nhận ra art lửa rõ
  (bom `(0,0)` bị cây decorative che). `10-late-enter` — trapped.
- late-700: `09-before-input` — owner `(2,0)`, chưa trapped; `10-late-enter` — ảnh sau cho ở `(1,0)`, **alive**
  (thời điểm vào ô chưa đo); `11-after-enter` — vẫn **alive**, không `120/11`.
- late-050/300: `09-late-enter` — trapped; `10-after-enter` — vẫn trapped. Không nhận dạng được art lửa
  trong ảnh nên **không** kết luận “lửa đã hết”.
- Ảnh sau chỉ cho thấy trapped/report **không lặp trong cửa sổ quan sát**; **không** suy được hit-once vs
  repeated khi đã trapped (trapped có thể chặn report/damage follow-up).

## Kết luận partial
- **Không** đặt upper/lower bound native TTL/cửa sổ từ input/report. Bảng trên chỉ là delay quan sát.
- Ba run trap sau khi vào ô; late-700 **gửi lệnh Left** ở +858.1 ms (KEYDOWN), ảnh sau cho ở `(1,0)`
  vẫn alive, không `120/11`; **thời điểm vào ô chưa đo**. Đây là **quan sát**, không suy bound/TTL.
- Chưa có client-receive/`t_enter`; `09` visible-early không chứng minh lửa đã hiện trước input ⇒
  **chưa loại trừ one-shot xử lý trễ**; chưa có TTL/one-shot vs window conclusive.
- Không suy fire 500 ms/2500 ms.

## Probe GDB (thất bại, không claim handler)
`host-flame-handler-return/gdb.log`: breakpoint `0x7324c0` →
`Cannot insert breakpoint -14. Cannot access memory at address 0x7324c0`;
`probe.gdb:24: Error in sourced command file: Command aborted.`;
`[Inferior 1 (process 8) detached]`. **Không hit entry/return**; địa chỉ/WOW64 chưa xác minh; Codex đã
stop namespace, không retry. **Không** claim handler/`151`-return/receipt.

## Bước tiếp (không loop)
Runtime hook đúng module/VN native address (không quét VM) hoặc audit semantic fixture devirt để chứng minh
client đã xử lý nổ trước input; nếu không, giữ partial với các quan sát trên.

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

## Giới hạn
- Không có clock hook client-receive; input time là lúc supervisor gửi phím, không phải lúc vào ô.
- Ảnh không timestamp từng frame; capture interval thực tế ~157–180 ms; `(0,0)` bị cây che.
- Fixture có devirt gameplay + experiment skip; semantic equivalence chưa chứng minh ⇒ candidate.
- Một client/map; fuse/S151 server giả `emu_choice`; không suy KR `confirmed`, không đổi RULES/profile.

Index chung: [docs/evidence/README.md](../../README.md) mục measurements (không sửa file index).
Canonical `VN_BATTLE_RUNTIME_REFERENCE` chờ PR242 merge/owner; chưa link để tránh sửa file dsh1.
