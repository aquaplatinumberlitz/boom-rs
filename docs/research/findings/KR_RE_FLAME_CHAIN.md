# RE tĩnh KR 00967 — nổ lan, vật cản, dây chuyền và thời gian lửa

Kết quả đọc tĩnh client KR `00967` cho [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148) /
Q-019 (mục 5 hàng đợi RE: propagation/chain và thời gian gây damage của lửa).
Claim trong [RULES_V1 §17](../../specification/gameplay/RULES_V1.md#17-re-kr-00967--nổ-lan-vật-cản-và-dây-chuyền-r-139r-141)
(R-139…R-141); metadata và `evidence_id` ở
[evidence KR static flame-chain](../../evidence/measurements/kr-static-flame-chain-2026-10-22/README.md).

## 1. Phạm vi, build và phương pháp

| Mục | Giá trị |
|---|---|
| Build | KR `00967`, module đã unpack `kr-full.exe`, 26.074.624 B |
| SHA-256 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` |
| Image base | `0x00400000`; `RVA = VA − 0x00400000` |
| Kiến trúc | x86 (32-bit), vtable 4 byte/slot, `__thiscall` |
| Đường chính | REA MCP `provider_id: "ida"` trên **bản sao** binary; `procedure_pseudo_code` + `xrefs` |
| Fallback | IDA MCP trực tiếp trên **bản sao** `.i64` cho `read_bytes`/`search`/`xrefs`/`decompile` (REA adapter thiếu `read_bytes`, `search_bytes`, `list_names`; `search_procedures` quá lớn nên timeout — ghi ở evidence) |
| Không chạy | không chạy client, không kết nối server, không mở DB gốc, không sửa binary |

Đối chiếu VN dùng notebook `/home/admin/boom-re/docs/oc_e-game.md` §5a (build `VBNB_116`), chỉ để
lấy **cấu trúc gói** và thứ tự hành vi; mọi kết luận dưới đây ghi nguồn `kr_static` trừ khi nói rõ.

## 2. Điểm vào: bảng đăng ký tên hàm của stage

Trong `.priv` có một hàm đăng ký tên (`0x01261F50`, IDA không tự tạo function; phải `add_func`) gán
**id tăng dần** cho tên hàm (`sub_A02B60(name,&id)` rồi `sub_455630`). Chuỗi tên cho biết các hàm
tồn tại và đúng chủ đề:

| id | Tên đăng ký | Ý nghĩa dùng trong bài |
|---|---|---|
| 2 | `kCPlayMap_ExplodeObjectByFlameExploded` | áp vụ nổ lên ô/vật thể |
| 12 | `kCPlayMap_IsBombExplodeByGround` | kiểm ô đang nổ |
| 14 | `kCGOBomb_IsExplodePos` | kiểm ô thuộc vùng nổ |
| 20 | `kCBombOwner_GetIgnitedBombCount` | đếm bom bị kích |
| 21 | `kCPlayMap_CalcBombDirectionExplosion` | **tính ô nổ theo hướng** |
| 24 | `kCGameStage_OnSetBombState` | đặt state bom |
| 32 | `kCPlayMap_CalcChainExplosion_Direction` | **tính ô nổ dây chuyền** |
| 35 | `kCBombOwner_GetIgnitedBombCount_ForCount` | đếm bom bị kích (biến thể) |

Bảng này **chỉ đăng ký tên + id** (không phải bảng con trỏ hàm), nên không dùng id để gọi. Nó xác
nhận *có* hai hàm calc riêng cho nổ theo hướng và nổ dây chuyền; việc định vị thân hàm làm bằng
truy vết consumer (mục 3–5). Đây là bổ sung cho ghi chú R-131 (cùng cơ chế đăng ký tên).

## 3. Hàm tính vùng nổ: `0x00E9C520` = `CPlayMap::CalcBombDirectionExplosion`

Chữ ký đọc được: `int __thiscall sub_E9C520(char *map, int bomb)`; thân `0x89C` byte.

### 3.1. Đầu vào lấy từ chính quả bom (virtual slot)

| slot (byte) | Giá trị dùng | Ý nghĩa suy ra từ cách dùng |
|---|---|---|
| `+12` (slot 3) | `v43` | toạ độ **hàng** của bom |
| `+16` (slot 4) | `v42` | toạ độ **cột** của bom |
| `+20` (slot 5) | `v37` | **tầm nổ** (số ô) |
| `+24` (slot 6) | `v27` | **kiểu nổ**: `1` = theo 4 hướng, `2` = vuông đầy |

Ô của bom được đăng ký trước (`sub_ECB820` + `sub_EBD3F0(bomb, cellIndex)`), rồi:

- **`v27 == 1`** — vòng 4 hướng × `j = 1..v37`:
  - ô = `(hàng + tbl[2i+1]·j, cột + tbl[2i]·j)`, bảng `dword_17929B0` tại `0x017929B0`
    = `[-1,0, 0,-1, 1,0, 0,1]` (8 dword). Ô được truy vấn theo thứ tự `(hàng, cột)` (`0x00E9F9B0`
    và `0x00EB7190`; chỉ số tuyến tính = `cột + hàng·số_cột`), nên `[2i]` là delta **cột** và
    `[2i+1]` là delta **hàng** ⇒ `i = 0` trái, `1` lên, `2` phải, `3` xuống — **khớp** R-120.
  - ô ra ngoài map (`sub_EA5D90(hàng,cột)==0`) ⇒ `break`.
  - ô trong map ⇒ `cellIndex = sub_EB7190` (chỉ số tuyến tính `cột + hàng·số_cột`) và nếu hợp lệ
    thì **đăng ký ô vào quả bom** (`sub_EBD3F0`) — đăng ký **trước** mọi kiểm chặn.
  - sau khi đăng ký mới kiểm chặn, theo thứ tự:
    1. `sub_EA1230(hàng,cột) != 0` ⇒ `break`;
    2. `sub_EA0B10(hàng,cột) != 0` ⇒ `break`;
    3. `sub_E9F9B0(hàng,cột)` trả object tại ô; nếu `sub_E90A80(object) != 0` (object không phải
       tile thường) **và** `sub_BA0F80(hướng) == 0` ⇒ `break`;
    4. kiểm ô kế tiếp (`j+1`): nếu trong map và object tại đó thoả `sub_E90A80 != 0` **và**
       `sub_BA0EC0(hướng) == 0` ⇒ `break`. (`sub_BA0EC0(d)` đọc cờ của hướng **đối diện**:
       `0↔2`, `1↔3`.)
  - `sub_BA0F80/BA0EC0` đọc bảng 4 byte tại `*(context[43]) + 32..35` trong `sub_BA0E30`; đây là **cờ theo
    hướng** dùng làm điều kiện cho object tại ô; nghĩa "cho xuyên" chỉ là **giả thuyết** (cần instance
    runtime để lấy giá trị và cần đọc nơi nạp bảng).
- **`v27 == 2`** — **vùng vuông đầy**: hàng `k = v43−tầm .. v43+tầm`, cột `m = v42−tầm .. v42+tầm`,
  mọi ô trong map đều được đăng ký (không kiểm chặn trong nhánh này).
- Khối thứ ba (khi `sub_B9E4C0(object tại ô bom) != 0`) quét **toàn map**, đăng ký ô hiện tại và
  các ô **kề 1 bước** quanh những ô thoả điều kiện — nhánh này chưa xác định nghĩa (xem §7).

### 3.2. Vì sao "đăng ký trước rồi mới chặn" là đúng

Ô chặn (tường/gạch) **nằm trong** tập ô nổ rồi vòng lặp mới dừng: tia không xuyên qua ô chặn và
không đi tiếp. Điều này khớp việc client phá gạch tại ô đó (`sub_E9E3E0`, C→S 135 ở bản VN) và việc
sprite lửa trên ô có gạch bị ẩn cho tới khi gạch vỡ (`CGOFlame` kiểm object tại ô ở `0x00B713D0`).
Không có nhánh nào cho tia đi xuyên ô chặn.

### 3.3. Polarity object/tile (đọc thêm để tránh nhầm)

- `sub_E9FCF0(hàng,cột)` = object tại ô nếu là **gạch vỡ được**: dynamic_cast `CGOTileFixed` /
  `CGOTileFixedBox`.
- `sub_E9FA90(hàng,cột)` = object tại ô nếu là **tường tĩnh**: `CGOTileFixed` / `CGOTileStatic`.
- `sub_E90A80(object)` = `*(*(object+43)+44)` — cờ lớp của object (≠ 0 là "không phải tile thường").
- `sub_EA1230`/`sub_EA0B10` tra object ô trong hai **set lớp** của map tại `map+7460` và `map+7508`
  (tập được nạp lúc nạp map; chưa đọc nội dung tập — xem §7).

## 4. Hàm dây chuyền: `0x00E9CDC0` = `CPlayMap::CalcChainExplosion_Direction`

`int __thiscall sub_E9CDC0(char *map, int row, int col)`, thân `0x33D`.

- Nếu ô ngoài map ⇒ thoát. Ngược lại lấy danh sách object tại ô (iterator `sub_ED8DE0`/`sub_EDA940`,
  duyệt `sub_ED0E20`/`sub_ED1AF0`, lấy object `sub_ED17A0`).
- Với **từng object** `v32` tại ô:
  - `v12 = vf+20` (tầm nổ), `v15 = vf+24` (**kiểu**); chỉ xử lý khi `v15 == 1`.
  - `v29 = sub_149B5E0(&điểm, …)` — lấy **hướng**; `v13 = sub_14708A0()` — lấy **chỉ số bắt đầu**;
    vòng `j = v13+1 .. v12` tính ô theo cùng bảng `0x017929B0` từ **điểm** `(v6,v7)`.
  - mỗi ô trong map được đăng ký **vào chính object `v32`** (`sub_EBD3F0(v32, cellIndex)`), rồi kiểm
    chặn y hệt §3.1 (`EA1230`, `EA0B10`, object + `BA0F80/BA0EC0`, ô kế tiếp).
- **Hệ quả:** khi lửa chạm ô có bom khác, client tự tính **phần vùng nổ còn lại của bom đó** (theo
  hướng/quãng của nó) và đăng ký vào bom đó; không cần server gửi danh sách ô. Đây là nhánh dây
  chuyền phía client.

Caller đã thấy (xref trực tiếp): `sub_E84650` (2 site — hàm áp descriptor nổ/di chuyển object),
`sub_EA5280`, `sub_EA56C0` (di chuyển object giữa 2 ô), `sub_B92E30`, `sub_B9CEC0`, `sub_B9FDD0`
(cụm bomber/pet), `sub_6D60A0`.

## 5. Đăng ký ô và "thời gian" — tách animation khỏi damage

### 5.1. Ô nổ được đăng ký thành **message** (không có biến đếm ms trong hàm calc)

`sub_EBD3F0(map, object, cellIndex)` dựng message `{object, id = 27, cellData, map+6468}` rồi gọi
`sub_EDB7E0` → `sub_EC2420(object, message)`: kiểm sức chứa (`sub_EDB280`) rồi **push vào danh sách
message của chính object**. Tức mỗi ô nổ là một sự kiện có id 27 gửi cho quả bom; việc tiêu thụ
message (biến ô thành lửa/áp damage) nằm ở consumer của object, **không** phải một biến đếm ms trong
hàm calc.

### 5.2. Thời gian sống **hiển thị** của lửa = số frame animation

- `CGOFlame` (RTTI/vtable `0x01745EF0`, ctor `0x00B70A80`, object 584 B = 0x248) giữ **nhiều segment**
  (`flame[141]` = số segment; mỗi segment có object animation `flame[i+36]` và bộ đếm `flame[i+51]`).
- `sub_B711C0` (update, gọi mỗi bước): `++flame[i+51]`; nếu `flame[i+51] >= sub_B333D0(flame[i+36])`
  thì xoá segment (dồn mảng) và giảm `flame[141]`.
- `sub_B333D0(this, objNo)` trả `sub_BA3FF0(objNo)` = `CGameObjectTemplate::GetAnimCount(objNo)`
  (số animation/frame của template đối tượng — cùng hàm đã ghi ở R-079/Q-019).
- Vậy **thời gian sống hiển thị = số frame × số lần update**, không phải hằng ms; quy đổi 30 ms/bước
  (R-013) chỉ là quy đổi, chưa đo nhịp update trên KR.

### 5.3. Damage TTL: chưa tìm thấy, và không lấy 2500 ms của emulator

- Trong đường đã đọc **không** có hằng ms nào cho "vùng lửa gây damage". Mỗi ô được đẩy thành message
  id 27 vào list message của object, nhưng **consumer (áp damage) chưa trace** — nên mô hình damage
  (một lần hay cửa sổ thời gian) là **candidate/unknown**, không suy từ đoạn này.
- Hằng `2500` trong binary xuất hiện ở `sub_E15810`/`sub_E15840`
  (`stage[490607] != 0 && sub_E131A0(stage) <= 0x9C4`) và ở nhánh chống hack `sub_E78350`
  ("Detected hacking…" + `sub_E529C60(stage+6924) > 2500`), **không** phải fuse của bom. Không thấy
  biến đếm fuse trong `CGOBomb`/`CGOGameBomb` update (`0x00B391E0` chỉ lo bật/nảy và state 1/2/3).
- `CGOBomb::Init` từ descriptor (`0x00B384E0`) nhận `desc+168/172` (u64) → bomb `[+98]/[+99]`,
  `desc+96` → slot `+104` (nghĩa chưa xác định, **không** phải ô), `desc+156` → `sub_B3BA90`; nghĩa
  `desc+168` **chưa xác định** (có thể là mốc thời gian/fuse do server gửi) — **không** suy ra 2500.
- Kết luận có phạm vi: thời gian sống hiển thị của lửa là **animation** (số frame), **không** phải một
  damage timer đã biết; giá trị damage window của KR **unknown**.

## 6. Ranh giới client/server (bắt buộc ghi)

- Gói nổ của bản VN (`S→C 151`) chỉ mang **id/kind/arg của bom**, không mang danh sách ô
  (`vn_runtime`; `oc_e-game` §5a-5b và F5 ở [VN_FEATURES_REFERENCE](VN_FEATURES_REFERENCE.md)).
  Tầm nổ khởi đầu 1 và +1 mỗi ItemFire là R-025 (`vn_runtime`/confirmed), khớp nghĩa "tầm = số ô"
  của slot `+20`. Vì vậy **hình học vùng nổ do client tính**; server quyết *bom nào nổ và lúc nào*.
- KR: hai hàm calc ô nổ được gọi từ **lớp bom** (`sub_B384E0`, `sub_B38A80`, `sub_B393D0`,
  `sub_B39AF0`, `sub_B3A3B0`, `sub_B3A790`, `sub_B76CD0`, `sub_B994C0`, `sub_B9AFA0`, `sub_E14360`,
  `sub_F0FD20` — xref trực tiếp tới `0x00E9C520`) và từ đường áp descriptor (`sub_E84650`). Điều này
  chứng minh **consumer phía client**, nhưng **handler gói nổ S→C của KR chưa định vị** trong lượt
  này (không suy opcode từ VN).
- Không kết luận gì về việc server KR tính lại vùng nổ; chỉ ghi rằng client có code tính và dùng nó.

## 7. Giới hạn và bước tiếp

1. **Handler gói nổ KR** (tương ứng S→C 151 VN) chưa định vị. Bước tiếp: tìm case dispatch trong
   `CGameStage` theo opcode, hoặc truy từ `sub_E9EEE0`/`sub_E9E660` ngược lên caller; nếu vào
   `.themida` thì ghi entry VM và dừng.
2. **Damage window**: chưa định vị consumer của message id 27 và chỗ kiểm bomber trong ô lửa (set
   state nhốt). Bước tiếp: đọc handler message id 27 trên `CGOBomb`/effect, rồi tìm hàm kiểm
   bomber-vs-ô-lửa; nếu không có thì ghi rõ damage là event-time (một lần) và animation chỉ là hình.
3. **Nghĩa nhánh thứ ba** của `E9C520` (`sub_B9E4C0 != 0`, quét toàn map, đăng ký ô kề) và nhánh
   "ô kế tiếp" (`BA0EC0`) chưa xác định — cần đọc thêm kiểu bom tương ứng.
4. **`sub_149B5E0` / `sub_14708A0`** (hướng + chỉ số bắt đầu của dây chuyền) chưa đọc thân: kết luận
   dây chuyền hiện `confirmed` ở mức "client tính và đăng ký ô", phần công thức hướng/điểm bắt đầu là
   **candidate**.
5. **Set lớp chặn** `map+7460` / `map+7508` và bảng cờ xuyên `*(context[43])+32..35` chưa đọc nội
   dung: chưa nói được "tường hay gạch chặn" theo từng lớp; mới chứng minh **cơ chế** (dừng sau ô
   chặn, ô chặn vẫn thuộc vùng nổ) và **có** cờ xuyên theo hướng của loại bom.
6. **Nhãn hướng 0/1/2/3**: đã đối chiếu — khớp R-120 (`0` trái, `1` lên, `2` phải, `3` xuống);
   slot `+12` = hàng, `+16` = cột; chỉ số ô = `cột + hàng·số_cột` (row-major). Không còn mục này
   trong danh sách bước tiếp.

## 8. Khác biệt với profile hiện tại và với BnB

| Đại lượng | Profile hiện tại | KR đọc được | Đề xuất |
|---|---|---|---|
| `bomb.flame_ms` | 500 ms (`oss`/design, R-115) | thời gian sống hiển thị = **số frame × update** (`GetAnimCount`), không phải hằng ms; nhịp update KR chưa đo | **Không** đổi trong PR này. Khi có nhịp update KR thì map `frames × tick`; không thay bằng 500 |
| `bomb.fuse_ms` | 2500 ms (`oss`/design, R-115) | không thấy fuse trong client; nổ do đường nhận/descriptor; `desc+168` chưa rõ nghĩa | Giữ design; **không** lấy 2500 của emulator làm fuse gốc |
| `player.trap_to_death_ms` | 4000 ms (`oss`/design) | không đổi (ngoài phạm vi lượt này) | Giữ nguyên |
| Hình học vùng nổ | chưa có claim | tia 4 hướng, tầm = slot +20, dừng **sau** ô chặn, ô chặn thuộc vùng; kiểu 2 = vuông đầy | Cần claim riêng khi làm server; chưa sửa data/rules |

Không có khác biệt nào buộc đổi `data/rules/*.toml` ngay; mọi thay đổi giá trị phải qua PR profile
riêng theo ADR 0013 sau review.

## 9. Lượt 2026-10-24 (phiên 2) — message 27: chưa thấy dispatch/damage trong các thân đã đọc

Phạm vi hẹp theo #148/Q-019: tìm **một** consumer nhận/dispatch message id 27 trên `CGOBomb` và chỗ
kiểm bomber-vs-ô-lửa. Ngân sách: 5 call phân tích, 3 thân mới (`ED28C0`, `ED6FD0`, `EC1130`);
`B391E0`/`EC2420` đã có từ lượt trước. Build KR `00967` SHA-256
`cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b`, imagebase `0x00400000`.
REA binding IDA trên bản sao; evidence: `ev_95b4134c…`, `ev_5cbb5977…`, `ev_2a48a8f2…`,
`ev_d6a164ca…`, `ev_a6953bcb…` (VA/RVA ở
[evidence](../../evidence/measurements/kr-static-flame-chain-2026-10-22/README.md)).

### 9.1. Dữ liệu dương (giữ nguyên)

- **`sub_EC2420(queue, object, msg)` là push của một hàng đợi**: kiểm sức chứa `sub_EDB280(queue)`,
  đếm ở `queue+4` (`*(queue+1)`), copy message `sub_ECB750(msg)`, thêm vào danh sách
  `sub_EBF450(sub_ED5270(queue), copy)`, tăng đếm, rồi `sub_ED7ED0(object)` + `sub_ECF590(...)`.
  **Trong thân push không có giá trị damage, không có ms/TTL** — mỗi ô nổ vẫn là một sự kiện message
  (id 27) được xếp hàng, khớp R-141.
- **`sub_EC1130(this, a2, a3)`** (một trong 5 caller `ED5270`, mới đọc): lock `sub_4481D0` →
  `sub_ED5270(this)` → `sub_ECC410(queue)`; `sub_ECBD00(&a2)` → `sub_EC01E0(*v4, a3)` →
  `sub_EC05F0(this)` → unlock `sub_44F190` → `sub_ECF6A0`. **Không thấy** đọc/so message id 27 hay
  vòng lặp dispatch trong thân; các callee `ECC410/ECBD00/EC01E0/EC05F0/ECF6A0` **chưa đọc** ⇒
  EC1130 nhiều khả năng là **quản lý/clear+notify** (`candidate`), **không** chứng minh là consumer.

### 9.2. Âm tính có phạm vi (không nâng thành "không áp damage")

- **`CGOBomb` update `0x00B391E0`** (vtable `0x174426C` slot 24): **chưa thấy** lời gọi trực tiếp
  `ED5270/EBF450/EC2420/ECF590` hay thao tác đọc/so id 27/damage trong **thân đã đọc**; thân chỉ
  bật/nảy + `switch` state 0/1/2/3. Tuy nhiên update **vẫn gọi các slot `+176/+164/+168/+172`
  chưa đọc**, nên **không loại trừ** đường gián tiếp dispatch/damage qua chính update.
- Helper `ED28C0` (đăng ký global `word_16EC4CE`, chèn `44F190`), `ED6FD0` (xoá `44EEC0` +
  `EC54A0/EC5550`), `EDACC0` (chỉ `EBD450`/`EBD580`) đã đọc thân: **không** thấy vòng/so message 27
  trong thân, nhưng **callee của chúng chưa đọc hết** ⇒ chỉ là "chưa thấy ở thân này", không phải
  loại trừ toàn bộ helper.

### 9.3. Hệ quả và blocker

- **Consumer message id 27 vẫn `unknown`**: chưa thấy trong các thân đã đọc; **không** kết luận
  "damage không áp trong update", "consumer không nằm trong update/helper", hay "phải ở pump ngoài
  object". Nó có thể ở callee/virtual chưa đọc, helper/pump, hoặc đường gián tiếp qua update.
- Vì vậy damage **một lần khi nhận** hay **vùng có cửa sổ tồn tại**, value/unit/cadence: vẫn
  **unknown**. Không nâng pointer/hàm push thành xác nhận damage.
- Đầu mối phiên 1 **chưa đọc** (chỉ candidate, không dùng trong kết luận): `EB73D0` (site `EB75C1`
  ghi field `+4` rồi `B70C10`) và `EB8000` (site `EB8FA8`/`EB910E` đưa `+268` vào `B70C10`, lửa/effect).
- **Không** đổi `bomb.flame_ms`/`bomb.fuse_ms`; không lấy `B711C0` (animation) làm TTL damage, không
  lấy `2500`/`500` làm gốc.
