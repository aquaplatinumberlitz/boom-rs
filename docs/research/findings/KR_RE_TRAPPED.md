# KR 00967 — bị nhốt (trapped): state, di chuyển, khoá và chết

Kết quả đọc tĩnh cho [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / Q-018,
nguồn `kr_static`. Claim ở [RULES_V1 §14](../../specification/gameplay/RULES_V1.md#14-re-kr-00967--bị-nhốt-r-126r-131);
định danh build và bảng VA/RVA ở
[evidence](../../evidence/measurements/kr-static-trapped-2026-10-21/README.md).
Không có binary, pack, IDA DB hay decompile nguyên văn trong repo.

**Kết luận chính:** chưa chứng minh được bomber người chơi bị nhốt có di chuyển. Chuỗi cổng/di chuyển
đã trace (`0xB95180 → 0xB93830`; `0xB93830` gọi cổng `0xB95970` rồi mover `0xB93890`) nằm trong đường
cập nhật **`CGOPet`** — theo R-134/R-135 (RULES_V1 §15, PR #185). Với bomber người
chơi, "bị nhốt đi được" vẫn là `candidate`/chưa chứng minh; khác R-036/R-108 (BnB design `trapped`
đứng yên) **không** phải cơ sở để đổi luật người chơi.

**Nhãn state (sau review vòng 3):** KR chỉ cho **số** state và các điều kiện/chuyển state đã đọc trong
cụm `CGOPet` (confirmed trong phạm vi pet). Nhãn "`3` = bị nhốt" cho cụm pet là **candidate** — không có
enum/consumer chung định nghĩa `sub_B95AC0`/`sub_B956C0`/virtual `+56`/`+60`. Packet VN 120 subtype 11
= state 3 là claim **riêng** `vn_runtime`/confirmed của bomber người chơi (R-026), **không** ghép sang pet.

## 0. Phạm vi và cách đọc

Phiên này dùng REA làm đường chính: `open_binary` binary **bản sao** với `provider_id: "ida"`, engine
seed sẵn database đã phân tích (`auto_analysis_ready`, `hexrays_ready`), nên mỗi hàm chỉ mất ~0,1–0,5 s.
Riêng `read_bytes`/`search_bytes` bị adapter IDA của REA từ chối (`capability_unavailable`,
"operation is not declared by this provider") nên phép đọc vtable/byte không thực hiện được trong phiên
này; mọi kết luận dưới đây chỉ dựa trên decompile + xrefs, không dựa trên byte đọc trực tiếp.

Đối tượng: state machine trong cụm `0xB95xxx` (`sub_B952E0`) và chuỗi cổng/di chuyển
`0xB95180 → 0xB93830 → (0xB95970, 0xB93890)`; consumer đã trace của chuỗi là **`CGOPet`**, chưa phải
bomber người chơi (chi tiết phạm vi lớp ở §1 và §2). Không bao gồm server, packet runtime, hay client
chạy thật.

## 1. Mã state trong cụm `0xB95xxx`

Đối tượng có getter/setter state là virtual `+100`/`+104`. Các giá trị đọc được và bằng chứng:

| State | Nhãn (mức) | Bằng chứng |
|---|---|---|
| `0`, `1`, `4` | không thuộc nhóm được đi (suy luận từ cổng) | `sub_B95970` loại `{0,1,4,13,14}` khỏi "được đi" |
| `2` | stop (suy luận) | `sub_B952E0` đặt `2` khi `sub_B95AC0(this) == 0` và state đang `3`; `sub_B95840` không dùng |
| `3` | **chưa xác định**; giả thuyết "bị nhốt" = **candidate** | `sub_B952E0` đặt `3` khi `sub_B95AC0(this) != 0`, `state != 3`, `sub_B956C0(this,-1) == 0`, state không thuộc `{13,14,15..18,19..22}`; không có enum/consumer định nghĩa `sub_B95AC0`/`sub_B956C0`. Packet VN 120/11 state 3 là claim riêng của bomber (`vn_runtime`/confirmed, R-026), không ghép sang pet |
| `8` | stop sau khi rời nhóm khoá (suy luận) | `sub_B95BD0(this, 0)` → `SetState(8)` |
| `9..12` | đi theo 4 hướng, nhóm thường (suy luận) | `sub_B95840`: `SetState(dir + 9)` |
| `13` | đang chết (suy luận) | `sub_B952E0` case 13 → `SetState(14)`; input dispatch `sub_B95180` bỏ qua state 13 |
| `14` | chết (suy luận) | `sub_B952E0` case 14 → `SetState(2)`; `sub_B95180` bỏ qua state 14 |
| `15..18` | nhóm "đã khoá" (suy luận, kèm `19..22`) | `sub_B95E00` trả true cho `15..18` **hoặc** `19..22` |
| `19..22` | đi theo 4 hướng, nhóm khoá (suy luận) | `sub_B95840`: `SetState(dir + 19)` khi đang ở nhóm khoá |

Chỉ **số** state, điều kiện và chuyển state trong bảng là `kr_static`/confirmed trong phạm vi `CGOPet`.
Cột "Nhãn" là suy luận từ số/offset, chưa có consumer định nghĩa ngữ nghĩa. Đặc biệt `3` = "bị nhốt"
là **candidate**, không phải sự thật đã đọc; packet VN 120/11 của bomber là claim riêng R-026
(`vn_runtime`/confirmed), không phải bằng chứng cho nhãn của pet.

**Phạm vi lớp của mục 1–4 (bổ sung sau review #157):** các hàm `sub_B952E0`, `sub_B95840`, `sub_B95BD0`,
`sub_B95E00`, `sub_B95970` nằm trong cùng cụm `0xB95xxx`. Consumer đã trace của `sub_B952E0` là
`sub_B93A60` `0xB93A60` (= slot 24 `??_7CGOPet@@6B@`), và `0xB93A60` gọi `sub_B95180(this)` rồi
`sub_B952E0(v2)` — tức đường cập nhật **`CGOPet`/AI**. Chưa có consumer nào thuộc lớp bomber người chơi
cho các hàm này; phát biểu này **không** loại trừ đường gián tiếp/wrapper/VM từ bomber, chỉ giới hạn
phạm vi bằng chứng. Vì vậy bảng state + tóm tắt dưới đây là mô tả cơ chế trong cụm đã đọc; nhãn
"3 = bị nhốt" đã là **candidate** ngay trong phạm vi pet, và càng không đủ cho bomber cho tới khi xác
định state machine của bomber.

Tóm tắt tự viết của `sub_B952E0` (state machine mỗi bước cập nhật, không phải nguyên văn):

```
nếu sub_B96D80(this+53) != 0:
    nếu sub_B95DA0(this): --this[235]
    ngược lại nếu sub_B95DD0(this): --this[236]
    nếu !sub_B95DA0(this) || !sub_B95DD0(this):
        nếu sub_B95E00(this):
            nếu controller->vf56 == 0:  sub_B95BD0(this, 0)
        ngược lại:
            nếu controller->vf56 != 0: sub_B95BD0(this, 1)
    nếu controller->vf60 != 0:
        nếu sub_B95AC0(this) == 0: sub_B95AE0(1)
    ngược lại:
        nếu sub_B95AC0(this) != 0: sub_B95AE0(0)
    v10 = state
    nếu sub_B95AC0(this) != 0:
        nếu v10 != 3 và sub_B956C0(this, -1) == 0
           và v10 not in {13,14,15..18,19..22}:
            nếu sub_B95E00(this): sub_B95BD0(this, 0)
            SetState(3)
    ngược lại nếu v10 == 3: SetState(2)
    case 13 -> (khi a2 == 2) SetState(14); case 14 -> (khi a2 == 2) SetState(2)
```

`vf56`/`vf60` chỉ là **byte offset** của virtual call; `sub_B95AC0`/`sub_B956C0` chỉ là các hàm chưa
rõ nghĩa. Tên "cờ sống"/"đang đi" **không** được dùng như sự thật: chưa có consumer định nghĩa chúng.

## 2. Cổng di chuyển của state `3` — chuỗi đã trace thuộc `CGOPet` (R-127)

Chuỗi đã trace trong cụm này:

`sub_B95180` `0xB95180` → `0xB93830` → (`sub_B95970` `0xB95970` = cổng; `0xB93890` = mover)

`0xB93830` gọi cổng `0xB95970` **trước** rồi mới gọi mover `0xB93890`; **không** phải
`0xB95180 → 0xB95970 → 0xB93830`. `0xB93830`/`0xB93890` không mang tên lớp `CGOBomber` — tên cũ
`CGOBomber::MoveGate`/`CGOBomber::Move` là gán sai lớp.

Ba mắt xích, trong **phạm vi lớp đã trace**:

1. **Input không bị chặn trong phạm vi này.** `sub_B95180` chỉ bỏ qua state `13`, `14`, `1`; state `3`
   đi tiếp vào nhánh lấy `(dir, len)` từ controller rồi gọi `0xB93830`.
2. **Cổng di chuyển cho qua.** `sub_B95970` trả `false` cho `{0,1,4,13,14}`; state `3` rơi vào nhánh
   mặc định: được đi nếu hai bộ đếm `this[235]`/`this[236]` đều `0`.
3. **Cổng được dùng đúng.** `0xB93830` trả `0` (không đi) khi `sub_B95970` trả `0`; ngược lại gọi
   `0xB93890` với nguyên `len` (không nhân/chia trong `0xB93830`/`0xB93890`/`CMovable::MoveSimple`).

**Phạm vi lớp (R-134/R-135, RULES_V1 §15, PR #185):** trong phạm vi xref đã quét,
caller duy nhất của `0xB95180` là `0xB93A82` trong `sub_B93A60` `0xB93A60`; `0xB93A60` là slot 24 của
`??_7CGOPet@@6B@` (`0x1746E40`). Slot 24 của `CGOGameBomber`/`CGOBomber` là hàm khác và không gọi
chuỗi trên. Vì vậy chuỗi này là đường **`CGOPet`/AI**, **không** phải bằng chứng cho bomber người chơi.
Phát biểu chỉ nói đường **đã trace** đi từ pet; **không** loại trừ đường gián tiếp/wrapper/VM khác từ
bomber — đường của bomber chưa được chứng minh.

**Lời gọi `(dir,len)`:** trong `sub_B95180`, `(dir,len)` là out-param của virtual call tại **byte +8**
(slot 2 trên x86), **không** phải "vf8"/`+0x20`. Controller của pet nằm ở `*(pet + 0xE0)` (getter
`0x00B96310` → `0x00B971C0` = `return *(this)`). **Không** gán `+0xE0` này cho bomber: input dispatch
của bomber dùng sub-object **nhúng** tại byte `+1412` (tức `bomber[353]` nếu `bomber` là `_DWORD*`);
vtable pointer nằm ngay tại byte đó — **không** phải dereference `bomber[353]` để lấy pointer tới
controller. Thân hàm virtual tại byte +8 **chưa định vị** ⇒ không suy ra `len > 0`.

**Kết luận:** với bomber người chơi, cả "cổng không chặn" và "đi được" đều chưa có consumer thuộc lớp
bomber ⇒ R-127 giữ **`candidate`**. V1 đang mô hình hoá `trapped` theo BnB (R-108, đứng yên); không
đổi profile từ kết quả này.

**Lượt thử bổ sung (2026-10-22):** tìm setter gán controller cho bomber và thân `(dir,len)`. Ctor
`CGOGameBomber` (`0xB79DC0`, `0xB73410`) chỉ gán vtable cho các sub-object **nhúng**, **không** gán
`+0xE0`/`+0xD4` — hai offset này là của `CGOPet`. Quét store vào `+0xE0`/`+0xD4` trên DB **bản sao**
(REA thiếu `read_bytes`/`search_bytes` nên fallback IDA trực tiếp, tuần tự sau khi đóng REA) không thấy
điểm nào thuộc bomber. Lớp controller của bomber và thân `(dir,len)` vẫn **unknown**; chưa vào VM.

## 3. Nhóm state đi "khoá" và bộ đếm giữ 20 bước (R-128, R-129)

**Phạm vi lớp:** `sub_B95840`/`sub_B95BD0`/`sub_B95E00` nằm trong cùng cụm `0xB95xxx` với state machine
ở §1; consumer đã trace của `sub_B952E0` là `CGOPet` (`0xB93A60`). Chưa có consumer thuộc lớp bomber
người chơi, nên cơ chế dưới đây là mô tả trong cụm đã đọc, chưa phải luật người chơi.

`sub_B95840(dir)` chọn họ state đi:

```
nếu sub_B95E00(this) == 0:  SetState(dir + 9)      // nhóm thường
ngược lại:                  (this+100)->vf104(dir); SetState(dir + 19)   // nhóm khoá
```

`sub_B95BD0(this, mode)` đổi nhóm và đặt bộ đếm giữ:

```
mode != 0: this[235] = 20; SetState(22); ...
mode == 0: this[236] = 20; SetState(8);  ...
this[234] = mode
```

`sub_B952E0` giảm `this[235]`/`this[236]` mỗi bước cập nhật và gọi `sub_B95BD0(0)` khi state thuộc nhóm
`15..18`/`19..22` và `controller->vf56 == 0`, gọi `sub_B95BD0(1)` khi ngoài nhóm và `vf56 != 0`; nhãn
"đang đi" cho `vf56` là suy luận, chưa có consumer định nghĩa. Trong lúc bộ đếm `> 0`, `sub_B95970` trả
`false` ⇒ **cổng di chuyển đóng 20 bước cập nhật**.
Ở 30 ms/bước (R-013, đo trên VN) thì ≈ **600 ms**; đây là **quy đổi**, chưa đo trên KR, và không phải
thời gian bị nhốt.

## 4. Chết (R-130)

**Phạm vi lớp:** chuỗi chết dưới đây đọc trong `sub_B952E0` (cùng cụm `0xB95xxx`, consumer đã trace là
`CGOPet`); chưa chứng minh là chuỗi chết của bomber người chơi.

- `13` = đang chết: khi điều khiển báo `a2 == 2`, gọi `vf128`, `sub_B936F0(this[59], this[58])`, rồi
  `SetState(14)`.
- `14` = chết: khi `a2 == 2`, nếu `this[234] == 1` thì `sub_B95BD0(this, 0)`, rồi `SetState(2)`.

Chưa xác định `this[58]`/`this[59]` mang gì (nghi người/bom gây chết) — không claim.

## 5. Timer nhốt → chết: chưa định vị (R-131)

Không tìm thấy vòng đếm thời gian nhốt trong cụm state machine. Cái còn lại là **hàng đợi defer**:
bảng tên của binary có `kCGameStage_DeferBomberLockedIn` (`0x17953C8`) và
`kCGameStage_DeferBomberReleaseOrDie` (`0x1795074`) — đúng ngữ nghĩa "nhốt xong thì cứu hoặc chết",
nhưng bảng tên chỉ đăng ký **id**, không trỏ tới hàm, và mọi xref tới hai chuỗi này chỉ là bảng đó.

Vì vậy:

- **không** dùng 6359 ms (R-026) làm timer; đó vẫn là khoảng packet;
- R-115 (`trap_to_death_ms = 4000`, BnB) giữ nguyên;
- bước tiếp theo: tìm bộ thực thi defer (switch theo id) rồi lấy case của `DeferBomberReleaseOrDie`;
  nếu thân hàm nằm trong `.themida` thì dừng và ghi entry VM theo [DEVIRT_METHOD](DEVIRT_METHOD.md).

## 6. Kim/thoát nhốt: chỉ có chuỗi, chưa có consumer

Chuỗi tồn tại trong binary: `Niddle`/`Needle` (`0x16EE8C8`/`0x16EE8D0`),
`ItemSkill_HaveItemSkillNeedle` (`0x16EE8D8`), `Item/AT_Needle` (`0x16EE970`),
`Item/ItemNiddle` (`0x16F01A8`), `useneedle` (`0x16EC594`). Đây là **asset/chuỗi**, không chứng minh
cơ chế tự thoát — theo AGENTS §4 "asset tồn tại ≠ tính năng chạy". Giữ `candidate`/`unknown` ở Q-018.

## 7. Khác biệt với profile hiện tại và với BnB

| Khóa / claim | Hiện tại | KR đọc được | Đề xuất |
|---|---|---|---|
| `trapped` đứng yên (R-036, R-108) | đứng yên | chưa có consumer thuộc lớp bomber; chuỗi cổng đã trace là `CGOPet`; `len`/mover của bomber **unknown** | **giữ nguyên** BnB; chỉ xét đổi mô hình sau khi tìm được mover/`len` **của bomber** + trace; không dùng chuỗi pet |
| `player.trap_to_death_ms` (R-115) | `4000` (`oss`/design) | chưa xác định (R-131) | giữ |
| `player.trap_touch_distance_px` (R-116) | `28` (`oss`/design) | chưa đọc (đường chạm/cứu chưa lần ra) | giữ |
| Cứu/chọc (R-028 `vn_runtime`) | địch chạm → chết | chưa đọc được nhánh chạm | giữ, Q-018 |

## 8. Giới hạn

- Chỉ tĩnh, không chạy client; không có trace KR cho state 3 hay bộ đếm 20.
- Phạm vi lớp: chuỗi `0xB95180 → 0xB93830 → (0xB95970, 0xB93890)` và state machine `0xB95xxx` có
  consumer đã trace là `CGOPet`; **không** chứng minh cho bomber người chơi, cũng **không** loại trừ
  đường gián tiếp/wrapper/VM từ bomber.
- Nhãn "3 = bị nhốt" cho cụm pet là **candidate**: KR chỉ có số/điều kiện/chuyển state, không có
  enum/consumer chung. Packet VN 120/11 state 3 là claim riêng `vn_runtime`/confirmed của bomber
  (R-026), **không** ghép sang pet.
- `15..18` chưa gán được nhãn ODT ("Locked In"/"Locked In Stop"); chỉ biết chúng nằm cùng nhóm khoá.
- Chưa định vị thân virtual call tại **byte +8** (slot 2 x86) của controller pet, nên không suy ra
  `len > 0`; cũng chưa đọc được vtable/byte trực tiếp (REA thiếu `read_bytes`).
- Không bao gồm: chạm/cứu, kim, thứ tự timeout-vs-touch, tốc độ khi bị nhốt.

## 9. Câu hỏi mở

[QUESTIONS](../../research/QUESTIONS.md) Q-018 (timer nội bộ, cứu/chạm, mover/`len` **của bomber**),
Q-017 (tốc độ khi bị nhốt nếu có giảm).
