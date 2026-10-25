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

## Predicate ô passable của player — R-179

Câu hỏi đã chốt bằng code VN: nhánh trong actual player mover59DA6F lấy object theo tọa độ ô,
lọc/cast `CGOTileFixed→CGOTilePassable`, rồi đọc byte cho hướng để đặt cờ chặn. Đây là **một nhánh
restriction của tile passable**, không phải toàn bộ collision hoặc hurtbox. Anchor player/vtable
đã có R-176; lượt này chỉ đọc bản sao PE SHA `bc875ba5…`, không chạy lại client/góc30–35.

Nguồn chuẩn có [metadata/hash](cell-passability.json) và [bốn case](cell-passability.csv):

1. `59DD50→7293B4`: context từ `actor+2694→6E206E`; đọc
   `DWORD[context+B328+(row*101+column)*4]`. Các guard và helper kiểm tra được ghi chính xác
   trong metadata; không gán nghĩa cho thân callee chưa đọc. Call729402 nhận RTTI label
   `CGOTileFixed/CGOTilePassable`; EAX của cast được giữ tới ret8. Nhánh guard-fail72940C trả0.
2. `59DD5E→5C1027→5C0E98`: bốn dir0/1/2/3 trả unsigned byte
   `BYTE[DWORD[tile+AC]+4+dir]`; dir khác trả0. Không tự đặt tên hướng trong claim này.
3. Return0 đặt `[EBP-34]=1`; caller đi nhánh kẹp rectangle biên và callback actor.vFC. Return
   nonzero chỉ bỏ qua restriction này. Nhánh virtual actor+AC và các kiểm tra geometry khác còn riêng.

Bundle có **10 instruction units, 1.372B requested, 189B retained cộng dồn/179B code duy nhất**;
4B epilogue predicate và6B epilogue lookup bị đếm lặp giữa clips, đã ghi rõ. Tổng32 hash của clips,
cache caller và data RTTI được root đóng băng ngoài repo. Case logic và epilogue phủ control flow
cục bộ của hai hàm cùng một thunk; callee/transitive semantics không được gọi là đã đọc hết.
Lượt triage A21800 trước đó có code lân cận objdump trả thừa, đã giữ/sửa ledger riêng; không dùng
phần đó làm proof cho R-179 hoặc giấu thành một thân. Không publish rawasm/pack/binary.

Kết luận `vn_static / confirmed` cho dataflow/byte predicate này; **chưa quan sát runtime** rằng
nhánh tile passable đã chạy trong hai ca R-176/R-177. Chưa chốt giá trị property, producer, native
hurtbox, ngưỡng corner tối đa, mọi mode hoặc C00/C01. Profile,12candidate,G2 và tiến độ giữ nguyên.

## Curve tốc độ cùng actor — R-183/R-184/R-185

Câu hỏi #296/#146/#149: logicalpoint đã nối source có tương ứng tốc độ thực nào ở4..8,
và coefficient24 V1 có còn phù hợp? [Metadata/mọi hash](speed-curve.json),
[12window native](speed-curve.csv). Không thêm staticbody; reuse R170/171/173/180/182,
ưu tiên RE để xác định actor/field/consumer, đo trực tiếp VN khi chưa có clock cùng đại lượng.

Run chính `host-c01-speed-curve-146-r2-tail-control`, actor449284224/C664C4/controllerC6630C,
key1/C01/base4/max8, Factory01map193/room19, fixturebc875… như đầu tài liệu. Bốn native
715E90entry→71B717return theo đúng actor/thread cho4→5→6→7→8; mỗi phase có motionDown/Up,
baseline4 lặp thêm một cặp. Có4C117/4S158kind2 và0C111/C120/S151. ActualpickupY239/204/156/127;
không dùng target điều khiển hoặc lịch server làm tọa độ đo. Server giả chỉ tạo item/ACK,
ROUND_TIME600EMU không phải velocityclock; không ghi field game/gọi inferior.

| Logicalpoint | Window | Mean OLS native px/s | Mean/point | Engine33 px/s |
|---|---|---|---|---|
| 4 | 4 | 133,698888 | 33,424722 | 132 |
| 5 | 2 | 166,077857 | 33,215571 | 165 |
| 6 | 2 | 200,698866 | 33,449811 | 198 |
| 7 | 2 | 233,232185 | 33,318884 | 231 |
| 8 | 2 | 267,052717 | 33,381590 | 264 |

Reader ngoài process giải header12/payload10/cookie/checksum của X/Y/base/current/max;900row,
0invalid được giữ, không gọi accessor có tái mã hóa. Sampler bỏ snapshot scalar hoàn toàn không đổi,
không suy cadence từ khoảng row.9stop có begin/end clock,3hook; không window nào chồng breakpoint.
Cả sourceprobe4ec0f1f3…/harnessebc5cf07…/serveref688ad5… được băm trong40rawfile; raw chỉ local
clone1 `.re-work/vn-grace-240/`. Protected payload transient không bị xóa; ca này không rowinvalid.

**Clock/window:** HOST CLOCK_MONOTONIC begin/end mỗi memoryread, dùng midpoint để fit nativeXY;
không clock nội bộ client/packet/keydown. Trong từng MOVE_TO/MOVE_DONE scope lấy first/last
nativeYchange rồi trim150ms mỗi đầu; giữ bracket trọn window, >=1s, actor/type/point đúng.
Thuật toán được chọn sau R1 rồi áp không đổi cho R2; không giả preregistration. Dòng ngoài window
vẫn trong raw, không gán tất cả là acceleration. OLS=abs(sum((t−mean_t)(Y−mean_Y))/sum((t−mean_t)^2));
CSV giữ first-last và Theil để đối chiếu. Trung bình các window theo point, rồi normalizedmean/point;
không lấy medianpoint4 cho cột ghi mean. Root tái tính khớp DSH2 dưới1e−6 và DSH1 kiểm manifest.

**Sai số/giới hạn:** full difference fit begin/end bracket khoảng0,001–0,078px/s, chỉ phản ánh thời gian
đọc. MaxobservedΔY21 có thể alias nhiều update, không native tick. Sensitivity mô hình bảo thủ
±(2×21/windowspan + full read-bracket difference) khoảng31–39px/s, không95%CI hoặc độ chính xác
của hệ số. Các endpointbands riêng dùng maxstep từng window cũng giữ ởJSON, không lẫn với global21.
Đường gần tuyến tính trong ca hỗ trợ hiệu chuẩn; không chứng minh33 khác34 hoặc native hằng33/FPS.

**R1 giữ riêng:** `host-c01-speed-curve-146-r1`, actor449268984,34rawfile, valid1235row/0invalid,
point4/5 có window nhưng fullcurve thiếu. Controller đọc toàn log chậm, overshoot và lastACK2 bịACK3
ghi đè làm waittimeout dù appendlog cóACK2.7stop/exit2, không9stop/ackcomplete; không gọi đó là game
không đạt point6/7. Không gộp actor/tính trung bình với R2. R2 thay selector boundedtail/scalar-dedup,
settle trướcorigin, ACKappendlookup và targets điều khiển; probe/typedfield/guards không đổi.
Offline controlbenchmark giữ riêng, không số tốc độ native. Thay đổi phương pháp có mock/independent
preflight, không rerun mù. Mọi34hashR1 và40hashR2 trongJSON, kể cả lỗi/overshoot.

**Kết luận:** R183confirmed chỉ scopedrates; R184DESIGN33 chọn nguyên gần normalizedcentral33,358116,
thay24 (chậm khoảng28% trong ca), engine4..8 lệchmean−0,65…−1,34%. Point1..3/actor/mode khác là
ngoại suy DESIGN. R185 xác nhận+1logicalpoint từ bốn nativeACK cùng source R173; không+1px/s/cap
mọi mode. Không thay trapped17/hurtbox13/assist12/30Hz hoặc claimnativeFPS. Hai run đã dừng,
R2stop2/exit0/ownednamespacegone/runtimeownerfalse. G2 cần reviewer chấp nhận calibration mới;
G6/gói mới/minimumHW/releasefalse/V1-036 riêng, không lấy packageQA cũ làm proof profile33.

Kiểm local: fullcargoxtaskciPASS,37sourceprecedencetests và actualdevmovement regressionPASS.
OracleO012 vẫn in engine120 từ **profile fixture riêng lịch sử**, không phải devprofile33; không sửa
fixture/golden để hết cảnh báo đãghi nhận. GóiQA95985fe cũng là profile trước hiệu chuẩn này.
