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

- **IDA MCP trực tiếp** trên DB **bản sao** (`execute_python`, `ida_bytes`/`ida_xref`/`ida_hexrays`):
  toàn bộ phép quét byte/dword, đọc slot vtable, decompile `0xB93A60`/`0xB77B10`/`0xB44000`.
  Lý do: REA thiếu `read_bytes`/`search_bytes` (đã biết) — đây là fallback được phép, làm tuần tự sau
  khi đóng REA, không chạy hai worker song song.
- **REA 6.7.0** (`open_binary` bản sao, `provider_id: ida`): session-level (`binary_session`,
  `get_evidence_bundle`) chạy; **mọi operation cấp target đều lỗi `unreadable_output`**:
  `IDA MCP returned malformed or unsupported producer data: success: Invalid input: expected true; session: Invalid input: expected object, received undefined`
  (`batch_decompile 0xB93A60/0xB95180`, `xrefs 0xB93A60`). `retryable:false`; không retry.
  ⇒ **không có `evidence_id`** cho lượt này. `close_binary` → `{"result":null}`; không có bundle
  (0 record) nên không export.
- Ghi chú cho điều phối: lỗi này xuất hiện ở 6.7.0 với cả `procedure_address`, `procedure_pseudo_code`,
  `xrefs` — nên REA hiện **không dùng được cho provider IDA cấp target**; xem lại trước khi dựa vào REA
  cho các lượt sau.

## Giới hạn

- Không có công thức `(dir,len)` cho bomber: controller `bomber+0xE0` của chuỗi đã đọc là của `CGOPet`.
- Không xác định được mover của bomber trong lượt này (slot 24 `0xB44000`/`0xB77B10` chưa đọc hết).
- Chưa đối chiếu VN cho câu hỏi "VN có dùng `CMovable::MoveSimple` cho bomber không" — bước tiếp số 2.
