# Đo tĩnh KR 00967 — nổ lan, vật cản, dây chuyền và thời gian lửa (2026-10-22)

Metadata cho lượt điều tra [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148) / Q-019,
mục 5 hàng đợi RE (propagation/chain và thời gian gây damage của lửa). Claim tương ứng:
[RULES_V1 §16](../../../specification/gameplay/RULES_V1.md#16-re-kr-00967--nổ-lan-vật-cản-và-dây-chuyền-r-138r-140)
(R-138…R-140); suy luận, giới hạn và bước tiếp ở
[KR_RE_FLAME_CHAIN](../../../research/findings/KR_RE_FLAME_CHAIN.md).

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `kr-full.exe` (bản sao `/home/admin/boom-rs-dsh/re-kr2/.re-tmp/kr-full.exe`) | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Không commit binary/DB ảnh game. Bản sao chỉ nằm ngoài repo (`.re-tmp/`, đã `git/info/exclude`) để
reviewer tái kiểm; DB gốc của phiên một không bị mở/sửa.

## Công cụ và provenance

| Bước | Tool | Ghi chú |
|---|---|---|
| Bind binary | REA `open_binary` `provider_id: "ida"` trên **bản sao** `kr-full.exe` | hash khớp build; REA dùng private seeded DB, không mở `.i64` gốc |
| Đọc thân hàm / xref | REA `procedure_pseudo_code`, `xrefs` | `evidence_id` dưới đây |
| Thiếu operation | IDA MCP trực tiếp trên **bản sao** `.i64` | REA adapter **không khai báo** `read_bytes`, `search_bytes`, `list_names`; `search_procedures` trên ~26 MB timeout. Đã dùng IDA cho `read_bytes` (bảng hướng/vtable/byte), `xrefs`, `decompile` các hàm phụ, và tìm hàm đăng ký tên không được IDA tạo tự động (`0x01261F50`, phải `add_func`) |
| Chạy client | không | chỉ tĩnh/offline |

Không chạy hai analysis worker song song: REA mở → export bundle → đóng; sau đó IDA MCP mở bản sao
`.i64`, đóng trước khi mở lại REA để lấy evidence lần cuối.

## Bằng chứng REA trong phiên

| `evidence_id` | operation | Đối tượng | Dùng cho |
|---|---|---|---|
| `ev_4fd7d146f8136905aaa23a5b375598418d46fea8300551873660c5f47c21872b` | `procedure_pseudo_code` | `0x00E9C520` | R-138 (calc vùng nổ theo hướng) |
| `ev_d667d89c13df7ffe72a2cfb39560bea219f57873673d0dc4d1eca9bbd7afceef` | `procedure_pseudo_code` | `0x00E9CDC0` | R-139 (calc dây chuyền) |
| `ev_a86e37b2b769d7f1d2189b68ff6c4b5f057aed675ff8849184aac54ceed26b07` | `procedure_pseudo_code` | `0x00B711C0` | R-140 (segment timer của `CGOFlame`) |
| `ev_bd381d4f9084eb583d0185eff670edd190494de4cdd3b4d3c56fd6a807e05f20` | `procedure_pseudo_code` | `0x00B333D0` | R-140 (`GetAnimCount` = số frame) |
| `ev_ff03ae20e14d3338d14a84546b8876a2cfe8c977c2c1bc542d338e095a5e19f7` | `procedure_pseudo_code` | `0x00ACC340` | loại trừ: hàm dựng descriptor vẽ, **không** phải trigger dây chuyền |
| `ev_cfe43f097228e4830d4b9953bbea8c10e05765c668249e109bb0d5bdf280176b` | `procedure_pseudo_code` | `0x00E9F9B0` | kiểm thứ tự ô `(hàng, cột)`: `CGOTileFixed`/`CGOTilePassable`; đối chiếu nhãn hướng R-120 |

Các `evidence_id` trên được lấy trong hai lease REA (5 record + 1 record kiểm thứ tự ô). File bundle local
`.re-tmp/evidence-flame-chain-148.json` hiện chỉ giữ record của lease cuối (1 record) do lần export sau ghi đè;
không commit bundle. Tái lập bằng `procedure_pseudo_code` theo bảng địa chỉ ở trên. Hai lệnh `xrefs` vtable
(`0x016FC62C` → `ev_31c9b3b9f8e72ffced092b0e13413d9bb19533f420f2a52761dc86d4144eadc3`,
`0x0173CC28` → `ev_32d3aca3eb797829398db6ecbb67aa9d7b9d5e7cf5e31aea7f8177b45566e1b2`) thuộc lượt
REA trước khi đóng, chỉ dùng làm mỏ neo tên lớp, không nằm trong bundle xuất cuối.

## Mỏ neo chính (VA → RVA)

| Vai trò | VA | RVA | Ghi chú |
|---|---|---|---|
| `CPlayMap::CalcBombDirectionExplosion` | `0x00E9C520` | `0x00A9C520` | thân `0x89C`; vòng 4 hướng × tầm; kiểu 2 = vuông đầy |
| `CPlayMap::CalcChainExplosion_Direction` | `0x00E9CDC0` | `0x00A9CDC0` | thân `0x33D`; đăng ký ô cho object tại ô |
| Áp vụ nổ (descriptor) | `0x00E9E660` | `0x00A9E660` | thân `0x644`; gọi `sub_EB8000`, `sub_EB3780` |
| Áp vụ nổ (biến thể) | `0x00E9EEE0` | `0x00A9EEE0` | gọi `0x00B384E0` (init bom từ descriptor) |
| Descriptor nổ/di chuyển object | `0x00E84650` | `0x00A84650` | 2 site gọi `0x00E9CDC0` |
| `CGOBomb::Init` từ descriptor | `0x00B384E0` | `0x007384E0` | đọc `desc+168/172` (u64) → bomb `[+98]/[+99]`; `desc+96` → slot `+104` (chưa rõ nghĩa); `desc+156` → `sub_B3BA90` |
| `CGOBomb` update | `0x00B391E0` | `0x007391E0` | chỉ bật/nảy + state 1/2/3; **không** có fuse timer |
| Ctor `CGOBombExplosionEffect` | `0x00B23370` | `0x00723370` | vtable `0x0174252C`; tên anim `BombExplosion%03d` (`0x017443A0`) |
| Bảng hướng (dùng chung) | `0x017929B0` | `0x013929B0` | `[-1,0, 0,-1, 1,0, 0,1]` (8 dword) |
| Đăng ký ô → message id 27 | `0x00EBD3F0` | `0x00ABD3F0` | `sub_EDB7E0` → `sub_EC2420(object,msg)` push vào list của object |
| Kiểm object tại ô (gạch) | `0x00E9FCF0` | `0x00A9FCF0` | `CGOTileFixed` / `CGOTileFixedBox` |
| Kiểm object tại ô (tường) | `0x00E9FA90` | `0x00A9FA90` | `CGOTileFixed` / `CGOTileStatic` |
| Xoá object/ô | `0x00E9E3E0` | `0x00A9E3E0` | dùng khi phá gạch |
| `CGOFlame` vtable | `0x01745EF0` | `0x01345EF0` | 36 slot; slot 0 = `0x00B723B0` (scalar deleting destructor, nghi dtor) |
| `CGOFlame` ctor | `0x00B70A80` | `0x00770A80` | object 584 B; `Flame/Flame` |
| `CGOFlame` update segment | `0x00B711C0` | `0x007711C0` | tăng `flame[i+51]`, so `sub_B333D0` |
| `CGOFlame` per-frame anim | `0x00B713D0` | `0x007713D0` | kiểm object tại ô trước khi vẽ |
| `GetAnimCount(objNo)` | `0x00BA3FF0` | `0x007A3FF0` | trả số animation của template |
| Đăng ký tên hàm stage | `0x01261F50` | `0x00E61F50` | IDA không tạo function; 44 tên + id |

## Giới hạn của lượt này

- **Handler gói nổ S→C của KR chưa định vị**; ranh giới client/server mới ghi theo cấu trúc gói VN
  (`vn_runtime`, R-026) + consumer client KR đã thấy.
- **Damage window chưa định vị**: chưa đọc consumer của message id 27 và chưa tìm chỗ kiểm bomber
  trong ô lửa; chỉ chứng minh **damage được áp theo sự kiện ô**, tách khỏi timer animation.
- **Không** đọc nội dung hai set lớp chặn `map+7460`/`map+7508`, bảng cờ xuyên `*(context[43])+32..35`,
  và thân `sub_149B5E0`/`sub_14708A0` (hướng/điểm bắt đầu dây chuyền).
- Hằng `2500` đã thấy ở `sub_E15810`/`sub_E15840` (ngưỡng so `sub_E131A0(stage)`) và nhánh chống
  hack `sub_E78350`; **không** phải fuse. Không suy fuse từ emulator.
- `search_procedures`/`list_names` của REA không dùng được (timeout/không khai báo) nên việc tìm tên
  hàm dựa trên `idautils.Names()` qua IDA trực tiếp.
