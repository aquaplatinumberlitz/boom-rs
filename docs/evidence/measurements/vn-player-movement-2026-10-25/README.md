# VN player mover: dữ liệu đầu vào và hai ca rẽ (R-176)

Nguồn VN VBNB_116, fixture SHA-256
`bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, base400000.
Fixture có IAT/devirt/offline reconstruction; chưa chứng minh tương đương vanilla. VN được dùng
theo quyết định chủ dự án/ADR0002/0013, không yêu cầu đo lại KR cho cùng hành vi. Phạm vi ca:
Factory01, room19, một local slot0; actor449278600/vtableC664C4, không suy C00/C01 KR.

[Metadata/payload, trajectory và60hash](player-movement.json);
[13 endpoint native](movement-controls.csv). Raw/game/ảnh/asm dài chỉ giữ local.

## Điểm quan sát thuộc actor người chơi

R-171 đã chụp call v244 trên actor này cùng loại. Vtable C664C4+244 là **slot C66708**, chứa59DA6F;
không dùng C66708 làm base vtable. Entry có nhánh trả sớm; nhánh59DABE gọi594B68 với outptr
EBP−90 tại59DAD4, rồi chuyển dữ liệu này tới90019A. Bốn block/prefix tĩnh giữ523/864byte;
không gọi chúng là bốn full function hoặc kết luận toàn mover từ nhánh trả sớm.

Rare breakpoint **59DAD9**, sau return594B68, được bật sau khi initializer kết thúc, rồi tự tắt
sau một capture hợp lệ. Cùng actor/thread, native X/Y=560/519; out16 giải thành bốn signed dword
**`[560,480,40,40]`**, tức `[X,Y−39,40,40]` tại mốc này. Static helper đọc X/Y qua58D56D/58D575,
dùng Y−39 và hai local40 trước callA21520. EDI là pointer có dword5, EBX=1; giữ vai trò hai đối số
opaque trong kết luận này. Return thực59D908; movementframe khác initializerframe.

Đã xác nhận **dữ liệu argument trên mover được quan sát**. Cách hiểu đó là rectangle mô tả
toàn bộ collision hẹp, damage/hurtbox hoặc hitbox vuông40×40 vẫn chưa chứng minh. Không đổi
radius13/assist12 của profile từ tuple này; không gán số PET/AI MoveSimple cho bomber KR.

## NativeXY và đối chứng hướng

Sau bootstrap có S191, getter X/Y hợp lệ và các hook khởi tạo đã tắt. Probe chỉ sáu stop trong
toàn lượt; lấy2.952 dòng XY bằng reader ngoài process, không gọi inferior hay ghi bộ nhớ game.

| Điều khiển | Native XY quan sát | Giới hạn |
|---|---|---|
| Up từ spawn | (560,519)→(560,39) | Không quy đổi thời gian giữ phím thành vận tốc |
| Lặp Up tại endpoint | giữ(560,39) | Chỉ stop response trong ca, chưa là collider toàn mode |
| Right tại endpoint | giữ(560,39) | Có opposite-direction control tiếp theo |
| Left rồi Right | (420,39) rồi(560,39) | Native chuyển động thật, không chỉ keydown marker |
| Down rồi Up khi X560 | Y174 rồiY39 | Down hoạt động trên lối này |

Hai ca rẽ đều bắt đầu với X/Y ổn định trước lệnh Down. Hàm move_to yêu cầu X550/X540 nhưng
vị trí thực đạt **X540/X535**; giữ nguyên overshoot, không đặt lại tọa độ bằng bộ nhớ.

| Ca | Native trước Down | Chuỗi quan sát khi chỉ có lệnh Down | Endpoint trước reset |
|---|---|---|---|
| A | (540,39) | X540→545→550→555→560 rồi Y tăng | (560,109) |
| B | (535,39) | Mẫu đầu cửa sổ Down đã là540, sau đó545→550→555→560; tính từ mốc trước là25px | (560,104) |

Trong cửa sổ Down không có Right keydown; trước đó đã nhả Left, nativeXY ổn định. Metadata
giữ trajectory và khoảng đọc; ở caB không gọi20px trong các mẫu sau marker thành25px được
đo hoàn toàn sau khi input native nhận. Up/Right reset sau từng ca trở về(560,39).

Quan sát sửa hướng ngang khi điều khiển xuống hỗ trợ **candidate corner assistance** trên đường
player thật. Chưa có native direction flag hoặc negative offset control để chốt consumer/ngưỡng
tối đa; không tuyên bố max25/34, snap tức thời hay cùng hành vi trên mọi vật cản/nhân vật.
Không lấy timestamp packet/keydown làm thời điểm hành vi native hoặc phép đo latency/velocity.

## Kiểm và bước tiếp

Header12/payload10 của X/Y tại rare stop được giải mã lại, kiểm cookie/checksum. Có60 raw/static
hash, namespace user/net/PID/mount riêng, actual socket chỉlo; không111/117/120/151/158.
Run `host-char-key-upstream-149-r1` đóng stop4/exit0, giữ mọi raw kể cả overshoot/endpoint ban đầu.
Khởi tạo cùng actor là5/7/5; key0 và caller6EB8A3 chỉ là context, không bridge identity KR.

Bước tiếp là negative offset control cạnh ca thành công hoặc consumer90019A/rectangle có điểm
quan sát cụ thể, không mở lại PET/AI/full scan. #146 vẫn PARTIAL: hurtbox, ngưỡng assist, speed
level→time model, diagonal và non-owner chưa đủ. Profile, gate và registry không đổi.

## Đối chứng lệch30 và35 trên cùng actor — R-177

[9 endpoint native](corner-boundary.csv), [trajectory, payload và42hash](corner-boundary.json).
Run riêng `host-key-producer-type-149-r1`, actor449280160/C664C4, cùng fixture/map/room như trên.
Base/max/current của **chính actor này** là5/7/5. Không ghép pointer hoặc clock với actor R-176.
Toàn bộ hook khởi tạo đã tắt trước điều khiển; không movement breakpoint trong lượt này.

| Ca | Requested X | Actual trước Down, Y39 | Native trong cửa sổ chỉDown | Kết quả trong ca |
|---|---|---|---|---|
| Lệch30 | 535 | 530 | X530→535→540→545→550→555→560, sau đó Y139 | Có sửa ngang và đi xuống |
| Lệch35 | 530 | 525 | Giữ(525,39) | Không di chuyển |
| Aligned positive | — | 560 | Down→(560,164), Up→(560,39) | Cùng actor vẫn di chuyển được |

Giữ overshoot requested535/530→actual530/525. Trước mỗi lệnh Down có XY ổn định; bộ kiểm dùng
các mẫu trong100ms trước SNAP tới Down, hai mẫu hợp lệ mỗi ca. Riêng khoảng hẹp SNAP→Down của
ca âm chỉ có một mẫu; không xóa khác biệt này hoặc dùng số mẫu làm ngưỡng. Trong cả hai cửa sổ
Down không có Right keydown; các lệnh Up/Right reset và positive aligned được giữ trong raw.

**Confirmed**: phản ứng native ở offset30 khác35 trong đúng actor/current5/ca này. Cách hiểu
đó là corner assistance vẫn `candidate` khi chưa trace đầy đủ consumer hoặc native direction flag.
Chưa xác nhận ngưỡng chính xác34, tất cả offset giữa30–35, mọi hướng/tốc độ/mode, hurtbox hoặc
hitbox40×40. Không đổi assist12/radius13 hay công thức tốc độ của profile.

Run đóng stop4/exit0,4.387XYrows/42filehash,6hook/5stop trước movement; S191, only-lo và các
đối chứng đã kiểm độc lập. Không111/117/120/151/158. Timing host/packet/debugger không được dùng
để suy velocity/latency hoặc thời điểm native nhận phím. Attempt đọc field context qua helper
ngoài debugger không lấy được snapshot; được giữ local riêng và không dùng làm proof cho ca này.

Bước tiếp cho #146 là consumer gây phản ứng khác nhau hoặc một ca có ngưỡng/đơn vị cần phân biệt,
không lặp hai offset để tìm lại kết quả. Những nhánh geometry khác vẫn giữ unknown rõ ràng.
