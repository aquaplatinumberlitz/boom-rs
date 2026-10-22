# KR 00967 — đường `(dir, len)` khi bị nhốt: truy vết và phạm vi lớp

Kết quả cho yêu cầu bổ sung **R-125** (cấp tốc độ → px/bước) và **R-127** (bị nhốt có đi được),
Refs [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) /
[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / #176.
Nguồn: `kr_static` (đọc tĩnh trên DB bản sao — xem §6). Không có binary, pack, DB hay decompile
nguyên văn trong repo.

## 0. Kết luận ngắn

- **Đường đã trace**: `sub_B95180 → MoveGate 0xB93830`. Bên trong `0xB93830`, nhánh kiểm cổng gọi
  `sub_B95970` rồi mover `0xB93890 → CMovable::MoveSimple 0x1310200`.
- Trong phép **xref trực tiếp** đã chạy, `0xB95180` chỉ có caller `0xB93A82` trong `0xB93A60`;
  `0xB93A60` là slot 24 của `??_7CGOPet@@6B@` (`0x1746E40`), và ô đó có một data xref
  `0x1746EA0`. `0xB95970` có một caller trực tiếp `0xB9384A` trong `0xB93830`.
- Slot 24 của lớp bomber là hàm khác: `CGOGameBomber` `0x1745FE8` → `0xB77B10`; `CGOBomber`
  `0x1744964` → `0xB44000`. Trong **thân hai hàm đã đọc**, không thấy tham chiếu trực tiếp tới chuỗi
  trên hay `CMovable::MoveSimple`.
- **Giới hạn của kết luận**: đây là kiểm cục bộ (xref trực tiếp, đọc hai thân slot 24, quét một
  vùng `.rdata`). Nó **không** loại trừ đường gián tiếp/wrapper/con trỏ tính toán/VM từ bomber.
  Vì vậy **không** kết luận "bomber không dùng chuỗi này"; câu hỏi bomber đi bằng đường nào vẫn
  **unknown**.
- Với bomber, phần **đã trace** chỉ là nhận hướng: `CGameContext 0xE13D00` → `vf3(dir)` của
  controller **nhúng tại byte +1412** (R-135). Hàm đọc lại dir và tạo/giữ `len`/mover của bomber
  **chưa định vị**.
- Lượt nối tiếp **R-136**: `vf3` (`0xB32390`) ghi dir vào protected field `+0x1C` của sub-object
  (`bomber+1440` = `0x5A0`); reader `0xB40D60` trả field này. **Chưa nối** từ reader/predicate tới
  setter vị trí `0xB334C0`/`0xB43750` hay `CMovable::MoveSimple` — mover/`len` vẫn **unknown** (§7).
- Lượt **R-137** truy ngược setter: caller **trực tiếp** là **NPC/AI** (`0xB479E0` từ `0xF087E0` của
  `CGOHedgehogBomber`, cùng các wrapper NPC captain/square); `B479E0` ghi `[+0x930]/[+0x934]` từ x,y
  của caller, không phải từ dispatch dir. Re-check 2026-10-22 thấy thêm call **gián tiếp** slot
  `+116` trong update bomber (`0xB77B10`/`0xB44000`) với toạ độ tính/clamp, nguồn chưa truy. Input
  dir→position của player vẫn **unknown** (§8).
- R-125 và R-127 **chưa đóng**; R-134 chỉ xác nhận **đường đã trace** thuộc `CGOPet`.

## 1. Vì sao trước đây tưởng là bomber

`sub_B95180` nằm trong cụm `0xB93xxx–0xB96xxx` cạnh các hàm có tên gợi ý bomber
(`kCGOBomber_IsLockedInState`, `kCGOGameBomber_DeferBombOper_AutoKick` trong bảng tên), và nhận
`this` kiểu `char*` với các offset `+0xE0`, `+0xD4`, `+100` khớp vùng dữ liệu của một "movable".
Nhưng tên trong bảng đăng ký (`k...`) chỉ là **bảng id**, không trỏ hàm; không có bằng chứng trực
tiếp nào nối `sub_B95180` với lớp bomber. Kiểm vtable cho thấy **đường đã trace** thuộc `CGOPet`,
không phải bomber (mục 0, 2).

## 2. Controller sub-object tại `this+0xE0` trong `B95180` — của `CGOPet`, không phải bomber

Trong `sub_B95180(char* this)`:

```
nếu sub_B96D50(this+0xE0):
    v7 = sub_B96310(this+0xE0)          // = *(this+0xE0)  (con trỏ)
    mode = v7->vf[+4](v7, this)          // BYTE +4 (slot 1 x86)
    nếu mode == 1:
        ok = v7->vf[+8](v7, &dir, &len)  // BYTE +8 = slot 2 x86, out-params
        nếu ok: 0xB93830(this, dir, len) // MoveGate → cổng 0xB95970 rồi mover 0xB93890
```

- Getter `sub_B96310` → `sub_B971C0` = `return *(this)` (đã đọc: `0xB971A0/B0/C0` đều là getter một dòng).
- `vf[+8]` là **byte offset +8** (slot 2 trên x86), không phải "vtable index 8"; đây là chỗ lấy
  `(dir, len)` dưới dạng out-param.
- Trong đường đã trace từ `0xB93A60` (slot 24 của `??_7CGOPet@@6B@`), `sub_B95180` nhận `this` là
  đối tượng `CGOPet`; do đó `this+0xE0` là sub-object của `CGOPet`. **Không** phải trường
  `bomber+0xE0`. Controller của bomber là sub-object **nhúng tại byte +1412** (R-135), khác đối tượng này.
- Quét store vào `+0xE0`/`+0xD4` (mọi thanh ghi nguồn, mod=10, có/không SIB): 66 + 62 điểm; trong dải
  `0xB3xxxx–0xB9xxxx` thấy `0xB8C3F0` (máy trạng thái hiệu ứng), `0xB9D540`/`0xB9D750`
  (`CGOTileGround` ctor), `0xB6B3F0` (`CGOChatBalloon` ctor); ctor `CGOGameBomber` `0xB79DC0`
  không thấy gán hai offset này **trong thân đã đọc**. Phép quét không chứng minh "không lớp nào
  khác gán"; nó chỉ cho thấy controller của đường đã trace không phải bomber.
- Chưa đọc được thân `vf[+8]` của controller `CGOPet`, nên **không** có công thức `(dir,len)` từ
  đường này. Không suy ra công thức cho bomber từ đây.

## 3. Phía bomber: cái gì đã chứng minh được (và giới hạn)

- Input dispatch `CGameContext` `0x00E13D00` (đã trace ở #146) lấy bomber theo slot rồi gọi
  `vf3(dir)` của sub-object nhúng tại **byte +1412** (`bomber[353]`, vtable pointer nằm ngay tại đó —
  ctor `0xB73410`/`0xB79DC0` gán vtable cho các offset này). Đây là đường **đã thấy** đưa hướng vào
  bomber; nó chỉ chứng minh *hướng được nhận*, không chứng minh *quãng đi* hay `len`.
- Slot 24 của bomber là `0xB77B10` (805 B) và `0xB44000` (5.482 B). Trong hai thân đã đọc **không**
  thấy gọi chuỗi di chuyển ở §0 và **không** thấy gọi `CMovable::MoveSimple`. Đây là đọc cục bộ,
  chưa đọc hết mọi nhánh/đường gián tiếp của hai thân.
- Các caller của `CMovable::MoveSimple 0x1310200` **đã ghi nhận trong lượt này**:

  | VA | Ghi nhận | Phân lớp |
  |---|---|---|
  | `0x61D600` | dispatcher `CMovable::Move` dùng cho `CGOCatchMon`/`CGOMinibot`/`CGORepairMinibotWatcher` (theo lượt #146) | AI/pet |
  | `0x64F2F0` | chưa đọc thân trong lượt này | **chưa phân lớp** |
  | `0x684E60` | chưa đọc thân trong lượt này | **chưa phân lớp** |
  | `0xB93890` | mover trong đường đã trace `0xB93830` | `CGOPet` |
  | `0xF2C420` | chưa đọc thân trong lượt này | **chưa phân lớp** |
  | `0xFC6160` | chưa đọc thân trong lượt này | **chưa phân lớp** |
  | `0xFA…` | chỉ ghi lại dải, **không đủ VA** trong ghi chú của lượt | **chưa phân lớp** |

  Danh sách này **không** được khẳng định là toàn bộ caller; xref có thể bỏ sót hoặc không thấy
  đường gián tiếp. Vì vậy **không** có kết luận bao phủ kiểu "mọi caller không thuộc lớp bomber".
- Trong phạm vi đã đọc, **chưa thấy** lớp bomber gọi `CMovable::MoveSimple`; **đường cập nhật vị
  trí/`len` của bomber chưa định vị**. Đây là phép tìm âm tính trong phạm vi đã đọc, **không** loại trừ
  wrapper/đường gián tiếp; "chưa chứng minh", không phải "đã chứng minh không dùng".

## 4. Hệ quả cho claim đang mở

| Claim | Trước | Sau lượt này |
|---|---|---|
| R-119…R-122 (hộp 40×40, trợ góc) | `kr_static` confirmed | Vẫn đúng **cho `CMovable`**: consumer đã thấy là AI/pet (`CGOPet`, `CGOCatchMon`); áp dụng cho bomber người chơi vẫn `candidate`/chưa chứng minh |
| R-125 (cấp tốc độ → px/bước) | `unknown` | Vẫn `unknown`. `vf[+8]` (byte +8 = slot 2) là của controller `+0xE0` **của `CGOPet`**, không dùng được cho bomber; controller bomber nhúng tại byte +1412 nhưng `len`/mover chưa định vị |
| R-127 (cổng nhốt) | confirmed phần cổng | Cổng `sub_B95970` và chuỗi đã trace thuộc `CGOPet`. Với bomber chỉ còn R-123 (dispatch dir vào controller); "bị nhốt đi được" là `candidate` |
| R-134 (đường đã trace) | — | `confirmed` **chỉ cho đường đã trace thuộc `CGOPet`**; quan hệ bomber với đường này **unknown**, không loại trừ wrapper/VM |
| R-135 (dispatch bomber) | — | `confirmed` cho dispatch `vf3(dir)` vào controller nhúng `+1412`; `len`/mover **unknown** |

Trạng thái PR liên quan: #152 đã merge (R-119…R-125 vào `main`); #157 đang mở (R-126…R-131);
#177 đã đóng, không merge nên R-132/R-133 không có trong nguồn chuẩn — không còn đề nghị hạ nhãn
R-132.

## 5. Còn thiếu và bước tiếp (có căn cứ)

1. **Tìm mover của bomber**: đã đọc `CGOBomber` slot 24 `0xB44000` và `CGOGameBomber` slot 24
   `0xB77B10`; hai thân này **không** đọc field `+0x5A0`/`+1440`. `0xB44000` chỉ ghi đè vị trí tại
   bốn điểm clamp (`0xB4498D`, `0xB449D9`, `0xB44A23`, `0xB44A6F`) từ `[this+0x930]`/`[this+0x934]`;
   chưa thấy nơi tính vị trí mới từ dir. Bước tiếp: tìm caller của setter `0xB334C0`/`0xB43750` có
   truyền giá trị suy ra từ input/field, hoặc chứng minh mover dùng input state trực tiếp.
2. **Đối chiếu VN** (đòn bẩy): VN `CMovable::MoveSimple 0x95199B` — kiểm caller của nó trong VN;
   nếu VN cũng chỉ dùng cho AI/pet thì giả thuyết "bomber không dùng cùng chuỗi này" được củng cố ở cả hai
   build, nhưng vẫn **không** thay thế consumer KR. Hướng cần kiểm là tìm lớp `CGOBomber` bên VN
   (notebook `oc_e-game.md` §5a-3 nói input đi vào controller `bomber+1524`, tương ứng `+1412` của
   KR — cùng một sub-object).
3. **Controller của bomber**: đã xác định vtable/f3 (R-136 §7): `0xB32390` ghi dir vào protected
   field `+0x1C` (= `bomber+1440`), reader `0xB40D60`. **Chưa** nối reader/predicate tới setter vị
   trí; đây là chỗ cần trace tiếp, không suy ra mover/`len`.
4. Nếu bước 1–3 rơi vào `.themida` thì dừng và ghi entry VM theo [DEVIRT_METHOD](DEVIRT_METHOD.md).

## 6. Provenance công cụ (ghi rõ)

- **REA 6.7.0** (`open_binary` bản sao, `provider_id: ida`, seed DB): `xrefs 0xB93A60` →
  `ev_e4565b5af7657ad73d2cf0806e5d76f36974c8c15bde173738d71c6ef036d80d` (1 data xref
  `0x1746EA0`); `batch_decompile [0xB93A60, 0xB95180]` →
  `ev_9e2c7e089309a3142baa4668a7ea2a22eb0b5d0791fb70cf0ade95a51a5b4b82`. Bundle export
  `/home/admin/boom-team/tmp/re-kr/evidence-trapped-speed-147.json` (2 record) trước
  `close_binary` (`{"result":null}`).
- **IDA MCP trực tiếp** trên DB **bản sao**: các phép quét byte/dword, đọc slot vtable 22–26 của 3
  lớp, đọc `??_R2CGOGameBomber@@8`, decompile `0xB77B10`/`0xB44000` — `read_bytes`/`search_bytes`
  là operation REA không khai báo, nên đây là fallback được phép, làm tuần tự sau khi đóng REA.
  Các phép này **không** có `evidence_id`.
- **Sự cố đã xử lý**: lượt đầu REA trả `unreadable_output` cho mọi operation cấp target. Nguyên
  nhân không phải lỗi phiên bản REA: phiên IDA trực tiếp trước đó đã **ghi lại** DB bản sao, làm
  seed pin fail-closed (`Seed database changed; refresh its pin explicitly before reuse`); helper
  trả `{"error": …}` (không có `success`/`session`) nên REA báo lỗi schema thay vì lỗi upstream.
  Sau khi làm mới pin của DB bản sao, chạy lại OK và có 2 `evidence_id` ở trên. Không ghi đường
  dẫn cấu hình/backup cá nhân vào repo.

## 7. Lượt mover 2026-10-22 — dispatch dir vào sub-object `+1412` (R-136)

### 7.1 Công cụ và phạm vi

- Lượt đầu **REA 6.7.0 CLI** bind được bản sao nhưng operation cấp target trả `execution_failure` —
  `EROFS` ở `~/.cache/rea/seeded-workspaces`; khi đó fallback sang IDA MCP trực tiếp trên bản sao
  riêng (binary + DB copy, SHA-256 khớp `cf72da11…e63b`). Sau đó **REA MCP trong phiên** mở lại
  đúng target `provider_id: ida` và chạy được; re-check 2026-10-22 dùng REA MCP, không sửa
  config/pin, không chạy hai worker song song, không chạy client.
- Re-check đọc lại đúng các cạnh dưới bằng `batch_decompile`/`procedure_assembly`/`xrefs`; evidence
  ID và phạm vi truy vấn ở §7.4. Không đưa raw decompile vào repo.

### 7.2 Các cạnh đã trace

| Cạnh | VA (RVA) | Quan sát |
|---|---|---|
| Dispatch dir | `0x00E13D00` (`0xA13D00`), call `0x00E13DFD` (`0xA13DFD`) | `v19 = sub_E12FC0(a5)`; `v19[353]` là vtable pointer của sub-object `+1412`; gọi `[v19[353] + 12]` với `this = v19 + 353` và arg `dir` (`0..3`/`0x11..0x14` → `v17`). Cùng case còn ghi `v17` vào input state qua `sub_8444F0(a5)` |
| Factory/ctor | `0xAECE00` (`0x6ECE00`), call `0xAECEB6`; `0xB79DC0` (`0x779DC0`) | `sub_E12A30` (MakeBombers) → `sub_AECE00` → `sub_B79DC0`; ctor gán `v33[353] = CGOGameBomber::vftable` `0x01745FE8` |
| Ctor thay thế | `0xB73410` (`0x773410`) → `0xB3BE30` (`0x73BE30`) | `0xB73410` cũng gán `CGOGameBomber::vftable` cho `+0x584`, nhưng cuối thân gọi ctor `CGOBomber` `0xB3BE30`, thân này ghi đè `v19[353]` bằng `CGOBomber::vftable` `0x01744964`. Cả hai vtable có slot `+12` = `0xB32390`; đường factory đã trace dùng `0xB79DC0` |
| `vf3` nhận dir | `0xB32390` (`0x732390`), vtable +12 | Disasm: lấy `arg_0` (dir), `sub_55FD30(this+0x1C, dir)`; sau đó ghi `0x80000000` vào `this+0x70` và `this+0x7C`. `sub_55FD30` là setter protected-field (xor/checksum), `sub_57A870` là reader tương ứng |
| Field nhận dir | `bomber+1440` (`0x5A0`) = sub-object `+1412` + `0x1C` | Cùng địa chỉ vật lý; chưa có enum/label trong binary |
| Reader | `0xB40D60` (`0x740D60`) | `return sub_57A870(this + 1440)`; trả lại giá trị field |
| Predicate dùng reader | `0xB40030` (`0x740030`), `0xB40290` (`0x740290`), `0xB405E0` (`0x7405E0`), `0xB407E0` (`0x7407E0`), `0xB40A50` (`0x740A50`), `0xB422F0` (`0x7422F0`), `0xB42880` (`0x742880`), `0xB45570` (`0x745570`) | Mọi caller đã đọc dùng kết quả làm **điều kiện** (`== 0`/`!= 0`) cho valid/idle/state; không caller nào truyền giá trị đó vào setter vị trí |
| Vị trí bomber | getter `0xB335C0` (`0x7335C0`, +`0x4C`), `0xB335E0` (`0x7335E0`, +`0x58`); setter `0xB334C0` (`0x7334C0`), `0xB33520` (`0x733520`), `0xB33540` (`0x733540`), virtual `0xB43750` (`0x743750`) | `0xB43750` gọi `0xB334C0(this,x,y)`; chưa thấy caller nào lấy `dir`/field `+1440` để tính x,y |
| Update slot 24 | `0xB77B10` (`0x777B10`) → `0xB44000` (`0x744000`) | `0xB77B10` gọi base update `0xB44000`; `0xB44000` **không** đọc `+0x5A0`/`+1440`. Nó ghi đè vị trí tại `0xB4498D`/`0xB449D9`/`0xB44A23`/`0xB44A6F` từ `[this+0x930]`/`[this+0x934]` (clamp). `0xB77B10` còn gọi gián tiếp slot `+116` (= `0xB43750`) tại `0xB77C98` với toạ độ từ `0xB7D460(this+3230)` |

### 7.3 Kết luận và blocker

- **Đã nối được**: dispatch dir → vf3 `0xB32390` → protected field `+0x1C` (`bomber+1440`) → reader
  `0xB40D60`.
- **Chưa nối được**: reader/predicate → setter vị trí `0xB334C0`/`0xB43750`; `0xB77B10`/`0xB44000`
  không đọc field này và không gọi `CMovable::MoveSimple` (`0x1310200`). Vì vậy **đường cập nhật vị
  trí/`len` của bomber vẫn chưa định vị**. Đây là phép tìm âm tính trong phạm vi hàm/xref đã đọc,
  không loại trừ wrapper/đường gián tiếp/VM.
- **Không** thấy công thức `len`/speed/trapped trên các cạnh này; không suy ra px/s, không suy ra
  "bị nhốt đi được".
- **Bước tiếp cụ thể**:
  1. Tìm caller của `0xB43750`/`0xB334C0` có truyền giá trị suy ra từ `dir` hoặc từ input state
     (`sub_8444F0(a5)`); kiểm `0xB422F0`/`0xB42880`/`0xB45570` và event path `0xB57340`/`0xB77EA0`.
  2. Xem `[this+0x930]`/`[this+0x934]` (nguồn clamp ở `0xB44000`) là vị trí đích từ đâu; nếu là
     server correction thì tách khỏi mover client.
  3. Nếu một trong hai nhánh vào `.themida`, ghi entry VM và dừng theo [DEVIRT_METHOD](DEVIRT_METHOD.md).
- **Giới hạn**: chỉ đọc tĩnh; không trace runtime. Lượt đầu không có `evidence_id` vì REA CLI fail
  trước khi cấp; output IDA trực tiếp khi đó nằm trong `.re-work/evidence-bomber-mover-146.json`.
  Re-check 2026-10-22 bằng REA MCP đã cấp evidence (§7.4); bundle hợp nhất
  `.re-work/evidence-r136-r137-146.json` (12 record, không commit). Không đưa DB/binary/decompile
  vào repo.

### 7.4 Re-check bằng REA MCP — `evidence_id` và phạm vi truy vấn

Target `kr-full.exe`, SHA-256 `cf72da11…e63b`, `provider_id: ida`, image base `0x400000`. Không mở
DB gốc, không sửa config. Bundle: `.re-work/evidence-r136-r137-146.json` (12 record, không commit).

| `evidence_id` | Operation | Phạm vi truy vấn | Dùng cho |
|---|---|---|---|
| `ev_cb75c455ba65ad6e665b64110ef15330092274bbefff5ae5d9e1945646ae057f` | `batch_decompile` | `0xE13D00`, `0xB32390`, `0xB40D60` | cạnh dispatch→vf3→reader |
| `ev_f7f08144b13343bc4d981d5518a1c6f6ca38903f71fe3a2e31a031ecb0521b4f` | `procedure_assembly` | `0xB32390` | `add ecx,1Ch`; `+0x70`/`+0x7C` |
| `ev_e7dc2bc2ed637c527cd155f7e069e79bf1aa23c63c9f7306e213dcfa5ac70404` | `xrefs` | `0xB32390` | data ref `0x1745FF4` (= `0x1745FE8`+12), `0x1744970` (= `0x1744964`+12) |
| `ev_d43835b3517b3f1a1aeb7ed125ee2b121c2e10588281814466a64b137ce792d7` | `batch_decompile` | `0xB77B10` | gọi `0xB44000`; call site setter `0xB77C98` |
| `ev_8550bffa595b6511d0ad39c5836fb481be138397c891cdad103d45f11beb973b` | `xrefs` | `0xB77B10` | data ref `0x1746048` = slot 24 vtable `0x1745FE8` |
| `ev_3067b6d912e48695f8d34a2e2ed2e49254a38d19bbf8642a6cd0754feec26185` | `batch_decompile` | `0xB44000` | clamp `+0x930`/`+0x934`; call site setter `0xB44942` |
| `ev_fca1d17acccfa1c7e41e4656c54ada1ad7038b593afdbaf30a00a1212defae38` | `xrefs` | `0xB7D460` | 5 code ref, không data ref (không phải virtual) |

## 8. Lượt setter 2026-10-22 — caller native và nguồn clamp (R-137)

### 8.1 Công cụ / phạm vi

- Lượt đầu dùng IDA MCP trực tiếp vì REA CLI fail `EROFS` (§7.1); re-check 2026-10-22 chạy lại bằng
  **REA MCP** (`provider_id: ida`, target `kr-full.exe`, SHA-256 `cf72da11…e63b`), không sửa
  config/pin, không chạy client. Evidence và phạm vi ở §8.4.
- Truy ngược từ setter `0xB43750`/`0xB334C0`, writer `[+0x930]/[+0x934]`, và đọc thêm thân slot 24
  `0xB77B10`/`0xB44000` để kiểm phép tìm âm tính; có đọc lại dispatch/reader R-136 để xác nhận.
- Bundle hợp nhất `.re-work/evidence-r136-r137-146.json` (12 record, không commit).

### 8.2 Cạnh caller → setter

| Cạnh | VA (RVA) | Quan sát |
|---|---|---|
| Setter | `0xB43750` (`0x743750`) → `0xB334C0` (`0x7334C0`) | `0xB43750` là vtable slot **+116**; gọi `sub_B334C0(a2,a3)` tại `0xB43764`; `0xB334C0` ghi protected x/y tại `+0x4C`/`+0x58` qua `0x55FD30` |
| Caller chính | `0xB479E0` (`0x7479E0`) | Vtable slot **95** (byte `+0x17C`) của `CGOGameBomber::vftable` `0x1745FE8` (ptr tại `0x1746164`); gọi `0xB43750(this, a3, a4)` tại `0xB47A7B`; sau đó ghi `[this+0x930]=a3`/`[this+0x934]=a4` tại `0xB47AD8`/`0xB47AE4` |
| Caller của `B479E0` | `0xF087E0` (`0xB087E0`), call `0xF087FD` (`0xB087FD`) | Direct caller **duy nhất** trong xref trực tiếp. `0xF087E0` là vtable slot 95 của `CGOHedgehogBomber::vftable` `0x176E2C8` (ptr tại `0x176E444`), gọi `sub_B479E0(a2,a3,a4)` — `a2` là đối tượng bomber, `a3/a4` là x,y |
| Wrapper setter | `0x69BD60` (`0x29BD60`), `0x69FE80` (`0x29FE80`), `0xF06770` (`0xB06770`), `0x11260C0` (`0xD260C0`) | Gọi `0xB43750(this,a2,a3)` rồi cập nhật sub-object `+3254`/`+3253`/`+3202`; các pointer này nằm trong vtable `CGONpcCaptain2Bomber`/`CGONpcCaptainBomber`/`CGOCaptainBomber`/`CGOLobbySquareBomber`/`CGOSquareBomber`/`CGOSquareNPCGuideBomber` (slot 29, byte +0x74) — đều là lớp NPC/AI, không phải `CGOGameBomber` |
| Nguồn clamp / gọi lại setter | `0xB44000` (`0x744000`), call `0xB44942` (`0x744942`); clamp `0xB44975`/`0xB44A08` | **Đọc** `[this+0x930]`/`[+0x934]`, rồi gọi **gián tiếp** virtual slot `+116` (= `0xB43750`) với chính hai field này tại `0xB44942`, sau đó clamp tại `0xB44975`/`0xB44A08`. Không gọi trực tiếp `B479E0`/`B43750`/`B334C0` |
| Setter từ update slot 24 | `0xB77B10` (`0x777B10`), call `0xB77C98` (`0x777C98`) | Slot 24 vtable `0x1745FE8` (data ref `0x1746048`). Gọi **gián tiếp** `*this+116` (= `0xB43750`) với `(v10[0]-20, v10[1]+15)`; `v10` lấy từ `0xB7D460(this+3230)` (không phải virtual, 5 code ref) |
| Event path | `0xB57340` (`0x757340`), `0xB77EA0` (`0x777EA0`) | Không thấy call trực tiếp hay qua slot `+116` tới setter; không nối được input dir |

### 8.3 Kết luận / blocker

- **Đã chứng minh**: caller native của setter trong phạm vi xref trực tiếp là **NPC/AI** — `B479E0`
  (một virtual của base bomber) được gọi từ `F087E0` của `CGOHedgehogBomber`, và các wrapper NPC
  captain/square gọi trực tiếp `B43750`. `B479E0` nhận x,y từ caller (`a3/a4`), gọi setter, rồi ghi
  chính x,y đó vào `[+0x930]/[+0x934]`. Đây là **correction/position sync của NPC/AI**, không phải
  input dir của `CGOGameBomber` người chơi.
- **Đính chính phạm vi âm tính (re-check 2026-10-22)**: câu "`0xB44000`/`0xB77B10` không gọi setter"
  ở bản trước chỉ đúng với **call trực tiếp**; thực tế hai hàm này gọi **gián tiếp** qua virtual slot
  `+116` (= `0xB43750`) tại `0xB44942`/`0xB77C98`. `0xB44000` gọi lại bằng `[+0x930]/[+0x934]` rồi
  clamp; `0xB77B10` (slot 24 `CGOGameBomber`) gọi bằng toạ độ `(v10[0]-20, v10[1]+15)` từ
  `0xB7D460(this+3230)`. Vì vậy "caller chỉ thuộc NPC/AI" chỉ đúng cho **direct xref**; setter còn
  được chính lớp bomber gọi trong update/correction. Nguồn toạ độ `0xB7D460` chưa truy; không có
  cạnh từ field `+0x5A0`/reader `0xB40D60` tới hai call site này, nên input dir→position của player
  vẫn **unknown**.
- **Chưa chứng minh**: nguồn toạ độ của hai call site gián tiếp trong lớp player (`0xB77C98` từ
  `0xB7D460`, `0xB44942` từ `[+0x930]/[+0x934]`) và việc `CGOGameBomber` có gọi `B479E0`/`B43750`
  qua đường khác hay không. Vtable player có slot 95 = `B479E0` nhưng **không có caller native trực
  tiếp** thuộc player được thấy; nguồn `a3/a4` của `F087E0` cũng chưa truy được vì call site là
  indirect vtable slot. Vì vậy input dir → position của player vẫn **unknown**; không thay
  R-125/R-127.
- Không thấy công thức `len`/speed/trapped; không suy px/s/nhịp server.
- **Bước tiếp**: tìm producer `a3/a4` của `F087E0` (bảng dispatch/network/AI) để biết correction hay
  AI target; nếu thấy entry VM thì dừng. Nếu vẫn không nối được, **dừng mover** và chuyển mục 3
  BomberAttr/item theo chỉ đạo review #189 — không lặp lại cùng mỏ neo.

### 8.4 Re-check bằng REA MCP — `evidence_id` và phạm vi truy vấn

Cùng target/bundle như §7.4. Các truy vấn R-137:

| `evidence_id` | Operation | Phạm vi truy vấn | Kết quả |
|---|---|---|---|
| `ev_ea00a69a6347a948e0f5d779fe57ea2c482daecf62ff3c3d349e6b66b94244c2` | `batch_decompile` | `0xB43750`, `0xB334C0`, `0xB479E0`, `0xF087E0` | setter gọi `B334C0`; `B479E0` gọi `B43750(this,a3,a4)`; `F087E0` gọi `B479E0` |
| `ev_977db7f09462abc2d9652652d449c722b33ddfb3b478a2724bb8620e1e2ff8ca` | `xrefs` | `0xB43750` | 36 ref: 4 code (`0x69BD74`, `0x69FE94`, `0xF06784`, `0x11260D4`); data gồm `0x174605C` = slot `+116` vtable `0x1745FE8` |
| `ev_dd433bf57dc9d4cb8d8cd63f2a26551ebd33c9eed246af79b05c294fbac41892` | `xrefs` | `0xB479E0` | 37 ref: 1 code (`0xF087F8`); data gồm `0x1746164` = slot 95 vtable `0x1745FE8`, `0x1744AE0` = slot 95 vtable `0x1744964` |
| `ev_21be21c334ef3589abc9f9aa41528eff54195c06a441bfc8b924b5eb50a93af7` | `procedure_assembly` | `0xB334C0` | `add ecx,4Ch` và `add ecx,58h`; `retn 8` (thiscall + 2 arg) |
| `ev_dd7c5854a9323890c9a0400d0b436df5d8f81733d55ba20fdba8f5249dc9a806` | `batch_decompile` | `0xB57340`, `0xB77EA0` | không thấy call trực tiếp hay slot `+116` tới setter |
| `ev_d43835b3517b3f1a1aeb7ed125ee2b121c2e10588281814466a64b137ce792d7` | `batch_decompile` | `0xB77B10` | gọi `0xB44000` tại `0xB77E27`; call slot `+116` tại `0xB77C98` với toạ độ từ `0xB7D460(this+3230)` |
| `ev_8550bffa595b6511d0ad39c5836fb481be138397c891cdad103d45f11beb973b` | `xrefs` | `0xB77B10` | 26 ref: 22 code; data `0x1746048` = slot 24 vtable `0x1745FE8` (xác nhận lớp `CGOGameBomber`) |
| `ev_3067b6d912e48695f8d34a2e2ed2e49254a38d19bbf8642a6cd0754feec26185` | `batch_decompile` | `0xB44000` | call slot `+116` tại `0xB44942` bằng `[+0x930]/[+0x934]`; clamp `0xB44975`/`0xB44A08` |
| `ev_fca1d17acccfa1c7e41e4656c54ada1ad7038b593afdbaf30a00a1212defae38` | `xrefs` | `0xB7D460` | 5 code ref (`0xB74767`, `0xB77C3D`, `0xB77CA5`, `0xB78FC7`, `0xB7B4C2`), 0 data ref |

Phạm vi âm tính đã chạy: xref trực tiếp tới `0xB43750`/`0xB479E0`, decompile toàn thân
`0xB57340`/`0xB77EA0`/`0xB44000`/`0xB77B10`, và kiểm call site virtual slot `+116`. Kết quả **không**
loại trừ đường gián tiếp khác/VM; chỉ xác nhận hai call site gián tiếp nêu trên và việc không có cạnh
từ field `+0x5A0`/reader `0xB40D60` tới chúng.
