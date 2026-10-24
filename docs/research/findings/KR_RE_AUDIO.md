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
Village id 48). **Biến thể** round-end/start được **remap** bằng `rand()%N + base` **chỉ khi guard
`sub_76FBF0` bật**; guard tắt thì giữ id gốc (và `rand()` 15-bit nên `%5`/`%6` có modulo bias — không claim
phân bố đều). Nổ không remap mà chuyển 7↔8 theo cửa sổ 200 ms. Không thấy **direct call/literal** cho
brick-break hoặc `BombExplosion001` (471) trong phạm vi quét — **không** kết luận gạch vỡ không có tiếng.

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

Dispatcher dùng singleton `0x19D8A88` (`sub_54EAF0` trả con trỏ). `Play` = `0x14BF4D0` nhận
engine `[manager+0x68]`, không phải chính singleton; cạnh receiver/count được bổ sung ở §11.

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

`0xD1CD10` **chỉ khi** `sub_76FBF0(&v24) != 0` mới lấy `v20 = sub_15625C2()`; nếu guard trả false thì
`a2` **giữ nguyên** (không remap). Hàm rand là **LCG kiểu MSVC**:

```
v0 = sub_1579D1A();                       // state
v1 = 214013 * state + 2531011; state = v1;
return (v1 >> 16) & 0x7FFF;
```

Bảng remap (chỉ khi guard `0x76FBF0` trả true; công thức modulo, **không** claim phân bố đều vì
`rand()` 15-bit % 5/% 6 có bias):

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
| nổ bom | `0xEB3720` | 7 (hoặc 343) | `sub_B38390(bomber)==9` → 343; `a3=1` = dedup theo id (R-207), **không** phải cờ loop |
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

`0x14BF4D0(engine, id, a3, a4)` (engine=`[manager+0x68]`, §11):

```
nếu id == -1 hoặc engine+8/+24/+36 == 0: return 0
nếu a3 != 0:
    quét engine+16[voice slots, count=engine+12]; nếu voice cùng id → return handle (KHÔNG phát thêm)
tìm voice có state == 2 (rảnh)
nếu không có: return 0                     // bỏ, không đuổi voice cũ
nếu có: nạp file .ogg, volume = v27 * *(float*)(engine+48) / const, gắn callback, play
```

Hệ quả: **pool cố định** (block con trỏ `engine+16`, count `engine+12`), **dedup theo id khi `a3 != 0`**,
**không ưu tiên/không eviction**. Cửa sổ dedup thấy trong `0xD1CD10`: 200 ms cho cặp 7/8; 90 ms cho
nhóm id `337..342`. R-212/§11 đã nối khởi tạo tới vòng tối đa16voice, lưu số handle tạo thành công. Runtime count
chưa đo; không claim8voice DESIGN của V1 là sốnative hoặc luôn có16voice trên mọi máy.

## 6. Bounded negative cho brick-break và `BombExplosion001` (R-208)

- Bảng 649 entry **không có tên** "brick"/"break" (chỉ `Npc_Pen_NpcEggBreak`, `Ui_BasketEggBreaking`,
  `Item_MarbleEggBreaking`…): đây là negative theo **tên entry**, không chứng minh gạch vỡ không có tiếng.
- `CPlayMap::ExplodeObjectByFlame` `0xEB9840` **không gọi trực tiếp** `0xD1CD10`, nhưng phá tile qua
  **virtual `vf+148`** (thân chưa đọc) — cạnh này **chưa nối/loại trừ**. Vì vậy không kết luận "tiếng nổ chỉ
  gắn với bom" hay "không có tiếng theo từng viên gạch".
- `Wave/BombExplosion001` (id 471) **không** có site `push 471` trong 597 site gọi trực tiếp `0xD1CD10`.
  Id động (tính từ biến), wrapper và đường script `PlayWaveEffect` (`0xD16410` → `0xD1CD10`, phát theo tên)
  **chưa bị loại trừ**, nên chỉ kết luận: **không thấy consumer native trực tiếp** trong phạm vi đã quét.

## 7. Đề xuất cho AUDIO_V1 (không tự đổi trong PR này)

| Event V1 | V1 hiện tại | Client KR | Đề xuất |
|---|---|---|---|
| `bomb_exploded` | `Wave/BombExplosion001` (471) | 7/8 (+343 tuyết), 7↔8 theo 200 ms | đổi sang **7/8**, thêm 343; 471 **không thấy consumer native trực tiếp** (chưa loại trừ id động/wrapper/script) |
| `bomb_placed` | `Wave/Cha_BombIgnite` (9) | 9 / 344 tuyết | giữ 9, thêm 344 nếu có bom tuyết |
| `player_trapped` | `Wave/Itm_UseTrap` (174) | 77 solo / 37 couple (+444 help) | **đổi** sang 77/37; 174 là tiếng dùng item trap |
| `player_died` | `Wave/Cha_Die` (42) | 42 | khớp |
| `item_revealed` | `Wave/Itm_ItemPop` (69) | 69 | khớp |
| `item_picked` | `Wave/Itm_ItemEaten` (68) | 68 | khớp |
| round end/start | `Ntc_Win/Lose/Draw` | id 175/78/43 → remap Win1..5/Lose1..6/Draw1..2 **khi guard `sub_76FBF0` bật** (guard tắt giữ id; modulo bias) | giữ entry gốc, ghi nhận remap có điều kiện |
| `bgm_loop` | `MP3/Village` | id 48, tên từ record map | giữ; R-081 |

Policy V1 (8 voice + ưu tiên + bỏ mức thấp nhất) **khác** client (pool cố định, bỏ khi hết, dedup
theo id) — đề xuất giữ design V1 nhưng ghi khác biệt ở R-207/§5.

## 8. Giới hạn

- Chỉ tĩnh; không chạy client, không nghe thử, không có packet/log runtime.
- Bảng id→entry đọc từ **bản dump đã init**; 649 entry là số entry parse được, có thể còn id trống.
- Chưa đọc record map trong data pack để chứng minh map Village trỏ tới tên `Village` (R-081 phủ phần
  entry/hash).
- Khởi tạo pool tối đa16voice đã nối ở R-212; runtime count/hằng volume chưa kiểm đầy đủ, loop flag BGM chưa thấy.
- "Không thấy consumer" cho 471/brick-break là **bounded negative** chỉ cho direct call + literal push
  trong 649 entry + 597 site `0xD1CD10`; virtual `vf+148`, id động, wrapper và script theo tên chưa bị loại trừ.

## 9. Bước tiếp

1. Đọc record map (pack) để nối map Village → tên BGM; xác nhận map BGM cho từng map V1.
2. Khởi tạo/count đã nối ở §11; runtime count/volume và BGMloop còn thiếu, không đọc lại D1C330/ctor để tìm writer cũ.
3. Kiểm tra script/pack có gọi `PlayWaveEffect("BombExplosion001")` không (đường theo tên).
4. Nếu muốn đổi AUDIO_V1: mở task riêng theo ADR 0013, sửa cả `data/assets/audio-v1.toml` + bảng
   AUDIO_V1 + test, không gộp vào PR RE này.

## 10. Lượt 2026-10-24 (phiên 2) — init pool voice: ctor `CSoundManRenew` đã đọc, count chưa thấy

Câu hỏi hẹp #196: **số voice của pool `CSoundManRenew` lấy từ đâu** (hằng/param/config), nối writer
`manager+12` với manager thật + đường init. Build KR `00967`, SHA-256
`cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b`, imagebase `0x00400000`.
Ngân sách thực dùng: **6 call phân tích**, **4 thân mới** (`0x60FA30`, `0xD20E10`, `0xD217F0`,
`0xD1C0A0`); không đọc lại thân #200. Evidence: `ev_2f07cd5a…`, `ev_378e543a…`, `ev_7fad1e9a…`,
`ev_425ff988…`, `ev_d48e48f7…` (VA/RVA ở
[evidence](../../evidence/measurements/kr-static-audio-2026-10-23/README.md)).

### 10.1. Chuỗi init (dương, trong thân đã đọc)

- `dword_19D8A88` (singleton) có 4 xref data: getter `0x54EAF0`, `0x60FA34`, `0xD20E3D`, `0xD21800`.
- `sub_60FA30` (`0x0060FA30`, 0x24): `return dword_19D8A88 != 0;` — **kiểm đã khởi tạo**.
- `sub_D20E10` (`0x00D20E10`, 0x3A): gán `*this = &TSingleton<CSoundManRenew>::vftable` rồi
  `dword_19D8A88 = (int)(this - 1)` — **ghi con trỏ singleton**; xref code duy nhất từ `sub_D1C0A0`
  tại `0xD1C0D3`. (Ý nghĩa `this - 1` chưa diễn giải.)
- `sub_D217F0` (`0x00D217F0`, 0x1E): gán vftable rồi `dword_19D8A88 = 0` — nhánh reset/huỷ.
- Đường gọi: `sub_1231BD0` (`0x01231BD0`, 0x2C62) gọi `sub_D1C0A0` tại `0x1233BEA`.

### 10.2. Thân ctor `sub_D1C0A0` (0x00D1C0A0, 0x178) — không thấy gán count

Thân đọc được (rút gọn, giữ đúng thứ tự):

```
sub_D21700(this); sub_D20E10(this+1);                 // base/singleton ctor
*this = &CSoundManRenew::vftable; *(this+1) = &...;   // vftable
sub_D216C0(this+2);
*(this+5)=0; *(this+6)=0; sub_4474A0(this+7);
*(this+13)=0; *(this+14)=0; *(this+15)=1000;
sub_D21620(this+16); sub_D215D0(this+18); sub_4474A0(this+20);
sub_D21580(this+27); *(this+29)=0; *(this+30)=-1; sub_D21670(this+31);
dword_16EA2B8("Mp3dec.asi");
v2 = sub_14EF990(60) ? sub_14BE290(1,1) : 0;
*(this+26)=v2; sub_D24140(this+31, v2);
sub_D1C330(this);                                     // cuối thân
```

- **Không có store tới `+12`/`+16`** (byte) trong thân; các store thấy ở `+20/+24/+52/+56/+60=1000/
  +104=v2/+116/+120=-1`. `+60 = 1000` **không** phải `+12`; **không** suy đây là số voice.
- `sub_D1C330(this)` ở cuối là ứng viên init/reset pool (**candidate**, chưa đọc).
- Chưa kiểm lại ánh xạ `dword_19D8A88` (singleton) ↔ `manager` mà `0x14BF4D0` nhận; không dùng offset
  của ctor để suy count.

### 10.3. Kết luận / blocker tại lượt PR231 (R-212 cập nhật ở §11)

- **Số voice vẫn `unknown`**: nguồn literal/param/config **chưa thấy**; ctor chỉ zero-init + gọi
  helper, không gán `manager+12`/`+16`. **Không** lấy 8 design V1, 649 entry, hay dung lượng container.
- `read_bytes` REA IDA trả `capability_unavailable` (1 lần, không retry). Ngân sách 6 call/4 thân đã
  dùng ⇒ dừng đúng hạn; không mở provider khác.
- Bước tiếp: kiểm xem `sub_D1C330` có store `[manager+0Ch]`/`+10h` và nguồn giá trị **không** — chỉ
  kết luận sau khi thấy store + dataflow; nếu vào VM/Themida thì ghi entry và dừng.


## 11. Count trên đúng receiver của Play — R-212 (2026-10-24)

Câu hỏi con #196 đã trả lời: **nguồn count/pointer của engine thực sự nhận14BF4D0**.
[Manifest7hash/PEanchors](../../evidence/measurements/kr-static-audio-2026-10-23/pool-count.json)
reuse ctor REA cùng SHA và hai thân mới+callsite PE đọc chỉđịnh; không mở provider/worker/runtime mới.

Mấu chốt là tách singleton dispatcher khỏi engine. CtorD1C0A0 đã cache gọi14BE290(1,1), lưu kết quả
vào `this[26]`=manager+0x68. TạiD1D12A..D1D142, dispatcher nạp ECX từ `[manager+0x68]` rồi gọi
14BF4D0. Vì vậy poolcount+0x0C/pointer+0x10 thuộc **engine**, không phải managerCSoundManRenew.
D1C330 đọc đầy đủ1428byte chỉ khởi tạo/đăng ký dữ liệu; không directstore hai offset đó trong thân,
không suy toàn callgraph hoặc lấy652registration records thành649validsound entries.

14BE290 là ctor/helper trênECXexistingobject, không phải factory tự cấp cảmanager. Hai đối số1,1
được lưuengine+0x24/+0x28. Helper zero count+0x0C vàpointer+0x10, cấp block0x48byte: handle driver
ởblock+0, các voice ởblock+4+4i, handle riêng+0x44. Loop từ14BE3F0 tạo handle, dừng khi handle0;
14BE414 tăngEDI,14BE415 soEDI16,14BE418 lặp khi nhỏhơn;14BE423 lưuEDI tạiengine+0x0C.
Đó là **số voice tạo thành công, tối đa16**, không phải48voice,652sound hay1000ms/count.
Thân463byte gồmRET8 tại14BE45C..45E; adjacent14BE460 không được đọc làm bằng chứng.

Nguồn/staticdataflow confirmed; số thực tạo trên máy chạy chưa đo, lỗi device/handle có thể làm ít
hơn hoặc0. Voice8/priority/eviction củaV1 giữ DESIGN R-083 khácnative; PR này không đổiaudioengine.
Câu hỏi tiếp có căn cứ là runtime enginecount/volume, BGMloop hoặc brick-breakconsumer; không tiếp
quét bảng597soundcalls hayD1C330 đã giải.
