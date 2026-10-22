# KR 00967 — đường `(dir, len)` khi bị nhốt: truy vết và phát hiện phạm vi lớp

Kết quả cho yêu cầu bổ sung **R-125** (cấp tốc độ → px/bước) và **R-127** (bị nhốt có đi được),
Refs [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) /
[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / #176.
Nguồn: `kr_static` (đọc tĩnh, IDA trực tiếp trên DB bản sao — xem §6 về REA). Không có binary,
pack, DB hay decompile nguyên văn trong repo.

## 0. Kết luận ngắn

Chuỗi bằng chứng đang dùng cho **R-127** —
`sub_B95180 → sub_B95970 → CGOBomber::MoveGate 0xB93830 → Move 0xB93890 → CMovable::MoveSimple 0x1310200` —
**không phải của bomber người chơi**. Nó chỉ vào được từ `CGOPet::vf24` = `sub_B93A60`.

- `sub_B93A60` có **đúng một** tham chiếu trong toàn binary: ô vtable `0x1746EA0` = slot 24 của
  `??_7CGOPet@@6B@` (`0x1746E40`).
- Slot 24 của bomber là hàm **khác**: `CGOGameBomber` `0x1745FE8` slot 24 = `0xB77B10`;
  `CGOBomber` `0x1744964` slot 24 = `0xB44000`; `CGOPet` `0x1746E40` slot 24 = `0xB93A60`.
- Không vtable nào trong `.rdata` chứa `0xB95180`, `0xB93830`, `0xB93890`, `0xB95970`
  (quét toàn vùng `0x1600000–0x1A80000` cho cả 4 địa chỉ: 0 kết quả) — tức chúng là hàm gọi trực tiếp,
  không phải slot, và chỉ có một đường gọi duy nhất như trên.
- `sub_B95970` (cổng "được đi") có **đúng một** caller: `0xB93830` (MoveGate) — nhất quán với chuỗi trên.
- Thân của `CGOGameBomber` slot 24 (`0xB77B10`) và `CGOBomber` slot 24 (`0xB44000`) **không** tham chiếu
  `0xB95180`/`0xB93890`/`0xB95970`/`0x1310200`/`0x61D600`.

⇒ Với bomber người chơi, **chưa có** bằng chứng nào cho việc `len` được tạo dương khi bị nhốt, và cũng
chưa có bằng chứng cho cả đường di chuyển của bomber trong KR. R-125 và R-127 **không thể đóng** bằng
chuỗi hiện có; phải hạ phạm vi (xem §4).

## 1. Vì sao trước đây tưởng là bomber

`sub_B95180` nằm trong cụm `0xB93xxx–0xB96xxx` cạnh các hàm mang tên gợi ý bomber
(`kCGOBomber_IsLockedInState`, `kCGOGameBomber_DeferBombOper_AutoKick` trong bảng tên), và nhận
`this` kiểu `char*` với các offset `+0xE0`, `+0xD4`, `+100` khớp vùng dữ liệu của một "movable".
Nhưng tên trong bảng đăng ký (`k...`) chỉ là **bảng id**, không trỏ hàm; không có bằng chứng nào nối
`sub_B95180` với lớp bomber. Kiểm vtable là kiểm quyết định và nó cho kết quả ngược lại.

## 2. Controller sub-object tại `bomber+0xE0` (và vì sao không dùng được)

Trong `sub_B95180(char* this)`:

```
nếu sub_B96D50(this+0xE0):
    v7 = sub_B96310(this+0xE0)          // = *(this+0xE0)  (con trỏ)
    mode = v7->vf[+4](v7, this)          // byte +4
    nếu mode == 1:
        ok = v7->vf[+8](v7, &dir, &len)  // BYTE +8 (slot 2), out-params
        nếu ok: CGOBomber(MoveGate) 0xB93830(this, dir, len)
```

- Getter `sub_B96310` → `sub_B971C0` = `return *(this)` (đã đọc: `0xB971A0/B0/C0` đều là getter một dòng).
- Đây là **controller của CGOPet**, không phải của bomber: `this` là đối tượng CGOPet (slot 24 của nó).
- Quét store vào `+0xE0`/`+0xD4` (mọi thanh ghi nguồn, mod=10, có/không SIB): 66 + 62 điểm; các hàm
  trong dải `0xB3xxxx–0xB9xxxx` là `0xB8C3F0` (máy trạng thái hiệu ứng), `0xB9D540`/`0xB9D750`
  (`CGOTileGround` ctor), `0xB6B3F0` (`CGOChatBalloon` ctor) — **không** có bomber.
  Ctor `CGOGameBomber` `0xB79DC0` cũng không gán hai offset này.
- Vì vậy không đọc được vtable của controller ⇒ **không** lấy được thân `vf[+8]` ⇒ không có công thức
  `(dir,len)`.

## 3. Phía bomber: cái gì đã chứng minh được

- Input dispatch `CGameContext` `0x00E13D00` (đã trace ở #146) lấy bomber theo slot rồi gọi
  `vf3(dir)` của sub-object nhúng tại **byte +1412** (`bomber[353]`, vtable pointer nằm ngay tại đó —
  ctor `0xB73410`/`0xB79DC0` gán vtable cho các offset này). Đây là **đường duy nhất đã thấy** đưa
  hướng vào bomber; nó chỉ chứng minh *hướng được nhận*, không chứng minh *quãng đi*.
- Slot 24 của bomber là `0xB77B10` (805 B) và `0xB44000` (5.482 B); `0xB77B10` có 15 caller trực tiếp,
  `0xB44000` là thân lớn. Cả hai **không** gọi chuỗi di chuyển ở §0 và **không** gọi
  `CMovable::MoveSimple`.
- `CMovable::MoveSimple 0x1310200` chỉ có các caller sau (toàn bộ): `0x61D600` (dispatcher `CMovable::Move`
  dùng cho `CGOCatchMon`/`CGOMinibot`/`CGORepairMinibotWatcher`), `0x64F2F0`, `0x684E60`, `0xB93890`
  (CGOPet), `0xF2C420`, `0xFC6160`, `0xFA…`. **Không** có hàm nào trong lớp bomber.
- Vậy: KR **không** cho thấy bomber người chơi đi qua `CMovable::MoveSimple`; vị trí/độ dài của bomber
  phải đến từ một đường khác (chưa định vị).

## 4. Hệ quả cho claim đang mở

| Claim | Trước | Sau lượt này |
|---|---|---|
| R-119…R-122 (hộp 40×40, trợ góc) | `kr_static` confirmed | Vẫn đúng **cho `CMovable`**: nhưng consumer đã thấy là AI/pet (`CGOPet`, `CGOCatchMon`), **chưa** chứng minh cho bomber người chơi |
| R-125 (cấp tốc độ → px/bước) | `unknown` | Vẫn `unknown`; đường `vf[+8]` là **của CGOPet**, không dùng được cho bomber |
| R-127 (cổng nhốt) | confirmed phần cổng | **Hạ**: cổng `sub_B95970` là của CGOPet. Với bomber chỉ còn `kr_static` confirmed ở mức "input dispatch đưa dir vào controller" (R-123); "bị nhốt đi được" là `candidate` |

Khuyến nghị cho **PR #177** (đang giữ R-132 confirmed "quyền đi", R-133 design "tốc độ"):
R-132 nên hạ về `candidate` hoặc nêu rõ phạm vi lớp, vì consumer `sub_B95970` là CGOPet, không phải
bomber; nếu giữ `confirmed` thì phải bổ sung consumer thuộc lớp bomber.

## 5. Còn thiếu và bước tiếp (có căn cứ)

1. **Tìm mover của bomber**: đọc `CGOBomber` slot 24 `0xB44000` (5.482 B) và `CGOGameBomber` slot 24
   `0xB77B10`; tìm nơi cập nhật vị trí (`sub_B335C0/B335E0` là getter x,y; setter ở vtable `+116` của
   đối tượng vị trí như trong `MoveSimple`). Đây là chỗ phải có `len` của bomber.
2. **Đối chiếu VN** (đòn bẩy): VN `CMovable::MoveSimple 0x95199B` — kiểm caller của nó trong VN; nếu VN
   cũng chỉ dùng cho AI/pet thì giả thuyết "bomber đi bằng đường khác" được củng cố ở cả hai build, và
   hướng đúng là tìm lớp `CGOBomber` bên VN (notebook `oc_e-game.md` §5a-3 nói input đi vào controller
   `bomber+1524`, tương ứng `+1412` của KR — cùng một sub-object).
3. **Controller của bomber**: xác định vtable của sub-object nhúng tại `+1412` (ctor gán cùng một symbol
   `CGOGameBomber::vftable` cho nhiều offset nên IDA không phân biệt); đọc `vf3` thật (SetDirection) để
   biết nó lưu dir vào đâu, rồi tìm nơi đọc dir đó.
4. Nếu bước 1–3 rơi vào `.themida` thì dừng và ghi entry VM theo [DEVIRT_METHOD](DEVIRT_METHOD.md).

## 6. Provenance công cụ (ghi rõ)

- **REA 6.7.0** (`open_binary` bản sao, `provider_id: ida`, seed DB): `xrefs 0xB93A60` →
  `ev_e4565b5af7657ad73d2cf0806e5d76f36974c8c15bde173738d71c6ef036d80d` (1 xref duy nhất `0x1746EA0`);
  `batch_decompile [0xB93A60, 0xB95180]` →
  `ev_9e2c7e089309a3142baa4668a7ea2a22eb0b5d0791fb70cf0ade95a51a5b4b82`. Bundle export
  `/home/admin/boom-team/tmp/re-kr/evidence-trapped-speed-147.json` (2 record) trước `close_binary`
  (`{"result":null}`).
- **IDA MCP trực tiếp** trên DB **bản sao**: các phép quét byte/dword, đọc slot vtable 22–26 của 3 lớp,
  đọc `??_R2CGOGameBomber@@8`, decompile `0xB77B10`/`0xB44000` — `read_bytes`/`search_bytes` là operation
  REA không khai báo, nên đây là fallback được phép, làm tuần tự sau khi đóng REA.
- **Sự cố đã xử lý**: lượt đầu REA trả `unreadable_output` cho mọi operation cấp target vì DB bản sao đã
  bị phiên IDA trực tiếp trước đó ghi lại (SHA-256 `e612361a…` → `20904663…`), khiến launcher seed
  fail-closed (`Seed database changed; refresh its pin explicitly before reuse`) và REA báo lỗi schema
  thay vì lỗi upstream. Đã refresh pin trong `~/.config/rea/helpers/ida-db-seeds.json` (có backup
  `…bak-20261022-213458`) rồi chạy lại — OK. Quy ước: IDA trực tiếp phải mở **bản sao của bản sao**.
