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
| State machine bomber mỗi bước cập nhật (set state 3, 13→14→2) | `0x00B952E0` | `0x007952E0` |
| Cổng "được di chuyển" (loại `{0,1,4,13,14}`) | `0x00B95970` | `0x00795970` |
| Chọn họ state đi `dir+9` / `dir+19` | `0x00B95840` | `0x00795840` |
| Đổi nhóm khoá + đặt bộ đếm giữ `20` | `0x00B95BD0` | `0x00795BD0` |
| Đọc bộ đếm giữ (`this[235]`, `this[236]`) | `0x00B95DA0` / `0x00B95DD0` | `0x00795DA0` / `0x00795DD0` |
| Nhận diện nhóm state `15..18` / `19..22` | `0x00B95E00` | `0x00795E00` |
| Cập nhật điều khiển bomber (bỏ qua state `13`,`14`,`1`) | `0x00B95180` | `0x00795180` |
| Cổng vào di chuyển | `0x00B93830` | `0x00793830` |
| Vòng lặp di chuyển | `0x00B93890` | `0x00793890` |
| Trạng thái di chuyển theo dir (kiểm `state-9`/`state-19`) | `0x00B956C0` | `0x007956C0` |
| Cờ "còn sống" (từ controller) | `0x00B95AC0` / `0x00B95AE0` | `0x00795AC0` / `0x00795AE0` |

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

`evidence_id` chỉ có giá trị trong phiên REA khi target còn mở; bản bundle được export ra
`/home/admin/boom-team/tmp/re-kr/evidence-trapped-147.json` (ngoài repo) trước khi đóng target.

## Giới hạn

- Adapter IDA của REA từ chối `read_bytes`/`search_bytes` (`capability_unavailable`), nên **không** đọc
  trực tiếp vtable/byte trong phiên này; kết luận chỉ từ decompile + xrefs.
- Chưa có trace runtime; "20 bước ≈ 600 ms" là quy đổi theo R-013 (30 ms/bước, đo trên VN), chưa đo KR.
- Số state `3` = bị nhốt ghép từ code KR + tài liệu packet VN, không phải nhãn có sẵn trong binary.

## Đối chiếu consumer trước khi áp dụng (#176, 2026-10-22)

Codex đọc bundle đã export của dsh cho `0x00B95970` và kết quả REA cho `0x00B95180`,
rồi gọi lại REA `procedure_pseudo_code` tại `0x00B93830` trên binary bản sao có cùng SHA-256.
Hàm kiểm cổng `0x00B95970` rồi chuyển nguyên hướng/quãng đường sang `0x00B93890`;
đối chiếu này xác nhận đường gọi của R-127, không đo tốc độ trapped hay timer KR.
Evidence `ev_e945af3e122cceb19b4d35e256dbc56b364c75adc67b6be1338d75ac88106546`;
bản JSON lưu ngoài repo: `/home/admin/boom-team/tmp/re-kr/rea-review-176-movegate.json`.
Không thêm decompile nguyên văn vào repository.
