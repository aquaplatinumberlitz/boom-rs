# RE tĩnh KR 00967 — bộ đếm bomber → chết, chạm/chọc và biên chuyển life

Kết quả đọc tĩnh client KR `00967` cho [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147) /
Q-018 (mục 4 hàng đợi RE: timer nhốt → chết, chạm/chọc, biên chuyển life). Claim ở
[RULES_V1 §19](../../specification/gameplay/RULES_V1.md#19-re-kr-00967--bộ-đếm-bomber--chết-r-200r-202)
(R-200…R-202; phiên này dùng dải R-200…R-249). Định danh build, bảng VA/RVA và `evidence_id` ở
[evidence KR trap timer](../../evidence/measurements/kr-static-trap-timer-2026-10-23/README.md).

**Kết luận chính:** tìm được **cạnh native** của một bộ đếm trên object bomber: trường `+0x2748`
giảm 1 mỗi lần update, về `≤ 0` thì gọi virtual slot 202; ở `CGOGameBomber` slot 202 (`0x00B7A760`)
gửi **C→S packet 167** (báo chết) và reset trường. Trường này cũng được đưa vào **status packet 184**
(dưới dạng byte "timer đang chạy"). **Chưa** chứng minh đây là "timer nhốt": không có store giá trị
dương nào trong code native (quét đủ 10 site của displacement `0x2748`), và vị từ trạng thái thứ hai
trong update là thunk Themida `0x00B534D0 → 0x02012376`. Vì vậy R-201 là **blocker có phạm vi**, không
suy ra thời gian nhốt/ms.

## 1. Phạm vi, build, phương pháp

| Mục | Giá trị |
|---|---|
| Build | KR `00967`, module đã unpack `kr-full.exe`, 26.074.624 B |
| SHA-256 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` |
| Image base | `0x00400000`; `RVA = VA − 0x00400000`; x86 32-bit |
| Đường chính | REA MCP `provider_id: "ida"` trên **bản sao** binary (`procedure_pseudo_code`, `xrefs`) |
| Fallback | IDA MCP trực tiếp trên **bản sao** `.i64` cho `read_bytes`, quét byte (displacement), đọc vtable, disasm thunk VM — REA adapter thiếu `read_bytes`/`search_bytes`/`list_names` |
| Không chạy | không chạy client, không kết nối server, không mở DB gốc, không sửa binary |

Không chạy hai analysis worker song song: IDA (quét/đọc byte) → đóng → REA (decompile + evidence) →
export bundle → đóng.

## 2. Anchor: bộ đếm trên bomber

```
0x006833D0  bool Pred(this)          { return this[2514] > 0; }      // 0x2748 / 4
0x00B44000  CGOBomber::Update(this)                                  // slot 24 vtable 0x01744964
    if (Pred(this) || sub_B534D0(this)) {
        sub_621A20(this + 0x272C);
        --this[2514];                       // 0x00B4409A..0x00B440A6
        (*(*(this + 0x0D74) + 0x60))(this + 0x0D74);
        if (this[2514] <= 0)                // 0x00B440C7..0x00B440DE
            (*(*this + 0x328))(this);       // slot 202
    }
0x00B77B10  CGOGameBomber::Update                                    // slot 24 vtable 0x01745FE8
    ... ; return sub_B44000(this);           // call site 0x00B77E22
0x00B7A760  CGOGameBomber slot 202
    this[2514] = 0;
    if ((*(this+356->vtbl+56))(this+356) != 0) {
        ctx = sub_E704A0(sub_60A400());
        ctx->vtbl[12](ctx, 1);
        CSendPacket(167);                    // 0x01164C10
        sub_14EBCF0((*(this+353->vtbl+40))(this+353));   // giá trị 1
        sub_14EBCF0(6);                                  // hằng 6
        send();                              // 0x01164D50 / 0x01164C80
    }
0x00B5D630  CGOBomber slot 202           // rỗng (no-op)
```

Ghi chú:

- `this[2514]` là **dword tại byte `+0x2748`**; chỉ số dword 2514 × 4 = 10056 = `0x2748`.
- Update chỉ chạy nhánh đếm khi **hoặc** trường `> 0` **hoặc** `sub_B534D0(this)` trả true. Nếu
  `sub_B534D0` là "đang ở trạng thái bị khoá" thì khi vào trạng thái với trường `= 0`, update sẽ
  giảm xuống `-1` và gọi chết **ngay** — điều này chỉ hợp lý nếu seed được đặt dương cùng lúc vào
  trạng thái (xem §4).
- `CGOBomber` slot 202 là hàm rỗng; chỉ lớp dẫn xuất `CGOGameBomber` mới gửi packet. Vì vậy cạnh
  "chết do bộ đếm" áp cho **bomber người chơi/AI dùng `CGOGameBomber`**, không phải mọi object kế thừa
  `CGOBomber`.

## 3. Consumer thứ hai: status packet 184

`0x00E84460(this, stream)` duyệt danh sách bomber của context (`0x00E12C40(i)`), với mỗi bomber ghi
4 byte:

```
v4 = Pred(bomber)                 // 0x006833D0 — byte "bộ đếm đang chạy"
v5 = *(bomber + 0x5E8)            // sub_B48580
v6 = *(bomber + 0x24DC)           // sub_B4FC90
v7 = f(bomber + 0x804, v4, v5)    // sub_B64F40 → sub_57A870(this+144) (đếm phần tử)
stream: [1, v7, v6, v5, v4]
```

`0x00E67E70` gọi `0x00E84460` rồi mở `CSendPacket` type **184** (`push 0B8h` tại `0x00E68201`). Như
vậy trạng thái "bộ đếm đang chạy" **rời client** trong status packet 184, không chỉ là biến cục bộ.
Không kết luận server dùng byte này thế nào (chưa có handler server/packet capture).

## 4. Setter/seed: không có trong code native (R-201)

Quét toàn image mọi encoding của displacement `0x2748` (`[reg+2748h]`, `lea reg,[reg+2748h]`,
`add reg,2748h`): **10 site** (chi tiết bảng ở evidence). Trong đó:

| Vai trò | Hàm / địa chỉ |
|---|---|
| đọc (predicate) | `0x006833D0` (`0x006833DA`) |
| đọc–giảm–ghi (RMW) | `0x00B44000` (`0x00B44098`/`0x00B440A4`) |
| zero-init | `0x00B3DCB0` (`0x00B3DFD6`), `0x00B3D1D0` (`0x00B3D70D`), `0x00B58B60` (`0x00B59786`), `0x00B7A760` (`0x00B7A78F`), `0x01036700` (`0x01036718`) |
| không phải code | dword trong bảng `0x017A60CC` |

Không có site nào ghi **giá trị dương** vào trường, và không có `lea`/`add` nào tạo địa chỉ trường
(ngoài các site trên). Hệ quả: seed của bộ đếm (thời lượng) và **điểm vào trạng thái** không nằm
trong code native đã thấy.

Vị từ thứ hai của update, `sub_B534D0` `0x00B534D0`, là thunk Themida: prologue native
(`push ebp; mov ebp,esp; sub esp,0Ch; ...`) rồi `jmp near ptr 0x02012376`; đích nằm trong segment
`________` (`0x01D7B000–0x02076000`) và byte tại đích là `0xFF` (bytecode/VM, không phải code native).
Cùng thunk được gọi từ `0x00B53330`, `0x00B46760`, `0x00B46830`, `0x00B46940`, `0x00B78780`,
`0x00682790`. Theo quy ước [DEVIRT_METHOD](DEVIRT_METHOD.md): **ghi entry VM và dừng nhánh này** —
không suy thời gian nhốt.

## 5. Chạm/chọc và thứ tự (R-202)

- R-028 (`vn_runtime`/confirmed): chạm địch → chết. Lượt này **không** đọc được khoảng cách/hộp/ngưỡng
  chạm trong KR, cũng không nối được nhánh chạm vào `0x00B7A760`; `0x00B7A760` chỉ có xref từ vtable
  (slot 202) và từ `0x00B78780` (virtual khác).
- Thứ tự trong một bước: bộ đếm nằm trong `CGOBomber::Update` (`0x00B44000`), được
  `CGOGameBomber::Update` (`0x00B77B10`) gọi ở **cuối** hàm. Áp vụ nổ/lửa nằm ở đường map
  (`0x00E9E660`/`0x00E9EEE0`, R-139…R-141). Vòng tick của stage quyết thứ tự gọi giữa hai đường; lượt
  này **chưa trace** nên không claim "hết hạn trước/sau chạm/lửa".
- Cứu đồng đội/kim: chỉ có chuỗi asset (`Item/AT_Needle`, `useneedle`), chưa có consumer — giữ
  `candidate`/`unknown` như R-131 §6.

## 6. Defer theo tên: xác nhận lại R-131

`kCGameStage_DeferBomberLockedIn` (`0x017953C8`) và `kCGameStage_DeferBomberReleaseOrDie`
(`0x01795074`) vẫn chỉ xuất hiện trong bảng đăng ký tên của stage (`0x01261F50` đăng ký 44 tên + id;
`DeferBomberReleaseOrDie` = id 1, `DeferBomberLockedIn` = id 30). Không có xref nào khác trong code ⇒
bảng tên **không** trỏ tới thân hàm (R-131 đúng). Lượt này **không** dùng tên defer làm bằng chứng
cho bộ đếm; cạnh `+0x2748` được tìm độc lập từ update của bomber.

## 7. Khác biệt với profile hiện tại và với BnB

| Khóa / claim | Hiện tại | KR đọc được | Đề xuất |
|---|---|---|---|
| `player.trap_to_death_ms` (R-115) | `4000` (`oss`/design) | bộ đếm **1 đơn vị/update**, chết khi `≤ 0`; **seed unknown** (VM) | **giữ nguyên**; chỉ đổi khi có seed + nhịp update KR |
| Timer nhốt (R-131) | `unknown` | thêm cạnh đếm→packet 167 và status 184; entry/seed vẫn `unknown` | giữ `unknown`; Q-018 thêm bước devirt/VM hoặc runtime |
| Chạm/chọc (R-028) | VN: chạm → chết | KR chưa định lượng | giữ; không suy |
| Packet chết/status | chưa có claim | client gửi **167** (chết) và **184** (status có byte timer) | cần đối chiếu server trước khi đổi contract V1 |

Không sửa `data/rules/*.toml` trong PR này.

## 8. Giới hạn

- Chỉ tĩnh; không chạy client, không có packet capture/VM devirt.
- Chưa chứng minh `+0x2748` là **timer nhốt**: chưa nối state 3/`IsLockedInState` với trường này;
  nhãn này là `candidate`.
- Chưa đọc nghĩa `sub_B534D0` (VM), `sub_B64F40`, `sub_B4FC90`, `sub_B48580`; chỉ dùng vị trí/byte.
- Chưa xác định nhịp update của bomber trên KR (30 ms/bước là đo trên VN, R-013) nên không quy đổi ms.
- Packet 167/184 chỉ đọc phía client; chưa có handler server hoặc log runtime để xác nhận nghĩa.

## 9. Bước tiếp

1. Devirt/đọc VM tại `0x00B534D0 → 0x02012376` (theo DEVIRT_METHOD) để lấy seed + điều kiện vào
   trạng thái; nếu không làm được thì đo runtime theo Q-009 (entry trường, state, thời điểm packet 167).
2. Trace consumer của `sub_B534D0` trong `0x00B78780`/`0x00B53330` để gán nghĩa "khoá/bị nhốt".
3. Tìm nhánh chạm/chọc trong stage (so khoảng cách/hộp giữa 2 bomber) và thứ tự gọi so với update map.
4. Đối chiếu packet 167/184 với server VN/KR để chốt contract (không đổi V1 trước khi có server).
