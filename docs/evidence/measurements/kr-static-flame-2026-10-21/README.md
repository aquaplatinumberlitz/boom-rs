# Đo tĩnh KR 00967 — nổ/lửa, thời lượng frame (2026-10-21)

Metadata cho lượt điều tra [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148) / Q-019.
Lượt này **chưa** tạo được claim `kr_static` mới: chưa định vị consumer phát frame và consumer chọn
clip nổ/lửa theo hướng. Ghi lại mỏ neo, giới hạn công cụ và bước tiếp để lượt sau không lặp lại.
Trạng thái câu hỏi ở [QUESTIONS](../../../research/QUESTIONS.md) Q-019.

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `/home/admin/boomkr-re/out/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Đọc qua REA `provider_id: "ida"`, input là **bản sao** `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`;
engine seed database đã phân tích. Không mở DB gốc, không chạy client.

## Đã kiểm trong lượt này

| Việc | Kết quả |
|---|---|
| `CDataI16Alpha` frame 48 byte có trường thời gian? | **Không** — xác nhận lại [MAPS §3](../../../specification/assets/MAPS.md): 12×i32 mỗi frame, không có `ms`. R-079 giữ nguyên |
| Bảng state ODT có `(frame, duration)`? | **Có** — đã chốt ở R-094 (đọc từ pack); thời lượng nằm ở **bảng state**, không ở blob frame |
| Consumer phát frame (đọc duration từ bảng state, tiến frame theo thời gian) | **Chưa định vị** |
| Consumer chọn clip nổ/lửa theo hướng (tâm/thân/đuôi) | **Chưa định vị** |

## Mỏ neo đã tìm được (VA → RVA)

| Vai trò | VA | RVA | Ghi chú |
|---|---|---|---|
| `CGameObjectTemplate::GetAnimCount(objNo)` | `0x00BA3FF0` | `0x007A3FF0` | nhận `objNo`, trả số animation của template; chuỗi assert `CGameObjectTemplate::GetAnimCount(%d): %s` ở `0x017482E4` |
| Truy cập bảng template theo `objNo` | `0x007E6D20` | `0x003E6D20` | được `GetAnimCount` gọi |
| Đọc số animation từ record `+4` | `0x00B8E5A0` | `0x0078E5A0` | getter nhỏ; 10 caller |
| Cụm caller của getter (template lib) | `0x00BA3530`, `0x00BA40F0`, `0x00BA41D0`, `0x00BA42F0`, `0x00B8E2A0` | — | ứng viên cho parser/bảng anim |
| Caller trong cụm game object | `0x00B99990` | `0x00799990` | đã đọc: là hàm **đặt vật thể/bóng** (so chỉ số anim với số anim, vẽ tại `y−20`), **không** phải bộ phát frame |

## Bằng chứng REA trong phiên (evidence_id)

| operation | evidence_id | Nội dung |
|---|---|---|
| `xrefs` | `ev_576c37aa050834e85dbbd9166054644f700e37ced63048f2b71320f211c10fae` | `0x17482E4` → `sub_BA3FF0` |
| `procedure_pseudo_code` | `ev_389ea7fc7f8cd8d1a4d93ca56b92bc286a0b3f3ba1a3f16c3487f71d40fddb91` | `0xBA3FF0` |
| `xrefs` | `ev_d0992441939aff68ca44120307b01367acbb8bb35933f7a2e9759a1c3c9b2d61` | `0xB8E5A0` → 10 caller |
| `procedure_pseudo_code` | `ev_171dc7ec10abce4f34916f27ac00e33c01c64c666e11633d7aeba56f4fb2e04c` | `0xB99990` (không phải bộ phát frame) |

## Giới hạn công cụ (quan trọng cho lượt sau)

- Adapter IDA của REA **không khai báo** `read_bytes` và `search_bytes` (`capability_unavailable`,
  "operation is not declared by this provider"), nên lượt này **không** đọc được vtable, bảng dữ liệu
  hay quét hằng số.
- `search_strings` của adapter trả về **toàn bộ** bảng chuỗi (raw_result dùng `find_regex` với
  `(?s).*`) dù đã truyền pattern — rất tốn chỗ và không lọc được; chỉ `normalized_result` là danh sách
  khớp thật.
- Hệ quả: để tìm bộ phát frame và bộ chọn clip nổ, cần một trong hai: (a) provider có `read_bytes`
  (IDA MCP trực tiếp trên DB bản sao — đã dùng ở #146), hoặc (b) đi từ một mỏ neo tên/vtable đã biết.

## Bước tiếp (đề xuất, theo thứ tự)

1. Đọc `sub_BA3530` / `sub_BA40F0` / `sub_BA41D0` / `sub_BA42F0` — cụm template lib quanh
   `GetAnimCount`; tìm vòng lặp `(frame, duration)` và field offset của duration.
2. Từ field offset đó, tìm hàm đọc nó mỗi khung hình (bộ phát frame). Ứng viên: các lớp `CGameObject*`
   có `Update`/`Draw` nhận `elapsed`.
3. Với nổ/lửa theo hướng: đọc vtable `??_7CGOHiddenCatchBombExplodeEffect@@6B@` (`0x016FC62C`) và
   `??_7CGOBomberEffect@@6B@` (`0x0173CC28`) — cần `read_bytes` để lấy slot, hoặc xref tới chính
   vtable để lấy constructor rồi đọc các hàm lân cận.
4. Nếu cả hai đường đều vào `.themida` thì dừng và ghi entry VM theo
   [DEVIRT_METHOD](../../../research/findings/DEVIRT_METHOD.md).

## Bổ sung 2026-10-23 — R-151 blocker: duration ODT/nhịp update (R-141 frame)

REA MCP `provider_id: ida` trên bản sao `kr-full.exe` SHA-256 `cf72da11…e63b`; bundle
`.re-work/evidence-r151-anim-148.json` (20 record, không commit). Không đọc lại blob 48 byte; R-079
giữ `unknown`.

| Kiểm | VA | Kết quả |
|---|---|---|
| `CGOFlame` update | `0xB711C0` (vtable `0x1745EF0`, slot 24 `0x1745F50`) | `++flame[i+51]` mỗi lần update; xoá khi `>= GetAnimCount(flame[i+36])`; **không** đọc `dt`/elapsed/duration ODT |
| GetAnimCount | `B333D0` → `BA3FF0` → `B8E5A0(template+4)` | số animation/frame của template |
| Per-anim field | `BA40F0` (`[anim+4]`), `BA41D0` (`[anim]`), stride 8 | field thứ hai; chưa chứng minh là duration/ms |
| Tổng field | `BA4160` cộng `BA40F0` | caller `B338F0`/`B9CEC0`/`B9FDD0`; `B33B30` dùng `sub_15625C2` = **RNG**, không elapsed |
| State code | `B44000` | kiểm mã `24`/`27–30`/`39–42` + `B33230`; chưa nối nhãn ODT idle/walk/trapped/dead |
| Nhịp update | `0xB711C0` xref | chỉ data `0x1745F50`; không thấy call site trực tiếp ⇒ chưa quy đổi frame→ms |

**Kết luận:** R-141 confirmed lửa/segment đếm **frame** (không lặp claim); **R-151 là blocker** cho
ms/nhịp update, đơn vị field ODT, state→nhãn và consumer tâm/thân/đuôi/hướng lửa. **Đề xuất
presentation: giữ frame-based, không đặt ms; không đổi `bomb.flame_ms` hay profile.**

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_7ff15ec058e6221ae2a6398c54d0bdd3dd4d8752f9f6a129aa97d60603928f8d` | `batch_decompile` `B711C0`/`B333D0`/`BA3FF0`/`BA40F0` | frame counter/GetAnimCount |
| `ev_719cc83beac83d74646cc55759c5e7e805547d2be1f7a15c6d06adfb29739710` | `xrefs` `B711C0` | chỉ vtable `0x1745F50` |
| `ev_b03800902665906819d03a66f788684b2ae98e57e109d988950b2af791ab6e11` | `batch_decompile` `B33980`/`BA4160`/`B33AA0` | tổng field/frame count |
| `ev_582f04e64a5b5797cf47c4f62a56d69c95ce4dba65ebb544e8b923f4f1970e54` | `batch_decompile` `B338F0`/`B33B30`/`F016B0` | consumer/RNG phase |
| `ev_630f12978d13751bbb0f9be5fc428f8938fc17cde371b646897bfd3a5255ddd7` | `batch_decompile` `B9CEC0`/`B9FDD0` | effect dùng `BA4160` |
| `ev_2ae5cf02c0cdedc46533eb46360c1243004d15cb388a9e6d7e761d2ab9f237e0` | `batch_decompile` `15625C2` | RNG |
| `ev_2a06c640eab604e3b430f5959a636b2f53bc281dc2adb4f3ee7afa0620c55e68` | `batch_decompile` `B77B10`/`B44000` | update/state mã |
