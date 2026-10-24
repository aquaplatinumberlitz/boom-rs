# VN player grace — runtime owner-only, 2026-10-24

Task [#240](https://github.com/aquaplatinumberlitz/boom-rs/issues/240), scope **VN runtime**. Đây là
một quan sát owner-only; **không** phải timer/grace toàn hệ thống và không dùng làm luật KR.
Raw local (không commit): `.re-work/vn-grace-240/host-run-2/`.

## Fixture / provenance

| Thành phần | Giá trị |
|---|---|
| Client | VN `VBNB_116`, `ca_fixed_patched.exe` SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, PE32 i386 |
| Source copy | `/home/admin/boom-re/wine/Boom` (git HEAD `9ac2075`), copy riêng trong `.re-work/vn-grace-240/game` |
| Map | `assets/fx/Map/Village01.map`, SHA-256 `1ca7e03ecb9b343fc54ae5d51dca006588cd2cd7e6f23cf212681dd66ff81b28`; version 1000, `W=15,H=13` |
| Spawn slot0 | `(col14,row0)` — map spawn list index 0/3; packet 111 x=14,y=0. **Không** dùng spawn `(13,1)` của KR |
| Server | fakeserver chỉ ack 111→152, không tự tính collision/damage; fuse **60 s = emu_choice** |
| Flow | `501/189 mode1/190/189 mode0/191`; `host-run-2/ready.json` = `match_ready`, stages 10/11/12/13 |
| Raw packet | `host-run-2/packets.log:195–197` |

## Quan sát

| Ảnh / packet | Thời điểm | Trạng thái |
|---|---|---|
| `packets.log:195` C→S 111 | 10:39:04.867 | bomb record: `ctx0 slot0 serial0 y0 x14 power1`, r1c1, flag0 |
| `packets.log:196–197` S→C 152 | 10:39:04.867 | id1; bomb xuất hiện, **chưa 151** |
| `06-bomb.png` | sau 0.4 s | owner đứng trên ô bom `(14,0)`; bomb cùng ô; owner center ≈ (760,62) screen |
| `07-out-left.png` | hold Left 0.45 s + 0.2 s | owner sang trái ≈ (719,62); bomb vẫn ≈ (770,62); owner rời ô bom |
| `08-return-right.png` | hold Right 0.45 s + 0.2 s | owner vẫn ≈ (717,62), quay mặt phải; **không vào lại ô bom** trước 151 |
| Stop | command 2 | `09-pre-stop` + stop; không có 151/120 trong run |

### Grid / scale (actual VN render)

Map grid 15×13; packet xác nhận bomb cell `(14,0)`. Ảnh 1024×768 có playfield xấp xỉ
`x=22..794`, `y=36..713`; 15 cột/13 hàng ⇒ cell render ≈`51.5×52.1` px (native map cell 40 px).
Owner screen center đổi từ ≈`(760,62)` sang ≈`(719,62)`, cell thay đổi `(14,0) → (13,0)`.
Bomb vẫn ≈`(770,62)`. Đây là **grid metadata**, không phải tọa độ native pixel tuyệt đối.
Map cell `(13,0)` = tile1005/obj996 (floor); `(14,1)` = tile1005/obj996; `(12,0)` có tree
(obj khác) nên **không** dùng Right/Up từ corner.

## Kết luận / gap

- **Owner-only, một run:** owner rời ô bom được (Left) và **không vào lại** ô bom khi quay phải,
  trước 151; input Right có tác dụng (owner facing right). Không suy được TTL ms, grace toàn hệ
  thống, hay non-owner từ run này.
- Chưa có `151`/`120`, nên không kết luận trap/damage sau nổ; fuse 60 là emu_choice.
- Cần Codex repeat: đứng chờ trên bom 5 s → out → return, cùng map/fixture; và non-owner đứng chồng
  ô lúc 152 nếu cần. Không dùng `0.45 s` làm TTL.
- Position: chưa có probe gdb vị trí player đã xác minh; `tools/gdb/spawn.gdb` chỉ in spawn, không
  phải player. Đề xuất Codex dùng screenshot grid đã hiệu chuẩn hoặc xác minh getter
  `0xB335C0/0xB335E0` trên đúng bomber object trước khi dùng số tuyệt đối.

## Raw / limitations

Raw ảnh/packet/supervisor nằm local `.re-work/vn-grace-240/host-run-2/`; repo không chứa binary,
game media, pack, DB hoặc ảnh thô. Đây là `vn_runtime` owner-only; không thay RULES/KR claim.

Raw hashes: `06-bomb.png` `e7513548…d796`, `07-out-left.png` `eceae22b…5b`, `08-return-right.png`
`f24bde2b…e9eb`, `packets.log` `ad9ae6ed…4c62`, `supervisor.log` `ea149669…c3b`, `ready.json`
`73ccd66d…73ed`. Map row1 col13 là brick; run dừng trước 151 nên **không** có trap/damage evidence.
