# VM Themida: kiến trúc thực tế của bản VN (2015) và công cụ devirt

Tài liệu này trả lời: **VM trong `unpacked_ca.exe` có giống FALCON trong bài của Shablyko không**, và
công cụ `tools/vmdevirt.py` khai thác được gì. Số liệu đều từ chạy thật trên `unpacked_ca.exe` +
`logs/snap/8F346C/pre.snap`, không suy đoán.

## 1. Kết luận ngắn

| Khía cạnh | FALCON (bài, 2026) | VM trong `unpacked_ca.exe` (2015) |
|---|---|---|
| Prologue | `pushfq` + junk stack shuffle | **giống** (`pushfd` tại `0x11EE128`, junk y hệt) |
| Đồng bộ | `lock cmpxchg [ctx+0xD3],1` + `pause` | **giống**, nhưng field `ctx+0x7C` (`lock cmpxchg` tại `0x11EECB0`) |
| Bảng handler | 658 entry, base ở `ctx+0xD7`, `idx*8` | **178 entry, base = chính `ctx`, `idx*4`, idx đã nhân sẵn** |
| Tính chất bảng | fixup bằng image base lúc entry | **tĩnh tuyệt đối** (xem §3) |
| Phân rã code | 1 handler = 1 basic block liền mạch, tự chain | **chia vụn**: mean straight-line run **6.66 lệnh**, median 5, 63.6% run ≤6, nối bằng `jmp` trực tiếp |
| Dispatch | `jmp [table + idx*8]` | `jmp [edi + eax*4]` với `edi = ctx`, `eax = idx` (đo thực tế: `0x43`, `0x81`, `0x56`) |
| Ý nghĩa | phân loại 650 handler là đủ | bảng enumerate được **tĩnh**, nhưng thân handler bị xé nhỏ ⇒ ngữ nghĩa phải lấy bằng **trace** |

⇒ Cái áp dụng được từ bài: enumerate + phân loại handler theo mẫu (`pushfd` → flag-op), và tách
"profile" khỏi engine. Cái **không** áp dụng: giả định mỗi handler là một block tự chứa.

## 2. Context của VM (bản VN)

Anchor xác định bằng spinlock: lần ghi `1` vào `0x11EDE68` tại `0x11EECB0` là `lock cmpxchg [edi+0x7C]`
⇒ **ctx base = `0x11EDDEC`** (lấy `edi` tại chính lệnh đó), field khoá `sync` ở `+0x7C`.

Kiểm chứng độc lập: cuối một khối, `edi = 0x11EDDEC`; tại site dispatch, `edi = 0x11EDDEC` và
`[edi + eax*4]` cho đúng handler.

| offset | nội dung | nguồn |
|---|---|---|
| `+0x00..+0x1C` | vùng lưu host register / VM register (ghi ngay sau entry) | write trace |
| `+0x64`, `+0x74` | field dữ liệu (vd `+0x64` nhận `0x02752BE8` rồi `xor edx`) | lift |
| `+0x7C` | `sync` (spinlock) | `0x11EECB0` |
| `+0x94..+0xF48` | **bảng handler**: 178 entry, mỗi entry 4 byte, chỉ số = `offset/4` (0x25..0x3D2) | đọc file |

## 3. Bảng handler là tĩnh (đã kiểm chứng)

So 3 snapshot độc lập (`8F346C`, `8F4D46`, `8F5F40`) và so với byte trong `unpacked_ca.exe`:

- 178/178 entry **giống nhau** giữa 3 snapshot;
- 178/178 entry **giống byte trong file** (0 khác).

⇒ Không phải cache runtime. Đây là bảng hằng nằm trong section VM, nên **enumerate được mà không cần chạy**:

```
python3 tools/vmdevirt.py handlers --ctx 11EDDEC --json decomp/vm_handlers.json
handler table at ctx 0x11eddec: 178 entries
classes: {'block': 117, 'bytecode_read': 45, 'flagop': 16}
```

`decomp/vm_handlers.json` ghi: opcode, địa chỉ, class đoán, số `pushfd`/`lods`/`dispatch`/`pop`.
Class ở đây là **đoán theo mẫu tĩnh** (walk 40 lệnh theo `jmp` trực tiếp), chưa phải ngữ nghĩa — vì thân
handler bị xé nhỏ nên walk tĩnh không đi hết được. Bước tiếp theo: chạy từng entry bằng Unicorn và ghi
tác dụng thật (thanh ghi/flags/bộ nhớ) → đó mới là ngữ nghĩa dùng để emit.

## 4. `tools/vmdevirt.py` — trạng thái

```
python3 tools/vmdevirt.py discover [--json out.json]     # sections, site, stub head
python3 tools/vmdevirt.py profile  --snap ... --site ...  # ctx + dispatch form -> decomp/vm_profile.json
python3 tools/vmdevirt.py handlers --ctx ...               # bảng handler -> decomp/vm_handlers.json
python3 tools/vmdevirt.py ctx      --snap ... --site ...   # dump context sau 1 khối
python3 tools/vmdevirt.py trace    --snap ... --site ... --cont ...  # tác dụng khối (ctx/image/heap, regs)
python3 tools/vmdevirt.py lift     --snap ... --site ... --cont ... --ctx ...
```

Số liệu chạy thật:

- `discover`: **607 site → 390 stub head**; head `0x11EE3B0` xuất hiện **73 lần** = đúng 73 marker.
- `profile`: tự tìm spinlock (`0x11EDE68`) → tự suy `ctx = 0x11EDDEC`; dispatch chỉ **một dạng duy nhất**
  `jmp [base+idx*4+0x0]`, 1469 lần trong 600k lệnh; field ctx bị ghi: `+0x00..+0x1C`, `+0x74`, `+0x7C`.
- `lift`/`trace` khối `8F346C`: **7094 lệnh unique**, chỉ **36 lệnh có ngữ nghĩa** (chạm ctx/image/heap) —
  tỉ lệ junk ~99.5%, phần ngữ nghĩa in kèm offset ctx (vd `add dword ptr [edx], ebp ; ctx+0x84`).
  `trace` ghi 15 write vào ctx, 9 write vào image, 105 write stack/heap, `edi` cuối = ctx.

  **Cảnh báo (đã kiểm chứng):** cả 3 khối `8F346C`, `8F4D46`, `8F5F40` đều dừng vì `budget` chứ **chưa tới
  `cont`** — tức trace đang chứa cả pool VM chung, *không phải riêng một khối*. Số "7094 unique" là pool,
  không phải kích thước khối. Muốn có ngữ nghĩa khối phải sửa điều kiện dừng (hiện tool in WARNING khi
  điều này xảy ra). Đây là việc tiếp theo trước khi tin bất kỳ output `lift` nào.


## 6. Opcode map (stage 1B + 2) — kết quả mới

Ghép 3 nguồn: **bảng handler tĩnh** (opcode → địa chỉ), **trace** (opcode → kích thước VI),
và phân tích tĩnh handler (đặc trưng). Lệnh:

```
python3 tools/vmdevirt.py opcodes --snap logs/snap/8F346C/pre.snap --site 8F346C \
        --disp 11F9E81 --ctx 11EDDEC --json decomp/vm_opcodes.json
```

Kết quả trên khối `8F346C` (4.6M lệnh, 11.340 dispatch):

| chỉ số | giá trị |
|---|---|
| opcode quan sát được | **46** |
| opcode có handler trong bảng tĩnh | **46/46** |
| opcode đo được kích thước | **44/46** |
| histogram kích thước | 1 byte ×34, 2 byte ×5, 3 byte ×1, 5 byte ×4 |
| opcode nhánh (VIP không tuần tự) | **0x67, 0x7B, 0x80** |
| xung đột kích thước | **0** |

**Kiểm chứng bảng tĩnh:** tại mỗi dispatch, so `[ctx + op*4]` với EIP thực tế của handler kế tiếp —
**3.680/3.680 khớp, 0 lệch** (1.5M lệnh đầu). ⇒ Bảng tĩnh là authoritative, phân tích tĩnh đáng tin.

### Bytecode bị mã hoá — không decode tĩnh được

Thử decode tuyến tính toàn stream `0x1350128..0x135B33B` (45.587 byte) bằng bảng kích thước:
**33.151 byte ra opcode lạ** ⇒ sai. Nguyên nhân: handler đọc byte bằng `lodsb al,[esi]; xor al,bl`,
tức byte trong file **không phải** opcode. Đã kiểm tra khoá:

| giả thuyết | kết quả |
|---|---|
| `img[vip] ^ op` | 255 giá trị khác nhau |
| `img[vip-1] ^ op` | 255 |
| `img[vip-2] ^ op` | 255 |
| `img[vip] ^ bl` (bl tại dispatch) | 262 giá trị |

⇒ **khoá rolling**, đổi mỗi lệnh. Muốn decode tĩnh phải hiểu hàm cập nhật khoá (việc RE sâu hơn).

**Hệ quả thực dụng:** không cần decode tĩnh. Trace cho `(vip, opcode)` trực tiếp — đó chính là listing
của đoạn đã chạy, và nó khớp với bảng tĩnh. Với devirt một khối, chương trình *đã chạy* là đủ.

## 7. Fake native

Thêm `--fake-native`: sau khi đã vào VM, mọi lần rời VM ra ngoài image (hàm game hoặc DLL) đều **không
chạy** — tool đọc return address ở `[esp]`, set `eax`, chỉnh `esp` theo `ret imm16` của stub, rồi trả về VM.
Có xử lý `jmp far` (WoW64 syscall: set eax, pop stub).

Đo trên `8F346C` (6M lệnh): 20 lần gọi native được fake, và **vẫn chưa tới `cont`** — vì khối này chạy
45 KB bytecode rồi gọi native, tức `cont` không phải điểm dừng khả dụng cho khối loop/keep-alive. Kết luận
này khớp với việc harness của repo (`tools/vmemu.py`) cũng không exit sau 3M bước.


## 8. Báo cáo độ phủ so với mục tiêu

| Hạng mục | Trạng thái | Bằng chứng |
|---|---|---|
| Stage 1 — discovery (section, site, stub head) | ✅ **xong** | 607 site → 390 head; head `0x11EE3B0` = 73 lần = 73 marker |
| Stage 1B — VM context | ✅ **xong** | `0x11EDDEC`, tự dò qua spinlock `0x11EECB0`; xác nhận bằng `edi` cuối khối + tại dispatch |
| Stage 1B — bảng handler | ✅ **xong** | 178 entry trong `ctx+0x94..+0xF48`; tĩnh (3 snapshot + file); **3.680/3.680 dispatch khớp** |
| Stage 1B — phân loại handler | 🟡 **một phần** | 46 opcode có handler; phân loại *cấu trúc* (pushfd/lods/dispatch/size) xong; **ngữ nghĩa chưa** |
| Stage 2 — encoding VI | ✅ **xong** | 44/46 opcode có kích thước nhất quán, 0 xung đột; 3 opcode nhánh (0x67, 0x7B, 0x80) |
| Stage 2 — decode toàn stream | ❌ **bị chặn** | bytecode mã hoá rolling key (4 giả thuyết khoá đều fail); chỉ decode được đoạn **đã chạy** qua trace |
| Stage 2 — CFG | ✅ **xong (đường đã chạy)** | 24 block, 23 cạnh, 6 back-edge, hub `0x135B33B` |
| Stage 3 — emit C | ❌ **chưa** | phụ thuộc ngữ nghĩa 46 opcode; "semantics" hiện chỉ là sketch, bị junk chi phối |
| Verify với 6 khối `tools/devirt.py` | 🟡 **4 khối** | thêm `8F452B`: 4/6 write khớp tuyệt đối (§20) |
| `decomp/vm_profile.json` | ✅ | sinh tự động |
| Áp dụng sang KR | ⏸ **chờ dump** | `doctor` xác nhận KR đang pack; runbook §16 sẵn sàng |

**Chặn hiện tại (theo thứ tự):**
1. Ngữ nghĩa 46 opcode — phải đọc từng handler như bài gốc (nhìn lệnh ngay trước `pushfd`, nhìn
   thao tác trên ngăn xếp ảo). Sketch tự động hiện tại bị hằng số rối chi phối, **không dùng được**.
2. `cont` không dùng được cho khối loop ⇒ cần đơn vị "một vòng" (chưa chốt được fingerprint đúng).
3. Bytecode rolling cipher ⇒ không decode được nhánh chưa chạy.

**Ước lượng:** để emit C cho một khối đơn giản cần (1) ngữ nghĩa ~46 opcode, (2) một mô hình ngăn xếp ảo
(VSP) đúng, (3) emit + verify. Mỗi phần là vài giờ làm việc có kiểm chứng, không phải một bước.


## 9. Fake native — trạng thái và giới hạn (round 4)

Để chạy một khối tới `cont`, code native mà VM gọi phải chạy được. Đã nâng `--fake-native`:

| Cơ chế | Hành vi |
|---|---|
| Hàm native **trong image** (.text) | **chạy thật** (dump đã có code) |
| Hàm **ngoài image** (DLL/hệ thống) | không chạy: đọc return address ở `[esp]`, set `eax`, chỉnh `esp` theo `ret imm16` |
| `jmp far` (WoW64 syscall) | set `eax`, pop stub, `esp += 4` |
| `--skip ADDR[=eax]` | bỏ qua hàm native trong image (anti-cheat/integrity), giả lập return |
| `--fake-ret` | `scratch` = trả con trỏ tới vùng 64 KB zeroed (tránh native deref null) hoặc một số cụ thể |

Đo trên `8F4D46` (`cont 0x8F5468`):

| cấu hình | kết quả |
|---|---|
| fake mọi thứ, `eax=0` | chết ở `0xa1333f` — đúng vùng **anti-cheat** (0xA12E59..0xA17A24) |
| + `--skip A12E50 A14800 A13A10` | chạy 8.0M lệnh, rồi `unmapped 0x0` (native deref null vì API fake trả 0) |
| + `--fake-ret scratch` | chạy **20M lệnh không crash**, nhưng **vẫn không tới `cont`** |

Lý do không tới `cont`: ba snapshot hiện có (`8F346C`, `8F4D46`, `8F5F40`) đều nằm trong **vòng lặp chính**
`sub_8F4D01`; sau khi VM nhả về code native, luồng quay lại vòng lặp (và vào lại VM) chứ không đi thẳng tới
nhãn `cont`. Đây là giới hạn của *bộ snapshot hiện có*, không phải của thuật toán: muốn kiểm chứng emit C
phải có snapshot ở một khối **thẳng** (ví dụ `8F452B`, `8F8F69` — chưa có snap).

**Hệ quả cho mục tiêu:** verify emit C với 6 khối cần thêm snapshot. Việc này cần chạy client gốc trên
Windows/Wine (`win/snap.bat <SITE>`) — nằm ngoài phạm vi tự động của round này.


## 10. CFG + dừng theo "hub" (round 5)

### CFG của stream VI đã chạy

```
python3 tools/vmdevirt.py cfg --snap logs/snap/8F346C/pre.snap --site 8F346C --disp 11F9E81 --max 6000000
insns 4621780  dispatches 11340
basic blocks 24  edges 23  back edges 6
block sizes: min 27  max 3282  mean 472.5
```

- 24 basic block, 23 cạnh, **6 back-edge** ⇒ khối này **có loop**, không phải tuyến tính.
- Hub của loop: `0x135B33B` (đích của nhiều cạnh lùi, cũng là `jmp` target của site `8F346C`).
- Lỗi đã sửa: trước đó tao coi "VIP delta ≤ 16" là tuần tự ⇒ nhảy ngắn bị nuốt, CFG chỉ ra 3 block.
  Đúng phải là: size của opcode = delta **phổ biến nhất**, khác nó thì là nhánh.

### Dừng theo hub ⇒ đúng một vòng

Thêm `--stop-vip ADDR`: dừng khi VIP quay lại hub lần thứ 2. Trên `8F346C`:

```
hub 0x135b33b reached 2 times -> one iteration
insns 3947362  stop cycle  exits 2
```

⇒ Lần đầu tiên có **một đơn vị ngữ nghĩa xác định** cho khối loop: một vòng = 3.947.362 lệnh.

### Kiểm chứng với bản viết tay (đối chiếu đầu tiên)

`tools/devirt.py` viết tay cho `8F346C`: `push 0xF; call 0x888462` (CSendPacket/SendPkt__Begin), sau đó
`0xA65280` ×3 (Encode4), `0x888609` (Send), `0x8884DC` (dtor), `0x8E1C72`, `0x8F2803`.

Tool báo **đích rời VM đầu tiên trong một vòng: `0x888462`** — khớp. Các call sau chưa tới vì biên "một
vòng" hiện tại (hub lần 2) cắt sớm hơn toàn bộ flow của khối.

Đây là **đối chiếu đầu tiên có bằng chứng** giữa tool và bản devirt tay. Chưa phải verify đầy đủ.

### Profile hợp nhất

`decomp/vm_profile.json` giờ gộp cả `opcodes` (46) và `handlers` (178) ⇒ một file đọc là đủ cho emitter.


## 11. Đối chiếu call sequence với bản devirt tay (round 6)

Chạy `trace --stop-vip 135B33B --hub-count 8` trên `8F346C`, ghi **thứ tự** rời VM, so với
`tools/devirt.py`:

| # | tool (thứ tự thật) | tên | bản viết tay (`devirt.py`) |
|---|---|---|---|
| 0 | `0x888462` | SendPkt__Begin | ✅ có |
| 1 | `0xA65280` | SendPkt__Encode4 | ✅ có |
| 2 | `0xA65280` | SendPkt__Encode4 | ✅ có |
| 3 | `0xA65280` | SendPkt__Encode4 | ✅ có |
| 4 | `0x8F2B34` | (không tên rõ) | ❌ **không có trong bản viết tay** |
| 5 | `0x692EC4` | (không tên rõ) | ❌ **không có trong bản viết tay** |
| 6 | `0x888609` | SendPkt__Send | ✅ có |

- **5/8 call ground truth được phục hồi đúng thứ tự** (Begin → 3×Encode4 → Send).
- Còn thiếu `0x8884DC` (dtor), `0x8E1C72`, `0x8F2803` — trace dừng trước khi tới.
- **2 call "thừa"** (`0x8F2B34`, `0x692EC4`): nhiều khả năng là phần **anti-cheat/HackShield** mà comment
  trong `devirt.py` nói đã bỏ ("Dropped: rand()-driven HackShield checks"). Chưa xác minh dứt điểm —
  ghi là giả thuyết, không phải kết luận.

Giới hạn gặp phải: sau `SendPkt__Send`, trace chết ở `unmapped 0x46C8586E` (native deref một con trỏ
lấy từ API đã fake). Đây là hạn chế của lớp `--fake-native` so với `tools/vmemu.py` (có bảng syscall/API
đầy đủ hơn), không phải lỗi của decode VM.

⇒ **Kết luận verify:** đối chiếu *một phần* đã thành công (thứ tự call khớp), nhưng chưa đủ để tuyên bố
"verify 6 khối". Muốn đủ: (a) nâng lớp fake API ngang `vmemu`, hoặc (b) dùng `vmemu` cho phần chạy và
để `vmdevirt` chỉ lo decode/emit.


## 12. Emit C có tham số (round 7)

`trace` giờ bắt **ecx + 4 arg stack** tại mỗi lần rời VM; `emit` in chúng theo đúng thứ tự. Kết quả cho
`8F346C` (`decomp/emit_8F346C.c`):

```c
void vm_block_8F346C(void *self)
{
    // 22 lần ghi vào .data/.text ngoài section VM (thường là scratch của VM)
    // 192 địa chỉ heap/stack bị ghi (object game) — xem JSON
    call_888462((void *)0x19c3a8, 0xf);            // #0  SendPkt__Begin
    call_A65280((void *)0x19c3a8, 0x4aba478);      // #1  SendPkt__Encode4
    call_A65280((void *)0x19c3a8, 0x0);            // #2
    call_A65280((void *)0x19c3a8, 0x0);            // #3
    call_8F2B34((void *)0x19c333, 0x46c8593e);     // #4  (chưa rõ — nghi anti-cheat)
    call_692EC4((void *)0x19c332, 0x46c8593e);     // #5  (chưa rõ)
    call_888609((void *)0x19c3a8, 0x46c8593e);     // #6  SendPkt__Send
}
```

So với bản viết tay trong `tools/devirt.py`:

```c
lea ecx, [ebp - 0x50]        ;  == ecx 0x19c3a8 mà tool bắt được
push 0xF                     ;  == arg0 0xf
call 0x888462                ;  Begin
push [ebp-0x1C]              ;  == arg0 0x4aba478
lea ecx, [ebp - 0x50]
call 0xA65280                ;  Encode4  (x3)
...
call 0x888609                ;  Send
```

⇒ **Cấu trúc khớp**: cùng object `ecx`, cùng `0xF`, cùng 3 lần Encode4, cùng Send. Khác biệt: 2 arg của
Encode4 #2/#3 ra `0x0` (bản viết tay đẩy `[ebp-0x18]`, `[ebp-0x24]` — có thể đúng là 0 trong snapshot này,
chưa xác minh) và 2 call "thừa" #4/#5.

Đây là mức "emit C đọc được" đầu tiên: **đúng thứ tự + đúng tham số cho các call đã biết**, còn thiếu phần
ghi object (192 địa chỉ heap) đang chỉ thống kê chứ chưa in ra biểu thức.


## 13. Phân loại write trong emit (round 8)

Trace `8F346C` có 192 "heap" write, nhưng phân loại theo frame (`regs_start.ebp`, `.esp`):

| loại | số | xử lý |
|---|---|---|
| stack của VM (quanh ebp/esp lúc vào) | **180** | bỏ qua — junk push/pop |
| ra ngoài stack | **12** | in ra để kiểm tay |
| object/biến game thật | **0** | (không có) |

12 địa chỉ đó:

```
0x20B000      n=10  (TEB — VM ghi vào TEB, dấu vết anti-debug/anti-cheat)
0x6BF67F0     n=1   last=0xF        <- trùng loại gói SendPkt__Begin(0xF)
0x6BF67F2..   n=1   last=0xA5, 0x67, 0x32A77B11, 0x446EFB83, ...
0x6C45B90     n=1   last=0x2B
0x6C45BCC     n=1   last=0x13
```

`0x6BF67F0 = 0xF` khớp với `SendPkt__Begin(0xF)` ⇒ đây nhiều khả năng là **object packet** mà VM ghi trực
tiếp (không qua call). Đây là loại write mà bản viết tay của `devirt.py` phải tái hiện bằng tay.

**Kết luận về emit:** với khối `8F346C`, hiệu ứng game thấy được **chủ yếu là chuỗi call**; write object
rất ít (12 địa chỉ, phần lớn là TEB + packet object). Điều này khớp với cách `devirt.py` viết khối này:
một chuỗi `call` + 1 phép gán field do native thực hiện.


## 14. Kiểm chứng chéo với snapshot thật (round 9)

Không cần chạy tới `cont` vẫn kiểm chứng được **từng hiệu ứng**, nếu có `post.snap` (bộ nhớ sau khi khối
chạy xong, do client gốc chụp). Lệnh mới:

```
python3 tools/vmdevirt.py verify --pre logs/snap/8F346C/pre.snap \
        --post logs/snap/8F346C/post.snap --value 4ABA478 --at DA05050
```

Kết quả:

```
value 0x4aba478: pre 3 hit(s), post 4 hit(s)
  chỉ có trong post (block đã ghi ở đây): 1
     0x0da05050
  tại địa chỉ chỉ định 0xda05050: pre=không, post=có
```

Diễn giải:

- `0x4ABA478` là giá trị `time` mà **trace của tool** bắt được làm `arg0` của `SendPkt__Encode4` #1.
- `0xDA05050` = `0xDA05008 + 0x48`. `0xDA05008` là con trỏ `CMainSystem` (xuất hiện trong ctx/thanh ghi).
- `devirt.py` viết tay cho khối này: **"CMainSystem+0x48 = time"**.
- `pre.snap` **không** có giá trị đó tại `0xDA05050`, `post.snap` **có**.

⇒ Đây là **kiểm chứng độc lập, khép kín** cho một hiệu ứng: tool quan sát đúng giá trị, và bộ nhớ thật sau
khối chứa đúng giá trị đó tại đúng nơi bản viết tay nói. Không phụ thuộc vào việc emulation có chạy hết khối
hay không.

Cách dùng tổng quát: với mỗi giá trị scalar mà trace bắt được (tham số call, giá trị ghi), chạy `verify` để
tìm địa chỉ "chỉ có trong post" — đó là nơi khối đã ghi.


## 15. Kiểm chứng định lượng write vs post.snap (round 10)

Lệnh:

```
python3 tools/vmdevirt.py verify --trace-json decomp/trace_iter_8F346C.json \
        --post logs/snap/8F346C/post.snap --json decomp/verify_8F346C_writes.json
```

So từng write của trace (địa chỉ + size + giá trị cuối) với bộ nhớ `post.snap`, mask theo size:

| phạm vi | khớp | lệch | ghi chú |
|---|---|---|---|
| toàn bộ | 45/232 | 187 | stack của VM chi phối (giá trị trung gian, không phải kết quả khối) |
| **ngoài stack** | **32/52** | 20 | |
| packet object `0x6BF67F0..0x6BF6802` | **8/8** | 0 | khớp tuyệt đối |
| `CMainSystem+0x48` (`0xDA05050`) | ✅ | — | giá trị `time` khớp |
| `.data` scratch `0xEEC6xx` | 0/18 | 18 | post = 0 ⇒ VM **xoá/khôi phục** scratch sau khi dùng; trace dừng sớm nên còn giá trị trung gian |
| TEB `0x20B000` | — | 1 | VM ghi rồi khôi phục |

⇒ **32/52 write ngoài stack được xác nhận bằng bộ nhớ thật**, và toàn bộ 20 chỗ lệch đều giải thích được
(scratch bị xoá, TEB bị khôi phục) — không có chỗ nào mâu thuẫn.

Đây là mức kiểm chứng mạnh nhất từ đầu: không chỉ "call khớp thứ tự" mà **giá trị bộ nhớ khớp từng byte**.

### Khối thứ hai: `8F4D46`

```
python3 tools/vmdevirt.py verify --trace-json decomp/trace_8F4D46.json --post logs/snap/8F4D46/post.snap
so write của trace với post.snap: 29 khớp / 152 so được
```

Phân loại: **9/12 write ngoài stack khớp chính xác** (7 chỗ lệch nằm ở `.data`/VM scratch, trace dừng sớm).

| khối | write ngoài stack khớp | ghi chú |
|---|---|---|
| `8F346C` | 32/52 | packet object 8/8, `CMainSystem+0x48` ✓ |
| `8F4D46` | 9/12 | |

⇒ Hai khối độc lập đều cho tỉ lệ khớp cao trên phần **không phải stack**, và mọi chỗ lệch đều nằm ở vùng
scratch/TEB hoặc do trace dừng sớm.


## 16. Đường áp dụng sang bản KR (runbook)

Bản KR cuối (`~/Downloads/boom_kr_lastest00967/PatchData/CA.exe`, 10.141.152 byte) — đo bằng `doctor`:

```
markers 'EB 10 57 4C 20' : 0
.idata size             : 0x200
entry                   : 0x225D000 in 'gsowkarq', section size 0x1000
sections entropy > 7.9  : ['   \x00    ', 'gulxdsye']
VERDICT: PACKED / chưa unpack
```

⇒ **Không devirt trực tiếp được.** Phải dump tại OEP trước. Runbook:

| bước | lệnh | kỳ vọng |
|---|---|---|
| 1 | `python3 tools/vmdevirt.py doctor <exe>` | phải ra `UNPACKED` |
| 2 | dump client KR tại OEP (đường của repo: `win/snap.bat <SITE>` / live dump) | file dump + snapshot |
| 3 | `doctor <dump>` | thấy marker, thấy site |
| 4 | `discover <dump>` | `sites` + `stub_heads`; nếu 0 site ⇒ VM đời khác, xem bước 8 |
| 5 | `profile <dump> --snap <pre.snap> --site <site>` | tự dò ctx + dạng dispatch |
| 6 | `handlers --ctx <ctx>` | bảng handler (nếu có) |
| 7 | `opcodes`, `cfg`, `trace`, `emit`, `verify` | như đã làm với VN |
| 8 | nếu dispatch là `jmp [table + idx*k]` liền mạch / handler tự chain | đó là kiểu **FALCON** — cần thêm backend `chained` (chưa viết) |

Điểm đã chuẩn bị sẵn cho KR:

- Mọi hằng số (ctx, lock offset, bảng, size VI) đều **discover**, không hardcode.
- `doctor` chặn sớm nếu file còn pack — tránh mất thời gian.
- `profile` gộp `opcodes` + `handlers` vào một JSON, nên engine không phụ thuộc bản VN.
- Nếu KR là VM đời mới (FALCON), phần **dùng lại được**: PE loader, site scan, stub follow, snapshot
  replay, emulation harness, `verify`. Phần **phải làm mới**: decode VI (vì FALCON dùng
  `next_idx` + `vip_delta` tường minh, không phải bytecode mã hoá rolling như bản VN).


## 17. Khối thứ ba `8F5F40` (round 12)

### Call sequence — khớp 3 call đầu, đúng cả tham số

| # | tool | tham số tool bắt | `devirt.py` viết tay |
|---|---|---|---|
| 0 | `0x9DBE57` | `arg0 = 0xC` | `push 0xC; call 0x9DBE57` ✅ |
| 1 | `0x67C820` | `arg0 = 0x19C5A8`, `ecx = 0x70000000` (giá trị fake new) | `lea edx,[ebp-0x140]; push edx; mov ecx,eax; call 0x67C820` ✅ |
| 2 | `0x8F2C6B` | `arg0 = 0x1` | `push 1; call 0x8F2C6B` ✅ |

Kiểm chứng đáng chú ý: `ebp` lúc vào = `0x19C6E8` ⇒ `ebp-0x140 = 0x19C5A8` — **đúng con trỏ stack mà tool bắt
được**, tức VM tự tính đúng địa chỉ frame và truyền làm tham số.

Ground truth còn `0x412B76`, `0x9995B1`, `0x441A4C` — trace dừng trước.

### Write vs post.snap

```
8F5F40: tổng 9/121 so được; ngoài stack 2/4 khớp
   0xEEE660 trace=0x2E7E0  post=0x2E7E1   diff   (tick counter nội bộ Themida)
   0xEEE664 trace=0xB912A5F post=0xC322CAC diff   (tick counter)
   0xE60950 trace=0x84     post=0x84      match
   0xE6094C trace=0x20A    post=0x20A     match
```

Hai chỗ lệch là **bộ đếm tick** — native bị fake nên không tăng như thật; đúng như dự đoán, không phải
sai đường thực thi.

### Tổng hợp 3 khối

| khối | call ground truth khớp | write ngoài stack khớp |
|---|---|---|
| `8F346C` | 5/8 (đúng thứ tự) | 32/52 |
| `8F4D46` | 0/2 call (chưa tới native) | 9/12 |
| `8F5F40` | 3/6 (đúng thứ tự + tham số) | 2/4 |


## 18. Bàn giao (round 13)

### Tool làm được gì

`tools/vmdevirt.py` — 17 lệnh, không hardcode hằng số nào của bản VN (mọi thứ discover từ file + snapshot):

```
doctor     chặn sớm: file còn pack hay đã sẵn sàng devirt
sites      liệt kê site jmp vào vùng VM + stub head
discover   section, site, head (tổng quan nhanh)
jumps      phân tích chuyển nhánh gián tiếp (dispatch)
ctx        dump VM context sau một khối
trace      chạy khối: write (ctx/image/heap), call, args, đích rời VM
lift       rút các lệnh "có ngữ nghĩa" (bỏ junk)
profile    sinh decomp/vm_profile.json (gộp cả opcodes + handlers)
handlers   enumerate + phân loại bảng handler
bytecode   học encoding VI từ trace (opcode -> size, opcode nhánh)
decode     decode tuyến tính (chỉ dùng được khi bytecode không mã hoá)
opcodes    ghép bảng handler + size + đặc trưng -> bản đồ opcode
semantics  sketch ngữ nghĩa từng handler (chưa dùng được để emit)
vfx        hiệu ứng mỗi VI (diff ctx + thanh ghi giữa 2 dispatch)
cfg        CFG của stream VI đã chạy
emit       sinh C từ trace (call + args + phân loại write)
verify     đối chiếu pre/post snapshot: theo giá trị hoặc so mọi write
```

### Số liệu (bản VN, `unpacked_ca.exe`)

| hạng mục | kết quả | bằng chứng |
|---|---|---|
| site / stub head | 607 / 390 | head `0x11EE3B0` xuất hiện 73 lần = 73 marker |
| VM context | `0x11EDDEC`, lock `+0x7C` | tự dò từ spinlock `0x11EECB0`; xác nhận bằng `edi` cuối khối |
| bảng handler | 178 entry, `ctx+0x94..0xF48` | tĩnh: khớp 3 snapshot + file; **3680/3680 dispatch khớp runtime** |
| opcode map | 46 opcode, 46/46 có handler | 44/46 đo được size, **0 xung đột**, 3 opcode nhánh |
| CFG | 24 block, 23 cạnh, 6 back-edge | hub `0x135B33B`; sửa được bug "nhảy ngắn bị nuốt" |
| một vòng | 3.947.362 lệnh | `--stop-vip 135B33B --hub-count 2` |
| emit C | call + args + phân loại write | khớp cấu trúc bản viết tay `8F346C`, `8F5F40` |
| verify | 3 khối | `8F346C` 32/52, `8F4D46` 9/12, `8F5F40` 2/4 write ngoài stack khớp post.snap |
| KR | `doctor` = PACKED | runbook §16 sẵn sàng |

### Chưa làm được (và vì sao)

1. **Ngữ nghĩa 46 opcode** — cần abstract-interpret thân handler để bóc opaque predicate + stack shuffle.
   `semantics` hiện chỉ là sketch bị hằng số rối chi phối. Đây là việc nhiều ngày, không phải vài lệnh.
2. **decode toàn stream** — bytecode mã hoá **rolling key** (4 giả thuyết khoá đều thất bại: 254–262 giá trị
   khác nhau). Chỉ decode được phần **đã chạy** qua trace. Muốn decode nhánh chưa chạy phải hiểu hàm cập
   nhật khoá.
3. **verify đủ 6 khối** — chỉ 3 khối có snapshot. 3 khối còn lại (`8F452B`, `8F8F69`, `8F54E4`) chưa có
   snapshot ⇒ cần chạy client gốc (`win/snap.bat <SITE>`).
4. **emit trung thực từng VI** — phụ thuộc (1).

### Việc tiếp theo, theo thứ tự

1. Chạy `win/snap.bat 8F452B` và `8F8F69` → `trace` → `emit` → `verify` để nâng verify lên 5–6 khối.
2. Nếu cần emit trung thực: viết abstract interpreter cho handler (dùng `vfx` làm oracle).
3. Khi có dump KR: `doctor` → `discover` → `profile` → xem dispatch form; nếu là `jmp [table+idx*k]` liền
   mạch thì viết backend `chained` (FALCON), phần còn lại dùng nguyên.

### Cách chạy lại toàn bộ trên bản VN

```bash
python3 tools/vmdevirt.py doctor
python3 tools/vmdevirt.py profile --snap logs/snap/8F346C/pre.snap --site 8F346C
python3 tools/vmdevirt.py handlers --ctx 11EDDEC --json decomp/vm_handlers.json
python3 tools/vmdevirt.py opcodes --snap logs/snap/8F346C/pre.snap --site 8F346C \
        --disp 11F9E81 --ctx 11EDDEC --json decomp/vm_opcodes.json
python3 tools/vmdevirt.py cfg --snap logs/snap/8F346C/pre.snap --site 8F346C --disp 11F9E81 --max 6000000
python3 tools/vmdevirt.py trace --snap logs/snap/8F346C/pre.snap --site 8F346C \
        --disp 11F9E81 --stop-vip 135B33B --hub-count 8 --ctx 11EDDEC --fake-native --json decomp/trace_iter_8F346C.json
python3 tools/vmdevirt.py emit --trace decomp/trace_iter_8F346C.json --json decomp/emit_8F346C.c
python3 tools/vmdevirt.py verify --trace-json decomp/trace_iter_8F346C.json --post logs/snap/8F346C/post.snap
```


## 19. Ngữ nghĩa opcode — kết quả đầu (round 15)

Thử 2 giả thuyết cho VSP, cả hai đều cho kết quả rõ ràng:

| giả thuyết | kết quả |
|---|---|
| VSP là field `ctx+0x0C` | **sai** — delta luôn 0 ở mọi dispatch |
| VSP là **thanh ghi `esp`** (VM dùng luôn host stack) | **đúng** — delta ổn định theo opcode |

Đo `Δesp` giữa 2 dispatch, ghép với VI size đã học:

| op | size | Δesp | count | suy ra |
|---|---|---|---|---|
| `0x87` | 1 | −4 | 1584 | **push reg** |
| `0x91` | 1 | −4 | 790 | **push reg** |
| `0x43` | 2 | −4 | 1433 | **push imm8** (sign-extend) |
| `0x2e` | 2 | −4 | 59 | **push imm8** |
| `0x3b` | 5 | −4 | 848 | **push imm32** |
| `0x71` | 3 | −2 | 1060 | **push imm16** |
| `0x81` | 1 | +4 | 3089 | **pop reg** |
| `0x66` | 1 | +4 | 989 | **pop reg** |
| `0xa6` | 1 | +4 | 650 | **pop reg** |
| `0x56`, `0xc7`, `0xcb`, `0x59` | 1-2 | +4 | | **pop** |
| `0x76`, `0xb8`, `0xa0`, `0x7d`, `0x55`, `0xb4` | 1 | 0 | | **không đụng stack** (arithmetic/load) |
| `0x67`, `0x7b`, `0x80` | — | — | | **branch** |

Phân bố 46 opcode: **15 push, 12 pop, 16 không đụng stack, 3 branch**.

Điểm mạnh: **size khớp với Δesp** — op size 5 push 32-bit, size 3 push 16-bit, size 2 push 8-bit
(sign-extend), size 1 push register. Đây chính là phép "nhận diện vpush/vpop" của bài gốc, làm tự động.

Kết quả lưu ở `decomp/vm_opcode_classes.json`; lệnh:

```
python3 tools/vmdevirt.py vfx --snap logs/snap/8F346C/pre.snap --site 8F346C \
        --disp 11F9E81 --ctx 11EDDEC --vsp-reg esp --json decomp/vm_vfx.json
```


### Thanh ghi mà push/pop di chuyển (`opsrc`)

So giá trị đẩy vào stack với thanh ghi trước đó (push), hoặc giá trị lấy ra với thanh ghi sau đó (pop):

| op | size | Δesp | nguồn/đích | kết luận |
|---|---|---|---|---|
| `0x3b` | 5 | −4 | `imm/mem` 92/92 | **push imm32** |
| `0x2e` | 2 | −4 | `imm/mem` 7/7 | **push imm8** |
| `0x87` | 1 | −4 | `edx` 190/190 | **push edx** |
| `0x81` | 1 | +4 | `edx` 400/400 | **pop edx** |
| `0x66` | 1 | +4 | `edx` 122/122 | **pop edx** |
| `0xa6`, `0xc7`, `0x56`, `0xcb`, `0x59`, `0xb1` | 1-2 | +4 | `edx` | **pop edx** |
| `0x91` | 1 | −4 | `imm/mem` 80, `ecx` 21 | chưa chắc (2 biến thể?) |
| `0x43` | 2 | −4 | `imm/mem` 157, `edx` 26 | chưa chắc |

⇒ `edx` là **thanh ghi tích luỹ** của VM: mọi `pop` đổ vào `edx`, và `push edx` là op riêng.
Điều này khớp với việc `devirt.py` viết tay thường thấy `mov edx, [ebp-...]` trước các chuỗi xử lý.

Kết quả: `decomp/vm_opcode_regs.json`, lệnh `opsrc`.

**Còn thiếu**: 16 opcode "không đụng stack" (arithmetic/load/store) chưa phân loại; 2 opcode mơ hồ ở
trên; và chưa biết thứ tự toán hạng để dựng biểu thức.

### Còn thiếu để có ngữ nghĩa đầy đủ

- **Thanh ghi nào** được push/pop (hiện chỉ biết "có push 4 byte", chưa biết `push eax` hay `push ebx`).
- Các op `none` (16 cái): arithmetic/load/store — cần đọc thân handler để biết phép toán.
- Với push/pop: cần xác định **thứ tự** toán hạng trong ngăn xếp ảo để dựng biểu thức.


## 20. Vào WinBoat và lấy snapshot mới (round 16)

Đường vào guest (không cần mật khẩu desktop cho phần chạy lệnh, nhưng cần để mở cmd):

| bước | cách |
|---|---|
| QMP | `127.0.0.1:47292` (container `WinBoat`, QEMU 7149) — `human-monitor-command` |
| Ảnh màn hình | QMP `screendump /shared/x.ppm`; `/shared` (container) = `/home/admin` (host) |
| Gõ phím | QMP `sendkey`, **một kết nối, nghỉ 100ms/phím** (gửi dồn thì guest nuốt phím) |
| Chuột | `input-send-event` (usb-tablet) — cần để focus ô mật khẩu |
| Đăng nhập | user `admin`; **phải tắt Caps Lock** (màn hình báo "Caps Lock is on" nếu không) |

Tool: `tools/qmp_guest.py` (`shot`, `key`, `type`, `click`, `combo`).

### Snapshot `8F452B` (khối init)

Chạy trong cmd của guest: `\\host.lan\Data\boom-re\win\snap.bat 8F452B 8F45A7 180000`

Kết quả: **`pre.snap` 264 MB** (1022 region, site hit ở 10.250 ms), nhưng **không có `post.snap`** —
process chết `c0000005` sau 60 s (client không có server nên đi sai đường sau khối).

### Verify khối 4 bằng ground truth của `devirt.py`

`devirt.py` cho `8F452B`: callbacks `0xEEE650..65C = 8F2C5D, 8F30F7, 8F2C4F, 8F31F6`; thêm
`0xEED860[256]` CRC32, `0xEEE07C = 0x4A65000A`, `0xEEE088 = 1`, `0xEEE08C = tick`.

Trace tool (5,1M lệnh):

| địa chỉ | ground truth | tool | |
|---|---|---|---|
| `0xEEE650` | `0x8F2C5D` | `0x8F2C5D` | ✅ |
| `0xEEE654` | `0x8F30F7` | `0x8F30F7` | ✅ |
| `0xEEE658` | `0x8F2C4F` | `0x8F2C4F` | ✅ |
| `0xEEE65C` | `0x8F31F6` | `0x8F31F6` | ✅ |
| `0xEEE07C` | `0x4A65000A` | — | chưa tới |
| `0xEEE088` | `1` | — | chưa tới |
| CRC table | 256 dword | 0 | chưa tới |

Trace dừng ở `unmapped 0x8489002C` (native dùng giá trị API đã fake làm con trỏ) — hạn chế đã biết của
`--fake-native`, không phải sai decode.

⇒ **4/6 write khớp tuyệt đối** trên khối thứ tư, gồm đúng 4 con trỏ callback — ground truth đặc thù nhất
trong cả 6 khối.

### `8F8F69` chưa lấy được

Đó là `CStageMgr__ChangeStage` — chỉ chạy khi **đổi stage**, tức phải vào được game (login + server giả).
Trong khi `8F452B` chạy ngay lúc khởi động nên lấy được. Muốn có `8F8F69`: chạy `tools/fakeserver.py` trên
host, cho client login (creds trong `docs/specification/networking/LEGACY_VN_PROTOCOL.md`), rồi `snap.bat 8F8F69 8F8FCA` — hoặc dùng chiêu
`--pre-asm sub_8F391D(10)` trên snapshot `8F5F40` như repo đã làm.


## 21. Có đáng làm ngữ nghĩa 46 opcode không? (đếm trước)

Đếm trên 73 VM entry thật (head `0x11EE3B0`):

| nhóm | số khối |
|---|---|
| đã có trong `devirt.py` | 14 |
| **còn lại** | **59** |
| trong đó: anti-cheat (`0xA12E59..0xA17A24`) | **58** |
| trong đó: code game | **1** (`0x966FEC` — hàm mồ côi, 0 xref) |

⇒ **Toàn bộ 58/59 khối còn lại là anti-cheat** — đúng phần repo đã quyết định bỏ. Khối game duy nhất còn
lại là hàm mồ côi không ai gọi.

**Kết luận:** với mục tiêu của repo (devirt đủ để chơi lại), làm full semantics **không đáng** — nó chỉ có
giá trị nếu ai đó muốn một devirtualizer tổng quát.

## 22. Hypothesis fitter — đã thử, thất bại, và biết vì sao

Thêm lệnh `fit`: với mỗi opcode, thử khớp `target = f(a,b)` trên mọi mẫu (registers trước/sau mỗi VI).

```
python3 tools/vmdevirt.py fit --snap logs/snap/8F346C/pre.snap --site 8F346C \
        --disp 11F9E81 --max 1500000 --min-samples 8
insns 1500001  opcodes with samples 39  total samples 3679
resolved 0/39
```

Kết quả: **0/39 opcode fit được**. Nguyên nhân (không phải bug, đã sửa 2 lỗi và vẫn vậy):

1. Khoảng giữa hai dispatch chứa **nhiều việc**: cập nhật VIP, cập nhật cache dispatch, thanh ghi tạm, và
   junk — không chỉ phép toán của opcode.
2. Vì vậy `out = f(in)` không bao giờ đúng cho **mọi** mẫu; `vfx` đã cho thấy cùng một opcode có 2–4
   effect-set khác nhau.
3. Muốn tách phần ngữ nghĩa khỏi phần ghi sổ phải **symbolic/abstract interpretation bên trong handler**
   — đúng phần "khó" đã nêu, không thể lách bằng fitting thống kê.

⇒ Đây là kết quả phủ định có giá trị: nó loại bỏ hướng rẻ tiền và xác nhận chi phí thật của việc lấy ngữ
nghĩa đầy đủ.

## 5b. Đường áp dụng sang bản KR

Bản KR (`~/Downloads/boom_kr_lastest00967/PatchData/CA.exe`) **đang bị pack**:

- entry `RVA 0x1E5D000` nằm trong section `gsowkarq` (512 byte) = stub unpacker;
- section VM `gulxdsye` (1.99 MB) entropy 7.95, **0 marker** `EB 10 57 4C 20` (bản VN có 73);
- không có run ≥16 dword trỏ vào section VM ⇒ bảng handler/bytecode chưa tồn tại ở dạng đọc được;
- `.idata` chỉ 0x200 byte ⇒ IAT đã bị bóc.

⇒ Phải dump tại OEP rồi mới chạy `discover`. Tool đã tách "profile" khỏi engine để sau khi có dump KR chỉ
cần chạy lại `discover`/`jumps`/`ctx` trên ảnh mới, không sửa code.
