# KR 00967 — UI V1: rect/hitbox, layout phòng, native/scale (R-152)

Lượt RE [#150](https://github.com/aquaplatinumberlitz/boom-rs/issues/150) / Q-012/Q-021,
V1-032/033/035. Phạm vi hẹp: một consumer hitbox (danh sách phòng) + một màn layout (PrepareStage)
+ resource key. Không mở shop/quảng trường/mode later; không tự dịch/duyệt ảnh.

## 12.1 Build / phạm vi

- KR `00967`, bản sao `/home/admin/boom-team/tmp/re-kr/ida/kr-full.exe`, SHA-256
  `cf72da11…e63b`, image base `0x400000`; REA MCP `provider_id: ida`.
- Đã đọc: `sub_549180` (room-list hit test), `sub_1504A5C` (input point), `sub_149BBA0`/
  `sub_149BEF0` (origin), `sub_54BEF0`/`sub_4718F0` (placement/rect ctor), `sub_6F47C0`
  (PrepareStage layout), `sub_6F8790` (key join), `sub_6E3DF0` (caller), `sub_54D570`/`sub_54EB00`.
- Không chạy client/server; không dùng JPEG 800×600 làm bằng chứng framebuffer.

## 12.2 Lobby room list — hitbox đã nối

`sub_549180` (RVA `0x149180`) là hit test danh sách 6 phòng:

- Bảng vị trí `dword_19D8A58` (VA `0x19D8A58`), 6 cặp `(left,top)`:
  `(20,90)`, `(194,90)`, `(20,160)`, `(194,160)`, `(20,230)`, `(194,230)`.
- `width/height` lấy từ control hiện tại: virtual `+36` và `+40` (`v9`, `v8`).
- Điểm click: `sub_1504A5C` trả hai field `+136`/`+140` của input object; sau đó trừ gốc container
  `sub_149BBA0` (left) và `sub_149BEF0` (top) để được toạ độ local.
- Công thức hit:
  `x >= left && y >= top && x < left + width && y < top + height`.
  Đơn vị là **UI/input px**; trong hàm này **không** thấy nhân/chia scale.
- `sub_54BC00(i)` trả entry `i * 680` trong danh sách; `sub_54BDC0` trả singleton danh sách.
- Đây là consumer hitbox thực; chưa chứng minh focus dùng cùng rect (chưa đọc focus path).

## 12.3 PrepareStage — layout và resource key

`sub_6F47C0` (RVA `0x2F47C0`), gọi từ `sub_6E3DF0` tại `0x6E3E64`:

- `sub_54BEF0(dst, x, y, a4, a5)` chỉ lưu 4 int; getter tương ứng `[0]` `sub_149BBA0`, `[1]`
  `sub_149BEF0`, `[2]` `sub_54D570`, `[3]` `sub_54EB00`.
- Ready: `sub_54BEF0(..., 512, 494, 0x10000000, 0x10000000)`; `sub_6F8790(..., "PRE_Bt_Ready")`.
- Room info: `(344, 61, …)`; hai nút chọn nhân vật: `(541, 231)` và `(690, 231)`; nền:
  `(800, 600, 40, 48)`.
- `sub_6F8790` ghép key `"%s/%s"`: base lấy từ `sub_450370(this + 267088)`, key truyền vào.
  Với Ready, key pack là `Prepare/PRE_Bt_Ready` (khớp UI_STRINGS §3).
- **Chưa chứng minh**: 4 int của `sub_54BEF0` là `(x,y,w,h)` hay `(x,y,anchorX,anchorY)`; các literal
  `800,600,40,48` chỉ là tham số layout, không phải native framebuffer; scale/focus chưa nối.

## 12.4 Native size / scale / focus

- Hit test dùng toạ độ input trực tiếp, không thấy phép scale; nhưng chưa đọc hàm biến đổi input →
  client (có thể ở `sub_1504A5C` upstream) nên **chưa** kết luận scale = 1.
- Không thấy consumer native framebuffer/`GetClientRect`/`SetScale` trong phạm vi này.
- Ảnh JPEG 800×600 của login/Prepare đã bị loại trong UI_STRINGS; không dùng làm bằng chứng.

### Ghi chú cache 2026-10-25 (không claim mới)

Dùng lại đúng cached record `ev_640c445056119a02b7ced620e05f7a42c6717a9171003f96ffd5eb1205d9ae3e`
(R152, `batch_decompile`, PE `cf72…`; không mở body/provider mới). Trong hai full cached body
`5519A0`/`554C90`, cả hai là consumer index/scroll độc lập; **không** thấy lời gọi `549180` hoặc
tham chiếu trực tiếp bảng rect `dword_19D8A58` trong chính hai body này.

Điều này **không** chứng minh vắng call gián tiếp/toàn bộ callgraph và không phủ nhận việc dùng
chung rect ở chỗ khác. Owner của keyboard focus vẫn **UNPROVEN**; không đổi tên `5519A0`/`554C90`
thành hàm focus/set keyboard chỉ vì chúng dùng index/scroll. Số đo 6 rect R152 tại `19D8A58` vẫn
giữ nguyên proof static ban đầu; cache hiện chỉ có locator, không biến các số đã đo thành
“unknown numbers”.

## 12.5 C00/C01 → tên

- **Lịch sử R-152:** `room.character_c00`/`c01` chưa có locator catalog/asset native; không thấy
  bảng tên hiển thị C00/C01 trong phạm vi code này, chỉ có key resource `Character/C00`. Tên gốc
  native vẫn **unknown**.
- **Hiện hành 2026-10-25:** V1-035 đã done (PR241/246; 126 text/6 sprite đã duyệt). UI hiện dùng
  nhãn DESIGN “Nhân vật 1/2” gắn C00/C01; đây không phải tên native confirmed và không phải blocker
  V1-035. Tên gốc C00/C01 tiếp tục là native UI fidelity unknown tại #150.

## 12.6 Evidence

Bundle `.re-work/evidence-r152-ui-150.json` (17 record, không commit), R152 provenance ngày
2026-10-23; ghi chú current-context 2026-10-25 dùng lại đúng record, không tạo bằng chứng mới.

| `evidence_id` | Operation | Dùng cho |
|---|---|---|
| `ev_640c445056119a02b7ced620e05f7a42c6717a9171003f96ffd5eb1205d9ae3e` | `batch_decompile` `549180`/`5519A0`/`554C90` | hit test 6 ô; cache note: 5519A0/554C90 index/scroll, không thấy 549180/rect table trong hai body |
| `ev_60c239272475d48bf80556f25de048d0dc2348053e3bf04308726ccc4708ea98` | `batch_decompile` `1504A5C`/`54BDC0`/`54BC00` | input point, list |
| `ev_3493b642211afe6fefd4d301c899b7d3ce9b2163f886bc7dc9a0554e94619933` | `procedure_pseudo_code` `6F47C0` | PrepareStage layout/Ready |
| `ev_75acb00830251be9f2a5723d025b61b34141006722b898c812d01c4622fa599d` | `batch_decompile` `54BEF0`/`6F8790`/… | 4-int placement, key join |
| `ev_e8670730b09619530942d310298ce28dd51ced30e80eb7fe5cb84bae345626a3` | `xrefs` `6F47C0` | caller `6E3E64` |
| `ev_49871a279be4d2422613c3c06e1c749e54ef0c10347c6ef50d01785cb4877490` | `procedure_pseudo_code` `6E3DF0` | caller/layout context |
| `ev_54b8ae01a94a3396ff0f41c220e684dad7150d0e3f5ae3c3b312c4b572eabfea` | `xrefs` `PRE_Bt_Ready` | 2 data ref (layout) |

## 12.7 Kết luận / blocker

- `kr_static` confirmed: hit test 6 ô room list + bảng toạ độ + công thức inclusive/exclusive.
- `candidate`: Ready placement/key (`512,494` + `PRE_Bt_Ready`) và 4-int interpretation.
- **Native UI fidelity còn unknown:** framebuffer/scale, keyboard focus owner/path, các màn
  login/result, C00/C01→tên native. Không claim `549180` là focus path hoặc KR native focus đã xác nhận.
- **Trạng thái hiện hành:** G5 functional passed PR279; V1-035 done PR241/246. Đây là acceptance
  functional/DESIGN độc lập, **không** xác nhận native KR focus/framebuffer/scale và **không** thay
  GUI Windows/macOS/V1-036. Không đổi profile/presentation hay tự nâng gate; phạm vi fidelity #150
  vẫn **PARTIAL**.
