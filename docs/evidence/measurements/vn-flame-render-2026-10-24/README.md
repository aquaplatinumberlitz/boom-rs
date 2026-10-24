# State lửa: bảng ODT VN/KR và consumer VN (2026-10-24)

Root [#148](https://github.com/aquaplatinumberlitz/boom-rs/issues/148).
Nguồn: VN VBNB_116 fixture SHA bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1,
base400000/i386; giới hạn patch theo [owner-grace](../vn-player-grace-2026-10-24/README.md).
Phép đo state này không đo lại lifetime/damage R-156/R-158 và không đo tốc độ bằng clock bị debugger tác động.

## Bảng tài nguyên: member ODT, không phải entry pack độc lập

Chuỗi Flame/Flame là **member trong ObjectTable/ObjectTable**, nên tìm entry pack cùng tên sẽ không thấy.
Theo format đã có ở boom-assets/src/closure.rs (R-094), mỗi member chứa bảng state có nhãn, entry ảnh,
và các cặp chỉ số frame/thời lượng thô. Chỉ giải member này, không quét mọi dữ liệu game.

| Build | SHA payload ObjectTable | Member / khoảng byte |
|---|---|---|
| VN | 94643886a3d590384bf3108eeb42ee8c072d2062ac4d681a296efa7e87a18b16 | index328,708104..721708 |
| KR00967 | a68fd5ffcc815b24b5c5199e9910f10d14f1ac6f30aba4c237a23f4e279608dc | index584,1199510..1213114 |

Hai block **13.604 byte giống hệt**, SHA70ff67a04810fd5401006faeb3d6c6851f297105963d4192d043e4f298d001ba.
Cả76 nhãn/entry/toàn bộ cặp frame-duration được đối chiếu; không chỉ so first/last.
Payload VN từ **pack của private game thực chạy** cũng khớp byte với file đã trích.
Payload KR được trích bằng boom-assets pack-extract, chỉ đọc pack gốc; chưa chạy consumer KR.

[odt-states.csv](odt-states.csv) có76 state và hash chuỗi frame của từng state:
15 state cho mỗi nhóm C/L/U/R/D;75 state dùng entry Bomb/Fire00, còn state75 FireTimebomb dùng
Bomb/TimebombFrame.60 state nhánh có19 record; nhóm tâm có9..18 record, stateTimebomb20.
Tất cả duration thô bằng1: **chưa biết đơn vị**, không gọi là1ms hay một tick Rust.

## Consumer và quan sát runtime

Cache cùng SHA có constructor5ACBD7..5ACC2C: đặt vtableC35594, truyền keyFlame/Flame tới58D9BA,
và khởi tạo variant+240=0. Đây là đường truyền key mặc định, không khẳng định đã hiểu hết resolver.
Selector5ACC58 chọn segment cuối có counter+CC không âm; trả motion từ+90, counter từ+CC, owner từ+108.
Draw5AD00C đưa motion tới vtable+5C→58D3E9, counter tới58D517, rồi gọi baseDraw58DA94.

Hai helper chỉ có prefix đã ghi: chúng đưa địa chỉ boxed+28/+34/+40 tới413F32, không phải store
DWORD trực tiếp; imm16 của lệnh ret bị thiếu trong capture, không gọi là full body/ret4.
Một range draw còn xuất thêm hai body kề5AD104/5AD155; không phân tích chúng.
Leaf43byte vượt cap32byte11byte được giữ trong ledger, không giấu thành một read đủ ngân sách.

Codex là runtime owner duy nhất; namespace mạng chỉloopback/PID riêng, private Wineprefix/game,
WineGDBi386 có byte gate. Probe **chỉ sau selector hợp lệ**, tối đa8 stop/gate (không195 object trống mỗi frame):
5AD042 trước occlusion và5AD0F9 sau helper/trước baseDraw. Giá trị tại native ESI và output trên stack
được đọc cùng object. Raw trong [manifest.json](manifest.json) có **61 file size/SHA**, không chứa nội dung
ảnh/binary/decompile trong repo. [runtime.csv](runtime.csv) có11 record quan sát.

| Vụ bom / vị trí native | Vai trò hình học | Motion / nhãn ODT | Quan sát |
|---|---|---|---|
| bom(1,0), flame(40,39) | tâm | 0 / FireC01 | pre + post; boxedmotion0, counter tại+34=1 |
| bom(1,0), flame(0,39) | trái | 15 / FireL01 | pre + post; boxedmotion15, counter tại+34=1 |
| bom(1,0), flame(80,39) | phải | 45 / FireR01 | pre + post; boxedmotion45, counter tại+34=1 |
| bom(1,0), flame(40,79) | dưới | 60 / FireD01 | preselector; postgate đã tắt sau3 object khác |
| bom(1,1), flame(40,39) | trên | 30 / FireU01 | pre + post; boxedmotion30, counter tại+34=1 |
| bom(1,1), flame(40,79) | tâm lặp | 0 / FireC01 | pre + post; boxedmotion0, counter tại+34=1 |

Đối chiếu nhãn ODT ở đây chỉ dùng **motion ordinal**. Giá trị1 tại selector/+34 là counter native,
**không phải chỉ số frame hình của I16** hoặc first_frame của ODT. First_frame của L/U/R/D là4/5/6/7;
probe chưa đo phép chuyển từ counter sang frame hình bên trong baseDraw, không khẳng định đã vẽ ảnh1.

Tất cả object được quan sát có vtableC35594, variant+240=0, owner0. Nhãn được nối với **vị trí thực
so với bom**, không đoán hướng chỉ từ chữL/U/R/D. Trường raw tên resource_ptr_ac là tên tạm của probe:
giá trị0, **chưa xác định semantics của fieldAC**, không dùng nó để chứng minh resource pointer.
Pointer có thể tái sử dụng giữa hai run; chỉ đối chiếu identity giữa pre/post trong cùng run.

## Phạm vi đã giải và phần còn thiếu

- R-164 nối vị trí tâm/bốn hướng ở power1 với motion0/15/30/45/60 trên consumer VN bình thường;
  actual boxed motion/frame được kiểm ở bốn vai trò, riêngDown chỉ có preselector.
- R-165 xác nhận bảng ODT normal76state và dữ liệu byte-identical VN/KR. Không suy native code KR
  có cùng offset hoặc mọi mode/biến thể dùng cùng bảng.
- Chưa kiểm các ordinal biến thể02..10, quan hệ range/distance với motion, Snow/RPG/Timebomb,
  caller cadence, đơn vị duration, blend/layer đầy đủ hoặc pixel thực đã vẽ.
- Không đổi fuse/flame540ms/trap6350ms hay renderer Rust từ quan sát state này. Group state được biết
  không có nghĩa lifetime tâm và nhánh giống nhau; damage/lifecycle từng state cần phép đo riêng.
