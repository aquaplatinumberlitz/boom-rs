# Âm thanh V1 — event→entry, FxSound và chính sách chồng tiếng

Phạm vi **V1-034 phần 1+2 (issue #64, #104)**. Tài liệu chốt bảng event→entry, policy chồng tiếng,
cách đóng gói payload thô vào bundle schema 4 và cách client áp volume/mute. Nguồn chuẩn claim:
[RULES_V1](../gameplay/RULES_V1.md) §10 (R-081…R-083); câu hỏi còn mở:
[QUESTIONS](../../research/QUESTIONS.md) Q-015.

## 1. Nguồn và giới hạn bằng chứng

- Pack KR 00967 `FxSound` (đọc bằng `boom-assets`): **797 entry**, gồm 157 `MP3/` và 640 `Wave/`;
  mọi payload đọc được là **Ogg/Vorbis thô**. Tên thư mục `MP3/` là tên pack gốc, không phải định
  dạng payload.
- Bản VN đã giải nén tại `/home/admin/boom-re/assets/fx` có 124 `MP3/` + 492 `Wave/`; 15 entry được
  chọn cho V1 có mặt ở cả hai build và **SHA-256 trùng nhau** (bảng §2). Đây là bằng chứng asset,
  không phải bằng chứng event nào phát entry nào.
- Chuỗi client VN có `Wave/%s`, `Wave/Fanfare%03d`, `BombExplosion%03d` và tên các entry
  (`/home/admin/boom-re-l10n/ca_strings.txt`; bản `/home/admin/boom-re/ca_strings.txt` tương ứng).
  Không tìm thấy bảng event→entry hoặc trace `PlaySound` của consumer gốc trong phạm vi đã đọc.
- Tool l10n `kr_v1_content.py` chỉ chốt `music` = `FxSound/MP3/Village` (và `explosion_art` =
  `FxDesktop/Bomb/BombExplosion001`, không phải SFX); `idf.py` chỉ đọc/giải mã pack, không có bảng
  event→entry. `boom-re/docs/features.md` A10 (audio/BGM/SFX) vẫn ghi "chưa điều tra"; legacy server không chứa
  quyết định phát SFX (server chỉ gửi event gameplay). Vì vậy **R-081 chỉ confirmed entry/mapping BGM**; loop/một voice và mapping
  event→entry, policy chồng tiếng là **design** (R-083/R-082), không nhận `confirmed` KR/VN.
- `boom-assets` chỉ đọc/ghi payload thô và hash; không decode Ogg/MP3/WAV. Không commit file âm thanh;
  output đi vào `local/` khi kiểm tay.
- Bundle schema 4 mang `[[audio]]` + file `audio/<id>.<format>`; `content_hash` gồm metadata audio và
  SHA-256 payload. Client dùng `boom_content::bundle::AudioAsset`, không đọc pack.

## 2. BGM

| Dùng | Entry | Định dạng thô | Byte | SHA-256 | Nguồn / trạng thái |
|---|---|---|---:|---|---|
| Entry/mapping BGM Village | `MP3/Village` | ogg | 270601 | `7a34054f23a66a1f19cd1cd47b70705bda0c774b6d2afd094c4bd5b6d709038f` | `kr_static` + `vn_static` / confirmed (R-070, R-081; chỉ entry/mapping) |

Loop một bản, không tính vào số voice SFX và không restart khi reconcile là **design R-083**, không
phải bằng chứng asset; chi tiết policy ở §4.

## 3. Bảng event→entry V1

Nhãn `design` trong bảng có nghĩa: entry có thật (đã kiểm hash ở §2), nhưng **chưa có bằng chứng
consumer gốc** chọn entry đó cho event; đây là quyết định V1-034. `unknown` là chưa chọn được entry,
giữ im lặng tạm thời và theo dõi ở Q-015. Event core lấy từ `boom_core::EventKind`; các dòng
`start_*`/`ui_click` là event trình diễn UI, không phải `EventKind`.

| Event V1 | EventKind / nguồn | Entry | Nhãn | Ghi chú |
|---|---|---|---|---|
| `bgm_loop` | BGM theo map (R-070) | `MP3/Village` | `design` | Entry/mapping confirmed R-081; loop policy design R-083 |
| `bomb_placed` | `EventKind::BombPlaced` | `Wave/Cha_BombIgnite` | `design` | Tên fuse/ignite; chưa có trace consumer |
| `bomb_exploded` | `EventKind::BombExploded` | `Wave/BombExplosion001` | `design` | Pack có `001…032`; chưa rõ biến thể, `chained` dùng cùng entry |
| `item_revealed` | `EventKind::ItemRevealed` | `Wave/Itm_ItemPop` | `design` | Item lộ ra khỏi khối |
| `item_picked` | `EventKind::ItemPicked` (Bomb/Fire/Roller) | `Wave/Itm_ItemEaten` | `design` | Một entry chung; chưa có entry riêng cho ba item V1 |
| `player_trapped` | `EventKind::PlayerTrapped` | `Wave/Itm_UseTrap` | `design` | Ứng viên trap/bong bóng; **chưa xác nhận** đây là tiếng bị nhốt |
| `player_died` | `EventKind::PlayerDied` | `Wave/Cha_Die` | `design` | Entry chết chung; chưa có trace consumer |
| `round_ended_win` | `EventKind::RoundEnded { Win }` | `Wave/Ntc_Win` | `design` | Có thêm `MP3/Ntc_Win`; V1 chọn bản Wave |
| `round_ended_lose` | `EventKind::RoundEnded { Lose }` | `Wave/Ntc_Lose` | `design` | Có thêm `MP3/Ntc_Lose`; V1 chọn bản Wave |
| `round_ended_draw` | `EventKind::RoundEnded { Draw }` | `Wave/Ntc_Draw` | `design` | Có thêm `MP3/Ntc_Draw`; V1 chọn bản Wave |
| `start_count_3` | UI/round start | `Wave/Ntc_Count3` | `design` | Đếm 3→2→1→go |
| `start_count_2` | UI/round start | `Wave/Ntc_Count2` | `design` | |
| `start_count_1` | UI/round start | `Wave/Ntc_Count1` | `design` | |
| `start_go` | UI/round start | `Wave/Ntc_GameStart` | `design` | Có `Bt_c_GameStart`/`Ntc_GameReady`; chưa chốt |
| `ui_click` | UI click | `Wave/Bt_p_Default` | `design` | Nút mặc định; chưa có trace consumer |
| `player_turned` | `EventKind::PlayerTurned` | — | `unknown` | Không chọn tiếng bước/quay; Q-015 |
| `block_destroyed` | `EventKind::BlockDestroyed` | — | `unknown` | Không có entry "break brick" rõ ràng; Q-015 |
| `item_destroyed` | `EventKind::ItemDestroyed` | — | `unknown` | V1 hiện không phát; Q-015 |

Các phương án bị loại có chủ ý: `BombExplosion002…032` (chưa biết quy tắc biến thể), `Cha_BombExplode`
(trùng nghĩa, chưa có trace), `Itm_PutTrap`/`Itm_UseTrap` (trap item, không chắc bong bóng), và
`MP3/Ntc_*` (bản MP3 của cùng thông báo). Khi có trace, cập nhật R-082 và bảng này.

## 4. Chính sách chồng tiếng (design R-083)

| Quy tắc | Giá trị V1 | Lý do |
|---|---|---|
| Voice SFX | Tối đa **8** voice đồng thời | Đủ cho nhiều vụ nổ/nhặt trong một tick mà không nhân bản vô hạn; giá trị design, Q-015 |
| BGM | **1** voice loop riêng, không tính 8 SFX | BGM không được cắt vì SFX |
| Cùng entry trong một tick | Gộp thành **một** lần phát | Tránh cùng một file chồng lên nhau khi nhiều event cùng tick |
| Dedup event | Theo `(round_id, EventId)`; replay/reconcile không phát lại | `EventId` ổn định từ core (ADR 0008) |
| Ưu tiên khi thiếu voice | 1) round-end/start; 2) explosion; 3) trapped/died; 4) bomb/item; 5) UI click | Tiếng kết quả/start và nổ quan trọng hơn UI |
| Quá 8 voice | Bỏ mục ưu tiên thấp nhất; hòa giữ `EventId` nhỏ hơn (cũ hơn) | deterministic, không phụ thuộc thứ tự thread |
| Qua tick | Được phát lại nếu event mới; không đặt cooldown toàn cục | Âm thanh là presentation, không đổi mô phỏng |
| BGM khi reconcile | Không restart; chỉ đổi khi map/lượt đổi | Tránh nhấp/nháy BGM |

Số voice/priority là **design**, chưa có bằng chứng client gốc; Q-015 theo dõi.

### 4.1 Thực hiện trong bundle/client (V1-034 phần 2)

- Builder chỉ đưa entry có định dạng macroquad đọc được (`ogg`/`wav`). `mp3`/`unknown` bị **bỏ qua có
  log**, không đưa file không phát được vào bundle và không panic. Các entry V1 hiện tại đều là Ogg
  (bảng §5), nên bundle thật có đủ 15 entry.
- `crates/boom-client/src/audio.rs` tách phần quyết định thuần (`AudioState`) khỏi phần phát
  (`MacroquadAudio`): dedup `(round_id, EventId)`, gộp cùng cue trong tick, tối đa 8 voice, BGM loop
  một lần. `AudioMix` áp `master × {bgm,sfx}` và `mute`; không restart BGM khi reconcile.

## 5. Hash các entry V1

Sinh bằng `boom-assets audio <pack> --list`; hash là SHA-256 của **payload thô sau giải mã pack**,
không phải hash file đã decode. Toàn bộ 15 entry dưới đây có cùng hash ở KR và VN.

| Entry | Kind | Format | Byte | SHA-256 |
|---|---|---|---:|---|
| `MP3/Village` | bgm | ogg | 270601 | `7a34054f23a66a1f19cd1cd47b70705bda0c774b6d2afd094c4bd5b6d709038f` |
| `Wave/BombExplosion001` | sfx | ogg | 23168 | `aa61fe9b60e9c89290e4f14f8801c01a538668e6e794a41d1d32c719e6d5135d` |
| `Wave/Bt_p_Default` | sfx | ogg | 4574 | `20eeb969ad7f7cd1504aeef1b76eb7196c228648fe93f9e2b38458aca220bec0` |
| `Wave/Cha_BombIgnite` | sfx | ogg | 5119 | `9964e974bfcba4bbafaf2f7a996da3c912f4bfb2995032518645ec6ed3fcaebe` |
| `Wave/Cha_Die` | sfx | ogg | 8858 | `96ad61a176dae1205d8d6a791ce83320785a66e2a4c09fbb867e7f236fd29fd6` |
| `Wave/Itm_ItemEaten` | sfx | ogg | 8520 | `1c829ae9a499ceba0e50f7c4e584a09d19c590fdcc11e03b4fc4abca473ac266` |
| `Wave/Itm_ItemPop` | sfx | ogg | 5664 | `acc3b99bb749b06884307214a886a923b8aec3e1115c9ade482686d00f5fd45c` |
| `Wave/Itm_UseTrap` | sfx | ogg | 5639 | `bba70141174acd97d4d8593c2becea3361e82d7aab603a08585a302e5db24ab9` |
| `Wave/Ntc_Count1` | sfx | ogg | 5894 | `0e32af891d1d6e42eb80a5da2c2ca04ce7be1dcaa5fc96bce25b6c1f6208021c` |
| `Wave/Ntc_Count2` | sfx | ogg | 6135 | `bbd8f6c3da1a352e44be52a63c82ea69714ec44858dc2d0ef0ae7d7543a7fc45` |
| `Wave/Ntc_Count3` | sfx | ogg | 6580 | `0f24f9c745f1b4ca818cb887c4e0550a9a9aac4fa9ad8368600940950599c60e` |
| `Wave/Ntc_Draw` | sfx | ogg | 29125 | `86e37e95aded7325163218915d6ac797c8b67a387301cb2285484d7e0477b295` |
| `Wave/Ntc_GameStart` | sfx | ogg | 12418 | `8566bc99cf2ad83e0c939c0994a23edf6f55443932c7d5c8bfdd3c760798cc64` |
| `Wave/Ntc_Lose` | sfx | ogg | 44428 | `ed474eff4a74ae3c4a480f53856ea1f0f7130cb25589e692c0d5c6aa32f8bd56` |
| `Wave/Ntc_Win` | sfx | ogg | 30340 | `d5c2096a2acc884640185a44088c6f992c264097ba69b8ba5999a0904691df47` |

## 6. Dùng CLI

```sh
# Danh sách entry V1 + event→entry + hash (output ổn định, sort theo entry)
cargo run -p boom-assets --bin boom-assets -- audio /home/admin/boomkr-full --list

# Toàn bộ 797 entry FxSound
cargo run -p boom-assets --bin boom-assets -- audio /home/admin/boomkr-full --list --all

# Xuất raw (không decode) giữ cấu trúc MP3/..., Wave/... dưới thư mục đích
cargo run -p boom-assets --bin boom-assets -- audio /home/admin/boomkr-full --extract local/audio-v1
```

`--build vn` **không được hỗ trợ** cho lệnh `audio`: lệnh chỉ đọc pack KR `FxSound`; truyền
`--build vn` bị từ chối rõ (exit 2) thay vì đoán stem/khóa pack VN. Hỗ trợ VN nếu cần phải chọn đúng
stem `fx`/bảng khóa VN và kiểm trên pack VN thật, ngoài phạm vi KR của issue #64.

## 7. Chưa làm / câu hỏi

- Bundle schema 4 và client đã nối audio; kiểm tay BGM cần bundle thật + pack local (không commit
  asset).
- Chưa xác nhận event→entry bằng consumer gốc; chưa có trace `PlaySound`/log âm thanh. Q-015 cần
  trace client hoặc đối chiếu code client để nâng R-082.
- Chưa xác nhận 8 voice/priority bằng client gốc; R-083 là design, có thể đổi khi có trace.
- Chưa xử lý biến thể `BombExplosion001…032`, tiếng bước/quay, vỡ gạch, item bị lửa phá; hiện để
  trống/im lặng và ghi Q-015.
