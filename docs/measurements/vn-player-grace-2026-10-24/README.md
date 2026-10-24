# VN player grace — runtime owner-only, 2026-10-24

Task [#240](https://github.com/aquaplatinumberlitz/boom-rs/issues/240), scope **VN runtime**. Đây là
hai quan sát owner-only (run2/run3); **không** phải timer/grace toàn hệ thống và không dùng làm luật KR.
Raw local (không commit): `.re-work/vn-grace-240/host-run-2/` và `host-run-3/`.

## Fixture / provenance

| Thành phần | Giá trị |
|---|---|
| Client | VN `VBNB_116`, `ca_fixed_patched.exe` SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, PE32 i386 |
| Source copy | `/home/admin/boom-re/wine/Boom` (git HEAD `9ac2075`), copy riêng trong `.re-work/vn-grace-240/game` |
| Map | `assets/fx/Map/Village01.map`, SHA-256 `1ca7e03ecb9b343fc54ae5d51dca006588cd2cd7e6f23cf212681dd66ff81b28`; version 1000, `W=15,H=13` |
| Spawn slot0 | `(col14,row0)` — map spawn list index 0/3; packet 111 x=14,y=0. **Không** dùng spawn `(13,1)` của KR |
| Server | fakeserver chỉ ack 111→152, không tự tính collision/damage; fuse **60 s = emu_choice** |
| Flow | `501/189 mode1/190/189 mode0/191`; run2/run3 `ready.json` = `match_ready`, stages 10/11/12/13 |
| Raw packet | run2 `host-run-2/packets.log:195–197`; run3 `host-run-3/packets.log:163–165` |

## Quan sát

| Ảnh / packet | Thời điểm | Trạng thái |
|---|---|---|
| `packets.log:195` C→S 111 | 10:39:04.867 | bomb record: `ctx0 slot0 serial0 y0 x14 power1`, r1c1, flag0 |
| `packets.log:196–197` S→C 152 | 10:39:04.867 | id1; bomb xuất hiện, **chưa 151** |
| `06-bomb.png` | sau 0.4 s | owner đứng trên ô bom `(14,0)`; bomb cùng ô; owner center ≈ (760,62) screen |
| `07-out-left.png` | hold Left 0.45 s + 0.2 s | owner sang trái ≈ (719,62); bomb vẫn ≈ (770,62); owner rời ô bom |
| `08-return-right.png` | hold Right 0.45 s + 0.2 s | owner vẫn ≈ (717,62), quay mặt phải; **không vào lại ô bom** trước 151 |
| Stop | command 2 | `09-pre-stop` + stop; không có 151/120 trong run |

### Case 2 — host-run3, đứng chờ 5 s

`host-run-3/supervisor.log` command: `space → wait 0.4 → snapshot 06-bomb → wait 5 →
snapshot 07-dwell-five-seconds → hold Left 0.45 → wait 0.2 → snapshot 08-out-after-dwell →
hold Right 0.45 → wait 0.2 → snapshot 09-return-blocked → stop`.

| Ảnh / packet | Quan sát |
|---|---|
| `host-run-3/packets.log:163–165` | 10:41:07.362 C→S 111 `ctx0 slot0 serial0 y0 x14 power1`; S→C 152 id1 |
| `07-dwell-five-seconds.png` | owner vẫn đứng nguyên trên ô bom `(14,0)` sau 5 s; 152 còn hiệu lực |
| `08-out-after-dwell.png` | Left 0.45 s: owner sang `(13,0)`; bomb vẫn `(14,0)` |
| `09-return-blocked.png` | Right 0.45 s: owner vẫn `(13,0)`, quay mặt phải; không vào lại bomb |
| `host-run-3/packets.log:214` | 10:42:07.367 S→C 151 (sau ~60 s; **emu_choice**) |
| `host-run-3/packets.log:219` | 10:42:07.389 C→S 120 subtype **11** (trap) |
| `host-run-3/packets.log:231/235–236` | C→S 162 rồi C→S 120 subtype **13** (death); client tự báo, server không ép |

Run3 lặp lại kết quả run2: đứng chờ 5 s **không** làm mất quyền rời bomb trong case owner; quay lại
vẫn bị chặn. 151/120 xảy ra sau khi đã rời bomb; không dùng để suy timer.

### Grid / scale (actual VN render)

Map grid 15×13; packet xác nhận bomb cell `(14,0)`. Ảnh 1024×768 có playfield xấp xỉ
`x=22..772`, `y=36..686`; 15 cột/13 hàng ⇒ cell render ≈**50×50** px (native map cell 40 px).
Owner screen center đổi từ ≈`(760,62)` sang ≈`(719,62)`, cell thay đổi `(14,0) → (13,0)`.
Bomb vẫn ≈`(770,62)`. Render scale ≈**50 px/cell** (native cell 40); đây là **grid metadata/approx screen**, không phải tọa độ native tuyệt đối.
Map cell `(13,0)` = tile1005/obj996 (floor); `(14,1)` = tile1005/obj996; `(12,0)` có tree
(obj khác) nên **không** dùng Right/Up từ corner.

## Kết luận / gap

- **Owner-only, hai run:** run2 (rời/return trước 151) và run3 (đứng chờ 5 s, rời/return trước 151)
  đều cho owner rời ô bom được và **không vào lại** khi quay phải; input Right có tác dụng (facing
  right). Không suy được TTL ms, grace toàn hệ thống, hay non-owner từ hai run này.
- Run3 có `151` sau ~60 s, rồi client tự báo `120/11` (trap) và `120/13` (death); server không ép
  trap trong run. Đây là quan sát sau khi owner đã rời bomb, không phải bằng chứng timer/grace. Fuse 60
  là emu_choice. Nếu để nổ tại `(14,0)` khi owner ở `(13,0)`, map row1 col13 là brick nên **first
  probe có thể bị trapped sau nổ** nếu không đổi map/spawn.
- Run3 đã repeat đứng chờ 5 s → out → return; **còn non-owner đứng chồng ô lúc 152**. Không dùng
  `0.45 s` hay 5 s làm TTL; không claim all-player.
- Position: chưa có probe gdb vị trí player đã xác minh; `tools/gdb/spawn.gdb` chỉ in spawn, không
  phải player. Đề xuất Codex dùng screenshot grid đã hiệu chuẩn hoặc xác minh getter
  `0xB335C0/0xB335E0` trên đúng bomber object trước khi dùng số tuyệt đối.

## Host probe độc lập

Supervisor dùng trong hai run: `/home/admin/boom-team/tmp/vn240-host.py` (read-only; không chép vào
repo). Script tạo Xvfb `:141` + prefix/game copy riêng, fakeserver loopback với `BOMB_PLACE=1`,
`BOMB_ECHO=60`, `ROUND_TIME=600`, chờ stage10/11/12/13, ghi `ready.json`, rồi đọc `command.json` để
snap/hold key; cleanup terminate đúng PIDs con, không `pkill`/`wineserver -k`. Run3 command:
`space → wait0.4 → snap06 → wait5 → snap07 → Left0.45 → wait0.2 → snap08 → Right0.45 → wait0.2 →
snap09 → stop`. Đây là probe độc lập của Codex host; repo chỉ giữ mô tả, không game binary/pack/raw
log.

## Raw / limitations

Raw ảnh/packet/supervisor nằm local `.re-work/vn-grace-240/host-run-2/` và `host-run-3/`; repo không chứa binary,
game media, pack, DB hoặc ảnh thô. Đây là `vn_runtime` owner-only; không thay RULES/KR claim.

Raw hashes run2: `06-bomb.png` `e7513548…d796`, `07-out-left.png` `eceae22b…5b`, `08-return-right.png`
`f24bde2b…e9eb`, `packets.log` `ad9ae6ed…4c62`. Run3: `06-bomb.png` `82e3f58e…1029`,
`07-dwell-five-seconds.png` `3429440d…1497`,
`08-out-after-dwell.png` `99aedf07…c5a7`, `09-return-blocked.png` `61d82945…ebbe`, `packets.log`
`3ff879fd…12df`, `supervisor.log` `65433b4b…6977`, `ready.json` `b93acb7b…1d0d`. Map row1 col13
là brick; run3 có trap/death sau 151 nhưng không phải timer proof.
