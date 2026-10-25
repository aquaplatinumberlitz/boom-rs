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
