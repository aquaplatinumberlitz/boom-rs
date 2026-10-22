# RE tĩnh KR 00967 — consumer âm thanh theo event V1

Kết quả đọc tĩnh client KR `00967` cho [#196](https://github.com/aquaplatinumberlitz/boom-rs/issues/196) /
Q-015, V1-034 (mục 8 hàng đợi RE). Claim ở
[RULES_V1 § (đánh số khi merge)](../../specification/gameplay/RULES_V1.md#-đánh-số-khi-merge--re-kr-00967--âm-thanh-theo-event-r-203r-208)
(R-203…R-208; dải phiên R-203…R-249). Định danh build, bảng VA/RVA và `evidence_id` ở
[evidence KR audio](../../evidence/measurements/kr-static-audio-2026-10-23/README.md).

**Kết luận chính:** client KR phát SFX bằng **id số nguyên** qua dispatcher `0xD1CD10` (manager
`CSoundManRenew`), tra trong **bảng tĩnh 649 entry** tại `0x19E40D8` (id→`Wave/<tên>`). Đã trace được
consumer cho nổ bom (7/8, tuyết 343), đặt bom (9/344, domino 10), bị nhốt (77 solo / 37 couple, thêm
444 help), chết (42), nhặt/ăn item (68), item lộ (69), dùng trap (174), và BGM (record map → `Mp3/<tên>`,
Village id 48). **Biến thể** round-end/start chọn **random đều** bằng `rand()%N + base`; nổ không random
mà chuyển 7↔8 theo cửa sổ 200 ms. Gạch vỡ **không** có entry riêng; `BombExplosion001` (471) **không**
có consumer native.

## 0. Phạm vi và phương pháp

| Mục | Giá trị |
|---|---|
| Build | KR `00967`, `kr-full.exe`, 26.074.624 B |
| SHA-256 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` |
| Image base | `0x00400000`; `RVA = VA − 0x00400000` |
| Đường chính | REA `provider_id: "ida"` trên **bản sao** (`procedure_pseudo_code`) |
| Fallback | IDA MCP trực tiếp trên bản sao `.i64` cho: quét bảng id→tên, quét 597 site gọi `0xD1CD10`, đọc vtable/RTTI, disasm |
| Không chạy | không chạy client, không phát âm thanh, không kết nối server, không sửa binary/DB gốc |

Lần đầu gọi REA trong phiên trả lỗi schema (`data/module must be string`); đóng/mở lại binary thì
REA hoạt động và trả `evidence_id` bình thường. Không dùng kết quả của lần lỗi.

## 1. Bảng id → entry và API phát (R-203)

Mảng entry 0x20 B bắt đầu `0x19E40D8`, khởi tạo bởi `0x41C980` (hàm rất lớn, mỗi entry:
`mov ds:[base+0x20n], id`; `push "Wave/<tên>"`; `call sub_447420`; cuối entry ghi type vào `+0x1C`).
Bản dump có **649 entry**, id chạy `0..651` (một số id trống/không dùng). `+4` là con trỏ tới chuỗi
`Wave/...`; `+0x1C` là type (4/5/6/7…, chưa dùng trong claim này).

Dispatcher `0xD1CD10(manager, id, a3, a4)`:

```
nếu id == -1: return
nếu id > 335:  // nhóm điều khiển volume/stop đặc biệt
    id-337 ∈ 0..5 → v21 = 0 ; mặc định → LABEL_38
    LABEL_38: nếu hook 0x76FBF0: chọn biến thể random (xem §2)
              nếu a3 != 0 || v21 == -1 || count_cùng_id(90ms) <= v21:
                  Play(id, a3, a4)      // 0x14BF4D0
              return
switch id-7:
    case 0 (id 7): ... (7↔8, xem §2)
    case 32 (39?), 33, 41, 42, 63, 93, 68, 112, 198..206: xử lý riêng (stop/volume/loop)
    default: LABEL_38
```

Manager là singleton `0x19D8A88` (`sub_54EAF0` trả con trỏ). `Play` = `0x14BF4D0` (§5).

Id liên quan V1 (từ bảng):

| Id | Entry | Id | Entry |
|---:|---|---:|---|
| 7 | `Wave/Cha_BombExplode` | 68 | `Wave/Itm_ItemEaten` |
| 8 | `Wave/Cha_BombExplodeMulti` | 69 | `Wave/Itm_ItemPop` |
| 9 | `Wave/Cha_BombIgnite` | 77 | `Wave/Cha_LockedIn` |
| 10 | `Wave/Cha_DominoBombIgnite` | 78 | `Wave/Ntc_Lose` |
| 37 | `Wave/Cha_CoupleLockedIn` | 164 | `Wave/Ntc_TimebombExplode` |
| 42 | `Wave/Cha_Die` | 174 | `Wave/Itm_UseTrap` |
| 43 | `Wave/Ntc_Draw` | 175 | `Wave/Ntc_Win` |
| 47 | `Wave/Ntc_GameEnd` | 343 | `Wave/Cha_BombExplode_SnowBall` |
| 444 | `Wave/Ntc_Help2` | 344 | `Wave/Cha_BombIgnite_SnowBall` |
| 471 | `Wave/BombExplosion001` | 554/555 | `Wave/ItemSkill_Needle`/`Needle2` |

## 2. Chọn biến thể (R-205)

`0xD1CD10` lấy `v20 = sub_15625C2()`; hàm này là **LCG rand kiểu MSVC**:

```
v0 = sub_1579D1A();                       // state
v1 = 214013 * state + 2531011; state = v1;
return (v1 >> 16) & 0x7FFF;
```

Bảng biến thể (chỉ khi hook `0x76FBF0` trả true):

| Id gốc | Công thức | Entry đích |
|---:|---|---|
| 18, 128 | `rand()%4 + 436` | `Ntc_GameReady1..4` |
| 21 | `rand()%4 + 440` | `Ntc_GameStart1..4` |
| 43 | `rand()%2 + 428` | `Ntc_Draw1..2` |
| 47 | `rand()%6 + 430` | `Ntc_GameEnd1..6` |
| 64 | `rand()%2 + 445` | `Ntc_HurryUp1..2` |
| 78 | `rand()%6 + 447` | `Ntc_Lose1..6` |
| 175 | `rand()%5 + 453` | `Ntc_Win1..5` |

Nổ bom **không** random: id `7` xử lý riêng — nếu `7` đang phát (<200 ms) thì `Stop(7)`, `Stop(8)`,
`Play(8)`; nếu `8` đang phát (<200 ms) thì rơi xuống `Play(7)`; nếu cả hai không có thì `Play(8)`.
`0xEB3720` chọn **343** (`_SnowBall`) khi `sub_B38390(bomber) == 9`, còn lại **7**. Đặt bom:
`0xAF1140` chọn **344** khi type 9, ngược lại **9**. Không thấy nhánh nào chọn biến thể theo tầm/hướng.

## 3. Consumer từng event (R-204)

| Event V1 | Hàm phát | Id | Điều kiện / ghi chú |
|---|---|---|---|
| nổ bom | `0xEB3720` | 7 (hoặc 343) | `sub_B38390(bomber)==9` → 343; `7` với `a3=1` |
| nổ (object type 31) | `0xEB9C30` | 7 | nhánh `kItemOperCAPassItemHasHP.` |
| (âm thanh nổ chỉ khi trong khung nhìn) | `0x114A1B0` | — | gọi `0xEB3720` nếu toạ độ bom trong 2 rect `this+7712/+7720` |
| đặt bom | `0x71F490`, `0xAF1140` | 9/344 | `0xAF1140`: type 9 → 344, type 16 → 9 |
| domino | `0x8326F0` | 10 | |
| bị nhốt | `0xB42D60` | 77 (solo) / 37 (couple) | cờ `[ebp-0x34]`; `a3=1`; thêm 444 nếu `[this+0x2788] != 0` |
| chết | `0xB53C80` (slot 146 `CGOBomber`) | 42 | gọi `vf+320(this,-1)` trước |
| nhặt/ăn item | `0xB4B410` | 68 | `*(this+10) >= 0` |
| item lộ | `0xB8B680` | 69 | đếm `this[37]` về 0 |
| dùng trap | `0xB9DEF0` | 174 | `case a2==2` (dùng item trap) |

Ghi chú: `0xB42D60` còn gọi `sub_D1CD10` với id `444` (`Ntc_Help2`) khi `[this+0x2788] != 0` — tiếng
"help" của người bị nhốt; và trước đó có một id khác ở `0xB43200` (âm thanh tương tác, chưa gán event).

## 4. BGM (R-206)

- Bảng BGM 0x20 B tại `0x19E9258`, 117 entry; id `48` = `Mp3/Village` (chuỗi `0x1761FE8`, tham chiếu
  `0x41B590+838`).
- `0xE71D70(this, a2)`:
  1. `mapId = sub_123FC20(sub_544520())`;
  2. `idx = sub_E04F00(mapId)` — quét danh sách record (`this+56`, `sub_E01930(i)`);
  3. nếu `idx != -1`: `name = sub_E02110(idx)` (đọc chuỗi tại `record+68`); nếu bật `Xmas`
     (`sub_480B60("Xmas")`) thì `name = "Carol%02d"` với `rand()%4` (Carol00..03);
  4. nếu `idx == -1`: dùng chuỗi `Event`/`EventBgm`/`EventLyric` (nhánh event động);
  5. `"Mp3/%s"` rồi `sub_D1D710(name, a2)` — phát BGM.
- Vì vậy BGM **không hard-code**; tên lấy từ record map. R-081 (entry `Mp3/Village` + hash) vẫn đúng;
  phần "map Village trỏ tới tên Village" là `candidate` (chưa đọc record map trong data pack).
- Loop/một voice: không thấy trong đường này → giữ `design` R-083.

## 5. Voice / chồng tiếng (R-207)

`0x14BF4D0(manager, id, a3, a4)`:

```
nếu id == -1 hoặc manager+8/+24/+36 == 0: return 0
nếu a3 != 0:
    quét manager+16[0..manager+12-1]; nếu voice cùng id → return handle (KHÔNG phát thêm)
tìm voice có state == 2 (rảnh)
nếu không có: return 0                     // bỏ, không đuổi voice cũ
nếu có: nạp file .ogg, volume = v27 * *(float*)(manager+48) / const, gắn callback, play
```

Hệ quả: **pool cố định** (mảng `manager+16`, count `manager+12`), **dedup theo id khi `a3 != 0`**,
**không ưu tiên/không eviction**. Cửa sổ dedup thấy trong `0xD1CD10`: 200 ms cho cặp 7/8; 90 ms cho
nhóm id `337..342`. Số voice của pool (hằng khởi tạo) **chưa định vị** — không claim "8 voice" của V1
là của KR.

## 6. Gạch vỡ và `BombExplosion001` (R-208)

- Bảng 649 entry không có entry "brick"/"break" (chỉ `Npc_Pen_NpcEggBreak`, `Ui_BasketEggBreaking`,
  `Item_MarbleEggBreaking`…). `CPlayMap::ExplodeObjectByFlame` `0xEB9840` phá tile qua vtable (`vf+148`)
  và **không** gọi `0xD1CD10`; tiếng nổ là của quả bom (7/8/343), không phải từng viên gạch.
- `Wave/BombExplosion001` (id 471) **không** có site `push 471` trong 597 site gọi `0xD1CD10`.
  Tuy nhiên client có đường script `PlayWaveEffect` (`0xD16410` → `0xD1CD10`) phát **theo tên**, nên
  không loại trừ script/pack phát 471; chỉ kết luận "không có consumer native trong phạm vi đã quét".

## 7. Đề xuất cho AUDIO_V1 (không tự đổi trong PR này)

| Event V1 | V1 hiện tại | Client KR | Đề xuất |
|---|---|---|---|
| `bomb_exploded` | `Wave/BombExplosion001` (471) | 7/8 (+343 tuyết), 7↔8 theo 200 ms | đổi sang **7/8**, thêm 343; 471 không có consumer native |
| `bomb_placed` | `Wave/Cha_BombIgnite` (9) | 9 / 344 tuyết | giữ 9, thêm 344 nếu có bom tuyết |
| `player_trapped` | `Wave/Itm_UseTrap` (174) | 77 solo / 37 couple (+444 help) | **đổi** sang 77/37; 174 là tiếng dùng item trap |
| `player_died` | `Wave/Cha_Die` (42) | 42 | khớp |
| `item_revealed` | `Wave/Itm_ItemPop` (69) | 69 | khớp |
| `item_picked` | `Wave/Itm_ItemEaten` (68) | 68 | khớp |
| round end/start | `Ntc_Win/Lose/Draw` | id 175/78/43 → **random** Win1..5/Lose1..6/Draw1..2 | giữ entry gốc, ghi nhận biến thể random |
| `bgm_loop` | `MP3/Village` | id 48, tên từ record map | giữ; R-081 |

Policy V1 (8 voice + ưu tiên + bỏ mức thấp nhất) **khác** client (pool cố định, bỏ khi hết, dedup
theo id) — đề xuất giữ design V1 nhưng ghi khác biệt ở R-207/§5.

## 8. Giới hạn

- Chỉ tĩnh; không chạy client, không nghe thử, không có packet/log runtime.
- Bảng id→entry đọc từ **bản dump đã init**; 649 entry là số entry parse được, có thể còn id trống.
- Chưa đọc record map trong data pack để chứng minh map Village trỏ tới tên `Village` (R-081 phủ phần
  entry/hash).
- Số voice của pool và hằng volume chưa định vị; chưa thấy loop flag của BGM.
- "Không có consumer" cho 471/gạch vỡ là **bounded negative** trong phạm vi 649 entry + 597 site
  `0xD1CD10`; đường script theo tên chưa bị loại trừ.

## 9. Bước tiếp

1. Đọc record map (pack) để nối map Village → tên BGM; xác nhận map BGM cho từng map V1.
2. Tìm hằng khởi tạo pool voice + volume mặc định (`manager+12/16/48`) để chốt số voice KR.
3. Kiểm tra script/pack có gọi `PlayWaveEffect("BombExplosion001")` không (đường theo tên).
4. Nếu muốn đổi AUDIO_V1: mở task riêng theo ADR 0013, sửa cả `data/assets/audio-v1.toml` + bảng
   AUDIO_V1 + test, không gộp vào PR RE này.
