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
| Caller của `0x00B95180` | **1**: `0x00B93A82` trong `0x00B93A60` | — |
| Caller của `0x00B95970` | **1**: `0x00B9384A` trong `0x00B93830` | — |
| Caller của `0x00B93A60` | **1**: data ref `0x01746EA0` (slot 24 `CGOPet`) | — |
| Dword `0x00B95180`/`0x00B93830`/`0x00B93890`/`0x00B95970` trong `.rdata` | **0** (quét `0x1600000–0x1A80000`) | — |
| Store vào `+0xE0` / `+0xD4` (mọi thanh ghi, mod=10) | 66 / 62 điểm; trong dải bomber chỉ thấy `0xB8C3F0`, `0xB9D540`, `0xB9D750`, `0xB6B3F0` (không phải bomber) | — |
| Ctor `CGOGameBomber` `0x00B79DC0` gán `+0xE0`/`+0xD4`? | **Không** | — |
| `lea [reg+0x584]` (địa chỉ sub-object bomber tại byte +1412) | **0** điểm | — |
| Căn cứ lớp của bomber | `??_R2CGOGameBomber@@8` `0x017CB498`: `CGOGameBomber→CGOBomber→CGOBomberBase→CGOAvatar→CGameObject→CObject` | — |
| Caller của `CMovable::MoveSimple` `0x01310200` | `0x61D600` (dispatcher `CMovable::Move` cho `CGOCatchMon`/`CGOMinibot`/`CGORepairMinibotWatcher`), `0x64F2F0`, `0x684E60`, `0xB93890` (CGOPet), `0xF2C420`, `0xFC6160`, `0xFA…` — **không** có lớp bomber | — |

## Provenance công cụ

- **REA 6.7.0** (`open_binary` bản sao, `provider_id: ida`, seed DB) — bằng chứng đã cấp `evidence_id`:

  | operation | evidence_id | nội dung |
  |---|---|---|
  | `xrefs` | `ev_e4565b5af7657ad73d2cf0806e5d76f36974c8c15bde173738d71c6ef036d80d` | `0xB93A60` → **1** xref `0x1746EA0` (data, slot 24 `CGOPet`) |
  | `batch_decompile` | `ev_9e2c7e089309a3142baa4668a7ea2a22eb0b5d0791fb70cf0ade95a51a5b4b82` | `0xB93A60` (update pet: gọi `0xB95180` rồi `0xB952E0`, `0xB93AB0`) + `0xB95180` (`vf[+8]` byte +8 trả `(dir,len)`) |

  Bundle export trước `close_binary`: `/home/admin/boom-team/tmp/re-kr/evidence-trapped-speed-147.json`
  (10.573 B, 2 record). `close_binary` → `{"result":null}`.
- **IDA MCP trực tiếp** trên DB **bản sao** (`execute_python` + `ida_bytes`/`ida_xref`/`ida_hexrays`):
  quét store `+0xE0`/`+0xD4`, quét `0x8D`/disp `0x584`, quét dword vtable trong `.rdata`, đọc slot 22–26
  của 3 lớp, đọc `??_R2CGOGameBomber@@8`, decompile `0xB77B10`/`0xB44000`. Fallback hợp lệ vì REA thiếu
  `read_bytes`/`search_bytes`; làm tuần tự sau khi đóng REA, không chạy hai worker song song.

### Sự cố REA 6.7.0 `unreadable_output` (đã xử lý)

Lượt đầu, mọi operation cấp target trả
`unreadable_output` — `IDA MCP returned malformed or unsupported producer data: success: Invalid input: expected true; session: Invalid input: expected object, received undefined`.
Nguyên nhân **không phải** lỗi phiên bản REA: phiên IDA MCP trực tiếp trước đó đã **ghi lại** DB bản sao
(`kr-full.exe.i64` mtime `2026-10-22 21:17:05`, SHA-256 đổi `e612361a…` → `20904663…`), nên launcher
`seeded_idalib.py` fail-closed với `ValueError('Seed database changed; refresh its pin explicitly before reuse')`;
helper trả `{"error": …}` (không có `success`/`session`), và REA 6.7.0 báo lỗi schema thay vì lỗi upstream.
Đã làm: cập nhật `database_sha256` trong `~/.config/rea/helpers/ida-db-seeds.json` (backup
`ida-db-seeds.json.bak-20261022-213458`), rồi chạy lại — OK. Quy ước mới: khi cần `read_bytes`/`search_bytes`
thì mở **bản sao của bản sao** cho IDA trực tiếp, để DB seed giữ nguyên byte; nếu đã ghi thì refresh pin.

## Giới hạn

- Không có công thức `(dir,len)` cho bomber: controller `bomber+0xE0` của chuỗi đã đọc là của `CGOPet`.
- Không xác định được mover của bomber trong lượt này (slot 24 `0xB44000`/`0xB77B10` chưa đọc hết).
- Chưa đối chiếu VN cho câu hỏi "VN có dùng `CMovable::MoveSimple` cho bomber không" — bước tiếp số 2.
