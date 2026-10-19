# Devirtualize VM của Themida — hướng dẫn từ số 0

> Bài viết dành cho người **chưa từng làm VM devirtualization**. Đọc xong bạn sẽ hiểu VM của
> Themida/Code Virtualizer được dựng thế nào, và có một quy trình 8 bước để bóc nó ra thành code đọc được.
>
> Nguồn cảm hứng + phương pháp gốc: **Denis Shablyko, "Devirtualizing Themida's (Code Virtualizer)
> FALCON VM to readable code", 25/03/2026** — <https://blog.shablyko.com/posts/themida-falcon-devirt/>.
> Trong bài đó tác giả chỉ dùng **Unicorn + IDA**, không dùng Remill/Triton, và đi theo kiểu "nhật ký
> hành trình". Bài này hệ thống lại thành quy trình, thêm số liệu đo thật trên một client game 32-bit
> (Themida đời cũ) và các lỗi mà người mới hay mắc.

---

## Mục lục

1. [VM là gì, và tại sao nó khó bóc](#1-vm-là-gì-và-tại-sao-nó-khó-bóc)
2. [Bộ từ vựng tối thiểu](#2-bộ-từ-vựng-tối-thiểu)
3. [Chuẩn bị dụng cụ](#3-chuẩn-bị-dụng-cụ)
4. [Bước 1 — Tìm chỗ code chui vào VM](#bước-1--tìm-chỗ-code-chui-vào-vm)
5. [Bước 2 — Tìm context](#bước-2--tìm-context-bước-quan-trọng-nhất)
6. [Bước 3 — Tìm bảng handler](#bước-3--tìm-bảng-handler)
7. [Bước 4 — Phân loại handler](#bước-4--phân-loại-handler)
8. [Bước 5 — Đọc virtual instruction](#bước-5--đọc-virtual-instruction)
9. [Bước 6 — Chạy và dừng đúng chỗ](#bước-6--chạy-và-dừng-đúng-chỗ)
10. [Bước 7 — Lift thành code đọc được](#bước-7--lift-thành-code-đọc-được)
11. [Bước 8 — Kiểm chứng](#bước-8--kiểm-chứng)
12. [Hai đời VM: chained và threaded](#12-hai-đời-vm-chained-và-threaded)
13. [Bẫy thường gặp](#13-bẫy-thường-gặp)
14. [Case study: bản 32-bit đời cũ](#14-case-study-bản-32-bit-đời-cũ)
15. [Lộ trình 3 tuần cho newbie](#15-lộ-trình-3-tuần-cho-newbie)
16. [Phụ lục](#16-phụ-lục)

---

## 1. VM là gì, và tại sao nó khó bóc

Themida (và Code Virtualizer, VMProtect) biến một hàm bình thường thành **một chương trình con chạy
trên một máy ảo riêng**. Hàm gốc không còn là x86 nữa; nó thành **bytecode** của VM đó.

Ví dụ hàm gốc:

```c
int f(int a, int b) { return a * 3 + b; }
```

Sau khi virtualize, chỗ gọi hàm chỉ còn một lệnh `jmp` vào section lạ. Ở trong đó là một **trình thông
dịch** (interpreter) đọc bytecode và thực hiện từng "lệnh ảo" — mỗi lệnh ảo tương ứng một đoạn x86 thật
gọi là **handler**.

Tại sao khó?

| Khó vì | Cụ thể |
|---|---|
| Không có code để đọc | Hàm gốc đã bị ghi đè; bạn chỉ thấy interpreter + bytecode |
| Handler bị làm rối | Thêm lệnh rác, đổi thanh ghi, chèn nhảy |
| Bytecode không phải x86 | Muốn hiểu phải *giải mã* nó trước |
| Nhiều biến thể | Oreans có hơn hai chục VM khác nhau, mỗi cái một instruction set |
| Trạng thái nằm trong "context" | Muốn chạy được phải dựng lại đúng context |

Nhưng tin tốt: **mọi VM đều có cùng một bộ xương**. Hiểu bộ xương là bóc được.

---

## 2. Bộ từ vựng tối thiểu

Trước khi đi tiếp, nắm 6 khái niệm. Đây là toàn bộ từ vựng cần cho phần còn lại.

| Thuật ngữ | Nghĩa | Ẩn dụ |
|---|---|---|
| **VM context** (ctx) | Một struct trong bộ nhớ giữ toàn bộ trạng thái của VM: thanh ghi host đã lưu, flags, con trỏ ngăn xếp ảo, con trỏ lệnh ảo, bảng handler… | "Bàn làm việc" của VM |
| **VIP** (virtual instruction pointer) | Con trỏ tới **bytecode** đang đọc | Ngón tay chỉ vào sách |
| **VSP** (virtual stack pointer) | Ngăn xếp riêng của VM (nhiều VM dùng luôn stack thật) | Chồng đĩa riêng |
| **Handler** | Một đoạn x86 thực hiện **một** lệnh ảo (ví dụ: "cộng 2 toán hạng ảo") | Một "động tác" trong bộ xương |
| **Dispatcher** | Đoạn code quyết định handler kế tiếp là cái nào | Người điều phối |
| **Bytecode / virtual instruction (VI)** | Dữ liệu VM đọc để biết làm gì | Trang sách |

Một vòng đời đơn giản của VM:

```
   code game
      │  jmp
      ▼
 ┌─────────────┐   save regs      ┌──────────┐
 │  vm_entry   │ ───────────────► │ ctx      │
 └─────────────┘                  └──────────┘
      │                                 ▲
      ▼                                 │ đọc/ghi
 ┌─────────────┐   jmp [table+idx]  ┌──────────┐
 │ dispatcher  │ ────────────────►  │ handler  │
 └─────────────┘                    └──────────┘
      ▲                                 │
      └────── idx của VI kế tiếp ◄──────┘
      │
      ▼ (khi gặp vm_exit)
   restore regs → code game tiếp tục
```

Nhiệm vụ của bạn = dựng lại **ctx**, **bảng handler**, **định dạng VI**, rồi dịch từng handler về
ngữ nghĩa x86. Hết.

---

## 3. Chuẩn bị dụng cụ

Tối thiểu (đúng như bài gốc):

- **IDA** (bản Free cũng được) hoặc Ghidra — để đọc và đặt tên.
- **Python** + `unicorn` (giả lập CPU) + `capstone` (disassemble) + `pefile` (đọc PE).
- Một **bản dump đã unpack** (xem mục 13 — đây là điều kiện tiên quyết, không có thì đừng làm gì cả).

```bash
python3 -m pip install --user capstone unicorn pefile
```

Không cần: LLVM/Remill/Triton. Bài gốc cố tình không dùng để bạn hiểu bản chất.

---

## Bước 1 — Tìm chỗ code chui vào VM

### Nhìn gì

1. **Section lạ.** PE bình thường có `.text .rdata .data .rsrc`. Themida đổi tên hoặc để trắng:
   `gulxdsye`, `rfsnoyxf`, `        ` (toàn space). Section đó thường **rất lớn** (1–3 MB) dù chương
   trình nhỏ, và có entropy cao.
2. **Entry point** nằm trong một section bé xíu (vài trăm byte) → đó là **stub unpacker**, không phải
   code game.
3. **`.idata` bé bất thường** (vài trăm byte) → IAT đã bị bóc, dấu hiệu pack.

### Tìm "site"

Site = chỗ code game `jmp`/`call` vào vùng VM. Quét toàn `.text`:

```python
# quét E8/E9 (call/jmp rel32) có đích rơi vào vùng VM
for va in range(text_lo, text_hi - 5):
    if img[va] in (0xE8, 0xE9):
        tgt = va + 5 + int32(img[va+1:va+5])
        if vm_lo <= tgt < vm_hi:
            print(hex(va), "->", hex(tgt))
```

Mỗi site thường là `push <key>; jmp <stub>` hoặc chỉ `jmp <stub>`. `<key>` là **bộ chọn bytecode**
(cho biết hàm ảo nào sắp chạy) — nhớ lấy, nó là "số trang" của cuốn sách bytecode.

> **Kinh nghiệm:** đừng chỉ tìm `jmp` trực tiếp. Có site nhảy vào một stub, stub lại `jmp` tiếp mới tới
> `vm_entry`. Hãy *follow* các `jmp` trực tiếp cho tới khi gặp nhánh điều kiện/`ret`/`call`.

**Bài tập:** đếm xem có bao nhiêu site, và bao nhiêu đích khác nhau. Nếu số site ≈ số marker mà bạn tìm
được trong section VM (xem mục 13), bạn đang đi đúng.

---

## Bước 2 — Tìm context (bước quan trọng nhất)

Không có ctx thì mọi thứ sau đó vô nghĩa. Có 3 cách, dùng lần lượt.

### Cách A — nhìn quanh chỗ "khoá"

VM thường **đồng bộ** bằng một cờ trong ctx, vì nó không thread-safe. Mẫu kinh điển (chính là ảnh
`spinlock.png` trong bài gốc):

```asm
loc_401D0612:
    xor     eax, eax
    lock cmpxchg [rbx+rbp], ecx     ; nếu [ctx+off] == eax thì ghi ecx
    jz      short loc_401D0626
    pause
    jmp     loc_401D0612
```

Đọc được dòng này là bạn có **ctx + offset của cờ khoá** cùng lúc: `rbp` (hoặc reg base) = ctx.

- Bản FALCON trong bài: `ctx + 0xD3`.
- Bản game 32-bit mình đo: `lock cmpxchg [edi+0x7C], ecx` tại `0x11EECB0` ⇒ **ctx = `0x11EDDEC`**.
- Kiểm chứng chéo: cuối mỗi khối VM, `edi` vẫn = ctx; tại site dispatch `[edi + eax*4]` cũng dùng ctx.

### Cách B — nhồi magic value rồi soi stack/bộ nhớ

Bài gốc làm đúng cách này: đặt mỗi thanh ghi một giá trị nhận dạng được (`0x67`, `0x11223344`, …), chạy
tới sau prologue, rồi **dump stack vài chục qword** quanh RSP. Bạn sẽ thấy đúng các giá trị đó nằm ở
những offset cố định ⇒ đó là vùng "save host registers" trong ctx.

Với bản 32-bit mình làm, nó ra:

```
ctx+0x00..0x1C : 8 dword = 8 thanh ghi host đã lưu
```

### Cách C — quét write-trace

Chạy VM một đoạn và ghi lại **mọi lần ghi vào vùng VM**:

```
[0x011ede68] 4B <- 0x1        ; cờ khoá
[0x011eddec] 4B <- 0x0019c3f8 ; esp đã lưu
```

Cụm địa chỉ bị ghi dày nhất chính là ctx. Cách này tự động hoá được và là thứ tool nên làm.

> **Nguyên tắc:** đặt tên ngay khi có giả thuyết, kể cả sai. Bài gốc nói rất đúng: tên sai sẽ tự lộ ra
> khi bạn hiểu thêm; còn không đặt tên thì bạn sẽ quên mình đang nhìn cái gì.

---

## Bước 3 — Tìm bảng handler

Handler không nằm rải rác ngẫu nhiên — VM cần biết "idx → địa chỉ handler", nên có bảng.

### Dạng 1 (FALCON, đời mới)

Bảng nằm trong section VM, **được fixup lúc entry**: một vòng lặp cộng image base vào từng phần tử.

```asm
; trong vm_entry, sau khi lưu thanh ghi:
mov  ebx, 0x292          ; số phần tử (658)
loop:
add  [table + rbx*8], image_base
dec  ebx
jnz  loop
```

⇒ Nếu bạn thấy một vòng lặp cộng base vào từng qword của một mảng, mảng đó là **bảng handler**, và con
trỏ tới nó nằm trong ctx (FALCON: `ctx+0xD7`).

> **Cảnh báo cực quan trọng:** trong **dump tĩnh**, bảng này có thể đang ở trạng thái *nửa RVA, nửa
> absolute* tuỳ thời điểm chụp. Bài gốc ghi nhận `[ctx+0xD7]` đọc ra RVA (`0x1BAEF1`) còn VIP đọc ra
> địa chỉ tuyệt đối (`0x1402168CB` = base + 0x2168CB). Đừng giả định "toàn bộ là absolute".

### Dạng 2 (đời cũ, bảng nằm *trong* ctx)

Bản 32-bit mình làm: chính ctx chứa bảng, chỉ số đã nhân sẵn:

```asm
mov  rdi, ctx
jmp  dword ptr [edi + eax*4]      ; eax = chỉ số (đã là offset/4)
```

Đo thực tế: `eax = 0x43 → slot ctx+0x10C → 0x11F6CB8`.

Tìm bảng kiểu này bằng: đọc từng dword trong ctx, giữ những giá trị **trỏ vào section VM**, rồi kiểm tra
tính liên tục.

### Bảng tĩnh hay cache runtime?

Đây là câu hỏi bắt buộc phải trả lời, vì nó quyết định bạn có enumerate được mà không cần chạy hay không.

**Cách kiểm:** lấy **3 snapshot khác nhau** (3 site khác nhau), so từng entry, rồi so với byte trong file.

- Nếu giống hệt nhau và giống file ⇒ **bảng hằng** ⇒ enumerate offline được. (Bản 32-bit: 178/178 entry
  giống qua 3 snapshot và khớp file.)
- Nếu khác nhau ⇒ đó là **cache** (mỗi handler ghi địa chỉ handler kế vào slot riêng rồi `jmp [slot]`).
  Khi đó "bảng" không phải specification, chỉ là dấu vết; phải lấy ngữ nghĩa bằng trace.

---

## Bước 4 — Phân loại handler

Bạn không cần đọc 658 handler bằng mắt. Bài gốc dùng **3 heuristic tĩnh**, và chúng rất hiệu quả:

### Heuristic 1 — `vm_exit`

Handler nào **khôi phục nhiều thanh ghi host từ stack** (pop liên tiếp rồi `ret`/`jmp [rsp]`) là exit.

```asm
pop r8 ... pop r15, rdi, rsi, rbp, rbx, rdx, rcx, rax, popfq
jmp [rsp]
```

Bài gốc quét bảng và tìm được **10–11 handler** kiểu này. Chúng chia làm 2 loại:
- exit để **gọi hàm native** (vmcall),
- exit để **chạy một instruction chưa virtualize được** (Themida chép nguyên lệnh gốc vào section rồi
  `jmp` về `vm_entry`).

### Heuristic 2 — flag/bitwise: tìm `pushfd`/`pushfq`

Mọi lệnh bitwise/so sánh đều đổi RFLAGS, và VM phải lưu flags. Nên:

> Tìm `pushfd` (0x9C) / `pushfq` (0x9C trong 64-bit), rồi **nhìn lệnh ngay trước nó** — đó chính là phép
> toán thật (`add`, `and`, `cmp`, `shr`, …).

Bài gốc: **277 handler** xác định bằng cách này (trên 658).

### Heuristic 3 — `vpush` / `vpop` / `vbranch`

- `vpush`: đọc một qword từ ctx tại offset lấy từ bytecode, rồi `push` lên stack host.
- `vpop`: ngược lại.
- `vbranch`: đọc 1 byte "loại so sánh" từ bytecode, so với 16 mã JCC, set cờ `should_jump` trong ctx.

Sau khi phân loại, đặt tên tự động theo mẫu:

```
v{insn}_{size}_{index}     ; vd test qword ptr ds:[rsi], rdi → vm_test_qword_1337
```

Cách này giúp trace đọc được ngay: `vm_entry → save regs → vm_and → vm_or → vm_exit`.

---

## Bước 5 — Đọc virtual instruction

Mỗi handler đọc dữ liệu từ bytecode. Định dạng chung (theo bài gốc, và đúng với cả bản 32-bit):

```
+0x00 : payload riêng của handler (0, 2, 4 hoặc 8 byte)
+0x?? : u16  next_handler_index      ← nhân với sizeof(ptr) rồi cộng base bảng
+0x?? : i32  next_vip_delta          ← có dấu, âm được ⇒ nhảy lùi
```

Các offset này **khác nhau giữa các handler** — nên phải dò riêng từng cái. Mẹo của bài gốc: tìm đoạn
dispatcher (nơi handler tính handler kế), rồi lấy 3 điểm:

1. lần đọc `ctx->vip` để lấy **index**,
2. lần đọc `ctx->vip` để lấy **delta**,
3. lần **ghi** `ctx->vip` (VIP mới).

Ví dụ đo được (bản FALCON):

```asm
mov  rbx, rbp
add  rbx, 0xB2h        ; ctx->vip
mov  rbx, [rbx]
add  rbx, 2
mov  r8w, [rbx]        ; u16 → handler index
shl  r8, 3             ; *8 (con trỏ 64-bit)
...
movsxd rcx, [vip+4]    ; i32 delta (có dấu!)
add  [vip], rcx        ; VIP mới
```

Còn `vbranch` là VI phức tạp nhất, bài gốc vẽ hẳn sơ đồ byte (`vbranch_vi.png`) — **0x11 byte**:

| offset | size | ý nghĩa |
|---|---|---|
| `+0x0` | 2 | index handler nếu **nhảy** |
| `+0x2` | 2 | rác (không handler nào đọc) |
| `+0x4` | 4 | delta VIP (âm/dương) |
| `+0x8` | 2 | offset field ctx đem so sánh (luôn `0x40` = RFLAGS) |
| `+0xA` | 1 | loại so sánh (16 mã JCC) |
| `+0xB` | 2 | index handler nếu **không nhảy** |
| `+0xD` | 4 | kích thước VI (delta cho nhánh thường) |

> **Mẹo nhận dạng:** `movzx reg, word ptr [vip+N]` xuất hiện 5 lần ở đầu **mọi** handler trong bài gốc.
> Đó là "chữ ký" của VM. Bạn tìm chữ ký này trước, rồi mới suy offset.

---

## Bước 6 — Chạy và dừng đúng chỗ

Đây là phần newbie hay sai nhất, và cũng là phần mình vừa vấp thật.

### 6.1 Hai kiểu khởi động

| Kiểu | Khi nào dùng | Cần gì |
|---|---|---|
| **Snapshot replay** | VM cần trạng thái runtime (stack, object, TEB) — đúng với hầu hết block trong game | dump bộ nhớ tại đúng site |
| **Ảnh tĩnh + khung giả** | VM tự tính ctx/VIP từ hằng số | stack giả + TEB giả + FS qua GDT |

FS trong Unicorn 32-bit không set trực tiếp được; phải dựng **GDT** rồi trỏ `FS` vào descriptor có base =
TEB:

```python
def desc(base, limit, access, flags=0xC):
    d = (limit & 0xFFFF) | ((base & 0xFFFFFF) << 16) | (access << 40) | (((limit >> 16) & 0xF) << 48)
    return struct.pack("<Q", d | (flags << 52) | (((base >> 24) & 0xFF) << 56))
mu.mem_write(gdt+24, desc(teb, 0xFFF, 0xF2, 0x4))
mu.reg_write(UC_X86_REG_FS, (3 << 3) | 3)
```

### 6.2 Điều kiện dừng — 4 loại, dùng đúng loại

| Điều kiện | Khi nào | Ghi chú |
|---|---|---|
| `eip == cont` | block chạy thẳng tới cuối | chuẩn nhất, nhưng **không phải block nào cũng tới được** |
| `eip` rời vùng VM | gặp vmcall/vm_exit | đừng dừng ngay — VM còn quay lại |
| hết budget | để khảo sát | **không** được dùng để kết luận ngữ nghĩa |
| **chu kỳ lặp** | block là vòng lặp | dừng khi `(handler index, VIP)` lặp lại ⇒ đúng **một vòng** |

Mình đã dính đúng cái bẫy "budget": trace 1.5M lệnh, không tới `cont`, mà vẫn tưởng là đã chụp được
khối. Thực ra nó ghi **cả pool VM** — 3 khối khác nhau cho ra cùng một danh sách lệnh. Dấu hiệu nhận
biết: **nhiều khối khác nhau cho cùng `unique` count** ⇒ bạn đang đo pool, không đo khối.

Cách phát hiện chu kỳ:

```python
fp = (handler_index, vip)          # tại mỗi dispatch
if fp in seen: stop("cycle")       # lặp lại ⇒ xong một vòng
seen[fp] = dispatch_addr
```

### 6.3 Những gì cần log

- **ghi bộ nhớ**: chia 3 nhóm — ctx, image (biến global), heap/stack (object game).
- **chuyển nhánh gián tiếp**: `jmp/call [mem]` — đây là dispatcher; còn `jmp reg` là kiểu khác.
- **thanh ghi đầu/cuối**: để biết cái gì đã đổi.
- **các lần rời VM**: để biết có vmcall.

---

## Bước 7 — Lift thành code đọc được

Đây là đích của toàn bộ công việc: biến 600 dòng asm rối thành 20 dòng C.

### 7.1 Bỏ lớp truy cập ctx

Mọi handler đều làm `mov reg, ctx; add reg, imm; mov reg, [reg]`. Thay bằng truy cập trực tiếp
`ctx->field`:

```asm
mov  r14, rbp
add  r14, 0xB2h
mov  r14, [r14]        ; →  ctx->vip
```

Patch xong, **trace lại và so với bản gốc**: nếu khớp từng giá trị thì bạn đã bỏ đúng.

### 7.2 Bỏ dispatcher

Bạn biết chắc đoạn dispatcher ở đâu (3 điểm ở Bước 5). Xoá nó khỏi handler, rồi **nối các handler theo
đúng thứ tự luồng** — vì bạn đã biết index kế tiếp từ bytecode.

Kết quả: hàm ảo thành một dãy lệnh tuyến tính, IDA bấm **F5** là ra pseudocode.

### 7.3 Thay đọc bytecode bằng hằng số

Bytecode đã biết ⇒ thay `movzx reg, word ptr [vip+4]` bằng `mov reg, <giá trị thật>`. Việc này làm code
ngắn đi rất nhiều và xoá luôn sự phụ thuộc vào VIP.

### 7.4 Với VM kiểu "threaded" (code bị xé vụn)

Nếu mỗi handler chỉ dài 3–8 lệnh rồi `jmp` sang chỗ khác (bản 32-bit: **trung bình 6.66 lệnh**, 63.6%
đoạn ≤6 lệnh), thì "handler" không còn là đơn vị. Lúc đó:

1. Trace một vòng lặp (Bước 6.2).
2. Lọc **lệnh có ngữ nghĩa**: chỉ giữ lệnh chạm ctx / global / heap. Bản 32-bit: **36 / 7094 lệnh**
   (~0.5%) — tức 99.5% là rác.
3. In kèm offset ctx để đọc:

```
0x011f199f  add   dword ptr [edx], ebp      ; ctx+0x84
0x011f19a2  shl   dword ptr [edx], 4        ; ctx+0x84
0x011f19a5  shr   dword ptr [edx], 1        ; ctx+0x84
```

Ba dòng đó đọc là: `ctx->field84 = (ctx->field84 + ebp) * 8`.

---

## Bước 8 — Kiểm chứng

Không kiểm chứng thì bạn chỉ đang viết truyện. Ba mức:

1. **Trace khớp**: chạy bản đã patch, so từng bước với bản gốc — bài gốc làm đúng vậy và khoe "exact same
   flow, exact same values".
2. **Diff trạng thái**: chụp bộ nhớ trước/sau khối (frame + `.data` + object), so bản thật với bản dịch.
3. **Chạy thật**: build lại và chạy chương trình, xem hành vi có như cũ.

---

## 12. Hai đời VM: chained và threaded

Biết mình đang gặp đời nào sẽ tiết kiệm hàng tuần.

| | **chained** (FALCON, 2024+) | **threaded** (đời cũ, ~2010s) |
|---|---|---|
| Đơn vị | 1 handler = 1 basic block liền mạch | mảnh 3–8 lệnh, nối bằng `jmp` |
| Dispatch | `jmp [table + idx*8]` | `jmp [ctx + idx*4]` (bảng trong ctx) |
| Bảng handler | trong section, fixup lúc entry | có thể nằm trong ctx, tĩnh |
| Số handler | ~658 | ~178 |
| Cách bóc | phân loại handler là đủ | **phải trace**, rồi lọc lệnh có ngữ nghĩa |
| Junk | vừa | rất nhiều (99%) |

Cách phân biệt nhanh: chạy 100k lệnh, đo **độ dài đoạn thẳng trung bình**. > 15 lệnh ⇒ chained.
< 10 lệnh ⇒ threaded.

Bảng so sánh thật (bài gốc vs bản 32-bit mình đo):

| | FALCON (bài) | Bản 32-bit (đo được) |
|---|---|---|
| lock | `ctx+0xD3` | `ctx+0x7C` |
| ctx size | `0x178` | ≥ `0xF48` (bảng nằm trong) |
| VIP | `ctx+0xB2` (byte, không align) | `esi` (bytecode con trỏ) |
| bảng | `ctx+0xD7`, 658 entry × 8 | trong ctx, 178 entry × 4, idx = offset/4 |
| độ dài đoạn thẳng | liền mạch | 6.66 lệnh (median 5) |

---

## 13. Bẫy thường gặp

**1. Chưa unpack mà đã làm.** Kiểm tra 30 giây: section VM có entropy ~8, `.idata` bé, entry nằm trong
section vài trăm byte ⇒ **đang pack**. Devirt trên file pack là vô nghĩa. Phải dump tại OEP trước.

**2. Marker `EB 10 57 4C 20`.** Nhiều bản Themida kết thúc mỗi khối VM bằng marker này (`jmp $+0x12` qua
16 byte, có chữ `WL `). Nó cho bạn **biên khối**, nhưng:
- marker = cuối khối **không phải lúc nào cũng đúng từng byte** (có bản lệch 2–3 byte);
- code gốc tiếp tục ở `marker + 0x12` **chỉ khi** may mắn.

Luôn kiểm tra `cont` bằng disasm: nếu nó ra `pop/pop/leave/ret` thì đúng.

**3. Budget không phải là kết thúc.** Đã nói ở Bước 6: nếu nhiều khối cho cùng số lệnh unique, bạn đang
đo pool.

**4. Block loop thì không có `cont`.** Keep-alive/poll block có thể chạy mãi trong VM. Dùng chu kỳ.

**5. Đừng skip hàm anti-cheat mà VM kiểm tra kết quả.** Trong repo gốc từng gặp: skip một hàm thì VM rẽ
nhánh phá stack (ghi 0 từ đỉnh stack xuống). Nếu VM *dùng* giá trị trả về, phải giả lập đúng.

**6. Bảng có thể nửa fixup.** Xem cảnh báo Bước 3.

**7. 32-bit ≠ 64-bit.** `pushfd` vs `pushfq`, `*4` vs `*8`, không có `r8–r15`, `movsxd` biến mất. Mọi
offset phải dò lại, **không bê nguyên từ bài**.

**8. Con trỏ tĩnh trong code.** Bản 32-bit có `image_base + 0x1242D0` — khi patch/relocate phải tính lại.

**9. Đừng tin `unique` count như kích thước khối.** Nó là pool.

**10. VM synchronous ⇒ đừng chạy đa luồng.** Cờ khoá là thật; giả lập hai luồng sẽ deadlock.

---

## 14. Case study: bản 32-bit đời cũ

Tóm tắt những gì đo được, để bạn có mốc so sánh khi làm bài của mình.

```
PE32, imagebase 0x400000, section VM ~2.6 MB + 2.2 MB
entry 0x9E5EFE, 73 marker 'EB 10 57 4C 20'
607 site jmp vào vùng VM → 390 stub head khác nhau
  (head phổ biến nhất xuất hiện đúng 73 lần = 73 marker)

vm_entry 0x11EE128: pushfd + junk stack
spinlock 0x11EECB0: lock cmpxchg [edi+0x7C], ecx
  ⇒ ctx = 0x11EDDEC, sync field = ctx+0x7C

dispatch: jmp [edi + eax*4]      (eax = idx, đã nhân sẵn)
  đo: eax=0x43 → ctx+0x10C → 0x11F6CB8
      eax=0x81 → ctx+0x204 → 0x11F7467
      eax=0x56 → ctx+0x158 → 0x11F5F30

bảng handler: ctx+0x94..ctx+0xF48, 178 entry
  giống hệt qua 3 snapshot khác nhau VÀ khớp byte trong file ⇒ tĩnh

phân rã: mean run 6.66 lệnh, median 5, 63.6% ≤6 lệnh
lift: 36 / 7094 lệnh unique có ngữ nghĩa (chạm ctx/image/heap)
```

Bài học lớn nhất: **bảng tĩnh + code threaded** ⇒ enumerate offline được, nhưng **ngữ nghĩa phải lấy bằng
trace**, và **điều kiện dừng là thứ quyết định đúng/sai**, không phải thuật toán.

---

## 15. Lộ trình 3 tuần cho newbie

**Tuần 1 — làm quen, chưa cần bóc gì**

- Ngày 1–2: cài IDA + unicorn/capstone; viết script giả lập chạy 1 hàm x86 đơn giản.
- Ngày 3–4: tìm section lạ + site `jmp` trong một binary có Themida. Đếm site, đếm marker.
- Ngày 5–7: tìm ctx bằng 3 cách ở Bước 2 trên **chính binary của bạn**. Ghi lại offset.

**Tuần 2 — hiểu VM**

- Ngày 8–10: tìm bảng handler; xác định tĩnh hay cache (3 snapshot).
- Ngày 11–12: phân loại handler bằng 3 heuristic; đếm được bao nhiêu %.
- Ngày 13–14: đọc VI của 5 handler đơn giản, vẽ layout byte.

**Tuần 3 — lift**

- Ngày 15–17: trace một khối **không loop**, tới `cont`, log write.
- Ngày 18–19: lọc lệnh có ngữ nghĩa; viết lại bằng C; so trace.
- Ngày 20–21: làm một khối loop; sửa điều kiện dừng; làm lại.

Tiêu chí "xong": bạn dịch được **một** khối, và chứng minh được bằng trace khớp.

---

## 16. Phụ lục

### A. Checklist nhanh

```
[ ] binary đã unpack chưa?           (entropy section, .idata, entry)
[ ] đã tìm được site jmp vào VM?
[ ] đã tìm được ctx? offset cờ khoá?
[ ] đã xác định dispatch: jmp [base+idx*k] ?
[ ] bảng handler: tĩnh hay cache? bao nhiêu entry?
[ ] phân loại: exit / flag-op / vpush / vpop / branch?
[ ] VI layout của ít nhất 3 handler?
[ ] trace có dừng đúng (cont hoặc cycle) chưa?
[ ] lệnh có ngữ nghĩa / tổng lệnh = ?
[ ] đã so trace bản dịch với bản gốc?
```

### B. Lệnh hữu ích

```bash
# tìm section + entropy
python3 -c "import pefile,math,collections; ..."

# tìm site jmp vào VM (xem Bước 1)

# tìm marker VM_END
python3 -c "d=open('x.exe','rb').read(); import re; print([hex(m.start()) for m in re.finditer(rb'\xEB\x10WL ', d)])"
```

### C. Đọc thêm

- Bài gốc (bắt buộc đọc, có ảnh minh hoạ từng bước): <https://blog.shablyko.com/posts/themida-falcon-devirt/>
- Unicorn API: <https://github.com/unicorn-engine/unicorn>
- Capstone: <https://www.capstone-engine.org/>
- Nếu muốn tự động hoàn toàn hơn: tìm tài liệu về *VMProtect 2 devirtualization* (cùng họ ý tưởng, nhưng
  VMProtect dùng dispatcher loop rõ ràng hơn — dễ làm quen trước).

### D. Từ điển nhanh

| Anh | Việt | Ghi nhớ |
|---|---|---|
| handler | — | một "động tác" x86 của VM |
| dispatcher | bộ điều phối | chọn handler kế |
| context | ngữ cảnh | struct trạng thái VM |
| VIP | con trỏ lệnh ảo | trỏ vào bytecode |
| VSP | con trỏ ngăn xếp ảo | stack của VM |
| lift | nâng cấp | asm → IR/C |
| threaded code | code dạng sợi | block nhỏ nối bằng jmp |

---

*Viết cho repo `boom-re`, nhánh `feat/vmdevirt`. Số liệu "bản 32-bit" lấy từ đo thật trên
`unpacked_ca.exe` + `logs/snap/*/pre.snap`; số liệu FALCON lấy từ bài gốc của Denis Shablyko.*
