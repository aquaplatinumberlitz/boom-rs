# KR 00967 — movement, hộp va chạm và trợ góc (RE tĩnh)

Kết quả đọc tĩnh cho [#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) / Q-017,
nguồn `kr_static`. Claim nằm ở [RULES_V1 §12](../../specification/gameplay/RULES_V1.md#12-re-kr-00967--movement-và-va-chạm-r-119r-125);
định danh build và bảng VA/RVA ở
[evidence](../../evidence/measurements/kr-static-2026-10-21/README.md).
Không có binary, pack, IDA DB hay decompile nguyên văn trong repo.

Khác biệt với kết luận trước: [#146] trả lời phần **hộp va chạm, neo toạ độ, trợ góc** bằng consumer KR;
phần **quy đổi cấp tốc độ → px mỗi bước** vẫn chưa định vị được consumer (§5) nên giữ `unknown`.

## 0. Phạm vi

Đọc trong module đã unpack; không chạy client, không kết nối server, không đụng `.themida`.
Đối tượng: `CMovable` (va chạm/trượt ô), `CGOBomber` (điều khiển bomber người chơi),
`CGameContext` (dispatch input). Không bao gồm bomb pass, đứng trên bom, thuyền/vật thể đặc biệt.

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

**Ghi chú công cụ.** Lần đọc này bắt đầu bằng REA: `open_binary` bind đúng binary bản sao
`/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe` với `provider_id: "ida"`, SHA-256 khớp
`cf72da11…e63b`. Nhưng mọi thao tác phân tích sau đó trả `execution_failure` / "Request timed out"
sau khi idalib worker được spawn (`0x…:42835`): module 26 MB / 170.329 hàm vượt ngân sách phân tích
của adapter headless, `retryable:false`. Ghidra/Hopper không cấu hình. Vì vậy phần đọc hàm dùng
IDA MCP trực tiếp trên **database bản sao** đã phân tích sẵn (`kr-full.exe.i64`); không mở DB gốc.

## 2. Đường đi của một bước di chuyển (KR)

```
input (phím)  ->  CGameContext input dispatch 0xE13D00
                    -> bomber = getBomber(slot)            (0xE12FC0)
                    -> controller = bomber[353]  (byte +1412)
                    -> controller->vf3(dir)                (dir 0..3, dir 4 = đặt bom)

mỗi tick      ->  CGOBomber cập nhật điều khiển 0xB95180
                    -> controller->vf8(&dir, &len)         (cặp hướng + độ dài còn lại)
                    -> CGOBomber::MoveGate 0xB93830(dir, len)
                    -> CGOBomber::Move     0xB93890(dir, &len)   vòng lặp:
                         CMovable::MoveSimple 0x1310200(dir, &len)
                         nếu len != 0:
                           đòi thẳng hàng ô 0x1310C50(dir)   (x%40==0 hoặc (y-39)%40==0)
                           đòi ô đích tồn tại 0xEA5D90
                           trượt vuông góc: dir dọc -> 0x1310D30(dir, hàng đích, &len)
                                            dir ngang -> 0x1310EE0(dir, cột đích, &len)
```

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
`px/s = px_mỗi_bước × 33,33` nếu bước = 30 ms. **Cấp tốc độ không phải px/s** — bảng `BomberAttr`
chỉ cho cấp (R-040…R-042).

## 4. Trợ góc (corner assist) — dẫn xuất đầy đủ

Hàm `0x1310D30` chạy khi bomber đi **dọc** và vừa bị chặn; nó trượt theo trục X. `0x1310EE0` là bản
đối xứng cho đi ngang, trượt theo trục Y. Cả hai chỉ được gọi sau khi `0x1310C50` xác nhận **đã thẳng
hàng ô trên trục di chuyển** (nếu chưa thẳng hàng thì dừng, không trượt).

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
trống rồi vòng lặp ngoài thử đi tiếp. Đây là "snap" trong một bước, không phải trượt dần.

## 5. Tốc độ: chưa có consumer

`CMovable::MoveSimple` **không** biết tốc độ: nó nhận `len` (px còn lại) từ caller. Chuỗi gọi đã
truy được tới:

- `CGOBomber::Move` `0x00B93890(dir, len)` — vòng lặp, `len` nguyên vẹn từ tham số;
- `CGOBomber::MoveGate` `0x00B93830(dir, len)` — chỉ kiểm trạng thái rồi chuyển tiếp;
- `0x00B95180()` — lấy cặp `(dir, len)` từ **controller** `bomber+0xE0` qua `vf8` (offset `+0x20`),
  rồi gọi `0x00B93830`.

⇒ Hàm quy đổi **cấp tốc độ → px mỗi bước** nằm sau `vf8` của lớp controller tại `bomber+0xE0`.
Chưa định vị được lớp đó (không tìm thấy store vtable vào `+0xE0`; nhiều khả năng controller là
sub-object được khởi tạo trong ctor của bomber). Vì vậy:

- **không** claim px/s cho KR; R-114 (24 px/s/level, `oss`/design) giữ nguyên;
- R-043 (5 px/tick ở cấp 5, VN, checksum chưa xác thực) vẫn là `candidate`, **không** dùng để suy ra
  KR dù nó khớp giả thuyết "px mỗi bước = cấp tốc độ".

Bước tiếp theo cụ thể: tìm ctor của `CGOBomber` (xref tới vtable `0x01745FE8`), đọc sub-object
`+0xE0`, lấy vtable của controller, rồi đọc `vf8` (`+0x20`) — đó là nơi duy nhất còn lại của phép
quy đổi. Nếu thân hàm đó nằm trong vùng `.themida` thì dừng và ghi entry VM theo
[DEVIRT_METHOD](DEVIRT_METHOD.md).

## 6. Khác biệt với profile hiện tại và với BnB

Profile [`data/rules/v1-dev.toml`](../../../data/rules/v1-dev.toml) đang dùng R-116 cho va chạm:

| Khóa profile | Giá trị đang dùng | Nguồn hiện tại | KR đọc được | Đề xuất |
|---|---|---|---|---|
| `player.collision_half_px` | `13` (hộp 26×26) | `oss`/design R-116 | hộp bàn chân **40×40**, neo `y−39` | đổi sang hộp đầy ô 40×40; PR profile riêng |
| `player.corner_assist_px` | `12` | `oss`/design R-116 | trượt tới **34 px** nhưng chỉ khi đã thẳng hàng ô và chỉ để căn vào làn trống | đổi *cơ chế* (snap-to-lane) chứ không chỉ đổi số; PR riêng |
| `player.trap_touch_distance_px` | `28` | `oss`/design R-116 | chưa đọc (ngoài phạm vi #146) | giữ, #147 |
| `grid.tile_px` | `40` | `vn_static` R-110 | `40` dùng trực tiếp trong va chạm | không đổi; thêm consumer KR |
| `player.speed_px_per_s_per_level` | `24` | `oss`/design R-114 | chưa xác định | giữ, xem §5 |

BnB `server/game.js:31–32,150–153,173–275` mô hình hoá hộp 26×26 + assist 12 px. KR cho thấy mô hình
đó **không** phải mô hình gốc: hộp đúng một ô và assist là phép căn làn, không phải "nới" hộp.

## 7. Giới hạn

- Toàn bộ là đọc tĩnh. Chưa có trace runtime KR cho bất kỳ claim nào ở đây; không suy ra hành vi mạng
  hay thứ tự tick server.
- Ngưỡng trợ góc đọc từ hằng số; "tối đa 34 px" là suy ra hình học, chưa đo trong trận.
- `CMovable` là lớp dùng chung (quái, hiệu ứng, bomber). Đã xác nhận bomber người chơi đi qua đúng
  chuỗi `0xB95180 → 0xB93830 → 0xB93890 → 0x1310200`; chưa chứng minh mọi lớp con đều dùng cùng
  tham số.
- Điều kiện "ô bị chặn" (`0xEA0FD0`) gộp nhiều vị từ; polarity của `CGOTileFixed`/`CGOTilePassable`
  trong `0xE9F9B0` chưa chốt (R-124 để `candidate`).
- Không bao gồm bomb pass, đứng trên bom, cứu/được cứu, thuyền.

## 8. Câu hỏi mở

Theo dõi ở [QUESTIONS](../../research/QUESTIONS.md): Q-017 (speed/px-per-step còn thiếu), Q-004
(tick client 30 ms đã dùng để quy đổi nhưng chưa nối với KR).
