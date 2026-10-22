# KR 00967 — state → animation, frame duration, nhịp update (R-151)

Lượt RE mục 6 hàng đợi REWRITE_PLAN, [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148)
(phần frame) / V1-031. Mục tiêu: consumer `(frame, duration)`/elapsed, đơn vị, loop/hold/end, state
idle/walk/trapped/dead, cách chọn tâm/thân/đuôi/hướng lửa, và nhịp update gọi `0xB711C0`.
Không RE lại blob frame 48 byte (đã biết không có trường thời gian — R-079).

## 11.1 Build / phạm vi

- KR `00967`, bản sao `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`, SHA-256
  `cf72da11…e63b`, image base `0x400000`; REA MCP `provider_id: ida`.
- Đã đọc: `B711C0`, `B333D0`, `BA3FF0`, `BA3530`, `BA40F0`, `BA41D0`, `BA42F0`, `BA4160`,
  `B33980`, `B33AA0`, `B338F0`, `B33B30`, `B9CEC0`, `B9FDD0`, `B8B5B0`, `15625C2`,
  `BA6C50` (ODT/ObjectTable loader), `B77B10`/`B44000` (bomber update).
- Không chạy client/server; không mở DB gốc; không đọc lại blob 48 byte.

## 11.2 `B711C0`: bộ đếm frame của `CGOFlame`

`B711C0` là virtual slot 24 (`0x1745F50`) của `CGOFlame` vtable `0x1745EF0`:

```
for i in 0..flame.count-1:
    ++flame[i+51]                              // frame counter
    if flame[i+51] >= GetAnimCount(flame[i+36]) // template/anim id
        xoá segment i (memmove các mảng song song), --count
```

- `GetAnimCount` là `B333D0` → `BA3FF0` → `B8E5A0(template+4)` (`0xBA3FF0`).
- `B711C0` **không** nhận `dt`, không đọc `elapsed`, không đọc bảng duration ODT; mỗi lần update
  tăng đúng 1. Vì vậy đơn vị của lifetime lửa là **frame update**, không phải ms.
- Xref của `B711C0` chỉ là data `0x1745F50` (vtable); không có call site trực tiếp trong scan
  `call [reg+0x60]` (xem §11.5).

## 11.3 Consumer duration ODT: `BA40F0` / `BA4160`

- `BA40F0(objNo, animIdx)` trả `*(animEntry+4)` với `animEntry = BADF80(animIdx)`; `BA41D0` trả
  `*(animEntry)`; `animEntry` stride 8, template record stride 20 (`BADFA0`).
- `BA4160(objNo)` cộng `BA40F0` cho `0..GetAnimCount-1` — tổng các field per-anim.
- Caller: `B338F0` (trả tổng), `B9CEC0`/`B9FDD0` (effect nổ) truyền tổng vào `B8B5B0` như một tham
  số effect; `B33980` trả field của một anim; `B33B30` dùng tổng để chọn pha.
- **Chưa chứng minh** field `[anim+4]` là duration theo ms (hay frame count/ID). `B33B30` dùng
  `sub_15625C2()` — hàm **RNG** (`214013*state+2531011`, `HIWORD & 0x7FFF`), không phải elapsed —
  nên không được suy “duration ODT = ms” từ caller này.
- Vì vậy consumer `(frame,duration)` có tồn tại ở mức đọc field/tổng, nhưng **đơn vị và nghĩa
  loop/hold/end chưa xác định**.

## 11.4 State → animation (idle/walk/trapped/dead)

- Nguồn nhãn vẫn là bảng ODT (R-094/R-095, `kr_static`): `Stop <Dir>` → frame 0 `Walk<Dir>_A`;
  `Locked In`/`Locked In Move/Stop` → frame 0…1 `Die` giữ (duration 32767); `Die` 14 frame;
  `Walk<Dir>` 5 frame; `Start` 10 frame. Đây là **nhãn dữ liệu**, chưa có trace runtime nối state
  core V1 với nhãn.
- `B44000` (bomber update) kiểm các mã `24`, `27–30`, `39–42` và `B33230` (frame hiện tại) để chọn
  nhánh animation/effect, nhưng **không** đọc tên state ODT và không có bảng ánh xạ mã → nhãn trong
  phạm vi đã đọc. Không gán idle/walk/trapped/dead cho các mã này khi chưa có consumer.
- `B77B10` chỉ là update wrapper gọi `B44000`; không đọc duration/elapsed.

## 11.5 Nhịp update gọi `B711C0`

- `B711C0` chỉ có một xref: data `0x1745F50` (slot 24 vtable `0x1745EF0`). Không có caller trực
  tiếp trong scan toàn bộ `call DWORD PTR [reg+0x60]` ở vùng code; nghĩa là nó được gọi qua
  dispatch vtable từ vòng update/render chưa định vị.
- Không thấy hằng `Sleep(…)`, `timeGetTime`/delta trong đường này; **nhịp update chưa xác định**.
- Hệ quả: chưa có cơ sở `frame → ms`; mọi giá trị ms (kể cả `bomb.flame_ms=500`) vẫn là design,
  không phải suy ra từ KR.

## 11.6 Evidence

Bundle `.re-work/evidence-r151-anim-148.json` (20 record, không commit).

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_7ff15ec058e6221ae2a6398c54d0bdd3dd4d8752f9f6a129aa97d60603928f8d` | `batch_decompile` `B711C0`/`B333D0`/`BA3FF0`/`BA3530`/`BA40F0`/`BA41D0`/`BA42F0` | frame counter, GetAnimCount, playback |
| `ev_719cc83beac83d74646cc55759c5e7e805547d2be1f7a15c6d06adfb29739710` | `xrefs` `B711C0` | chỉ vtable `0x1745F50` |
| `ev_b03800902665906819d03a66f788684b2ae98e57e109d988950b2af791ab6e11` | `batch_decompile` `B33980`/`BA4160`/`B33AA0` | tổng field per-anim, frame count |
| `ev_582f04e64a5b5797cf47c4f62a56d69c95ce4dba65ebb544e8b923f4f1970e54` | `batch_decompile` `B338F0`/`B33B30`/`F016B0` | consumer tổng, RNG phase |
| `ev_630f12978d13751bbb0f9be5fc428f8938fc17cde371b646897bfd3a5255ddd7` | `batch_decompile` `B9CEC0`/`B9FDD0` | effect dùng `BA4160` |
| `ev_2ae5cf02c0cdedc46533eb46360c1243004d15cb388a9e6d7e761d2ab9f237e0` | `batch_decompile` `15625C2` | RNG, không elapsed |
| `ev_83bbc79caea1412fbc19598a4df58d60d265d505a3f0570bfd9341b81a1c6630` | `batch_decompile` `BA6C50` | ObjectTable loader (tên/ID) |
| `ev_2a06c640eab604e3b430f5959a636b2f53bc281dc2adb4f3ee7afa0620c55e68` | `batch_decompile` `B77B10`/`B44000` | update bomber, mã anim 24/27–30/39–42 |
| `ev_d3535a207d4a7dd5e3baab6001f39b6d4661a3a0028b5331f5a8e1075196bfcd` | `xrefs` `BA40F0` | 2 caller |
| `ev_c46f1fe04d2f47664f162e9efb5510fca3a2dce18f240812f5ff5b42147a5106` | `xrefs` `BA4160` | 3 caller |

## 11.7 Kết luận / đề xuất

- `kr_static` **confirmed**: lửa/segment đếm bằng **frame update**; xoá khi
  `frame >= GetAnimCount`; không có elapsed/duration ODT trong đường này.
- **Blocker có phạm vi**: chưa xác định nhịp update → chưa quy đổi frame→ms; consumer duration ODT
  (`BA40F0`/`BA4160`) chưa chứng minh đơn vị/loop/hold; state→nhãn ODT và chọn tâm/thân/đuôi/hướng
  lửa chưa có consumer runtime.
- **Đề xuất presentation**: giữ frame-based, không đặt ms; không đổi `bomb.flame_ms`, R-079 vẫn
  `unknown`. Nếu cần ms, phải đo nhịp update trên client gốc hoặc tìm call site vf24 rồi mở PR
  profile/presentation riêng.
