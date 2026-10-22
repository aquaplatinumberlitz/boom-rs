# Đo tĩnh KR 00967 — bị nhốt (trapped), 2026-10-21

Metadata cho claim `kr_static` R-126…R-131
([RULES_V1 §13](../../../specification/gameplay/RULES_V1.md#13-re-kr-00967--bị-nhốt-r-126r-131)).
Suy luận và giới hạn ở [KR_RE_TRAPPED](../../../research/findings/KR_RE_TRAPPED.md). Không có binary,
pack, DB hay decompile nguyên văn.

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `/home/admin/boomkr-re/out/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Đọc qua REA, `provider_id: "ida"`, input là **bản sao** `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`;
engine dùng database seed sẵn (`auto_analysis_ready`, `hexrays_ready`, `imagebase 0x400000`).
Không mở DB gốc, không chạy client.

## Hàm đã đọc (VA → RVA)

| Vai trò | VA | RVA |
|---|---|---|
| State machine trong cụm `0xB95xxx` (set state 3, 13→14→2); consumer đã trace: `CGOPet` | `0x00B952E0` | `0x007952E0` |
| Cổng "được di chuyển" (loại `{0,1,4,13,14}`) | `0x00B95970` | `0x00795970` |
| Chọn họ state đi `dir+9` / `dir+19` | `0x00B95840` | `0x00795840` |
| Đổi nhóm khoá + đặt bộ đếm giữ `20` | `0x00B95BD0` | `0x00795BD0` |
| Đọc bộ đếm giữ (`this[235]`, `this[236]`) | `0x00B95DA0` / `0x00B95DD0` | `0x00795DA0` / `0x00795DD0` |
| Nhận diện nhóm state `15..18` / `19..22` | `0x00B95E00` | `0x00795E00` |
| Cập nhật điều khiển gọi từ `CGOPet` slot 24 (bỏ qua state `13`,`14`,`1`) | `0x00B95180` | `0x00795180` |
| Cổng vào di chuyển (gọi `0xB95970` rồi `0xB93890`) | `0x00B93830` | `0x00793830` |
| Mover gọi `CMovable::MoveSimple` (đường `CGOPet`) | `0x00B93890` | `0x00793890` |
| Trạng thái di chuyển theo dir (kiểm `state-9`/`state-19`) | `0x00B956C0` | `0x007956C0` |
| Cờ "còn sống" (từ controller) | `0x00B95AC0` / `0x00B95AE0` | `0x00795AC0` / `0x00795AE0` |
| Ctor `CGOGameBomber` — gán vtable cho sub-object **nhúng** tại byte `+1412`, không gán `+0xE0`/`+0xD4` | `0x00B73410` / `0x00B79DC0` | `0x00773410` / `0x00779DC0` |

## Tên đăng ký (chỉ là bảng tên, không trỏ hàm)

| Chuỗi | VA | Ghi chú |
|---|---|---|
| `kCGameStage_DeferBomberLockedIn` | `0x017953C8` | xref duy nhất: bảng đăng ký tên → id |
| `kCGameStage_DeferBomberReleaseOrDie` | `0x01795074` | xref duy nhất: bảng đăng ký tên → id |
| `kCGOBomber_IsLockedInState` | `0x01795164` | như trên |

## Bằng chứng REA trong phiên (evidence_id)

| operation | evidence_id | Nội dung |
|---|---|---|
| `procedure_pseudo_code` | `ev_3fd539c12aabf55da14db16c0c6bff8703e8716e9e433f84a8a617c809b585e1` | `0xB93890` (probe phục hồi tool, thuộc lượt #146) |
| `batch_decompile` | `ev_f45ec31bfd107ad4c53e277f66177b5dcc0426a80b8024b270bdc35472f49cdf` | `0xB95E00`, `0xB95DA0`, `0xB95DD0`, `0xB95BD0`, `0xB95970` |
| `batch_decompile` | `ev_87c731414a6dbc4d4ab5db9f007b644b64ebd1388006f84e0f17f4c2fa4698bf` | `0xB93A20`, `0xB962A0`, `0xB962D0`, `0xB96310`, `0xB96D20`, `0xB96D50`, `0xB96EA0` |
| `batch_decompile` | `ev_a3817f56f2b1be6c5683cf97a731fcaa5c7f2562cb4dbe38f60aebfaddf66dfd` | `0xB93B60`, `0xB952E0` |
| `xrefs` | `ev_5b1ff16be036534b6f9a714cb5a5b055d443584576ce1471d513b36378f5dbf8` | 8 caller của `0xB95E00` |
| `xrefs` | `ev_7890c8d689cff1af34aab47f383f4e295dc8c0b66e8d3e92a6bdd79501cc9a1b` | caller của `0xB95AE0` (chỉ nội bộ) |
| `procedure_pseudo_code` | `ev_bbff42efafc1021bb968bfd28c098285f8f13c57cdb3ee18e7a1ff1a2c2f69c2` | `0xB95840` |
| `batch_decompile` | `ev_c068f66016a309c66620dcab4c609132ed4b27f67769b321866df9cf61bd722d` | `0xB73410`, `0xB79DC0` (ctor `CGOGameBomber`) |
| `batch_decompile` | `ev_010048b7028732ff0993ebfa5d8592d2a207562adaed779aef2d0f4134f5a927` | `0xB971A0/B0/C0` — getter `return *(this)` |

`evidence_id` chỉ có giá trị trong phiên REA khi target còn mở; bản bundle được export ra
`/home/admin/boom-team/tmp/re-kr/evidence-trapped-147.json` (ngoài repo) trước khi đóng target.

## Giới hạn

- Adapter IDA của REA từ chối `read_bytes`/`search_bytes` (`capability_unavailable`), nên phiên #157
  **không** đọc trực tiếp vtable/byte; kết luận chỉ từ decompile + xrefs (lượt #185 dùng fallback IDA
  trực tiếp trên **DB bản sao**, tuần tự sau khi đóng REA).
- Chưa định vị thân virtual call tại **byte +8** (slot 2 x86) của controller pet ⇒ không suy ra
  `len > 0`; chưa xác định mover/`len` của bomber.
- Chưa có trace runtime; "20 bước ≈ 600 ms" là quy đổi theo R-013 (30 ms/bước, đo trên VN), chưa đo KR.
- Số state `3` = bị nhốt ghép từ code KR + tài liệu packet VN, không phải nhãn có sẵn trong binary.

## Bổ sung 2026-10-22 — đường `(dir,len)` và phạm vi lớp

- Chuỗi đúng: `sub_B95180` `0x00B95180` → `0xB93830` `0x00B93830`; `0xB93830` gọi cổng `sub_B95970`
  `0x00B95970` **trước** rồi mover `0xB93890` (không phải `0xB95180 → 0xB95970 → 0xB93830`).
- `(dir,len)` là out-param của virtual call tại **byte +8** (slot 2 trên x86), **không** phải `vf8`/`+0x20`.
  Nó là controller của **`CGOPet`** tại `*(pet + 0xE0)` (getter `0x00B96310` → `0x00B971C0` =
  `return *(this)`); thân hàm tại byte +8 **chưa định vị**.
- **Không** gán `+0xE0`/`+0xD4` cho bomber: ctor `CGOGameBomber` `0x00B73410`/`0x00B79DC0` chỉ gán vtable
  cho sub-object **nhúng** tại byte `+1412` (`bomber[353]` nếu là `_DWORD*`; vtable pointer nằm ngay
  tại byte đó). Input dispatch bomber dùng sub-object nhúng này — **không** phải dereference
  `bomber[353]` thành pointer.
- Fallback IDA trực tiếp trên DB **bản sao** (REA thiếu `read_bytes`/`search_bytes`): quét store
  `+0xE0`/`+0xD4` → 66/62 điểm; trong dải `0xB3xxxx–0xB9xxxx` chỉ thấy `0xB8C3F0` (máy trạng thái hiệu
  ứng), `0xB9D540`/`0xB9D750` (`CGOTileGround` ctor), `0xB6B3F0` (`CGOChatBalloon` ctor) — không có
  bomber. Vì vậy lớp controller và mover/`len` của bomber vẫn **unknown**.

## Phạm vi lớp — tham chiếu pending PR #185

Bằng chứng lớp dùng ở trên lấy từ PR #185 (`re-kr-trapped-speed`, head `254d447` tại lúc đọc):
`ev_e4565b5af7657ad73d2cf0806e5d76f36974c8c15bde173738d71c6ef036d80d` (`xrefs 0xB93A60` → 1 data ref
`0x1746EA0` = slot 24 `CGOPet`) và
`ev_9e2c7e089309a3142baa4668a7ea2a22eb0b5d0791fb70cf0ade95a51a5b4b82` (`batch_decompile 0xB93A60`,
`0xB95180`). Hai `evidence_id` này thuộc bundle `evidence-trapped-speed-147.json` (ngoài repo), **chưa
merge vào `main`**: PR #157 chỉ tham chiếu pending, không coi là claim đã có trên main.

- Kết luận chỉ là: đường **đã trace** đi từ `CGOPet`; **không** loại trừ đường gián tiếp/wrapper/VM từ
  bomber. R-127 với bomber người chơi giữ `candidate` (không còn `confirmed` phần cổng).
