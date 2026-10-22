# Đo tĩnh KR 00967 — đường `(dir,len)` khi bị nhốt (2026-10-22)

Metadata cho lượt truy vết bổ sung R-125/R-127, Refs
[#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) /
[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / #176.
Kết luận và suy luận ở [KR_RE_TRAPPED_SPEED](../../../research/findings/KR_RE_TRAPPED_SPEED.md);
claim ở [RULES_V1 §15](../../../specification/gameplay/RULES_V1.md#15-re-kr-00967--phạm-vi-lớp-của-chuỗi-movement-r-134r-137).
Không có binary, pack, DB hay decompile nguyên văn.

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `/home/admin/boomkr-re/out/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Đọc trên **bản sao** `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe.i64`. Không mở DB gốc, không chạy client.

## Kết quả kiểm (VA → RVA)

| Kiểm | Kết quả | VA |
|---|---|---|
| `??_7CGOPet@@6B@` slot 24 | `0x00B93A60` | vtable `0x01746E40` |
| `??_7CGOGameBomber@@6B@` slot 24 | `0x00B77B10` | vtable `0x01745FE8` |
| `??_7CGOBomber@@6B@` slot 24 | `0x00B44000` | vtable `0x01744964` |
| Caller trực tiếp của `0x00B95180` | **1**: `0x00B93A82` trong `0x00B93A60` (xref trực tiếp; không bao gồm đường gián tiếp) | — |
| Caller trực tiếp của `0x00B95970` | **1**: `0x00B9384A` trong `0x00B93830` | — |
| Caller/data xref của `0x00B93A60` | **1**: data ref `0x01746EA0` (slot 24 `CGOPet`) | — |
| Dword `0x00B95180`/`0x00B93830`/`0x00B93890`/`0x00B95970` trong `.rdata` | **0** trong vùng đã quét `0x1600000–0x1A80000`: không thấy 4 hàm dưới dạng con trỏ ở vùng này; không kết luận cho vùng khác/đường gián tiếp | — |
| Store vào `+0xE0` / `+0xD4` (mọi thanh ghi, mod=10) | 66 / 62 điểm; trong dải `0xB3xxxx–0xB9xxxx` thấy `0xB8C3F0` (máy trạng thái hiệu ứng), `0xB9D540`/`0xB9D750` (`CGOTileGround` ctor), `0xB6B3F0` (`CGOChatBalloon` ctor); ctor `0x00B79DC0` không thấy gán hai offset này trong thân đã đọc | — |
| `lea [reg+0x584]` (địa chỉ sub-object bomber tại byte +1412) | **0** điểm; dispatch `0x00E13D00` lấy sub-object `+1412` bằng offset khác, không qua mẫu này | — |
| Căn cứ lớp của bomber | `??_R2CGOGameBomber@@8` `0x017CB498`: `CGOGameBomber→CGOBomber→CGOBomberBase→CGOAvatar→CGameObject→CObject` | — |
| Caller của `CMovable::MoveSimple` `0x01310200` **đã ghi nhận** | `0x61D600` (dispatcher `CMovable::Move`, AI/pet theo lượt #146), `0x64F2F0`, `0x684E60`, `0xB93890` (mover trong đường `CGOPet`), `0xF2C420`, `0xFC6160`, và một caller dải `0xFA…` **chưa đủ VA** để ghi. Danh sách **không** được khẳng định là toàn bộ; các mục chưa đọc thân là **chưa phân lớp** | — |
| Đường đã trace | `0x00B95180 → 0x00B93830`; trong `0x00B93830` gọi cổng `0x00B95970` rồi mover `0x00B93890 → 0x01310200` | — |

## Provenance công cụ

- **REA 6.7.0** (`open_binary` bản sao, `provider_id: ida`, seed DB) — bằng chứng đã cấp `evidence_id`:

  | operation | evidence_id | nội dung |
  |---|---|---|
  | `xrefs` | `ev_e4565b5af7657ad73d2cf0806e5d76f36974c8c15bde173738d71c6ef036d80d` | `0xB93A60` → **1** xref `0x1746EA0` (data, slot 24 `CGOPet`) |
  | `batch_decompile` | `ev_9e2c7e089309a3142baa4668a7ea2a22eb0b5d0791fb70cf0ade95a51a5b4b82` | `0xB93A60` (update pet: gọi `0xB95180` rồi `0xB952E0`, `0xB93AB0`) + `0xB95180` (`vf[+8]` byte +8 = slot 2 trả `(dir,len)` qua out-param) |

  Bundle export trước `close_binary`: `/home/admin/boom-team/tmp/re-kr/evidence-trapped-speed-147.json`
  (10.573 B, 2 record). `close_binary` → `{"result":null}`.
- **IDA MCP trực tiếp** trên DB **bản sao** (`execute_python` + `ida_bytes`/`ida_xref`/`ida_hexrays`):
  quét store `+0xE0`/`+0xD4`, quét `0x8D`/disp `0x584`, quét dword vtable trong `.rdata`, đọc slot 22–26
  của 3 lớp, đọc `??_R2CGOGameBomber@@8`, decompile `0xB77B10`/`0xB44000`. Fallback hợp lệ vì REA thiếu
  `read_bytes`/`search_bytes`; làm tuần tự sau khi đóng REA, không chạy hai worker song song. Các phép
  IDA trực tiếp này **không** có `evidence_id`.

### Sự cố REA 6.7.0 `unreadable_output` (đã xử lý)

Lượt đầu, mọi operation cấp target trả
`unreadable_output` — `IDA MCP returned malformed or unsupported producer data: success: Invalid input: expected true; session: Invalid input: expected object, received undefined`.
Nguyên nhân **không phải** lỗi phiên bản REA: phiên IDA MCP trực tiếp trước đó đã **ghi lại** DB bản sao,
nên seed pin của DB bản sao fail-closed (`Seed database changed; refresh its pin explicitly before reuse`);
helper trả `{"error": …}` (không có `success`/`session`), và REA 6.7.0 báo lỗi schema thay vì lỗi upstream.
Sau khi làm mới pin của DB bản sao rồi chạy lại, REA hoạt động lại và cấp 2 `evidence_id` ở trên. Không
ghi đường dẫn cấu hình/backup cá nhân vào repo.

## Bổ sung 2026-10-22 — dispatch dir vào sub-object `+1412` (R-136)

Lượt nối tiếp R-135: tìm reader của hướng sau dispatch `0x00E13D00` và thử nối tới setter vị trí.
**REA 6.7.0** (CLI, `provider_id: ida`, dùng `REA_IDA_MCP_CONFIG` hiện có) bind bản sao nhưng operation
cấp target trả `execution_failure` — `EROFS: read-only file system,
mkdtemp '/home/admin/.cache/rea/seeded-workspaces/rea-ida-Wm52UW'`. Không sửa config/pin; fallback
tuần tự sang **IDA MCP trực tiếp trên bản sao riêng của bản sao** (binary + DB copy trong workspace;
SHA-256 binary khớp). Một phiên idalib, không chạy song song, không chạy client. Không có `evidence_id`
REA cho lượt này.

| Kiểm | VA (RVA) | Kết quả |
|---|---|---|
| Dispatch dir | `0x00E13D00` (`0xA13D00`), call `0x00E13DFD` (`0xA13DFD`) | `v19[353]` = vtable pointer của sub-object `+1412`; gọi `[v19[353]+12]` với `this = v19+353`, arg = dir |
| Factory/ctor | `0xAECE00` (`0x6ECE00`), `0xAECEB6`; `0xB79DC0` (`0x779DC0`) | `sub_E12A30` → `sub_AECE00` → `sub_B79DC0`; ctor gán `+0x584` = `CGOGameBomber::vftable` `0x01745FE8` |
| Ctor thay thế | `0xB73410` (`0x773410`) → `0xB3BE30` (`0x73BE30`) | Gán `CGOGameBomber::vftable` rồi gọi ctor `CGOBomber`; thân sau ghi đè `+0x584` = `CGOBomber::vftable` `0x01744964`. Cả hai vtable slot `+12` = `0xB32390` |
| `vf3`/setter field | `0xB32390` (`0x732390`) | `sub_55FD30(this+0x1C, dir)`; ghi `0x80000000` vào `+0x70`/`+0x7C` |
| Field | `bomber+1440` (`0x5A0`) = `+1412 + 0x1C` | Protected field nhận dir; chưa có enum/label |
| Reader | `0xB40D60` (`0x740D60`) | `sub_57A870(this+1440)`; trả giá trị field |
| Predicate | `0xB40030`, `0xB40290`, `0xB405E0`, `0xB407E0`, `0xB40A50`, `0xB422F0`, `0xB42880`, `0xB45570` | Chỉ dùng kết quả reader làm điều kiện; không truyền vào setter vị trí |
| Vị trí | getter `0xB335C0`/`0xB335E0` (`0x7335C0`/`0x7335E0`); setter `0xB334C0`/`0xB33520`/`0xB33540`/`0xB43750` (`0x7334C0`/`0x733520`/`0x733540`/`0x743750`) | `0xB43750` → `0xB334C0(this,x,y)`; chưa thấy caller nào lấy dir/field `+1440` để tính x,y |
| Update slot 24 | `0xB77B10` (`0x777B10`) → `0xB44000` (`0x744000`) | Không đọc `+0x5A0`; ghi đè vị trí tại `0xB4498D`/`0xB449D9`/`0xB44A23`/`0xB44A6F` từ `[this+0x930]`/`[this+0x934]` (clamp); không gọi `CMovable::MoveSimple` |

**Kết luận:** đã nối dispatch→vf3→field→reader; **chưa nối** reader/predicate tới setter vị trí. Đường
cập nhật vị trí/`len` của bomber vẫn **unknown**; không có công thức len/speed/trapped trên các cạnh này.
Bước tiếp: caller của `0xB43750`/`0xB334C0`, event path `0xB57340`/`0xB77EA0`, input state `sub_8444F0`,
và nguồn clamp `[this+0x930]`/`[this+0x934]`.

## Bổ sung 2026-10-22 — caller setter và nguồn clamp (R-137)

Truy ngược từ setter vị trí `0xB43750`→`0xB334C0` và writer `[+0x930]/[+0x934]`. Cùng giới hạn
sandbox: REA fail `EROFS` ở `~/.cache/rea/seeded-workspaces`; dùng IDA MCP trực tiếp trên bản sao
riêng của bản sao, một phiên idalib tuần tự, không sửa config/pin.

| Kiểm | VA (RVA) | Kết quả |
|---|---|---|
| Setter | `0xB43750` (`0x743750`) → `0xB334C0` (`0x7334C0`) | `0xB43750` gọi `B334C0(a2,a3)` tại `0xB43764`; `B334C0` ghi protected `+0x4C`/`+0x58` qua `0x55FD30` |
| Caller `B479E0` | `0xB479E0` (`0x7479E0`), call `0xB47A7B` | Vtable slot 95 (`+0x17C`) của `CGOGameBomber::vftable` `0x1745FE8` (ptr `0x1746164`); gọi `B43750(this,a3,a4)`, rồi ghi `[this+0x930]=a3`/`[this+0x934]=a4` tại `0xB47AD8`/`0xB47AE4` |
| Caller của `B479E0` | `0xF087E0` (`0xB087E0`), call `0xF087FD` | Direct caller duy nhất trong xref; là slot 95 của `CGOHedgehogBomber::vftable` `0x176E2C8` (ptr `0x176E444`), gọi `B479E0(a2,a3,a4)` |
| Wrapper setter | `0x69BD60`/`0x69FE80`/`0xF06770`/`0x11260C0` | Gọi `B43750(this,a2,a3)`; vtable thuộc `CGONpcCaptain2Bomber`/`CGONpcCaptainBomber`/`CGOCaptainBomber`/`CGOLobbySquareBomber`/`CGOSquareBomber`/`CGOSquareNPCGuideBomber` (slot 29, `+0x74`) — NPC/AI |
| Clamp | `0xB44000` (`0x744000`), read `0xB44975`/`0xB44A08` | Chỉ đọc `[this+0x930]`/`[+0x934]` để clamp; không gọi `B479E0`/`B43750`/`B334C0` |
| Event path | `0xB57340`/`0xB77EA0` | Không thấy call tới `B479E0`/`B43750`/`B334C0`; không nối input dir |

**Kết luận:** caller native tìm được của setter là **NPC/AI** (`CGOHedgehogBomber` và các NPC captain/
square). `B479E0` nhận x,y từ caller, gọi setter rồi ghi cùng x,y vào `[+0x930]/[+0x934]` — đường
position sync/correction, không phải input dir của player. Player `CGOGameBomber` có slot 95 nhưng
không có caller native trực tiếp được thấy; nguồn `a3/a4` của `F087E0` chưa truy được (indirect slot).
Input dir → position của player vẫn **unknown**; không đổi R-125/R-127.

## Giới hạn

- Không có công thức `(dir,len)` cho bomber: `vf[+8]` (byte +8 = slot 2 x86) là của controller `+0xE0`
  **của `CGOPet`**, không phải bomber; controller bomber nhúng tại byte `+1412` mới chỉ chứng minh
  nhận `dir`; R-136 chỉ nối tới reader, chưa tới setter vị trí.
- Không xác định được mover/`len` của bomber trong lượt này; slot 24 `0xB44000`/`0xB77B10` đã đọc
  nhưng không đọc field `+0x5A0` và không gọi `CMovable::MoveSimple`.
- R-137 chỉ thấy caller của setter thuộc **NPC/AI**; không có caller native trực tiếp của player được
  thấy. Không kết luận player không có đường khác (indirect/VM chưa loại trừ); input dir→position vẫn
  `unknown`.
- Danh sách caller `CMovable::MoveSimple` chưa phân lớp đủ (còn `0x64F2F0`, `0x684E60`, `0xF2C420`,
  `0xFC6160`, một caller `0xFA…` thiếu VA); không loại trừ đường gián tiếp/wrapper/VM từ bomber.
- Chưa đối chiếu VN cho câu hỏi "VN có dùng `CMovable::MoveSimple` cho bomber không" — bước tiếp số 2.
