# VN player grace — runtime owner-only, 2026-10-24

Task [#240](https://github.com/aquaplatinumberlitz/boom-rs/issues/240), scope **VN runtime**. Build
đo là VN `VBNB_116`; đây là hai quan sát owner-only (run2/run3). Theo **quyết định user 2026-10-24**,
kết quả đo động VN dùng được làm nguồn luật gần ngang tĩnh KR; không bắt re-equivalence. Hai quan sát
này là **confirmed owner-only physical observation**; provenance fixture vẫn ghi, nhưng không hạ cấp
chỉ vì devirt/VN đã biết. Chưa đo non-owner/TTL và không suy non-owner.
Raw local (không commit): `.re-work/vn-grace-240/host-run-2/` và `host-run-3/`.

## Fixture / provenance

| Thành phần | Giá trị |
|---|---|
| Client | VN `VBNB_116`, `ca_fixed_patched.exe` SHA-256 `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, PE32 i386 |
| Source / rebuild | `/home/admin/boom-re/wine/Boom` (git HEAD `9ac2075`), copy riêng trong `.re-work/vn-grace-240/game`. Fixture là output chuỗi rebuild có **devirt gameplay reconstruction**, không chỉ offline/localhost; hash đầy đủ + patch scope bên dưới. |
| Map | `assets/fx/Map/Village01.map`, SHA-256 `1ca7e03ecb9b343fc54ae5d51dca006588cd2cd7e6f23cf212681dd66ff81b28`; version 1000, `W=15,H=13` |
| Spawn slot0 | `(col14,row0)` — map spawn list index 0/3; packet 111 x=14,y=0. **Không** dùng spawn `(13,1)` của KR |
| Server | fakeserver gửi 152/151 theo timer, **không tự mô phỏng collision/damage**; fuse **60 s = emu_choice**. Run2 stop trước nổ; run3 S151 gửi ids0..7 nhưng chỉ 1 bom được đặt |
| Flow | `501/189 mode1/190/189 mode0/191`; run2/run3 `ready.json` = `match_ready`, stages 10/11/12/13 |
| Raw packet | run2 `host-run-2/packets.log:195–197`; run3 `host-run-3/packets.log:163–165` |

### Rebuild / patch scope

```
unpacked_ca.exe  13a0a33cccc8f5f5124cdd8dac863b68118e6dda83340649b897fc41faed364a
ca_fixed.exe     c88a23c8dd886a6c80e06ea52a4399ad8ef34932bc7a25d06b804bf8a1c3750c
ca_devirt.exe    f7e89f8db031655ada9e5f4a6a0213d9da21fbc9bc2b54861947e0073b1e5080
ca_fixed_patched bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1
```

Direct byte check: **25/25** `patch_client` patches, **10/10** IP slots→`127.0.0.1`, **15/15** devirt
sites `E9` into `.devirt` in `ca_devirt`/fixture. `patch_client.py` (`e728f6b` lines 15–26/36–47) là
offline/anti-cheat + localhost + lobby ctor repair; `fix_iat.py` (`471ff80` line 366) là IAT/loader.
`devirt.py` (`d422f56`, BLOCKS line 30) có **gameplay/runtime reconstruction**: `0x6EFD4A` bomb msg-cache
(line 225), `0x5788C9` CBombOwner **plain-skip EXPERIMENT** (line 341), random guards
`0x5A6E89`/`0x6F287E`/`0x754C82` (lines 280/289/298; bomber/NPC/stage-clear), `0x736C27` map load
(line 209), `0x71581A` packet552 (line 307), `0x9682E5` stage init (line 165). Không có manifest:
**semantic reconstruction/equivalence chưa chứng minh**; ghi provenance để truy vết, không hạ cấp quan
sát theo quyết định user; không tuyên bố vanilla equivalence.

## Quan sát

| Ảnh / packet | Thời điểm | Trạng thái |
|---|---|---|
| `host-run-2/packets.log:195` C→S 111 | 10:39:04.867 | bomb record: `ctx0 slot0 serial0 y0 x14 power1`, r1c1, flag0 |
| `host-run-2/packets.log:196–197` S→C 152 | 10:39:04.867 | id1; bomb xuất hiện, **chưa 151** |
| `06-bomb.png` | sau 0.4 s | owner đứng trên ô bom `(14,0)`; bomb cùng ô; owner center ≈ (760,62) screen |
| `07-out-left.png` | hold Left 0.45 s + 0.2 s | owner sang trái ≈ (719,62); bomb vẫn ≈ (770,62); owner rời ô bom |
| `08-return-right.png` | hold Right 0.45 s + 0.2 s | owner vẫn ≈ (717,62), quay mặt phải; **không vào lại ô bom** trước 151 |
| Stop | command 2 `stop=true` | Không có `09-pre-stop` (stop trước actions); chỉ supervisor `DONE`; run2 không có 151/120 |

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

Map grid 15×13; packet xác nhận bomb cell `(14,0)`. Ảnh 1024×768 cho **screen approximate**:
ước cũ `51.5×52.1` đo cả border, **không phải calibration chính xác**; render native cell 40 px.
Owner visual bounds khoảng x≈760→719; y center≈62, feet≈78 (dễ nhầm center/feet). Chỉ dùng để nói
owner đổi `(14,0) → (13,0)`; bomb `(14,0)` từ packet. **Không** suy tọa độ native từ pixel.
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
  `0.45 s` hay 5 s làm TTL; **không claim timer ≤/>5 s** vì chưa đo thời điểm nhận 152/tick.
  Không claim all-player.
- Position: chưa có probe gdb vị trí player đã xác minh; `tools/gdb/spawn.gdb` chỉ in spawn, không
  phải player. Không dùng getter KR `0xB335C0/0xB335E0` cho VN khi chưa xác minh; chỉ screen approx +
  packet bomb cell `(14,0)`.

## Host probe độc lập

Supervisor: `/home/admin/boom-team/tmp/vn240-host.py` (read-only; không chép vào repo). Bản hiện tại
là **bản đã chỉnh cho run3**, không mô tả chung cho run2. Cả hai dùng Xvfb `:141`, prefix/game copy
riêng, fakeserver loopback `BOMB_PLACE=1`, `BOMB_ECHO=60` và cleanup terminate đúng PIDs con (không
`pkill`/`wineserver -k`). `ROUND_TIME` thực tế khác nhau: **run2 = 120**, **run3 = 600**; supervisor
file hiện chỉ còn bản run3. Run3 command: `space → wait0.4 → snap06 → wait5 → snap07 → Left0.45 →
wait0.2 → snap08 → Right0.45 → wait0.2 → snap09 → stop`. Repo chỉ giữ mô tả, không game binary/pack/
raw log.

## Raw / limitations

Raw ảnh/packet/supervisor nằm local `.re-work/vn-grace-240/host-run-2/` và `host-run-3/`; repo không chứa binary,
game media, pack, DB hoặc ảnh thô. Đây là `vn_runtime` owner-only; không thay RULES/KR claim.

Raw hashes (SHA-256 đầy đủ, tính sau khi host stopped/đóng file):
- run2: `06-bomb.png` `e751354892cee5e357f4c2540f567f000381439030fd3ad7b914658d92afd796`;
  `07-out-left.png` `eceae22b8f16c9ce2d2041f992a2a9223b4cabcdf2a6bf752633da9595386b5b`;
  `08-return-right.png` `f24bde2b9e1f317c2283f9e2bbcf1dfa323a5c22ea5a04774ccbc683f336e9eb`;
  `packets.log` `ad9ae6ed7898f3e55425aba17ecf026f2c850a59b6eb53add27936c5f1444c62`;
  `supervisor.log` `ea14966929273482f641326a0f84704ea0eeab61f2e93cbe07e1da1fcd16bc3b`;
  `ready.json` `73ccd66d98a3919488c73cba292435c767775c3d5f438f6997c551adee0e73ed`.
- run3: `06-bomb.png` `82e3f58ef872d79504f903f6dbd02052a185b1c695135f8be832c18fcb611029`;
  `07-dwell-five-seconds.png` `3429440d366461c90e05cec733c1576b890ce2363eabf277d505b9c0a7b51497`;
  `08-out-after-dwell.png` `99aedf0768411647580c525859e2d411698cf425437ebe84d2427104dd19c5a7`;
  `09-return-blocked.png` `61d829458b89fdba0fa3fe45807006feffd7b989872502d636d841994e61ebbe`;
  `packets.log` `3ff879fde09f539f1a883de4f99164bec806bde8d01d96d4752216de388712df`;
  `supervisor.log` `65433b4bcfe76f351ffae36b35e9727aab6100549b5a5fc9e83b83e832046977`;
  `ready.json` `b93acb7b32c03f496fe6776c68a8581c4815c9c30b50b40e3d4d721ff1dc1d0d`.
- Root clock trong supervisor/ack là `monotonic_ns` của command/snapshot, **không phải fuse/TTL**;
  S151 ~60 s là giá trị emu_choice. Map row1 col13 là brick; run3 trap/death sau 151 không phải timer proof.
