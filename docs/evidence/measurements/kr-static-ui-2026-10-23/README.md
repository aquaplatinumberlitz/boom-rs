# Đo tĩnh KR 00967 — UI V1 rect/hitbox/layout (2026-10-23)

Metadata cho [#150](https://github.com/aquaplatinumberlitz/boom-rs/issues/150) / Q-012/Q-021,
V1-032/033/035. Claim R-152. Chỉ một consumer hitbox (lobby room list) + một màn layout
(PrepareStage); không mở shop/quảng trường/later; không dùng JPEG 800×600 làm framebuffer.

## Build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (unpacked) | `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe` | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x400000` | `VA − 0x400000` |

REA MCP `provider_id: ida` trên bản sao; không mở DB gốc, không chạy client.

## Đã đọc

| Hạng mục | VA | RVA | Ghi chú |
|---|---|---|---|
| Lobby room-list hit test | `0x00549180` | `0x00149180` | 6 ô; bảng `0x19D8A58`; width/height từ vtable `+36`/`+40`; điểm `sub_1504A5C` |
| Input point | `0x01504A5C` | `0x01104A5C` | trả field `+136`/`+140` |
| Container origin | `0x0149BBA0` / `0x0149BEF0` | — | getter `[0]`/`[1]` |
| 4-int placement ctor | `0x0054BEF0` | `0x0014BEF0` | chỉ lưu 4 int |
| PrepareStage layout | `0x006F47C0` | `0x002F47C0` | Ready `512,494`; info `344,61`; char `541/690,231`; nền `800,600,40,48` |
| Key join | `0x006F8790` | `0x002F8790` | `"%s/%s"` base `+267088` + key |
| Caller | `0x006E3DF0` | `0x002E3DF0` | gọi `0x6E3E64` |

## Công thức / đơn vị

- Hit: `left ≤ x < left+width`, `top ≤ y < top+height`; `(left,top)` từ bảng 6 ô, `width/height`
  từ control, `x/y` từ input trừ origin. Đơn vị **UI/input px**; không thấy scale trong hàm.
- Ready key: `PRE_Bt_Ready` → pack `Prepare/PRE_Bt_Ready`; layout tuple `(512,494,0x10000000,0x10000000)`.
- **Không** suy 800×600 là native framebuffer; **không** suy 4-int là `(x,y,w,h)` khi chưa đọc
  consumer diễn giải.

## Chưa đóng

Native size/scale/focus, các màn login/result, C00/C01→tên (`room.character_c00/c01` vẫn `unknown`),
ý nghĩa 4-int/anchors. Không đổi profile/presentation; không đóng #150.

## Evidence

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_640c445056119a02b7ced620e05f7a42c6717a9171003f96ffd5eb1205d9ae3e` | `batch_decompile` `549180`/`5519A0`/`554C90` | hitbox/công thức |
| `ev_60c239272475d48bf80556f25de048d0dc2348053e3bf04308726ccc4708ea98` | `batch_decompile` `1504A5C`/`54BDC0`/`54BC00` | input/list |
| `ev_3493b642211afe6fefd4d301c899b7d3ce9b2163f886bc7dc9a0554e94619933` | `procedure_pseudo_code` `6F47C0` | layout/Ready |
| `ev_75acb00830251be9f2a5723d025b61b34141006722b898c812d01c4622fa599d` | `batch_decompile` `54BEF0`/`6F8790` | placement/key |
| `ev_e8670730b09619530942d310298ce28dd51ced30e80eb7fe5cb84bae345626a3` | `xrefs` `6F47C0` | caller |
| `ev_49871a279be4d2422613c3c06e1c749e54ef0c10347c6ef50d01785cb4877490` | `procedure_pseudo_code` `6E3DF0` | caller context |
| `ev_54b8ae01a94a3396ff0f41c220e684dad7150d0e3f5ae3c3b312c4b572eabfea` | `xrefs` `PRE_Bt_Ready` | data ref layout |

Bundle `.re-work/evidence-r152-ui-150.json` (17 record, không commit).
