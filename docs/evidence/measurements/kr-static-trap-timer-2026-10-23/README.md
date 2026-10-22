# Đo tĩnh KR 00967 — bộ đếm bomber → chết, chạm/chọc (2026-10-23)

Metadata cho lượt điều tra [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) / Q-018,
mục 4 hàng đợi RE (timer nhốt → chết, chạm/chọc, biên chuyển life). Claim tương ứng:
[RULES_V1 § (đánh số khi merge)](../../../specification/gameplay/RULES_V1.md#-đánh-số-khi-merge--re-kr-00967--bộ-đếm-bomber--chết-r-200r-202)
(R-200…R-202); suy luận, giới hạn và bước tiếp ở
[KR_RE_TRAP_TIMER](../../../research/findings/KR_RE_TRAP_TIMER.md).

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `kr-full.exe` (bản sao `/home/admin/boom-rs-dsh/re-kr2/.re-tmp/kr-full.exe`) | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Không commit binary/DB. Bản sao chỉ nằm ngoài repo (`.re-tmp/`, đã `git/info/exclude`); DB gốc của
phiên một không bị mở/sửa.

## Công cụ và provenance

| Bước | Tool | Ghi chú |
|---|---|---|
| Bind binary | REA `open_binary` `provider_id: "ida"` trên **bản sao** | hash khớp build; REA dùng private seeded DB |
| Decompile + evidence | REA `procedure_pseudo_code` | `evidence_id` dưới đây |
| Thiếu operation | IDA MCP trực tiếp trên **bản sao** `.i64` | REA adapter **không khai báo** `read_bytes`, `search_bytes`, `list_names`; đã dùng IDA cho: quét displacement `0x2748`, đọc vtable slot 202/24, disasm thunk VM, đọc byte đích VM |
| Chạy client | không | chỉ tĩnh/offline |

Thứ tự tuần tự: IDA (quét/đọc byte) → đóng → REA (decompile) → export bundle → đóng. Không chạy hai
analysis worker song song.

## Bằng chứng REA trong phiên

| `evidence_id` | operation | Đối tượng | Dùng cho |
|---|---|---|---|
| `ev_6035bfe5f10f718e61f44e7d470a73bd57bca2ba08d6379d69220aa17d701e84` | `procedure_pseudo_code` | `0x00B44000` | R-200 (update: nhánh đếm, ngưỡng, gọi slot 202) |
| `ev_fb801a511408911422b5e648af13f64f247029ffd5561d3c7609273f8af0ff35` | `procedure_pseudo_code` | `0x006833D0` | R-200 (predicate `this[2514] > 0`) |
| `ev_3420dbcbf699227e94e39affc087832a393f65948a7bbdca86d871f602f97de1` | `procedure_pseudo_code` | `0x00B7A760` | R-200 (slot 202: reset + `CSendPacket(167)`) |
| `ev_c8ea4fa990450912e216e1b18ebefd2f3eb0f390642405420263baf377e1c9fa` | `procedure_pseudo_code` | `0x00E84460` | R-200 (serialize byte `predicate` cho status 184) |

Bundle xuất tại `.re-tmp/evidence-trap-timer-147.json` (4 record, không commit).

## Mỏ neo chính (VA → RVA)

| Vai trò | VA | RVA | Ghi chú |
|---|---|---|---|
| Predicate bộ đếm | `0x006833D0` | `0x002833D0` | `return this[2514] > 0` (dword tại `+0x2748`) |
| `CGOBomber::Update` (slot 24) | `0x00B44000` | `0x00744000` | nhánh đếm `0x00B44071`–`0x00B440DE`; RMW giảm ở `0x00B4409A`/`0x00B440A4`; gọi slot 202 ở `0x00B440D8`/`0x00B440DE` |
| `CGOGameBomber::Update` (slot 24) | `0x00B77B10` | `0x00777B10` | gọi `0x00B44000` ở `0x00B77E22` |
| Slot 202 `CGOGameBomber` | `0x00B7A760` | `0x0077A760` | reset trường; `CSendPacket(167)`; ghi `vf+40` của `+353` và hằng `6`; gửi |
| Slot 202 `CGOBomber` | `0x00B5D630` | `0x0075D630` | rỗng (no-op) |
| `CSendPacket::CSendPacket` | `0x01164C10` | `0x00D64C10` | nhận type; log `[enGameClientPacket] packet type %d` |
| Serialize status bomber | `0x00E84460` | `0x00A84460` | mỗi bomber ghi `[1, v7, v6, v5, v4]`, `v4 = predicate(bomber)` |
| Đóng gói status 184 | `0x00E67E70` | `0x00A67E70` | gọi `0x00E84460`; `push 0B8h` (184) tại `0x00E68201` rồi `0x01164C10` |
| Bảng tên + id của stage | `0x01261F50` | `0x00E61F50` | 44 tên; `DeferBomberReleaseOrDie` id 1, `DeferBomberLockedIn` id 30 (R-131) |
| Thunk VM (vị từ thứ hai) | `0x00B534D0` | `0x007534D0` | prologue native rồi `jmp near ptr 0x02012376` |

## Quét displacement `0x2748`

Quét toàn image (0x401000–0x1A98000) mọi encoding chứa disp32 `0x2748` (`[reg+2748h]`, `lea`,
`add`). Kết quả **10 site**:

| # | Địa chỉ | Lệnh | Hàm | Vai trò |
|---|---|---|---|---|
| 1 | `0x006833DA` | `cmp dword ptr [eax+2748h], 0` | `0x006833D0` | đọc (predicate) |
| 2 | `0x00B3D70D` | `mov dword ptr [edx+2748h], 0` | `0x00B3D1D0` | zero-init |
| 3 | `0x00B3DFD6` | `mov dword ptr [eax+2748h], 0` | `0x00B3DCB0` | zero-init (ctor/reset) |
| 4 | `0x00B44098` | `mov edx, [ecx+2748h]` | `0x00B44000` | đọc trong RMW |
| 5 | `0x00B440A4` | `mov [eax+2748h], edx` | `0x00B44000` | ghi sau `sub edx,1` |
| 6 | `0x00B440C5` | `cmp dword ptr [eax+2748h], 0` | `0x00B44000` | ngưỡng `<=0` |
| 7 | `0x00B59786` | `mov dword ptr [ecx+2748h], 0` | `0x00B58B60` | zero-init |
| 8 | `0x00B7A78F` | `mov dword ptr [eax+2748h], 0` | `0x00B7A760` | zero-init (slot 202) |
| 9 | `0x01036718` | `mov dword ptr [eax+2748h], 0` | `0x01036700` | zero-init |
| 10 | `0x017A60CC` | dword trong bảng | — | không phải code |

Không có site ghi giá trị dương; không có `lea`/`add` tạo địa chỉ trường ngoài bảng trên.

## Biên VM

- `0x00B534D0`: `55 8B EC 83 EC 0C 53 56 57 89 4D FC C7 45 F8 00 … E9 <rel>` → `jmp near ptr 0x02012376`.
- Đích `0x02012376` thuộc segment `________` (`0x01D7B000–0x02076000`); byte tại đích `0xFF` (không
  phải code native đã phân tích) — theo [DEVIRT_METHOD](../../../research/findings/DEVIRT_METHOD.md),
  ghi entry VM và **dừng** nhánh này.
- Cùng thunk được gọi từ `0x00B53330`, `0x00B46760`, `0x00B46830`, `0x00B46940`, `0x00B78780`,
  `0x00682790` (và trong update `0x00B44080`).

## Giới hạn

- Chưa chứng minh `+0x2748` là **timer nhốt**; chưa nối state 3/`IsLockedInState` với trường này.
- Seed/điểm vào trạng thái nằm ngoài code native đã quét (VM/không thấy store dương).
- Chưa có nhịp update KR → không quy đổi ms; 30 ms/bước là đo trên VN (R-013).
- Packet 167/184 chỉ đọc phía client; chưa có handler server/log runtime.
- Chạm/chọc và thứ tự hết hạn vs chạm vs lửa trong cùng bước: chưa định lượng.
