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

1. **Tìm mover của bomber**: đọc hết `CGOBomber` slot 24 `0xB44000` (5.482 B) và `CGOGameBomber`
   slot 24 `0xB77B10`; tìm nơi cập nhật vị trí (`sub_B335C0/B335E0` là getter x,y; setter ở vtable
   `+116` của đối tượng vị trí như trong `MoveSimple`). Đây là đầu mối cần truy cho `len`/mover của
   bomber; chưa chứng minh hai cập nhật trực tiếp tính `len` (có thể delegate/tiêu thụ trạng thái khác).
2. **Đối chiếu VN** (đòn bẩy): VN `CMovable::MoveSimple 0x95199B` — kiểm caller của nó trong VN;
   nếu VN cũng chỉ dùng cho AI/pet thì giả thuyết "bomber không dùng cùng chuỗi này" được củng cố ở cả hai
   build, nhưng vẫn **không** thay thế consumer KR. Hướng cần kiểm là tìm lớp `CGOBomber` bên VN
   (notebook `oc_e-game.md` §5a-3 nói input đi vào controller `bomber+1524`, tương ứng `+1412` của
   KR — cùng một sub-object).
3. **Controller của bomber**: xác định vtable của sub-object nhúng tại `+1412` (ctor gán cùng một
   symbol `CGOGameBomber::vftable` cho nhiều offset nên IDA không phân biệt); đọc `vf3` thật
   (SetDirection) để biết nó lưu dir vào đâu, rồi tìm nơi đọc dir đó.
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
