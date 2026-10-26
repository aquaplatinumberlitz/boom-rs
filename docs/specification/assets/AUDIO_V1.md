# Âm thanh V1 — event→entry, FxSound và chính sách chồng tiếng

Nguồn chuẩn mapping: [audio-v1.toml](../../../data/assets/audio-v1.toml), V1-034/#363.
Claim: [RULES_V1 §78 R-191](../gameplay/RULES_V1.md#78-f-12--mapping-âm-thanh-theo-consumer-kr-r-191-2026-10-26),
R-203…R-208 và R-213; [KR_RE_AUDIO](../../research/findings/KR_RE_AUDIO.md).
Q-015 trong [QUESTIONS](../../research/QUESTIONS.md) giữ các phần chưa xác nhận.

## 1. Nguồn và giới hạn

Pack KR00967 `FxSound` có797 entry (157 `MP3/`,640 `Wave/`), payload Ogg/Vorbis thô;
`MP3/` chỉ là tên thư mục. Consumer gọi dispatcher `0xD1CD10`, tra bảng id→entry `0x19E40D8`.
R-191 là `kr_static/candidate`: chưa đo âm thanh runtime. `player_trapped` giữ
`confirmed` theo consumer solo R-204 đã được nghiệm thu, không hạ nhãn theo claim tổng R-191.
BGM entry/map `MP3/Village` confirmed R-081; loop native có R-213, nhưng policy một voice,
không restart khi reconcile của V1 vẫn design R-083. Mapping UI click chưa có consumer, giữ design.

Bảng V1 hiện có **14 entry (1 BGM +13 SFX)**, tất cả có trong pack KR và đóng gói được (§5).
Ba entry mới so với bundle cũ15entry là `Cha_BombExplode`, `Cha_BombExplodeMulti`,
`Ntc_GameReady`: chúng **nằm ngoài subset audio đã dựng trước #363**, cần dựng lại bundle.
Bỏ `BombExplosion001`471 và `Ntc_Count1/2/3` khỏi subset mới. Không suy không có consumer ở
mọi đường: R-191 chỉ không thấy direct471/Count trong597 site đã khảo sát.
Lần đối chiếu cũ15entry KR/VN có hash trùng; lần này đo14entry trên KR, chưa đối chiếu lại ba entry mới ở VN.
Không commit payload âm thanh; artifact nằm local `/tmp/boom-audio-evidence/`.

## 2. BGM

`bgm_loop` dùng `MP3/Village` (id48 theo record map; R-070/R-081), loop một voice riêng,
không restart khi reconcile hoặc đổi lượt (policy design R-083). CallbackEOS native R-213
có static evidence; không coi đó là phép nghe hoặc xác nhận runtime volume/loop.

## 3. Bảng event→entry V1

Mapping có nguồn consumer R-191 chuyển từ design sang **candidate**, giữ entry đã khớp.
Chỉ trapped giữ confirmed R-204; voice/priority/coalesce và lịch warmup V1 không được nâng theo mapping.
Core event đi qua consumer chung offline/mạng; Ready/Start là cue trình diễn.

| Event/cue | Entry | Nhãn | Claim / giới hạn |
|---|---|---|---|
| `bgm_loop` | `MP3/Village` | `design` | R-081, R-083; entry/mapping confirmed R-081; loop một voice là design R-083 |
| `bomb_placed` | `Wave/Cha_BombIgnite` | `candidate` | R-191; id9; biến thể tuyết344 ngoài V1; R-191 candidate |
| `bomb_exploded` | `Wave/Cha_BombExplode` | `candidate` | R-191; id7; xen kẽ với id8 trong200ms theo dispatcher R-191; lần đầu/sau khoảng nghỉ dùng8 |
| `bomb_exploded_multi` | `Wave/Cha_BombExplodeMulti` | `candidate` | R-191; id8; lần đầu/sau200ms và xen kẽ7→8 trong200ms, R-191 candidate |
| `item_revealed` | `Wave/Itm_ItemPop` | `candidate` | R-191; id69, consumer0xB8B680; R-191 candidate |
| `item_picked` | `Wave/Itm_ItemEaten` | `candidate` | R-191; id68 dùng chung Bomb/Fire/Roller, consumer0xB4B410; R-191 candidate |
| `player_trapped` | `Wave/Cha_LockedIn` | `confirmed` | R-203, R-204; consumer solo 0xB42D60 phát id 77 Cha_LockedIn (R-204); nhánh couple 37 ngoài scope FFA; không phát 444 help + voice/priority/dedup/loop là policy/design V1 R-083, không phải kết luận KR/FFA không có help |
| `player_died` | `Wave/Cha_Die` | `candidate` | R-191; id42, consumer0xB53C80; R-191 candidate |
| `round_ended_win` | `Wave/Ntc_Win` | `candidate` | R-191; id175; giữ entry đã khớp; remap có guard chưa áp trong V1 |
| `round_ended_lose` | `Wave/Ntc_Lose` | `candidate` | R-191; id78; giữ entry đã khớp; remap có guard chưa áp trong V1 |
| `round_ended_draw` | `Wave/Ntc_Draw` | `candidate` | R-191; id43; giữ entry đã khớp; remap có guard chưa áp trong V1 |
| `round_start` | `Wave/Ntc_GameStart` | `candidate` | R-191; id21 GameStart, R-191 candidate; remap có guard chưa áp |
| `round_ready` | `Wave/Ntc_GameReady` | `candidate` | R-191; id18 GameReady, R-191 candidate; phát một lần đầu warmup, không phát Count3/2/1 |
| `ui_click` | `Wave/Bt_p_Default` | `design` | R-082; click UI mặc định; chưa có trace consumer gốc |

`player_turned`, `block_destroyed`, `item_destroyed` chưa chọn entry, giữ im lặng (Q-015).
Tuyết343/344, couple37, help444 có điều kiện, trap174 và remap Win/Lose/Draw/Ready/Start có guard
`sub_76FBF0` nằm ngoài policy V1 hiện hành; không suy sự vắng mặt ở V1 thành không tồn tại native.

## 4. Phát cue và chồng tiếng

- **Nổ R-191 candidate:** lần đầu hoặc sau khoảng nghỉ≥200ms chọn id8 (`bomb_exploded_multi`);
  trong<200ms xen kẽ8→7→8. Tính từ tick event với tần số profile, sort EventId khi batch mạng
  dồn nhiều tick; event lặp không đổi pha. Cả hai cue phải có trong bundle, thiếu thì giữ im lặng
  cho tới khi dựng lại, không dùng471 thay thế. Hai biến thể cùng ưu tiên nổ.
- Native kiểm voice đang phát và có nhánh Stop7/Stop8. V1 dùng tick của event để chọn biến thể;
  chưa mô phỏng native Stop/voice query, vẫn giữ policy R-083. Đây là giới hạn fidelity audio,
  không gọi regression test là xác nhận đã nghe/khớp client gốc.
- **Ready/Start:** phát GameReady18 một lần khi vào warmup, GameStart21 một lần khi tick đạt
  `warmup_ticks` của core. Client vào muộn sau warmup chỉ phát Start. Không phát Count3/2/1.
  Entry theo R-191 candidate; lịch warmup theo core V1/R-113 là policy design, chưa đo nhịp native.
- Dedup theo `(round_id, EventId)`; gộp cùng **cue** trong tick. Hai biến thể7/8 có thể cùng phát
  khi nhiều bom nổ cùng tick. BGM có một voice riêng; SFX tối đa8, ưu tiên round-end/start → nổ →
  trapped/died → bomb/item → UI; hòa giữ EventId nhỏ hơn. Đây vẫn là design R-083,
  không thay bằng native pool≤16 (R-212) hay suy policy native từ asset.
- `AudioMix` áp `master × {bgm,sfx}` và mute. Builder chỉ đóng gói ogg/wav, bỏ định dạng không
  hỗ trợ kèm log. `MacroquadAudio` nạp từ bundle, không đọc pack; lỗi file có log.

## 5. Hash payload KR hiện hành

Đo bằng `boom-assets audio /home/admin/boomkr-full --list`; SHA-256 payload sau giải mã pack,
không phải hash file âm đã decode. 14entry, tổng **457.183 byte**; cả ba entry mới có trong pack gốc.
Hash bundle schema4 không Việt hóa (baseline `cf76608`, chỉ PR audio) là
`84603280e035b7bb44aeb0b70a25f69b306162c8766dac5da68504c8676ffff3`.
Bundle lửa #330 độc lập đổi hash riêng; khi tích hợp cả hai phải dựng lại và cập nhật hash chung.

| Entry | Kind | Format | Byte | SHA-256 |
|---|---|---|---:|---|
| `MP3/Village` | bgm | ogg | 270601 | `7a34054f23a66a1f19cd1cd47b70705bda0c774b6d2afd094c4bd5b6d709038f` |
| `Wave/Bt_p_Default` | sfx | ogg | 4574 | `20eeb969ad7f7cd1504aeef1b76eb7196c228648fe93f9e2b38458aca220bec0` |
| `Wave/Cha_BombExplode` | sfx | ogg | 9772 | `b47d3180c67cc33446c668a265b8fba574eaef751fae2c50b8352844f69ebaeb` |
| `Wave/Cha_BombExplodeMulti` | sfx | ogg | 9777 | `f093b10a1b5985db8a9a7ac3fb1457373b3639603b71731da39eb57d99e4d276` |
| `Wave/Cha_BombIgnite` | sfx | ogg | 5119 | `9964e974bfcba4bbafaf2f7a996da3c912f4bfb2995032518645ec6ed3fcaebe` |
| `Wave/Cha_Die` | sfx | ogg | 8858 | `96ad61a176dae1205d8d6a791ce83320785a66e2a4c09fbb867e7f236fd29fd6` |
| `Wave/Cha_LockedIn` | sfx | ogg | 6810 | `10249aa325d55f8b8623c3ffc81f864619c580074d627215f0786f2512427606` |
| `Wave/Itm_ItemEaten` | sfx | ogg | 8520 | `1c829ae9a499ceba0e50f7c4e584a09d19c590fdcc11e03b4fc4abca473ac266` |
| `Wave/Itm_ItemPop` | sfx | ogg | 5664 | `acc3b99bb749b06884307214a886a923b8aec3e1115c9ade482686d00f5fd45c` |
| `Wave/Ntc_Draw` | sfx | ogg | 29125 | `86e37e95aded7325163218915d6ac797c8b67a387301cb2285484d7e0477b295` |
| `Wave/Ntc_GameReady` | sfx | ogg | 11177 | `ebb451afd77e8c05186e225ee931c52c7256fc060558a0f4f2e1318ba81500ce` |
| `Wave/Ntc_GameStart` | sfx | ogg | 12418 | `8566bc99cf2ad83e0c939c0994a23edf6f55443932c7d5c8bfdd3c760798cc64` |
| `Wave/Ntc_Lose` | sfx | ogg | 44428 | `ed474eff4a74ae3c4a480f53856ea1f0f7130cb25589e692c0d5c6aa32f8bd56` |
| `Wave/Ntc_Win` | sfx | ogg | 30340 | `d5c2096a2acc884640185a44088c6f992c264097ba69b8ba5999a0904691df47` |

## 6. Dựng và kiểm lại

```sh
CARGO_TARGET_DIR=/tmp/boom-audio-target BOOM_KR_PACK_DIR=/home/admin/boomkr-full \
  cargo run -p boom-assets --bin boom-assets -- bundle --map Village00 --out /tmp/boom-audio-bundle
CARGO_TARGET_DIR=/tmp/boom-audio-target \
  cargo run -p boom-assets --bin boom-assets -- audio /home/admin/boomkr-full --list
```

`audio --list --all` liệt kê797entry; `--extract <dir>` xuất raw. CLI audio chưa hỗ trợ `--build vn`:
truyền cờ đó bị từ chối rõ, không đoán stem/khóa VN.

## 7. Còn thiếu

Q-015 giữ runtime audible/volume/voice/loop, native Stop7/8 và remap có guard, các event chưa chọn
entry và variant ngoài FFA V1. Test chỉ kiểm mapping, pha200ms, dedup/round reset, Ready/Start và
bundle thật; chưa nghe đối chiếu KR/VN, không ký fidelity/G5 từ CI xanh.
