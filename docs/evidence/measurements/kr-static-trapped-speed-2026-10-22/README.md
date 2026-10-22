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
SHA-256 binary khớp). Một phiên idalib, không chạy song song, không chạy client. Lượt đầu không có
`evidence_id`; re-check REA MCP 2026-10-22 đã cấp evidence — xem mục "Re-check 2026-10-22" cuối trang.

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

## Bổ sung 2026-10-22 — R-137: cạnh đã đọc, lớp sử dụng `B479E0`, packet vị trí

### Cạnh đã đọc (confirmed)

| Kiểm | VA / offset | Kết quả |
|---|---|---|
| Setter | `0xB43750` (`0x743750`) → `0xB334C0` (`0x7334C0`) | `0xB43750` là vtable slot **+116**; gọi `B334C0(a2,a3)` tại `0xB43764`; `B334C0` ghi protected `+0x4C`/`+0x58` |
| `B479E0` | `0xB479E0` (`0x7479E0`) | Slot **95** (`+0x17C`) của `CGOGameBomber::vftable` `0x1745FE8` (cùng slot `CGOBomber` `0x1744964`); gọi `B43750(this,a3,a4)` tại `0xB47A7B`; ghi `[+0x930]=a3`/`[+0x934]=a4` tại `0xB47AD8`/`0xB47AE4` |
| Direct caller | `0xF087E0` (`0xB087E0`), call `0xF087FD` | **Direct caller duy nhất** của `B479E0`; slot 95 của `CGOHedgehogBomber::vftable` `0x176E2C8`. Không đủ để gán lớp sử dụng của `B479E0` |
| Wrapper setter | `0x69BD60`/`0x69FE80`/`0xF06770`/`0x11260C0` | Gọi trực tiếp `B43750(this,a2,a3)`; vtable `CGONpcCaptain2Bomber`/`CGONpcCaptainBomber`/`CGOCaptainBomber`/`CGOLobbySquareBomber`/`CGOSquareBomber`/`CGOSquareNPCGuideBomber` — **chỉ các wrapper này là NPC/AI** |
| Clamp / gọi lại setter | `0xB44000` (`0x744000`), call `0xB44942`, clamp `0xB44975`/`0xB44A08` | Đọc `[+0x930]/[+0x934]`, gọi **gián tiếp** slot `+116` (= `0xB43750`) với chúng, rồi clamp |
| Update slot 24 | `0xB77B10` (`0x777B10`), call `0xB77C98` | Slot 24 vtable `0x1745FE8`; gọi gián tiếp slot `+116` với toạ độ từ `0xB7D460(this+3230)` |

**Lớp sử dụng của `B479E0` = `unknown`.** Direct caller `F087E0` thuộc `CGOHedgehogBomber` không
chứng minh lớp người chơi dùng `B479E0`; chưa truy được caller ảo nào. **Giả thuyết candidate** (không
claim): `B479E0(a2,x,y)` đặt vị trí rồi lưu `[+0x930]/[+0x934]` để `B44000` clamp — khớp mẫu
**position sync từ server cho người chơi**.

### Quét lời gọi slot 95 (`call [vtable+0x17C]`)

Phạm vi: toàn bộ code bản sao (section `0x401000–0xB65C00` và `.priv` `0xB66000–0x1A98000`); 5 site
tĩnh, không site nào trỏ `B479E0`:

| Site | Hàm chứa | Đối tượng / vtable | Nguồn `(x,y)` | Slot 95 |
|---|---|---|---|---|
| `0xF2BDC5` | `sub_F2B760` | `CGONpcBasicMon` primary `0x17712D4` (object `this-0xB8`) | không arg | `0xF2D7C0` |
| `0xF34EB7` | `sub_F34DF0` | vtable `0x17719E4` | không arg | `0xF34ED0` |
| `0xF3EAD1` | `sub_F3E5D0` | vtable `0x177346C` | `(v13,v14)` tile từ `sub_B929C0`/`sub_B92A60` | `0xF3E3C0` |
| `0x12C2596` | `sub_12C2020` | object UI từ `sub_125E150` | không arg | chưa xác định |
| `0x12C2B5C` | `sub_12C2620` | object UI từ `sub_125E150` | không arg | chưa xác định |

### Packet vị trí S→C: VN opcode 176 ↔ KR opcode 183

- **VN** `sub_706C42`: `u8 id`, `u8 flags` (`flags & 0x7F` = slot; bit 7 = absolute); absolute đọc
  thêm `u8 y`, `u8 x`, rồi `sub_5B4AE9(id, slot, 40*x, 40*y+39)`; `sub_5B4AE9` đặt vị trí qua slot
  **26** (`+104`), **không** slot 95. Cấu trúc đọc từ decompile, chưa kiểm runtime.
- **KR** dispatch `sub_E5D3A0` (case = opcode−20; bảng byte `0xE5DE9C` + jump `0xE5DDC4`): opcode
  **183** → `0xE5D65B` → `sub_E65350`. Handler đọc cùng layout; nhánh absolute gọi
  `sub_B88800(ghost, id, slot, 40*x, 40*y+39)` tại `0xE654C2`; `sub_B88800` gọi slot **+116**
  (= `0xB43750`) tại `0xB8898A`, **không** slot 95/`B479E0`.
- Đơn vị: `x = 40*col`, `y = 40*row + 39` (px). VN dùng slot 26, KR dùng slot 29; opcode lệch
  (176 vs 183) — không suy slot/opcode giữa hai build.

### Trạng thái

- **Confirmed**: cạnh setter/writer, wrapper NPC captain/square, layout packet và việc handler dùng
  slot `+116` (không slot 95).
- **Unknown**: lớp sử dụng của `B479E0`; input dir→position của player; chưa kiểm runtime packet.
- **Candidate**: `B479E0` là position sync từ server cho người chơi.
- **Blocker có phạm vi**: 5 site slot 95 đã quét, decompile toàn thân `0xB57340`/`0xB77EA0`/
  `0xB44000`/`0xB77B10`, xref `0xB43750`/`0xB479E0`; không có caller ảo slot 95 trỏ `B479E0`,
  packet dùng setter khác. Không thấy entry `.themida` trong các hàm đã đọc.

## Re-check REA MCP — `evidence_id` và phạm vi

Target `kr-full.exe`, SHA-256 `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b`,
`provider_id: ida`, image base `0x400000`; không mở DB gốc, không sửa config/pin. Bundle:
`.re-work/evidence-r137-slot95-146.json` (23 record, không commit). Bundle R-136 cũ:
`.re-work/evidence-r136-r137-146.json` (12 record).

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_cb75c455ba65ad6e665b64110ef15330092274bbefff5ae5d9e1945646ae057f` | `batch_decompile` `0xE13D00`/`0xB32390`/`0xB40D60` | R-136 dispatch→reader |
| `ev_e7dc2bc2ed637c527cd155f7e069e79bf1aa23c63c9f7306e213dcfa5ac70404` | `xrefs` `0xB32390` | slot +12 hai vtable bomber |
| `ev_ea00a69a6347a948e0f5d779fe57ea2c482daecf62ff3c3d349e6b66b94244c2` | `batch_decompile` `0xB43750`/`0xB334C0`/`0xB479E0`/`0xF087E0` | setter/caller |
| `ev_977db7f09462abc2d9652652d449c722b33ddfb3b478a2724bb8620e1e2ff8ca` | `xrefs` `0xB43750` | wrapper + slot `+116` |
| `ev_dd433bf57dc9d4cb8d8cd63f2a26551ebd33c9eed246af79b05c294fbac41892` | `xrefs` `0xB479E0` | direct caller `0xF087E0` |
| `ev_21be21c334ef3589abc9f9aa41528eff54195c06a441bfc8b924b5eb50a93af7` | `procedure_assembly` `0xB334C0` | `+0x4C`/`+0x58` |
| `ev_ef51aa30c4dc2e156cbe3c0820767769f88599ccaf494a86987587881e7fd219` | `batch_decompile` 5 hàm chứa site slot 95 | phân lớp 5 site |
| `ev_13f01a9c3d8f9725fa195d4c8ef3dede8d1bb3b2b9f13be06ff15bcb9dda0a80` | `xrefs` `0xF3D830` | vtable `0x177346C`; slot 95 `0xF3E3C0` |
| `ev_fdf896d5fe568c326f1a48ad5cd2abf3cd11eba021492dfc8f175336b7c39901` | `xrefs` vtable `0x1771494` | `CGONpcBasicMon`; primary `0x17712D4` |
| `ev_b68075d5be124cdfc26fa26b48a9c19e43d80ba6b5958919eef03e38b356e48d` | `batch_decompile` `0xB88800`/`0xBA3100` | ghost reset; slot `+116` |
| `ev_d94e551ba16506fe15a1436713bef02613e14435bd4b4a064cb29b74b6b8063e` | `batch_decompile` `0xE65350`/`0xE84F00` | handler opcode 183 |
| `ev_f91194098e3c6b1557cfb770a8aea4a26cf5f190e1769492ae4dc7c8e5a57eae` | `xrefs` `0xE65350` | caller `0xE5D662` |

## Giới hạn

- Không có công thức `(dir,len)` cho bomber: `vf[+8]` (byte +8 = slot 2 x86) là của controller `+0xE0`
  **của `CGOPet`**, không phải bomber; controller bomber nhúng tại byte `+1412` mới chỉ chứng minh
  nhận `dir`; R-136 chỉ nối tới reader, chưa tới setter vị trí.
- Không xác định được mover/`len` của bomber trong lượt này; slot 24 `0xB44000`/`0xB77B10` đã đọc
  nhưng không đọc field `+0x5A0` và không gọi `CMovable::MoveSimple`.
- R-137: direct caller của setter là wrapper **NPC/AI**; `0xB479E0` là slot 95 của `CGOGameBomber`
  nhưng **lớp sử dụng `unknown`** (chưa có caller ảo). Quét 5 site slot 95 không site nào trỏ
  `B479E0`; packet VN 176/KR 183 dùng slot `+116`, không slot 95. Candidate: position sync server cho
  player. Không kết luận player không có đường khác (indirect/VM chưa loại trừ); input dir→position
  vẫn `unknown`.
- Danh sách caller `CMovable::MoveSimple` chưa phân lớp đủ (còn `0x64F2F0`, `0x684E60`, `0xF2C420`,
  `0xFC6160`, một caller `0xFA…` thiếu VA); không loại trừ đường gián tiếp/wrapper/VM từ bomber.
- Đã đối chiếu VN packet vị trí 176; chưa kiểm runtime và chưa thấy consumer `CMovable::MoveSimple`
  cho bomber.
