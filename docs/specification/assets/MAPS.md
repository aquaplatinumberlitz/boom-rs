# Định dạng `.map`, `Attr/*`, frame sprite 48 byte (phân tích tĩnh)

Nhánh `claude/opcode14-crash-20261016`, chỉ đọc tĩnh (không chạy game/Wine).
Nguồn chính: disasm `CGamePlayMap__Load` (VA `0x7354FD–0x736C27`, đối chiếu bằng
`objdump` trên `unpacked_ca.exe`) + parse thực nghiệm 100% file trong `assets/`
bằng `/tmp/opencode/parse_map.py` (không commit script, xem phụ lục cuối).
Khối Themida VM ở `0x736C27` do nhóm khác làm, không đụng tới.

Quy ước độ tin cậy: **cao** = disasm + dữ liệu thực khớp trên toàn bộ 965 file;
**trung bình** = khớp mẫu + suy luận hợp lý; **thấp** = giả thuyết chưa kiểm chứng.

## 1. Cấu trúc file `.map` (đã giải xong, độ tin cậy: cao)

Tổng số: 965 file `assets/fx/Map/*.map` + 54 file `*.i16` trong cùng thư mục mà
thực chất cũng là map (parse cùng parser, EOF-exact). Version chỉ có 2 loại:
`1000` (651 file) và `1001` (314 file).

> **Đối chiếu KR 2026-10-19:** header KR dài hơn VN 4 byte — thêm một dword `0` trước cặp
> `60000, 22000` (vị trí chính xác trong dãy số 0 chưa xác định); count spawn vẫn ở +20, tọa độ spawn bắt đầu **+80** (VN +76). Phần sau spawn
> (rect, lưới, bảng rơi, placement) cùng bố cục. Đã kiểm trên Village00 KR và VN; xem
> [RULES_V1](../gameplay/RULES_V1.md) R-002…R-004. `boom-assets` đọc cả hai theo build.

### 1.1. Thứ tự field (offset tuyệt đối từ đầu file, tất cả `i32` LE)

| Offset | Tên tạm | Nguồn trong loader (VA) | Ghi chú |
|---|---|---|---|
| +0 | `version` | `0x7355B3` đọc `[esi]`, so `>= 1000` ở `0x7355D1` | Chỉ gặp 1000/1001. Nhánh `< 1000` gán mặc định `W=ver,H=1000?` (xem 1.2) nhưng không có file nào thuộc nhánh này |
| +4 | `d1` | `0x7355D8` → obj+`0x8F4824` | Giá trị `{1,2,3,4,8,12}`; tương quan mạnh với họ map (1.4) |
| +8 | `d2` | `0x7355E8` → obj+`0x8F4830` | Giá trị `{1,2,4,8}`; tương quan với họ map (1.4) |
| +12 | `W` (rộng, số cột) | `0x7355F8` → obj+`0x8F4800`; là bound vòng lặp cột ở `0x736967` | Thường 15; lobby 49; xem 1.4. Decomp cũ render nhầm thành byte — disasm `mov eax,[esi]` chứng minh là dword |
| +16 | `H` (cao, số hàng) | `0x735646` → obj+`0x8F480C`; bound vòng lặp hàng ở `0x736981` | Thường 13; map đua dọc 101 (`Antarctic01`, `GolemRun01` 15×101) |
| +20..+52 | khối 8 dword | `0x735656` (`H`→`0x8F483C`, cursor **không** tiến), `0x735664` (`[esi+0x14]`→ setter obj+`0x8F47C8`, rồi `×1000` set lại), `0x735696` → obj+`0x8F47C4`, `0x7356A1` `esi+=0x20` | dword[0] = N1 (số spawn point, trùng `H` field); dword[5] (`[esi+0x14]`) là giá trị duy nhất trong khối được dùng, thường = 0; 60000/22000 hay gặp ở dword[6..7] (574 file `(1,0,0,0,0,60000,22000)`), nghi là giới hạn thời gian (ms) — trung bình |
| +52..+68 | 4 dword | `0x7356A4–0x7356CA` → obj+`0xA513F0/F4/F8/FC` | 858/965 file `(0,0,0,0)`; nếu game-mode ∈ {16,32} thì `A513F0` phải = 0 nếu không load fail (`0x7356EC`) |
| +68..+76 | skip 8 byte | `0x735714` `esi+=8`, không đọc | 377 file `(0,0)`; còn lại `(nhỏ,0)` hoặc `(0,lớn)` — chưa rõ nghĩa (thấp) |
| +76 | N1 cặp spawn `(x,y)` | `0x73571C–0x735781`: bound = obj+`0x8F483C`, mỗi vòng 2 dword | N1 ∈ {1,2,4,5,6,7,8,12} (8: 474 file, 4: 415 file). **100% tọa độ nằm trong [0,W)×[0,H)** trên cả 965 file → spawn point (cao) |
| tiếp | `C` + C rect 4 dword | `0x735783–0x735815`: `C=[esi]`, mỗi entry 4 dword | **C = 0 ở toàn bộ 965 file.** Code không giới hạn C theo bảng 20 entry (tràn stack nếu C lớn — quan sát, chưa kiểm động) |
| tiếp | lưới pre-cell W×H×(2\|3) dword | `0x735842–0x735896`: skip `W*H*12` (v1001) hoặc `W*H*8` (v1000); 0x89D56E KHÔNG tiêu thụ khối này (cursor của nó bắt đầu sau khối) | **Đây chính là dữ liệu lưới chính** (xem 1.3: hai-cursor). Mỗi cell = `(tileId, objId[, overlayId])`, `-1` = trống |
| tiếp | drop-table | `0x89D56E` phase A (`0x89D62B–0x89D6BA`): `L1=[cursor]`, mỗi entry `u32 len + len byte + u32 weight` | Tên item + trọng số rơi đồ (0–8650). L1 = 0 ở 96 file |
| tiếp | placement counts + placements | phase B (`0x89D6C0–0x89D799`): `L2=[cursor]` (chỉ reserve qua `0x8C2AB2`), `L3=[cursor]`, L3 entry `(u32 len, tên, u8 x, u8 y, u8 prob%)` | L2 ∈ {60,70,100} (100 ở 940 file, nghi là cơ số % — trung bình). `prob` theo item (Shoe/Shark 100, MaxPower 10, Fire/Bomb/Roller 30). Tọa độ trong lưới |
| tiếp | script1 `(L + bytes)` + script2 `(N + bytes)` | L ở `0x73593D`, parse TiXml một blob duy nhất | **L = 0 ở 938/938 file hợp lệ** → nhánh XML của loader luôn inert. script2 (N>0 ở 561 file) = CRLF padding + `<mapscript>` XML (StageClearScript rỗng, LobbySquareScript, RabbitCatchScript…) nhưng **loader không đọc tới** (không còn lệnh đọc file nào sau lưới — cao). 27 file Bula/Event có đuôi khác (1.5) |
| — | lưới chính (đọc bằng ESI) | vòng y/x `0x735BEB–0x736981`: mỗi cell `[esi]` tileId (`-1` = ô trống → obj rỗng `0xEC` qua `0x5C0095`), `[esi+4]` objId (`-1` = không có), v1001 thêm dword overlay (`0x736853`, `-1` = không) | tileId/objId là id tra cứu qua bảng runtime (`0x5C5965` + `0x97798C` + `0x72E942`), dispatch theo tag type 1–4 (`0x736108`) tạo obj `0xEC/0xF4/0xE0`-byte. Tile 40px (`idiv 0x28` ở `0x735DA9`), stride lưới cố định 101 (`imul 0x65`, lưới con trỏ ở obj+`0x13C4`, lưới obj ở +`0xB328`) |

### 1.2. Khác biệt version 1000 / 1001 (cao)

1. Pre-cell và lưới chính: **2 dword/cell (1000) vs 3 dword/cell (1001)** —
   disasm `0x73581E–0x735896` (`imul 12` vs `lea [esi+eax*8]`) và overlay
   per-cell chỉ đọc khi `version==1001` (`0x736859–0x736967`, `je` bỏ qua khi 1000).
   Overlay gần như không dùng: trên toàn bộ file 1001 chỉ 8 cell ≠ −1 (id 2429/2434/2435).
2. File `< 1000` (không tồn tại trong assets): chỉ đọc 1 dword version rồi gán
   `0x8F4800=ver, version=1000, 0x8F4824=12, 0x8F4830=1` (`0x735605–0x73563F`).

### 1.3. Mô hình hai cursor (cao, load-bearing)

- Register `ESI` = cursor lưới: sau khi đọc rect (`0x7357F5`, ESI = đầu pre-cell)
  không còn lệnh ghi ESI nào cho tới vòng y/x (quét `0x735783–0x735C6B` chỉ thấy
  `add esi` ở `0x735767/0x73578D/0x7357F5`; các `call` ở giữa giữ ESI theo
  callee-saved). Vòng y/x đọc lại đúng khối pre-cell (triple
  `(2784,2809,−1),(2785,−1,−1)…` ở Aqua01: tileId xen kẽ 2 loại, objId xen kẽ id/`−1`).
- `[ebp−0x314]` = cursor meta (truyền `&` cho `0x89D56E`, hàm này `mov [esi],eax`
  sau mỗi lần đọc: `0x89D63F/0x89D64C/0x89D696/0x89D6C7/…`).
- Kiểm chứng: parser hai-cursor parse **965/965 file EOF-exact** (với script1+script2,
  trừ 27 file Bula, xem 1.5).

### 1.4. Trường `d1/d2` (~trung bình: tương quan rõ, nghĩa chưa rõ)

`(v,d1,d2)` → họ map: `(1000,8,1)` battle thường (Ocean/Factory/Mine/Tomb);
`(1000,1,4)` quái/RPG; `(1001,8,1)` lobby/NXCL; `(1001,1,4)` quái tuyết;
`(1001,2,4)` Aqua/CAhero; `(1000,4,2)` Rabbit/Bula; `d2=8` chỉ ở MonBig/MonHorde;
`(1001,12,1)` IamBoss/ConfusedFight. Đoán là (nhóm tileset, chế độ) — cần kiểm động.

### 1.5. Đuôi 27 file Bula/Event (trung bình–thấp)

Sau placements: mảng tọa độ int-nhỏ (theo cặp/quad,疑似 waypoint/area) + danh sách
0-terminated mỗi entry `(u32 len=10?, 3 dword magic A/B/C, 3 byte)` (magic
`A∈{1702131023,1701736009}, B=1970230130`) + chunk script. Loader không tiêu thụ
(không lệnh đọc file sau lưới). Consumer chưa rõ — cần tìm loader khác hoặc kiểm động.

## 2. Cấu trúc `Attr/*` (cao cho khung, trung bình cho nghĩa field số)

- `BomberAttr.bin` (2501 B): **41 record `(u32 len=13, char[13], 11×i32)`, EOF-exact.**
  Tên `Character/C00–C18, N01–N19, Z01, B01`; f0 = index 0..42 (trừ 19, 40);
  f8 ∈ {9,10}, f9 = 100 hằng, còn lại 1..10 (nghi là chỉ số bomber: bom/lửa/tốc… — thấp).
- `ItemAttr.bin` (20140 B), `u32 total=60`, gồm ≥3 section:
  - §1: 16 record ride `(0, b, seq 1..16, u32 L, tên, 6×i32)` (`Ship/Airship…`;
    b=16 ở record đầu, còn lại 0; payload `(1|2, …)`疑似 loại/khả năng).
  - §2: ≥12 record shop `(0, 0, tên item, tên sprite, 6×i32 = (giá,0,1,seq,0,0))`
    (Bomb 3650, Fire 2800, Roller 2000, Shoe/Punch/MaxPower 150–300…). f0 = giá — trung bình.
  - §3+ (từ offset 2127, ~32 record event/flag: `GoldFlag, RuinsSlate…`): khung
    entry chưa chốt (thấy `(0,0,id,LB,zeros, L1,tên,L2,tên2, payload)` nhưng LB=28
    không khớp blob 16 byte) — thấp, còn mở.
- `MapGroup.bin` (663 B, EOF-exact): `(u32 ngroups=3, mỗi group (u32 L, tên, u32 N,
  N×(u32 ML, tên map)))`. Groups: `PraticeMapGroup` (45 map), `LadderMapGroup` (6),
  `ReBirthMapGroup` (2). (Lưu ý: byte `2D` sau tên group0 là byte thấp của N=45,
  không phải dấu `-` trong tên.)

## 3. Frame animation 48 byte của sprite `CDataI16Alpha` v2 (cao cho 8 field đầu)

Layout blob (khớp `tools/i16.py`): `u32 nFrames, =2, nImages, unk;
nImages×(u32 h, u32 w); nFrames×48B; nImages×(u32 len, RLE)`.
Mỗi frame = **12×i32**, validate trên 168.195 frame thật (fx/charte/emote):

| Field | Nghĩa (suy đoán) | Bằng chứng | Tin cậy |
|---|---|---|---|
| f0, f1 | offset đăng ký khung hình (registration; âm được) | RedZone hằng (0,39); Bomb dao động (0,40)→(2,30)→(−1,40) theo nhịp nổ; Dlg jitter ±2 (canh tay) | trung bình |
| f2 | flag/lớp (−1 phổ biến 82.5%, còn lại 0,4–7,13,46…) | 998 frame có f3 vượt `nimg` hầu hết đi với f2≠−1 → f2≠−1 có thể đổi nghĩa f3 | thấp |
| f3 | index ảnh trong blob (0..nimg−1) | RedZone hằng 0; Dlg chuyển 0→1 đúng khi kích thước cell đổi theo `sizes[]`; cell đúng khi `f6−f4 ≤ w, f7−f5 ≤ h` | cao |
| f4–f7 | source rect `(x0,y0,x1,y1)` trong ảnh | RedZone: `(0,0,40,40),(0,40,40,80),(0,80,40,120)` trên ảnh 160×40; Dlg: `(0,0,227,255),(227,0,452,255)` = nửa trái/phải ảnh 452×255. Chỉ 3/168k rect tràn, 68 src âm (cắt lề?) | cao |
| f8, f9 | kích thước canvas (w,h) | (40,40),(280,292),(118,118),(66,76/100/73): luôn ≥ cell src | trung bình |
| f10, f11 | pivot (px,py), hay nằm đáy (`py≈h−1`: (0,39),(13,75),(13,99),(39,117)) | RedZone (0,39) góc trái-đáy; Dlg (0,0) góc trái-đỉnh hợp với ảnh nền dialog | trung bình |
| — | **không** phải thời gian: f8–f11 hằng số theo sprite (trừ nhóm charte 3-canvas,疑似 3 anim gộp) | quét `set()` theo cột trên từng blob | trung bình |

## 4. Câu hỏi còn mở + đề xuất kiểm chứng động

1. Nghĩa `d1/d2` (+4/+8), `skip8` (+68), `block7.b0` (1/7/500…), `A513F0..FC`,
   `L2` (60/70/100), `f2` frame, pivot/canvas: tìm consumer — `d1/d2`/`A513Fx`
   được đọc ở những hàm nào (xref `0x8F4824/0x8F4830/0xA513F0`), render dùng
   f8–f11 ra sao (`CDataI16Alpha` render path từ `0xA3D850`).
2. Đuôi 27 file Bula/Event + script2 XML: ai đọc? (trong `CGamePlayMap__Load`
   không có lệnh đọc). Nghi loader mapscript/event riêng — tìm xref chuỗi
   `mapscript`, `aMapscript` (`0xC1DCB0`) và hàm `0x73022F/0x7353AF/0x73395A`.
3. `C=rect` luôn 0: dựng file test C=1 xem `0x5BFE80` làm gì (cần fakeserver/Wine —
   thuộc nhóm động, ngoài scope task này).
4. Overlay v1001 (8 cell/965 file): nội dung thực khi render (có thể là lớp hiệu ứng).
5. Giới hạn bảng rect 20 entry trên stack (`[ebp−0xB0]`, vòng lặp không kẹp C):
   xác nhận crash với C>20 bằng bản build test (nhóm động).

## Phụ lục: cách tái hiện (chỉ đọc)

- Disasm loader: `objdump -d --start-address=0x7354fd --stop-address=0x736c27
  -M intel unpacked_ca.exe` (base `0x400000`, `.text` RVA `0x1000`).
- Điểm neo trong disasm: header `0x7355B3–0x735641`; khối 8 dword `0x735656–0x7356A1`;
  spawn `0x735749–0x735781`; rect `0x735783–0x735815`; skip pre-cell theo version
  `0x73581E–0x735896`; `0x89D56E` phase A/B (`0x89D62B/0x89D6C0`);
  L+XML `0x73593D–0x735ABC`; lưới y/x `0x735BEB–0x736981`
  (tile `0x735C6B`, obj+switch tag `0x736108`, overlay v1001 `0x736859`);
  hậu xử lý rect `0x736A16` (`imul 0x65`, `0x5BFE80`); VM `0x736C27` (nhóm khác).
- Parser đã dùng: `/tmp/opencode/parse_map.py` (ngoài repo, không commit) cho kết quả
  965/965 EOF-exact; frame survey qua `tools/idf.py::iter_pack` + `struct <12i`.
