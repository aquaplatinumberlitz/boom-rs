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
- Lượt **R-137** đọc setter: các cạnh `0xB479E0`→`0xB43750`→`0xB334C0` ghi `+0x4C`/`+0x58`,
  `0xB479E0` ghi `[+0x930]/[+0x934]`; wrapper NPC captain/square gọi trực tiếp `0xB43750`. **Lớp sử
  dụng của `0xB479E0` = `unknown`** (là slot 95 của `CGOGameBomber`, chưa có caller ảo); không gán
  NPC/AI cho `B479E0`. Quét 5 site `call [vtable+0x17C]` không site nào trỏ `B479E0`; gói S→C vị trí
  tuyệt đối VN 176 / KR 183 dùng slot `+116` (= `0xB43750`), không qua slot 95. Giả thuyết
  **candidate**: position sync từ server cho người chơi; input dir→position của player vẫn
  **unknown** (§8).
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

## 8. Lượt setter 2026-10-22 — cạnh đã đọc, lớp sử dụng `B479E0`, packet vị trí (R-137)

### 8.1 Công cụ / phạm vi

- **REA MCP** (`provider_id: ida`) trên bản sao `kr-full.exe`, SHA-256 `cf72da11…e63b`; không mở DB
  gốc, không sửa config/pin. Bundle `.re-work/evidence-r137-slot95-146.json` (23 record, không commit).
- Quét lời gọi gián tiếp slot `+0x17C` bằng disassembly toàn bộ section code của bản sao (objdump
  Intel, đối chiếu raw byte `FF 90..97 7C 01 00 00`), rồi xác nhận từng hàm bằng REA
  `resolve_containing_procedure`/`batch_decompile`; đọc bảng jump của `sub_E5D3A0` trực tiếp từ byte.
- VN: docs `boom-re/docs/oc_e-game.md`, server giả lập `boom-re/server`, IDB bản sao
  `.re-work/vn/unpacked_ca.exe.i64` (không mở gốc) cho `sub_706C42`/`sub_5B4AE9`.

### 8.2 Các cạnh đã đọc (confirmed)

| Cạnh | VA / offset | Quan sát |
|---|---|---|
| Setter | `0xB43750` (`0x743750`) → `0xB334C0` (`0x7334C0`) | `0xB43750` là vtable slot **+116**; gọi `B334C0(a2,a3)` tại `0xB43764`; `B334C0` ghi protected x/y tại `+0x4C`/`+0x58` |
| `B479E0` | `0xB479E0` (`0x7479E0`) | Slot **95** (`+0x17C`) của `CGOGameBomber::vftable` `0x1745FE8` (cùng slot `CGOBomber` `0x1744964`); gọi `B43750(this,a3,a4)` tại `0xB47A7B`; ghi `[this+0x930]=a3`/`[+0x934]=a4` tại `0xB47AD8`/`0xB47AE4` |
| Direct caller | `0xF087E0` (`0xB087E0`), call `0xF087FD` | **Direct caller duy nhất** của `B479E0`; là slot 95 của `CGOHedgehogBomber::vftable` `0x176E2C8`, truyền `(a2,a3,a4)`. Đây **không** chứng minh lớp sử dụng của `B479E0` |
| Wrapper setter | `0x69BD60`/`0x69FE80`/`0xF06770`/`0x11260C0` | Gọi trực tiếp `B43750(this,a2,a3)`; vtable thuộc `CGONpcCaptain2Bomber`/`CGONpcCaptainBomber`/`CGOCaptainBomber`/`CGOLobbySquareBomber`/`CGOSquareBomber`/`CGOSquareNPCGuideBomber` (slot 29) — **chỉ các wrapper này là NPC/AI** |
| Clamp / gọi lại setter | `0xB44000` (`0x744000`), call `0xB44942`, clamp `0xB44975`/`0xB44A08` | Đọc `[+0x930]/[+0x934]`, gọi **gián tiếp** slot `+116` (= `0xB43750`) với chúng, rồi clamp |
| Update slot 24 | `0xB77B10` (`0x777B10`), call `0xB77C98` | Slot 24 vtable `0x1745FE8`; gọi gián tiếp slot `+116` với `(v10[0]-20, v10[1]+15)` từ `0xB7D460(this+3230)` |

**Lớp sử dụng của `B479E0` = `unknown`.** Nó là virtual của `CGOGameBomber` nhưng chưa truy được
caller ảo nào; direct caller `F087E0` thuộc `CGOHedgehogBomber` không đủ để gán lớp. **Giả thuyết
candidate** (không phải claim): `B479E0(a2,x,y)` đặt vị trí rồi lưu `[+0x930]/[+0x934]` để `B44000`
clamp — khớp mẫu **position sync từ server cho người chơi**; engine V1 là server-authoritative nên
đáng truy.

### 8.3 Quét lời gọi gián tiếp slot 95 (`call [vtable+0x17C]`)

Phạm vi: toàn bộ code có trong bản sao (section `0x401000–0xB65C00` và `.priv` `0xB66000–0x1A98000`);
5 site tĩnh duy nhất (không có `call DWORD PTR [reg+0x17C]` raw; 5 site là `mov reg,[vtable+0x17C];
call reg`):

| Site | Hàm chứa | Vtable/đối tượng | Nguồn `(x,y)` | Slot 95 thực tế |
|---|---|---|---|---|
| `0xF2BDC5` | `sub_F2B760` (slot 10 của `CGONpcBasicMon`) | object chính tại `this-0xB8`, vtable `0x17712D4` | không truyền arg | `0xF2D7C0` — **không** `B479E0` |
| `0xF34EB7` | `sub_F34DF0` | vtable `0x17719E4` | không truyền arg | `0xF34ED0` — **không** |
| `0xF3EAD1` | `sub_F3E5D0` | vtable `0x177346C` | `(v13,v14)` = toạ độ tile từ `sub_B929C0`/`sub_B92A60` trong switch hướng | `0xF3E3C0` — **không** |
| `0x12C2596` | `sub_12C2020` (UI/quest dialog) | object từ `sub_125E150` | không truyền arg (không có x/y bomber) | chưa xác định — không phải đường vị trí |
| `0x12C2B5C` | `sub_12C2620` (UI/quest dialog) | object từ `sub_125E150` | không truyền arg (không có x/y bomber) | chưa xác định — không phải đường vị trí |

Kết luận: **3 site bomber-family có slot 95 xác định ≠ `B479E0`**; 2 site UI không có x/y, không phải
đường vị trí bomber. Không thấy site nào gọi `B479E0`; nó chỉ có một direct caller `0xF087E0`.
Không loại trừ call qua thanh ghi tính toán/VM (chưa chạy runtime).

### 8.4 Packet vị trí S→C: VN opcode 176 ↔ KR opcode 183

- **VN** `CGameStage__OnPacket` case 176 → `sub_706C42` (0x706C42): `u8 id`, `u8 flags`
  (`flags & 0x7F` = slot; bit 7 = absolute); nếu absolute đọc thêm `u8 y`, `u8 x` rồi gọi
  `sub_5B4AE9(id, slot, 40*x, 40*y+39)`; `sub_5B4AE9` đặt vị trí qua `(*this+104)` (slot 26).
  Không có nhánh nào gọi slot 95. Layout/units đọc từ decompile; **chưa kiểm runtime**.
- **KR** `sub_E5D3A0` (dispatch kiểu `CGameStage__OnPacket`, case = opcode−20, bảng byte
  `0xE5DE9C` + jump `0xE5DDC4`): opcode **183** → `0xE5D65B` → `sub_E65350` (0xE65350). Handler đọc
  đúng layout trên; nhánh absolute gọi `0xE654C2`: `sub_B88800(ghost, id, slot, 40*x, 40*y+39)`.
  `sub_B88800` gọi virtual slot `+116` (= `0xB43750` ở họ vtable bomber) tại `0xB8898A`, **không**
  gọi slot 95/`B479E0`.
- Hệ quả: có **packet → setter vị trí** cho ghost/bomber (đơn vị 40 px; `y = 40*row+39`), nhưng
  **không có** cạnh packet → slot 95 → `B479E0`. Vì vậy giả thuyết "position sync cho người chơi"
  chỉ đúng ở mức packet→setter; vai trò của `B479E0` vẫn **unknown**.
- Ghi chú lệch build: cùng layout nhưng VN dùng slot 26 (`+104`), KR dùng slot 29 (`+116`); số
  opcode cũng lệch (VN 176, KR 183). Không suy slot/thứ tự opcode giữa hai build.

### 8.5 Kết luận / blocker

- **Confirmed**: các cạnh setter/writer ở §8.2; wrapper NPC captain/square; layout packet tuyệt đối
  VN 176/KR 183 và việc handler dùng slot `+116` (không slot 95), đơn vị 40 px.
- **Unknown**: lớp sử dụng của `B479E0` (chỉ có direct caller `F087E0`); input dir→position của
  player; chưa kiểm runtime packet.
- **Candidate** (không claim): `B479E0` là position sync từ server cho người chơi.
- **Blocker có phạm vi**: đã quét toàn bộ 5 site `call [vtable+0x17C]`, decompile toàn thân
  `0xB57340`/`0xB77EA0`/`0xB44000`/`0xB77B10`, xref trực tiếp `0xB43750`/`0xB479E0`, và map packet
  VN→KR. Chỗ đứt: không có caller ảo của slot 95 trỏ `B479E0`; packet dùng setter khác. Không thấy
  entry `.themida` trong các hàm đã đọc. Không suy px/s/nhịp server/trapped.

### 8.6 Evidence

Bundle `.re-work/evidence-r137-slot95-146.json`. Các `evidence_id` chính:

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_ea00a69a6347a948e0f5d779fe57ea2c482daecf62ff3c3d349e6b66b94244c2` | `batch_decompile` `0xB43750`/`0xB334C0`/`0xB479E0`/`0xF087E0` | cạnh setter/caller |
| `ev_977db7f09462abc2d9652652d449c722b33ddfb3b478a2724bb8620e1e2ff8ca` | `xrefs` `0xB43750` | wrapper trực tiếp + data slot `+116` |
| `ev_dd433bf57dc9d4cb8d8cd63f2a26551ebd33c9eed246af79b05c294fbac41892` | `xrefs` `0xB479E0` | 1 direct caller `0xF087E0` |
| `ev_21be21c334ef3589abc9f9aa41528eff54195c06a441bfc8b924b5eb50a93af7` | `procedure_assembly` `0xB334C0` | `+0x4C`/`+0x58` |
| `ev_ef51aa30c4dc2e156cbe3c0820767769f88599ccaf494a86987587881e7fd219` | `batch_decompile` 5 hàm chứa site slot 95 | phân lớp 5 site |
| `ev_13f01a9c3d8f9725fa195d4c8ef3dede8d1bb3b2b9f13be06ff15bcb9dda0a80` | `xrefs` `0xF3D830` | slot 79 + vtable `0x177346C`; slot 95 = `0xF3E3C0` |
| `ev_fdf896d5fe568c326f1a48ad5cd2abf3cd11eba021492dfc8f175336b7c39901` | `xrefs` vtable `0x1771494` | `CGONpcBasicMon` ctor; primary vtable `0x17712D4` |
| `ev_b68075d5be124cdfc26fa26b48a9c19e43d80ba6b5958919eef03e38b356e48d` | `batch_decompile` `0xB88800`/`0xBA3100` | ghost reset; slot `+116` |
| `ev_d94e551ba16506fe15a1436713bef02613e14435bd4b4a064cb29b74b6b8063e` | `batch_decompile` `0xE65350`/`0xE84F00` | handler opcode 183 |
| `ev_f91194098e3c6b1557cfb770a8aea4a26cf5f190e1769492ae4dc7c8e5a57eae` | `xrefs` `0xE65350` | caller `0xE5D662` |

VN dùng IDB bản sao, không phải REA; địa chỉ VN ghi theo IDB `unpacked_ca.exe.i64`.
