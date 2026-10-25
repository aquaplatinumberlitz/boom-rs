# KR 00967 — movement, hộp va chạm và trợ góc (RE tĩnh)

Kết quả đọc tĩnh cho [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) / Q-017,
nguồn `kr_static`. Claim nằm ở [RULES_V1 §13](../../specification/gameplay/RULES_V1.md#13-re-kr-00967--movement-và-va-chạm-r-119r-125);
định danh build và bảng VA/RVA ở
[evidence](../../evidence/measurements/kr-static-2026-10-21/README.md).
Không có binary, pack, IDA DB hay decompile nguyên văn trong repo.

Khác biệt với kết luận trước: [#146] trả lời phần **hộp va chạm, neo toạ độ, trợ góc** bằng consumer KR;
phần **quy đổi cấp tốc độ → px mỗi bước** vẫn chưa định vị được consumer (§5) nên giữ `unknown`.
Sau review 2026-10-22, phạm vi lớp được thu hẹp: chuỗi consumer đã trace thuộc `CGOPet`/AI
(R-134/R-135 của PR #185 đang mở), **chưa chứng minh cho bomber người chơi**.

## 0. Phạm vi

Đọc trong module đã unpack; không chạy client, không kết nối server, không đụng `.themida`.
Đối tượng: `CMovable` (va chạm/trượt ô), mover `0x00B93890` và `CGameContext` (dispatch input bomber).
Không bao gồm bomb pass, đứng trên bom, thuyền/vật thể đặc biệt.

**Phạm vi lớp.** Chuỗi consumer của `CMovable::MoveSimple` đã trace — `0x00B95180 → 0x00B93830`
(rồi `0x00B93830` gọi cổng `0x00B95970` và mover `0x00B93890`) — thuộc `CGOPet`/AI theo R-134/R-135
của PR #185 (`re-kr-trapped-speed`, **đang mở, chưa có trên main**). R-119…R-122 vì vậy `confirmed`
**chỉ trong phạm vi `CMovable`/consumer đã trace**; áp dụng cho bomber người chơi là
`candidate`/chưa chứng minh. Đây **không** phải khẳng định bomber không dùng, cũng không phải chứng
minh không tồn tại đường gián tiếp/wrapper/VM từ bomber — chỉ là **chưa chứng minh cho bomber**.

## 1. Cách đối chiếu VN ↔ KR

Hai build cùng codebase nhưng địa chỉ khác hoàn toàn, nên không so offset. Ba mốc neo đã dùng:

1. **Chuỗi trace nhúng tên hàm.** Mỗi hàm thuộc diện này mở đầu bằng tham chiếu tới chuỗi
   `<Class>::<Method>` (ví dụ `CGameContext::Init`, `CMovable::MoveSimple`). Lọc chuỗi khớp
   `^C[A-Za-z0-9_]+::[A-Za-z0-9_~]+$` rồi lấy hàm chứa xref → cặp địa chỉ có *tên*, không phải
   suy đoán theo thứ tự. KR có 314 hàm tự khai tên kiểu này, VN có 186; phần giao cho cặp VN↔KR.
   Đây là mốc chính và đã cho ngay `CMovable::MoveSimple` ở hai build.
2. **RTTI/vtable MSVC.** Database đã có sẵn `??_7...@@6B@`; so slot cùng hành vi giữa hai build
   (VN `CGameContext` 11 slot, `Init` slot 3; KR 12 slot, `Init` slot 4 → lệch một slot ở đầu,
   xác nhận bằng việc slot 9 KR `0x00E13D00` hành xử đúng như slot 8 VN `0x006E5E05`).
3. **Hành vi trong thân hàm.** Mọi cặp còn lại được kiểm bằng việc hàm KR gọi đúng họ hàm mà
   bản VN gọi (cùng bảng hằng số, cùng thứ tự tham số).

Bảng cặp đã dùng: xem [evidence §Kết quả tra cứu](../../evidence/measurements/kr-static-2026-10-21/README.md#kết-quả-tra-cứu-va--rva).

**Tên lớp.** Bản nháp đầu gán `CGOBomber::Move`/`CGOBomber::MoveGate` cho `0x00B93890`/`0x00B93830`;
đó là **gán sai** và đã bỏ. Kiểm tra vtable/xref của PR #185 (R-134/R-135, pending) cho thấy mover
`0x00B93890` nằm trên đường `CGOPet` (slot 24 `0x00B93A60`). Trong tài liệu này chỉ gọi là
"mover `0x00B93890`" hoặc "đường `CGOPet`/AI", kèm nhãn pending khi dùng làm bằng chứng phạm vi.

**Ghi chú công cụ.** Lần đọc này bắt đầu bằng REA: `open_binary` bind đúng binary bản sao
`/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe` với `provider_id: "ida"`, SHA-256 khớp
`cf72da11…e63b`. Nhưng mọi thao tác phân tích sau đó trả `execution_failure` / "Request timed out"
sau khi idalib worker được spawn (`0x…:42835`): module 26 MB / 170.329 hàm vượt ngân sách phân tích
của adapter headless, `retryable:false`. Ghidra/Hopper không cấu hình. Vì vậy phần đọc hàm dùng
IDA MCP trực tiếp trên **database bản sao** đã phân tích sẵn (`kr-full.exe.i64`); không mở DB gốc.

## 2. Đường đi của một bước di chuyển (KR)

```
Đường bomber — chỉ phần đã trace:
input (phím) -> CGameContext input dispatch 0xE13D00
                 -> bomber = getBomber(slot)            (0xE12FC0)
                 -> controller = (byte*)bomber + 1412   // sub-object NHÚNG; con trỏ vtable nằm tại đây
                 -> controller->vf[+12](dir)            // slot 3 (byte +12) của vtable đó; dir 0..3, dir 4 = đặt bom
   // chưa thấy nơi đọc lại dir hoặc tạo/giữ len cho bomber

Đường CMovable — consumer đã trace; theo R-134/R-135 (PR #185 pending) là CGOPet/AI,
KHÔNG phải bomber:
0xB95180  ->  0xB93830   (B93830 gọi cổng 0xB95970 rồi mover 0xB93890)
                 -> mover 0xB93890(dir, &len)   vòng lặp:
                      CMovable::MoveSimple 0x1310200(dir, &len)
                      nếu len != 0:
                        đòi thẳng hàng ô 0x1310C50(dir)   (x%40==0 hoặc (y-39)%40==0)
                        đòi ô đích tồn tại 0xEA5D90
                        trượt vuông góc: dir dọc -> 0x1310D30(dir, hàng đích, &len)
                                         dir ngang -> 0x1310EE0(dir, cột đích, &len)
```

**Phạm vi bằng chứng:** với bomber, đường đã trace chỉ chứng minh **hướng đi vào controller**
(`vf3(dir)`). Chuỗi mô phỏng trong tick là của `CGOPet`/AI, **không** được gộp thành mô phỏng bomber.
Đường này cũng **không** chứng minh KR không gửi gói C→S mang vị trí: controller bomber nhúng tại
`+1412` và các đường mạng/khác chưa được trace, và R-047 là bằng chứng của build VN chứ không phải
phủ định cho KR. (`+0xE0` là controller sub-object của `CGOPet`/AI ở nhánh pet §5, **không** phải
controller bomber.) Vì vậy R-123 chỉ `confirmed` cho phần dispatch; mô phỏng bomber và phần phủ định để
`candidate`/chưa chứng minh với phạm vi đã tìm (Q-017).

Tóm tắt tự viết của `CMovable::MoveSimple` (không phải nguyên văn):

```
x = pos.x ; y = pos.y                       // y là mép DƯỚI của hộp bàn chân
feet = rect(x, y-39, 40, 40)
kéo dài feet theo hướng dir thêm `len` px    // rect quét
feet = feet ∩ vật cản
nLen = feet.w + feet.h - 80                  // 80 = 40+40 của hộp đầy đủ
nếu nLen < 0: lỗi "Should not happen"
len -= nLen ; pos = (x + nLen*dx, y + nLen*dy)
trả nLen
```

Hệ quả: quãng đường thực hiện đúng bằng phần hộp quét lọt qua vật cản (không vượt), phần dư trả lại
cho vòng lặp ngoài — vòng lặp đó mới là chỗ thử trượt vuông góc.

## 3. Hằng số và đơn vị

Các hằng số dưới đây đọc trong `CMovable`/mover (consumer đã trace: `CGOPet`/AI).

| Đại lượng | Giá trị KR | Đơn vị | Nơi đọc |
|---|---|---|---|
| Cạnh ô | `40` (`0x28`) | px | `0x1310D30` (chia `idiv 0x28`), `0x1310EE0`, `0xB93890` |
| Hộp bàn chân | `40 × 40` | px | `MoveSimple`: `rect(x, y-39, 40, 40)`, `w+h-80` |
| Neo dọc | `y = hàng·40 + 39` | px | `MoveSimple`: `(y-39)/40` là hàng, `y/40` là hàng cuối |
| Neo ngang | `x = cột·40` | px | `MoveSimple`: `x/40`, `(x+39)/40` |
| Vector hướng | `0=(-1,0) 1=(0,-1) 2=(1,0) 3=(0,1)` | — | bảng `0x17929B0`, chỉ số `2*dir` |
| Tâm hộp | `x+20`, `y-19` | px | `0x1310D30` dùng `x+20` cho ngưỡng trợ góc |
| Ngưỡng trợ góc | `14` và `26` quanh tâm | px | `0x1310D30`: `x+20 >= 40·nextCol+14`, `x+20 < 40·curCol+26` |
| Trượt vuông góc tối đa | `34` | px/bước | suy ra từ `r = x % 40 ∈ [6,34)` (§4) |
| Độ dài mỗi bước | do caller cấp | px | `MoveSimple(this, dir, &len)` không tự tính |

Quy đổi: 1 ô = 40 px; một bước mô phỏng client là 30 ms (R-013, đo trên VN), nên
`px/s = px_mỗi_bước × 33,33` nếu bước = 30 ms — **chỉ là quy đổi, chưa nối với KR**.
**Cấp tốc độ không phải px/s** — bảng `BomberAttr` chỉ cho cấp (R-040…R-042).

## 4. Trợ góc (corner assist) — dẫn xuất đầy đủ

Hàm `0x1310D30` chạy khi đối tượng `CMovable` đi **dọc** và vừa bị chặn; nó trượt theo trục X.
`0x1310EE0` là bản đối xứng cho đi ngang, trượt theo trục Y. Cả hai chỉ được gọi sau khi `0x1310C50`
xác nhận **đã thẳng hàng ô trên trục di chuyển** (nếu chưa thẳng hàng thì dừng, không trượt).

Với `r = x % 40`, `curCol = x/40`, `nextCol = (x+40)/40 = curCol+1`, và hai biến
`curBlocked = blocked(hàng đích, curCol)`, `nextBlocked = blocked(hàng đích, nextCol)`:

| `r` | `curBlocked` | Kết quả |
|---|---|---|
| `[0,6)` | không | trượt **trái** `r` px (0…6) về mép ô hiện tại |
| `[0,6)` | có | không trượt (đã sát mép trái, trượt phải sẽ quá xa) |
| `[6,34)` | có | trượt **phải** `40 − r` px (6…34) sang ô kế, nếu ô kế không bị chặn |
| `[6,34)` | không | trượt **trái** `r` px (6…34) về ô hiện tại |
| `[34,40)` | — | trượt **phải** `40 − r` px (0…6), nếu ô kế không bị chặn |

Ngưỡng `14`/`26` trong code chính là hai mốc `r ≥ 34` và `r < 6` viết theo tâm hộp (`x+20`).
Điểm quan trọng: trợ góc **không** cho đi xuyên ô bị chặn — nó chỉ căn hộp 40 px vào một làn còn
trống rồi vòng lặp ngoài thử đi tiếp. Đây là "snap" trong một bước, không phải trượt dần. Kết luận
này là của `CMovable` (consumer đã trace: `CGOPet`/AI); **chưa chứng minh cho bomber**.

## 5. Tốc độ: chưa có consumer cho bomber

`CMovable::MoveSimple` **không** biết tốc độ: nó nhận `len` (px còn lại) từ caller. Chuỗi gọi đã
truy được tới:

- mover `0x00B93890(dir, len)` — vòng lặp, `len` nguyên vẹn từ tham số;
- `0x00B93830(dir, len)` — gọi cổng `0x00B95970` (kiểm trạng thái) rồi mover `0x00B93890`;
- `0x00B95180()` — lấy cặp `(dir, len)` từ **sub-object `+0xE0` của `CGOPet`** qua `vf[+8]`
  (byte +8 = slot 2 trên x86), rồi gọi `0x00B93830`.

Theo R-134/R-135 (RULES_V1 §15, PR #185), chuỗi này thuộc `CGOPet`/AI và sub-object `+0xE0` là của
`CGOPet`, **không phải bomber**. Với bomber, đường đã trace mới chỉ là
`0x00E13D00 → controller nhúng +1412 → vf3(dir)` (nhận hướng); **chưa thấy** nơi tạo/đọc `len` hay
mover của bomber. Vì vậy:

- R-125 giữ `unknown`: **không** claim px/s cho KR; R11424 là lựa chọn lịch sử, hiện R18433 DESIGN theo curveVN R183;
- R-043 (5 px/tick ở cấp 5, VN, checksum chưa xác thực) vẫn là `candidate`, **không** dùng để suy ra
  KR dù nó khớp giả thuyết "px mỗi bước = cấp tốc độ".

Bước tiếp theo cụ thể: đọc slot 24 của bomber (`CGOGameBomber` `0x00B77B10`, `CGOBomber` `0x00B44000`)
tìm mover/setter vị trí; nếu thân hàm nằm trong vùng `.themida` thì dừng, ghi entry VM theo
[DEVIRT_METHOD](DEVIRT_METHOD.md) và không mở hướng devirt lớn.

## 6. Khác biệt với profile hiện tại và với BnB

Profile [`data/rules/v1-dev.toml`](../../../data/rules/v1-dev.toml) đang dùng R-116 cho va chạm.
Các giá trị KR dưới đây đọc trong `CMovable` (consumer đã trace: `CGOPet`/AI) — **không** đủ để đổi
luật người chơi:

| Khóa profile | Giá trị đang dùng | Nguồn hiện tại | KR đọc được (phạm vi) | Đề xuất |
|---|---|---|---|---|
| `player.collision_half_px` | `13` (hộp 26×26) | `oss`/design R-116 | hộp bàn chân **40×40**, neo `y−39` — trong `CMovable` (consumer `CGOPet`/AI); bomber chưa chứng minh | **không** đổi profile ở PR này; áp dụng cho bomber chỉ `candidate`, cần consumer bomber + PR profile riêng |
| `player.corner_assist_px` | `12` | `oss`/design R-116 | trượt tới **34 px** nhưng chỉ khi đã thẳng hàng ô và chỉ để căn vào làn trống — trong `CMovable` (consumer `CGOPet`/AI); bomber chưa chứng minh | **không** đổi cơ chế/số cho bomber khi chưa có consumer; chỉ `candidate`/thử nghiệm |
| `player.trap_touch_distance_px` | `28` | `oss`/design R-116 | chưa đọc (ngoài phạm vi #146) | giữ, #147 |
| `grid.tile_px` | `40` | `vn_static` R-110 | `40` dùng trực tiếp trong `CMovable` (consumer `CGOPet`/AI) | không đổi; thêm consumer KR |
| `player.speed_px_per_s_per_level` | `33` | `design`/design R-184 (curveVN R183) | chưa xác định | giữ, xem §5 |

BnB `server/game.js:31–32,150–153,173–275` mô hình hoá hộp 26×26 + assist 12 px. Trong phạm vi
`CMovable`/consumer đã trace, KR cho thấy mô hình BnB **không** phải mô hình gốc (hộp đúng một ô,
assist là phép căn làn). **Nhưng** consumer đã trace là `CGOPet`/AI, nên **không** thể dùng kết quả
này để kết luận cho bomber người chơi hay thay R-116 khi chưa có consumer bomber.

## 7. Giới hạn

- Toàn bộ là đọc tĩnh. Chưa có trace runtime KR cho bất kỳ claim nào ở đây; không suy ra hành vi mạng
  hay thứ tự tick server.
- Ngưỡng trợ góc đọc từ hằng số; "tối đa 34 px" là suy ra hình học, chưa đo trong trận.
- Chuỗi `0xB95180 → 0xB93830` (rồi cổng `0xB95970`, mover `0xB93890`) đã trace thuộc `CGOPet`/AI
  (R-134/R-135, PR #185 pending). **Chưa chứng minh** bomber người chơi dùng cùng đường; cũng **không**
  khẳng định bomber không dùng hay không tồn tại đường gián tiếp/wrapper/VM khác. Phần lớn caller của
  `CMovable::MoveSimple` chưa được phân loại lớp.
- Điều kiện "ô bị chặn" (`0xEA0FD0`) gộp nhiều vị từ; polarity của `CGOTileFixed`/`CGOTilePassable`
  trong `0xE9F9B0` chưa chốt (R-124 để `candidate`).
- Bomb pass/đứng trên bom đã đọc ở R-154 (§9); không bao gồm cứu/được cứu, thuyền.

## 8. Câu hỏi mở

Theo dõi ở [QUESTIONS](../../research/QUESTIONS.md): Q-017 (speed/px-per-step còn thiếu; mover/`len`
của bomber chưa định vị), Q-004 (tick client 30 ms đã dùng để quy đổi nhưng chưa nối với KR).

## 9. R-154 — bomb pass từ phía object bom (2026-10-23)

Lượt hẹp #146/Q-017, V1-012/013. Chỉ theo object bom → predicate → consumer movement;
không suy hộp người chơi/assist từ pet. Build KR `00967`, REA MCP bản sao `kr-full.exe`
SHA-256 `cf72da11…e63b`.

### 9.1 Predicate trên `CGOBomb`

- `CGOBomb` vtable base `0x174426C` (slot 24 update `0xB391E0`; Init `B384E0` slot 40).
- Slot **+156** = `0xB37DB0` (RVA `0x737DB0`): trả `0` nếu field `+328` (`0x148`) bằng `1` hoặc `2`
  (`B38250`/`B38290`); ngược lại trả `CSensorTrigger` flag tại `+152` (`11982F0(bomb+144)`). Vì vậy
  `+328 ∉ {1,2}` **không đủ** để predicate khác `0`: flag `+152 == 0` cũng cho `0`.
- `B37150` ctor gọi `1198240(bomb+144, 1)` ⇒ `CSensorTrigger` flag `+152 = 1` mặc định.
- `B384E0` Init ghi `+328` từ **u8 descriptor+108** (`sub_529C20(a2+108)`); chưa thấy setter runtime
  khác trong phạm vi đọc.

### 9.2 Consumer `MoveSimple` — chỉ nhánh pet/AI đã đọc

`CMovable::MoveSimple` `0x1310200` (consumer đã trace thuộc **pet/AI**, R-134/R-135; **chưa** nối mover
bomber) lấy object ở ô đích bằng `E9F9B0`, gọi virtual **+156** (`v32`/`v30`) và coi kết quả `0` là
chặn. Nhưng đây chỉ là **một** điều kiện: đường va chạm còn **OR `EA0FD0`** (quanh `0x13109BE`) và các
gate virtual **`+164`** phía trước. Predicate khác `0` **không đủ** kết luận đi qua.

- **Không** suy được player/bomber đi ra, đi vào, grace hay “không owner toàn hệ thống” từ nhánh pet/AI.
- **Trong thân `B37DB0` không đọc owner/player/overlap**. `B38370` (`+268`) là getter owner **candidate**;
  nó có **14 callsite/xref**; đã đọc **4 hàm** (`ACC340`, `AEF700`, `EB73D0`, `EB8000`), các callsite còn lại **unknown**, và
  **không** được `B37DB0` gọi.
- `+328` lấy từ descriptor+108; nghĩa/giá trị C00/C01, setter runtime và **thời điểm mất pass/timer**
  vẫn `unknown`. Không suy 40×40/assist từ pet.

### 9.3 Phân loại 4 caller của getter `B38370` (`+268`)

Batch đọc 4 thân của lượt trước; không đọc lại hàm R-154:

| Caller | Call `B38370` | Cách dùng `+268` | Kiểm collision/grace trực tiếp? |
|---|---|---|---|
| `ACC340` | `0xACC4BA` | gán vào `v20` rồi dựng descriptor event (`AD4A40`/`AD62F0`/`AD9F20`) | **Chưa thấy**; không gọi trực tiếp `B37DB0`/`EA0FD0`/`+152`/`+328` |
| `AEF700` | `0xAEF904` | **bỏ kết quả**; nhánh bomb/item dùng `B378F0`/`B37880`, đọc `a2+176` | **Chưa thấy** |
| `EB73D0` | `0xEB75C1` | ghi `*(v74+4)=owner`, truyền `B70C10` dựng effect | **Chưa thấy** |
| `EB8000` | `0xEB8FA8`, `0xEB910E` | truyền owner vào `B70C10` cho flame/effect | **Chưa thấy** |

- Cả 4 thân chỉ thấy dataflow **descriptor/event, effect/flame hoặc bỏ kết quả**; **chưa thấy kiểm
  collision/grace trực tiếp** (không gọi trực tiếp predicate `B37DB0`, `EA0FD0`, flag `+152`/`+328`
  hoặc `MoveSimple`). Helper/callee và **các callsite còn lại chưa loại trừ**.
- Vì vậy **không tạo R-155**: chưa thấy kết nối trực tiếp `+268` vào collision/grace; player/bomber vẫn
  `unknown`, không nâng owner thành confirmed.

### 9.4 Evidence

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_5e78903bb67e4aedd78c545ed1e9fb28e468e31b81f59c699dc5a5d52ab8b3b0` | `xrefs` `B391E0` | vtable slots |
| `ev_dd2e57841bf0c657c6ad6c5011585efee285b300934b2bf7ab61570413ce1748` | `xrefs` slot `17442CC` | vtable base |
| `ev_4b7e3be19829ef5c67ec2ea460e85d88a52474154ca11cf60bda5cd84c3190a7` | `xrefs` `174426C` | 2 ctor users |
| `ev_dc3d84632f04b7616773024c1058afe0ae3659040f8ff455e8c57a3a9883f73f` | `batch_decompile` `B37150`/`B37560`/`B384E0` | ctor/Init/`+328` |
| `ev_5d06d1cab12064f869b98fb3190c705d121c999678088da9a1bb3d47d26c28d7` | `batch_decompile` `B37DB0`/`B393D0` | pass predicate |
| `ev_a546b755dea30f480cb1532c4bb0d4c158c23e62c6bfdd451056bb6bfc2bac0c` | `batch_decompile` `B38250`/`B38290`/`11982F0` | `+328`/`+152` |
| `ev_3bb870f56d8520b9f50ac48277eb33172caa25940c5d0de42a78ddb71588cb45` | `batch_decompile` `CSensorTrigger` | ctor flag |
| `ev_6b5946b42a71659cbda31f34a6d4e2f65ca5af068973517f54a8cd8833fdc504` | `procedure_pseudo_code` `1310200` | MoveSimple consumer |
| `ev_1ed26a6f3c1bb7dff196630d81395bb64cda343c123acd908e9b8129e231dc4e` | `xrefs` `B38370` | owner getter callers |
| `ev_4d7754d19b3e1394445bc19e34f1ce72ea04598b46af0ce90b5fc890fc604d0b` | `procedure_pseudo_code` `B391E0` | update/state |

**Kết luận:** `kr_static` **confirmed** thân predicate `B37DB0`, ctor/flag và **nhánh `MoveSimple` có
phạm vi pet/AI**; `candidate` owner getter `+268` (chưa phân loại xref); `unknown` đường bomber, nghĩa/
giá trị `+328`/descriptor+108, setter runtime, owner role, player grace/timer, VN counterpart. Không đổi
profile; không suy bomb luôn pass hay player pass.

### 9.5 Caller `B38370` mới — B3AE90 / B9A990 / E3 sites (2026-10-24)

Tiếp #146, chỉ đọc thêm **3 nhóm callsite**; xref của getter là **14 callsite**, không phải 14 hàm. Lượt trước đã đọc 4 thân; các nhóm site còn lại chưa phân loại, không suy “4 hàm ⇒ 10 hàm còn lại”.
Provenance thật: batch `B3AE90`/`B9A990` subject
`/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`; batch `B996B0`/`B9A3B0` và xref `B9A3B0` subject
`/home/admin/boom-rs-dsh/re-kr2/.re-tmp/kr-full.exe`, DB copy `rea-ida-1e1Km8` (Codex xác nhận cùng
SHA `cf72da11…e63b`); E3 resolve subject boom-team, DB `rea-ida-GI3Raz`. Không nói đây là clone1.

| Call site | Containing function | Class/`this` | Dùng `+268` | Kết luận |
|---|---|---|---|---|
| `0xB3AECE` | `B3AE90` (RVA `0x73AE90`) | **candidate** effect/descriptor builder (`B3B150`); chưa có vtable proof | `B38370(this)` → `B3ADD0(owner)`, rồi chọn tên effect `BombRPGmonbomb`/`BombSprinklerb`/`BombRemotebomb`/`Bombdeco` | Chỉ **effect selector**; không thấy so player/overlap/grace |
| `0xB9A99C`, `0xB9A9C8` | `B9A990` (RVA `0x79A990`) | **candidate** object/effect; class chưa chứng minh | **guard `B38370(this) != -1`**: khi khác `-1`, `a = 1241780(B38370(this))`, `b = 1241780(1240B00(0))`, ghi `this[172] = (a == b)`; nhánh `-1` **không ghi/không reset** | Có **so hai kết quả transform**; helper chưa chứng minh nghĩa owner/local/team; **chưa** thấy collision/overlap/grace. `this[172]` = byte **+0x2B0 (688)**, không phải +172 |
| `0xE3C4A2`, `0xE3C563` | `not_in_procedure` (IDA không nhận function) | chưa xác định | chỉ có xref/resolve; **không** đọc body/disasm mới | Chỉ kết luận lookup không trả containing procedure; **không** kết luận class/consumer, không đồng nghĩa VM/không tồn tại |

- `B9A3B0` gọi `B9A990` sau tính vị trí/rect; `B996B0` là wrapper kiểm `this[172]` rồi gọi lại.
  Chưa nối `this[172]` (byte `+0x2B0`) tới collision/grace; không suy từ tên field.
- Helper vừa đọc (`ev_8134a6cc…`): `1240B00(this,a2)` **trả kết quả gọi `57A870` tại**
  `this+68+12*a2`; thân này chưa trực tiếp chứng minh đọc giá trị hay nghĩa trường. `1241780(this,a2)`
  guard `a2` trong `[0, this[41]-1]`, gọi accessor `529C60` trên `*(this+42)+792*a2+112`, ngoài range
  trả `-1`. Semantics callee vẫn **unknown**, không gán local/team.
- Các index khác trong body (`this[162/163/166/167/175/176]`) cũng là index `_DWORD*`, tương ứng byte
  +0x288/+0x28C/+0x298/+0x29C/+0x2BC/+0x2C0; không nhầm với virtual slot.
- **Không** mở các callsite còn lại; helper/callee chưa đọc vẫn **unknown**. Không đổi player grace,
  40×40/corner hay R-154. `+268` vẫn **candidate owner**, không phải owner confirmed.

Evidence: `ev_0216a581…` batch B3/B9; `ev_1e18205f…` batch B996/B9A3; `ev_fb3ed4dd…` xref B9A990;
`ev_d9c085e8…` xref B9A3B0; `ev_868df24e…`/`ev_c23c3eb1…` E3 resolve. Bundle
`.re-work/evidence-owner-player-sites-146.json`.

### 9.6 Player grace sau đặt bom — lượt 24/10 (#146/Q017/G2)

Hỏi: người **vừa đặt bom** có được đi ra/đi xuyên/quay vào không, theo owner/overlap/flag nào và mất
khi nào. Baseline `6a2473f`; chỉ đọc thêm 4 thân mới + xref; không giải lại B3AE90/B9A990/B996B0/
B9A3B0/1240B00/1241780/B37DB0/1310200/B44000/B77B10. Provenance: 7 record lọc
`.re-work/evidence-player-grace-146-r1.json`; subject boom-team; các xref record ghi DB copy `rea-ida-GUJhP1`,
hai batch record không có metadata DB riêng; cùng SHA `cf72da11…e63b`.

| Hàm / site | Dùng gì | Kết luận cho grace |
|---|---|---|
| `B40030` (RVA `740030`) | `this`; `B323D0(this)`, virtual `+100` lọc mã, `B40D60(this)==0`, virtual `+132(this,-1)==0` | Không thấy owner/query ô trong thân; nhãn **valid/idle/type chỉ candidate** (chưa proof); không gọi 0 là idle |
| `B40290` (`740290`) | receiver **`this-353`** (khác B40030); cùng cấu trúc `B323D0/vf+100` lọc mã, `B40D60==0`, `vf+132==0` | Cùng **candidate** valid/idle/type; không owner/grace |
| `B422F0` (`7422F0`) | `B40D60==0`; `v6/v7` lấy từ controller `this+353` qua vtable `+28/+32`, rồi `E9F6D0(v6,v7)`; `B9DB70` chỉ gọi helper, **body chưa đọc** | Map query **chưa chứng minh là ô đích**; `B9DB70` result/ID và action context chỉ **candidate**; không owner/timer |
| `B42880` (`742880`) | `B40D60==0`; sound/virtual `+384/+704`; `AE7710` | Nhánh action/state **candidate**; không owner/overlap/grace |
| `B37DB0` (`737DB0`) | `+328` type ∈ {1,2} → 0; ngược lại `11982F0(bomb+144)` = sensor flag `+152` | **Không đọc owner/player/overlap trực tiếp**. Xref chỉ **6 data** (vtable slots); `MoveSimple` đã biết pet/AI, không nối player |
| `B9A990` (`79A990`) | guard `B38370(this)!=-1`; so transform `1241780(B38370(this))` / `1241780(1240B00(0))`; ghi `this[172]` (byte `+0x2B0`) | Có **so owner-like** nhưng class/consumer chưa đọc. `0x1747328` là **entry slot 43**, không phải vtable base; chỉ `B9A3B0→B9A990` có call/body cache, chain `B994C0/B99CF0` **chưa confirmed** |

- `B40D60` chỉ trả field `+0x5A0` (= `bomber+1440`), đã biết từ R-137 là field nhận **dir/input**,
  không phải timer grace. Xref normalized là **24 site**, không phải 24 caller function; chỉ 4 thân
  `B40030/B40290/B422F0/B42880` được đọc. Không nói cả 24 site không owner/overlap.
- `B37DB0` là predicate **kiểu bom + sensor flag**; flag `+152` mặc định 1, setter runtime/consumer
  người chơi vẫn chưa đọc. Đây là negative **có giới hạn** trong 4 thân đã đọc, không loại trừ callee/
  virtual chưa đọc.
- **Player grace trên đường KR của lượt này = unknown**: chưa có bằng chứng owner-specific, overlap-specific hay timer; không
  suy từ pet/AI, không đổi 40×40/corner/grace hay R-154. `+268` vẫn candidate owner; `+0x2B0` vẫn
  unknown. Cạnh tiếp: đọc `B99CF0`/`B994C0` hoặc caller virtual `+156` của player — cần budget mới.

## 10. Player mover VN có capture riêng — R-176 (25/10)

Mục9 là lịch sử bounded-negative của đường KR, không phải hàng đợi runtime hiện hành. VN owner
grace đã có R-155; không dùng kết luận pet/AI để phủ định dữ liệu player mới.

[R-176](../../evidence/measurements/vn-player-movement-2026-10-25/README.md) có rare capture trên
actorC664C4 được điều khiển, moverv244→59DA6F, nhánh59DABE→594B68. Tại59DAD9, nativeXY560/519
và argument16byte=`[560,480,40,40]`. Nguồn X/Y−39/40/40 đã có prefix tĩnh; đó chưa chứng minh
hình hurtbox hoặc toàn narrow-phase collision. Không đồng nhất rectangle với R-119 chỉ vì cùng40.

Hai ca chỉ điều khiểnDown sau khi nativeXY ổn định tại540/39 và535/39 cho sửa ngang về560 rồi
Y tăng. Candidate corner assistance của player VN có trajectory và positive/opposite controls;
chưa có ca âm để chốt ngưỡng. Không gán max34 từ pet, không thay assist12/radius13DESIGN.
Tiếp theo là negative offset hoặc consumer đã định vị, không đọc lại các thân pet/owner đã dừng.

R-177 đã bổ sung [negative offset control](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#đối-chứng-lệch30-và35-trên-cùng-actor--r-177)
trên actor khác449280160: lệch30 có sửa ngang/Down, lệch35 đứng lại, alignedDown vẫn hoạt động.
Native coordinate response trong ca là confirmed; chưa chốt max34 hoặc consumer corner/hurtbox.
Giữ profile13/12DESIGN, unknown của KR và các ngữ cảnh chưa đo; không cần đo lại hai offset này.

## Nhánh tile passable của player VN — R-179

[Code path/case/RTTI](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#predicate-ô-passable-của-player--r-179)
bổ sung consumer VN cho player59DA6F, độc lập với PET/AI KR. Caller59DD50 lấy object qua7293B4,
guard/cast `CGOTileFixed→CGOTilePassable`, rồi59DD5E gọi5C1027/5C0E98. Bốn dir0..3 trả byte
`[[tile+AC]+4+dir]`; return0 đặt flag chặn. Row101stride/tableB328 và guard-zero/epilogue đã đọc.

Đây là static branch confirmed, chưa là runtime activation trong Factory01. Không đổi R119/R122
PET thành player; không chốt hurtbox hay maxassist34. Phần còn thiếu cụ thể là producer/property
values, nhánh actor.vAC tại59DE86/59DEA1 và cách geometry/corner quyết định kết quả. Các helper
rect adjustment/intersection không thay cho map/block predicate; không đọc lại chúng.

## Curve native VN và quy đổi profile — R183/R184

[12window cùngactor, method/hash/đối chứng](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#curve-tốc-độ-cùng-actor--r-183r-184r-185)
đo logicalpoint4..8:133,70/166,08/200,70/233,23/267,05px/s. Tĩnh xác định point/receiver/consumer;
rate lấy nativeXY với HOSTreadbracket ngoài breakpoint, không keyhold/packet/30msclientstep.
R184 chọn33DESIGN thay24; không nativeFPS/hằng33. R125KRunknown/R043candidate không nâng nhãn;
PET/AI source không làm bomberKRconfirmed. #146 vẫn mở hurtbox/corner/othermode/nonowner;
không đo lại curve này nếu chưa có thay đổi endpoint/clock/quantity cụ thể.
