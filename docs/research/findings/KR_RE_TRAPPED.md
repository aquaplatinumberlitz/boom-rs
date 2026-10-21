# KR 00967 — bị nhốt (trapped): state, di chuyển, khoá và chết

Kết quả đọc tĩnh cho [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / Q-018,
nguồn `kr_static`. Claim ở [RULES_V1 §13](../../specification/gameplay/RULES_V1.md#13-re-kr-00967--bị-nhốt-r-126r-131);
định danh build và bảng VA/RVA ở
[evidence](../../evidence/measurements/kr-static-trapped-2026-10-21/README.md).
Không có binary, pack, IDA DB hay decompile nguyên văn trong repo.

**Kết luận chính:** KR **cho người bị nhốt di chuyển** (R-127) — ngược với R-036/R-108 của V1
(`trapped` đứng yên, lấy từ BnB). Đây là claim `confirmed` có consumer, không phải suy đoán từ asset.

## 0. Phạm vi và cách đọc

Phiên này dùng REA làm đường chính: `open_binary` binary **bản sao** với `provider_id: "ida"`, engine
seed sẵn database đã phân tích (`auto_analysis_ready`, `hexrays_ready`), nên mỗi hàm chỉ mất ~0,1–0,5 s.
Riêng `read_bytes`/`search_bytes` bị adapter IDA của REA từ chối (`capability_unavailable`,
"operation is not declared by this provider") nên phép đọc vtable/byte không thực hiện được trong phiên
này; mọi kết luận dưới đây chỉ dựa trên decompile + xrefs, không dựa trên byte đọc trực tiếp.

Đối tượng: state machine của bomber (`sub_B952E0` và cụm `0xB93xxx–0xB96xxx`). Không bao gồm
server, packet runtime, hay client chạy thật.

## 1. Mã state của bomber

Bomber có getter/setter state là virtual `+100`/`+104`. Các giá trị đọc được và bằng chứng:

| State | Nghĩa | Bằng chứng |
|---|---|---|
| `0`, `1`, `4` | trạng thái nền (không di chuyển) | `sub_B95970` loại `{0,1,4,13,14}` khỏi "được đi" |
| `2` | đứng/stop | `sub_B952E0` set `2` khi cờ sống tắt; `sub_B95840` không dùng |
| `3` | **bị nhốt** | `sub_B952E0` set `3` ở nhánh "còn sống"; VN: packet C→S 120 subtype 11 = trạng thái 3 (bị nhốt) — `oc_e-game.md` §5b |
| `8` | stop sau khi rời nhóm khoá | `sub_B95BD0(this, 0)` → `SetState(8)` |
| `9..12` | đi theo 4 hướng (nhóm thường) | `sub_B95840`: `SetState(dir + 9)` |
| `13` | đang chết | `sub_B952E0` case 13 → `SetState(14)`; input dispatch `sub_B95180` bỏ qua state 13 |
| `14` | chết | `sub_B952E0` case 14 → `SetState(2)`; `sub_B95180` bỏ qua state 14 |
| `15..18` | nhóm "đã khoá" (kèm `19..22`) | `sub_B95E00` trả true cho `15..18` **hoặc** `19..22` |
| `19..22` | đi theo 4 hướng (nhóm khoá) | `sub_B95840`: `SetState(dir + 19)` khi đang ở nhóm khoá |

Tóm tắt tự viết của `sub_B952E0` (state machine mỗi bước cập nhật, không phải nguyên văn):

```
giảm bộ đếm giữ this[235] hoặc this[236] nếu > 0
nếu controller báo "đang đi" != (đang ở nhóm khoá): sub_B95BD0(1 hoặc 0)   // đổi nhóm + đặt 20
nếu cờ sống (controller vf60):
    nếu state != 3 và state không thuộc {di, nhốt-khoá, chết}: SetState(3)
ngược lại nếu state == 3: SetState(2)
switch (state): case 13 -> (khi a2==2) SetState(14); case 14 -> (khi a2==2) SetState(2)
```

## 2. Người bị nhốt vẫn di chuyển (R-127)

Ba mắt xích, đều có consumer:

1. **Input không bị chặn.** `sub_B95180` (cập nhật điều khiển bomber) chỉ bỏ qua state `13`, `14`, `1`.
   State `3` đi tiếp vào nhánh lấy `(dir, len)` từ controller và gọi `CGOBomber::MoveGate`.
2. **Cổng di chuyển cho qua.** `sub_B95970` trả `false` cho `{0,1,4,13,14}`; state `3` rơi vào nhánh
   mặc định: được đi nếu hai bộ đếm `this[235]`/`this[236]` đều `0`.
3. **MoveGate dùng đúng cổng đó.** `CGOBomber::MoveGate` `0xB93830` trả `0` (không đi) khi
   `sub_B95970` trả `0`; ngược lại gọi `CGOBomber::Move` `0xB93890` với nguyên `len`.

Hệ quả: `trapped` **không** phải trạng thái đứng yên trong client KR. V1 đang mô hình hoá theo BnB
(R-108) nên khác client gốc.

**Chưa chứng minh:** tốc độ khi bị nhốt có bị giảm hay không. `len` đi từ controller tới `MoveSimple`
không bị nhân/chia trong chuỗi này; nếu có giảm thì nằm trong controller (`vf8`), chưa định vị.

## 3. Nhóm state đi "khoá" và bộ đếm giữ 20 bước (R-128, R-129)

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

`sub_B952E0` giảm `this[235]`/`this[236]` mỗi bước cập nhật và chỉ gọi `sub_B95BD0` khi "đang đi" đổi
pha. Trong lúc bộ đếm `> 0`, `sub_B95970` trả `false` ⇒ **cổng di chuyển đóng 20 bước cập nhật**.
Ở 30 ms/bước (R-013, đo trên VN) thì ≈ **600 ms**; đây là **quy đổi**, chưa đo trên KR, và không phải
thời gian bị nhốt.

## 4. Chết (R-130)

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
| `trapped` đứng yên (R-036, R-108) | đứng yên | **đi được**, có họ state đi riêng | đổi mô hình: `trapped` vẫn nhận input; PR cơ chế riêng |
| `player.trap_to_death_ms` (R-115) | `4000` (`oss`/design) | chưa xác định (R-131) | giữ |
| `player.trap_touch_distance_px` (R-116) | `28` (`oss`/design) | chưa đọc (đường chạm/cứu chưa lần ra) | giữ |
| Cứu/chọc (R-028 `vn_runtime`) | địch chạm → chết | chưa đọc được nhánh chạm | giữ, Q-018 |

## 8. Giới hạn

- Chỉ tĩnh, không chạy client; không có trace KR cho state 3 hay bộ đếm 20.
- Nhãn "bị nhốt" cho số `3` ghép từ code KR (state machine) + tài liệu packet VN; bản thân KR không có
  chuỗi đặt tên cho state.
- `15..18` chưa gán được nhãn ODT ("Locked In"/"Locked In Stop"); chỉ biết chúng nằm cùng nhóm khoá.
- Không đọc được vtable/byte do adapter IDA của REA thiếu `read_bytes`; mọi suy luận về slot virtual
  dựa trên hành vi trong pseudocode.
- Không bao gồm: chạm/cứu, kim, thứ tự timeout-vs-touch, tốc độ khi bị nhốt.

## 9. Câu hỏi mở

[QUESTIONS](../../research/QUESTIONS.md) Q-018 (timer nội bộ + cứu/chạm), Q-017 (tốc độ khi bị nhốt
nếu có giảm).
