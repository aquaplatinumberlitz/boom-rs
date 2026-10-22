# Đo tĩnh KR 00967 — đường `(dir,len)` khi bị nhốt (2026-10-22)

Metadata cho lượt truy vết bổ sung R-125/R-127, Refs
[#146](https://github.com/aquaplatinumberlitz/boom-rs/issues/146) /
[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / #176.
Kết luận và suy luận ở [KR_RE_TRAPPED_SPEED](../../../research/findings/KR_RE_TRAPPED_SPEED.md);
claim ở [RULES_V1 §14](../../../specification/gameplay/RULES_V1.md#14-re-kr-00967--phạm-vi-lớp-của-chuỗi-movement-r-134-r-135).
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

## Giới hạn

- Không có công thức `(dir,len)` cho bomber: `vf[+8]` (byte +8 = slot 2 x86) là của controller `+0xE0`
  **của `CGOPet`**, không phải bomber; controller bomber nhúng tại byte `+1412` mới chỉ chứng minh
  nhận `dir`.
- Không xác định được mover/`len` của bomber trong lượt này (slot 24 `0xB44000`/`0xB77B10` chưa đọc hết).
- Danh sách caller `CMovable::MoveSimple` chưa phân lớp đủ (còn `0x64F2F0`, `0x684E60`, `0xF2C420`,
  `0xFC6160`, một caller `0xFA…` thiếu VA); không loại trừ đường gián tiếp/wrapper/VM từ bomber.
- Chưa đối chiếu VN cho câu hỏi "VN có dùng `CMovable::MoveSimple` cho bomber không" — bước tiếp số 2.
