# Đo tĩnh KR 00967 — movement/va chạm (2026-10-21)

Metadata của phép đọc tĩnh dùng cho claim `kr_static` R-119…R-125
([RULES_V1 §13](../../../specification/gameplay/RULES_V1.md#13-re-kr-00967--movement-và-va-chạm-r-119r-125)).
Không chứa binary, decompile nguyên văn hay dữ liệu pack. Kết luận và cách suy luận nằm ở
[KR_RE_MOVEMENT](../../../research/findings/KR_RE_MOVEMENT.md).

**Phạm vi lớp.** Các hàm `CMovable::MoveSimple`/trợ góc và chuỗi consumer `0x00B95180 → 0x00B93830`
(rồi cổng `0x00B95970`, mover `0x00B93890`) đã trace thuộc `CGOPet`/AI theo R-134/R-135 của PR #185
(`re-kr-trapped-speed`, **đang mở, chưa có trên main**). Vì vậy R-119…R-122 `confirmed` **chỉ** cho
`CMovable` trong phạm vi consumer đã trace; áp dụng cho bomber người chơi là `candidate`/chưa chứng
minh. R-123 chỉ `confirmed` cho dispatch `vf3(dir)` của bomber. Đây **không** phải khẳng định bomber
không dùng hay không tồn tại đường gián tiếp/wrapper từ bomber — mới là **chưa chứng minh cho bomber**.

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `/home/admin/boomkr-re/out/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |
| VN `VBNB_116` (đối chiếu) | `/home/admin/boom-re/unpacked_ca.exe` | — | `13a0a33cccc8f5f5124cdd8dac863b68118e6dda83340649b897fc41faed364a` | `0x00400000` | `VA − 0x00400000` |

Image base lấy từ database (`0x400000`); `.text` bắt đầu `0x401000`. Mọi địa chỉ dưới đây là VA trong
không gian module đã unpack, tức `RVA = VA − 0x400000`.

Hai database IDA được đọc là **bản sao**, không phải DB gốc:

- `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe.i64`
- `/home/admin/boom-team/tmp/re-kr/ida/unpacked_ca.exe.i64`

Không mở `.i64` gốc của `boomkr-re`/`boom-re` bằng công cụ có thể ghi. Không chạy client, không kết nối
server, không đụng `.themida`.

## Kết quả tra cứu (VA → RVA)

Bảng này là các hàm **đã đọc**, không phải danh sách đầy đủ caller/lớp: chưa phân loại hết caller ngoài
phạm vi và không loại trừ đường gián tiếp/wrapper/VM.

| Vai trò | KR VA | KR RVA | VN VA | Cơ sở khớp |
|---|---|---|---|---|
| `CGameContext::Init` | `0x00E0EEC0` | `0x00A0EEC0` | `0x006ED78A` | chuỗi trace nhúng `CGameContext::Init` |
| `CGameContext::MakeBombers` | `0x00E12A30` | `0x00A12A30` | `0x006EB9A9` | chuỗi trace nhúng `CGameContext::MakeBombers` |
| Input dispatch của context (vf +0x20) | `0x00E13D00` | `0x00A13D00` | `0x006E5E05` | cùng slot vtable + hành vi (lấy bomber theo slot → controller vf3(dir), case 4 đặt bom) — **đường bomber đã trace** |
| `CMovable::MoveSimple` | `0x01310200` | `0x00F10200` | `0x0095199B` | chuỗi trace nhúng `CMovable::MoveSimple` |
| Trượt vuông góc trục X (trợ góc) | `0x01310D30` | `0x00F10D30` | — | slot 1 vtable `CMovable`; cùng họ với `MoveSimple` |
| Trượt vuông góc trục Y (trợ góc) | `0x01310EE0` | `0x00F10EE0` | — | slot 2 vtable `CMovable` |
| Kiểm "đã thẳng hàng ô" trên trục di chuyển | `0x01310C50` | `0x00F10C50` | — | được mover `0x00B93890` gọi trước khi trượt |
| Mover `0x00B93890` (vòng lặp di chuyển; `CGOPet`/AI theo R-134/R-135 pending) | `0x00B93890` | `0x00793890` | — | gọi `CMovable::MoveSimple` + hai hàm trượt + bảng hướng |
| Cổng vào mover `0x00B93890` | `0x00B93830` | `0x00793830` | — | gọi cổng `0x00B95970` rồi `0x00B93890` |
| Cập nhật điều khiển `CGOPet` (lấy dir + độ dài) | `0x00B95180` | `0x00795180` | — | gọi `0x00B93830` với `(dir, len)` từ sub-object `+0xE0`; **không phải bomber** (R-134/R-135 pending) |
| Truy vấn ô bị chặn | `0x00EA0FD0` | `0x00AA0FD0` | — | được `MoveSimple`/trượt gọi `(row, col, ctx)` |
| Lấy object ô lưới + kiểm lớp tile | `0x00E9F9B0` | `0x00A9F9B0` | — | RTTI `CGOTileFixed`/`CGOTilePassable` |
| Bảng vector hướng (4 hướng × `dx,dy`) | `0x017929B0` | `0x013929B0` | — | được `MoveSimple`/mover `0x00B93890` chỉ số hoá `2*dir` |
| vtable `CMovable` | `0x0179B198` | `0x0139B198` | — | RTTI `??_7CMovable@@6B@` |
| vtable `CGameContext` | `0x017695B8` | `0x013695B8` | `0x00C5466C` | RTTI `??_7CGameContext@@6B@` + `CGameContext::Init` ở slot 4 (VN slot 3) |
| vtable `CHiddenCatchGameContext` | `0x01704574` | `0x01304574` | `0x00C1768C` | RTTI `??_7CHiddenCatchGameContext@@6B@` |

## Cách tái tạo

1. Mở bản sao DB (không mở DB gốc): `kr-full.exe.i64`.
2. Liệt kê chuỗi khớp `^C[A-Za-z0-9_]+::[A-Za-z0-9_~]+$`, lấy hàm chứa xref của từng chuỗi → tên hàm.
   KR có 314 hàm tự khai tên theo cách này, VN có 186; phần giao cho cặp địa chỉ VN↔KR.
3. Đọc vtable theo RTTI đã có sẵn trong DB (`??_7...@@6B@`) để lấy slot; so slot cùng hành vi giữa
   hai build (VN `CGameContext` có 11 slot, `Init` ở slot 3; KR có 12 slot, `Init` ở slot 4).
4. Từ `CMovable::MoveSimple`, đọc bảng vector hướng `0x17929B0` (4 cặp `dx,dy`) và hằng số `40`/`39`
   trong thân hàm; đọc tiếp hai hàm trượt `0x1310D30`/`0x1310EE0` để lấy ngưỡng `20/14/26`.

## Giới hạn

- Chỉ tĩnh: chưa có trace runtime KR cho bất kỳ claim nào ở đây; không suy ra hành vi mạng.
- Ngưỡng trợ góc đọc từ hằng số trong code, chưa đo trong trận; "tối đa 34 px" là suy ra từ hình học
  `r ∈ [6,34)`, không phải số đo.
- Chưa xác định hàm quy đổi **cấp tốc độ → px mỗi bước**; `CMovable::MoveSimple` chỉ nhận độ dài còn lại
  do caller cấp (xem `KR_RE_MOVEMENT` §5). Chuỗi `(dir,len)` đã đọc là của `CGOPet`, không phải bomber.
- Polarity của `CGOTileFixed`/`CGOTilePassable` trong `0xE9F9B0` chưa chốt (candidate).
- Bomb pass/đứng trên bom: xem bổ sung R-154 (predicate `B37DB0`, `+328`, `+152`, consumer `MoveSimple`).
- Phạm vi lớp: mới xác định **đường đã trace** thuộc `CGOPet`/AI; **chưa chứng minh** cho bomber và
  không loại trừ mọi caller/lớp/wrapper/VM mà bomber có thể dùng.

## Bổ sung 2026-10-23 — R-154 bomb pass từ object bom

REA MCP `provider_id: ida` trên bản sao; bundle `.re-work/evidence-r154-bombpass-146.json`
(15 record, không commit). Không đọc lại packet/setter R-136…R-138.

| Kiểm | VA | Kết quả |
|---|---|---|
| `CGOBomb` vtable | `0x174426C` | slot 24 update `0xB391E0`; slot 40 Init `0xB384E0`; **slot +156 `0xB37DB0`** |
| Pass predicate | `0xB37DB0` (RVA `0x737DB0`) | `0` nếu `[bomb+328]` (`0x148`) ∈ {1,2}; ngược lại trả flag `+152` (`11982F0(bomb+144)`) — flag `0` cũng cho predicate `0` |
| Ctor sensor | `B37150` → `1198240(bomb+144,1)` | `CSensorTrigger` flag `+152 = 1` mặc định |
| Init | `B384E0` | `[bomb+328] = u8 descriptor+108` (`529C20`) |
| Move consumer | `0x1310200` `CMovable::MoveSimple` | consumer đã trace thuộc **pet/AI**; gọi virtual +156 nhưng còn OR `EA0FD0` và gate +164; predicate khác 0 không đủ kết luận đi qua |
| Owner getter | `B38370` (`+268`) | **14 callsite/xref**; đã đọc 4 thân lượt trước; các site còn lại chưa phân loại; **không** được `B37DB0` gọi |

**Kết luận:** `kr_static` confirmed **thân predicate** `B37DB0`, ctor/flag và **nhánh `MoveSimple` có
phạm vi pet/AI**; `candidate` owner getter (xref chưa phân loại); `unknown` đường bomber, nghĩa/giá trị
`+328`/descriptor+108, setter runtime, owner role, player grace/ra/vào/timer, VN counterpart. Không suy
hệ quả player/bomber (ra/vào/grace) hay bomb-pass toàn hệ thống; không đổi profile/40×40/assist.

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_5d06d1cab12064f869b98fb3190c705d121c999678088da9a1bb3d47d26c28d7` | `batch_decompile` `B37DB0`/`B393D0` | predicate |
| `ev_a546b755dea30f480cb1532c4bb0d4c158c23e62c6bfdd451056bb6bfc2bac0c` | `batch_decompile` `B38250`/`B38290`/`11982F0` | `+328`/`+152` |
| `ev_3bb870f56d8520b9f50ac48277eb33172caa25940c5d0de42a78ddb71588cb45` | `batch_decompile` `CSensorTrigger` | ctor flag |
| `ev_6b5946b42a71659cbda31f34a6d4e2f65ca5af068973517f54a8cd8833fdc504` | `procedure_pseudo_code` `1310200` | MoveSimple |
| `ev_dc3d84632f04b7616773024c1058afe0ae3659040f8ff455e8c57a3a9883f73f` | `batch_decompile` `B37150`/`B37560`/`B384E0` | ctor/Init |
| `ev_1ed26a6f3c1bb7dff196630d81395bb64cda343c123acd908e9b8129e231dc4e` | `xrefs` `B38370` | owner getter callers |

### R-154 — phân loại 4 caller `B38370` (`+268`)

Batch REA `ev_f4a10721fe2b84a4ba9af47155213f3c6fa23ab83eadd0b5ccf5c000fad66787` (4 thân ưu tiên
trong 14 xref; không đọc lại hàm R-154). Bundle `.re-work/evidence-r154-owner-callers-146.json`.

| Caller | Call | Cách dùng `+268` | Kiểm collision/grace trực tiếp? |
|---|---|---|---|
| `ACC340` | `0xACC4BA` | descriptor event (`AD4A40`/`AD62F0`/`AD9F20`) | chưa thấy |
| `AEF700` | `0xAEF904` | bỏ kết quả; nhánh bomb/item `B378F0`/`B37880` | chưa thấy |
| `EB73D0` | `0xEB75C1` | ghi effect `*(+4)`; `B70C10` | chưa thấy |
| `EB8000` | `0xEB8FA8`/`0xEB910E` | owner cho flame/effect `B70C10` | chưa thấy |

Cả 4 chỉ thấy dataflow descriptor/event, effect/flame hoặc bỏ kết quả; **chưa thấy kiểm collision/grace
trực tiếp** (không gọi trực tiếp `B37DB0`/`EA0FD0`/`+152`/`+328`/`MoveSimple`). Helper/callee và **các callsite
còn lại chưa loại trừ**; không tạo R-155, không nâng owner thành confirmed; player/grace vẫn unknown.

### #146 — caller `B38370` mới (B3AE90/B9A990/E3), 2026-10-24

Bundle lọc `.re-work/evidence-owner-player-sites-146-r1.json` (**7 record** đúng lượt; ledger gốc 9 record có thêm 2 batch flame `ev_3f712b95…`/`ev_5c9de206…`; không commit). Codex đã xác nhận
hai bản sao cùng SHA `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b`, kích thước
26.074.624 B, base `0x400000`; dùng read-only với **provenance thật** dưới đây, không phải clone1.

| Evidence | Callsite → function | Actual subject / DB | Kết quả |
|---|---|---|---|
| `ev_0216a581…` | `B3AECE` → `B3AE90` (RVA `73AE90`) | `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`; DB không có trong batch record | `B38370` → `B3ADD0`; chọn effect RPG/Sprinkler/Remote/Deco; không so player trực tiếp |
| `ev_0216a581…` | `B9A99C`/`B9A9C8` → `B9A990` (RVA `79A990`) | cùng batch | guard `B38370(this) != -1`; khi khác -1 mới `a=1241780(B38370(this))`, `b=1241780(1240B00(0))`, ghi `this[172]` (byte `+0x2B0`); nhánh -1 không ghi/reset; helper/consumer **candidate/unknown** |
| `ev_1e18205f…` | `B996B0`/`B9A3B0` (RVA `7996B0`/`79A3B0`) | `/home/admin/boom-rs-dsh/re-kr2/.re-tmp/kr-full.exe`; DB copy `rea-ida-1e1Km8` | wrapper kiểm `this[172]` (byte `+0x2B0`); `B9A3B0` tính vị trí rồi gọi B9A990 |
| `ev_fb3ed4dd…` | xref `B9A990` | `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`; DB `rea-ida-GI3Raz` | 2 caller: B996B0/B9A3B0 |
| `ev_d9c085e8…` | xref `B9A3B0` | `/home/admin/boom-rs-dsh/re-kr2/.re-tmp/kr-full.exe`; DB `rea-ida-1e1Km8` | 3 callsite trong B99CF0 |
| `ev_8134a6cc…` | helper `1240B00`/`1241780` | `/home/admin/boom-rs-dsh/re-kr2/.re-tmp/kr-full.exe`; DB copy `rea-ida-1e1Km8` | `1240B00` trả kết quả gọi `57A870` tại `+68+12*a2`; `1241780` guard `this[41]`, gọi accessor `529C60` trên `*(this+42)+792*a2+112`, else `-1` |
| `ev_868df24e…`, `ev_c23c3eb1…` | E3C4A2/E3C563 | boom-team; DB `rea-ida-GI3Raz` | `not_in_procedure`; chỉ xref/resolve, không đọc body/disasm mới, không kết luận class/VM |

**Kết luận:** `B3AE90` chỉ là effect selector; `B9A990` **chỉ khi `B38370(this) != -1`** mới so hai
transform `1241780(B38370(this))` và `1241780(1240B00(0))`, ghi `this[172]` (byte `+0x2B0`); nhánh -1
không ghi/reset. `1240B00` chỉ trả kết quả gọi `57A870` tại `+68+12*a2`; `1241780` guard `this[41]` và
gọi accessor `529C60` trên bảng `*(this+42)+792*a2+112`; nghĩa owner/local/team và consumer `this[172]`
vẫn **candidate/unknown**, chưa thấy collision/grace. E3 chưa xác định function. Không mở các callsite
còn lại; helper/callee chưa đọc unknown. Không đổi R-154/player grace.

### #146 — player grace sau đặt bom (lượt 24/10)

Bundle lọc `.re-work/evidence-player-grace-146-r1.json` (**7 record**; không commit). Subject/actual:
`/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`; xref record ghi DB copy `rea-ida-GUJhP1`, batch
record không có metadata DB riêng; base `0x400000`, SHA
`cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b`; không nói đây là clone khác.

| Evidence | Hàm/site | Kết quả |
|---|---|---|
| `ev_0246c7eb…` | `xrefs B40D60` | **24 site**, không phải 24 caller function; chỉ 4 thân được đọc |
| `ev_71c34736…` | `B40030` (`this`), `B40290` (`this-353`) | cùng điều kiện `B323D0/vf+100` lọc mã, `B40D60==0`, `vf+132==0`; nhãn valid/idle/type **candidate** |
| `ev_10a00c3a…` | `B422F0`, `B42880` | `B422F0` query `E9F6D0(v6,v7)` với v6/v7 từ controller+28/+32; **chưa chứng minh ô đích**, `B9DB70` body chưa đọc; `B42880` action/state |
| `ev_64d026e3…` | `xrefs B37DB0` | 6 data xref (vtable); không direct player caller |
| `ev_1f14b5c5…` | `xrefs 11982F0` | chỉ `B37DB0` code + vtable data; sensor flag reader |
| `ev_6249ce45…`, `ev_84379c67…` | `B99CF0`, `B994C0` | `0x1747328` là **entry slot 43** (base `0x174727C+0xAC`), không phải vtable base; chain `B994C0/B99CF0` chỉ xref, chưa full callchain/body |

**Kết luận:** trong **4 thân đã đọc** không thấy owner/grace trực tiếp; đây là negative có giới hạn,
callee/virtual chưa đọc không bị loại trừ. `B40D60/+1440` là field dir/input; `B37DB0/+152/+328` là
predicate kiểu/sensor bomb, không owner; `B9A990` có owner-like transform nhưng chưa nối collision/
grace. Player grace giữ **unknown**; không đề xuất đổi luật.
