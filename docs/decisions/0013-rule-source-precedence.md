# 0013 — Thứ tự ưu tiên nguồn luật V1

Status: **Accepted**, 2026-10-21, quyết định trực tiếp của chủ dự án.
Tác giả: Codex (co-integrator); reviewer PR: Claude. Accepted là nguồn quyết định, không thay approval PR.
Thay một phần [ADR 0012](0012-v1-rules-from-bnb.md); giữ việc chọn BnB cho chỗ trống và backlog cơ chế.

## Quyết định

Với **cùng đại lượng, đơn vị, ngữ cảnh và build/profile áp dụng**, ưu tiên:

1. `kr_static` / `kr_runtime` **confirmed**, client KR 00967, RE/decomp có địa chỉ/hàm consumer.
2. `vn_static` / `vn_runtime` **confirmed**, client VBNB_116 cùng codebase, theo [ADR 0002](0002-vn-kr-trusted-sources.md).
3. `oss` BnB `tpoisonooo/BnB@5c108e8ad3ea07684d4666f60e6eec6c2f3e569e`, chọn có chủ ý thành `design`.
4. `design` riêng chỉ khi 1–3 không cung cấp cùng đại lượng.

ADR 0002 vẫn chấp nhận bằng chứng VN trực tiếp, không yêu cầu đo lại KR chỉ vì region;
ADR này bổ sung ưu tiên build target khi hai nguồn cùng xác nhận nhưng khác nhau.
`candidate` KR/VN, web, emu_choice không đè tầng confirmed. Không lấy trung bình.
Không đổi nhãn bằng chứng để hợp thức hóa giá trị profile.

**Ngoại lệ rõ ràng cho nhân vật:** giữ 12 khóa C00/C01 từ R-040/R-041 vì đúng build target,
không thay bằng template cân bằng BnB. Dãy số confirmed nhưng nghĩa f2–f7 R-042 vẫn candidate;
profile `kr_static` / `candidate`, `release = false`, R-117 ghi lựa chọn. Đây là ngoại lệ được quyết định
riêng, không phải quy tắc candidate KR luôn thắng OSS hoặc confirmed. RE consumer là điều kiện nâng nhãn.

## So khớp đại lượng và áp dụng ngay

[RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)
R-110…R-118, R-158 và R-160 là bảng lựa chọn **mọi khóa** của `data/rules/v1-dev.toml`.
Khôi phục round 120000 ms, tile 40 px, Bomb/Fire +1 và trọng số 3650/2800/2000.
R-051 confirmed số trong bảng, **vai trò bảng còn unknown** (R-052): dùng cho drop ba item V1 là
quyết định áp dụng riêng, không gọi trọng số là phần trăm hay xác suất drop gốc.

Giữ tick server 30 Hz: R-013 30 ms/bước **client** không xác nhận tick server.
Lựa chọn trap4000ms lịch sử dựa BnB; hiệu chuẩn R-160 hiện6350ms theo native endpoint VN ở bổ sung dưới.
R-026 6359ms giữa **packet** có xử lý/gửi vẫn không xác nhận timer nội bộ.
Giữ speed 24 px/s/level vì raw checksum R-043 chưa xác thực; cấp tốc độ không phải vận tốc.
Fuse/chance/hitbox/assist/touch/warmup giữ BnB vì chưa có confirmed cùng đại lượng.
Lửa dùng hiệu chuẩn R-158 theo bổ sung24/10 bên dưới.
Roller +1 giữ OSS design; không nâng R-045 candidate. Nhịp clip KR 100 ms giữ design riêng
vì BnB không cung cấp timing clip này. Không dùng asset tồn tại để xác nhận consumer.

## Luồng RE ưu tiên và thay luật

Owner **dsh**, reviewer Claude/Codex: lần lượt movement/hitbox/corner/speed per level;
trapped timer/di chuyển/cứu; nổ/lửa theo hướng và thời lượng frame; item/cap; UI luồng V1.
Câu hỏi và issue được liên kết trong [QUESTIONS](../research/QUESTIONS.md), kế hoạch ở
[REWRITE_PLAN](../planning/REWRITE_PLAN.md). RE KR là luồng ưu tiên fidelity hiện hành.

Mỗi phát hiện ghi claim `kr_static` có địa chỉ/hàm consumer, build, đơn vị, ngữ cảnh,
locator bằng chứng và giới hạn; evidence metadata/cách tái tạo trong `docs/evidence/`.
Không commit binary/pack/IDA DB. Chưa chứng minh consumer thì candidate, không tự nâng confirmed.
Khi có luật mới confirmed cùng đại lượng: mở **PR riêng** cập nhật claim + profile + sai khác,
review độc lập, kiểm hash/handshake/replay/scenario và consumer bị ảnh hưởng; đổi cơ chế thì sửa System
qua task có feature ID. Không âm thầm bless golden để che hành vi đổi.

## Quan hệ ADR 0012 và hệ quả

Phần “BnB đè KR/VN confirmed” và template nhân vật R-105 bị thay; lịch sử giữ nguyên.
BnB vẫn là nguồn bổ sung hiện tại cho chỗ trống và backlog phía server **#131/#132 giữ nguyên**;
RE KR có thể thay các lựa chọn/cơ chế này sau bằng PR có review. #133 không còn là chỉ dẫn chọn template BnB.
Không mở scope mode/item/map mới, không thay policy vận hành/phòng của SCOPE_V1.

Profile đổi identity/hash; client/server/replay phải cùng rules. Fixture `skeleton-test` dùng profile riêng,
không dự kiến đổi golden. CI xanh chỉ chứng minh regression, không chứng nhận fidelity.
Phương án giữ BnB đè confirmed bị bác theo quyết định mới; phương án bỏ VN bị bác vì tầng 2 được giữ.
Đảo quyết định bằng ADR mới, giữ claim và bằng chứng lịch sử.

## Bổ sung 2026-10-24 — dùng phép đo động VN cho cửa sổ lửa

Chủ dự án xác nhận kết quả đo động client VN dùng được làm nguồn luật, gần ngang phân tích tĩnh KR.
Không chờ đo lại KR hoặc chứng minh lại toàn bộ fixture trước khi dùng quan sát VN có build, điều kiện
và giới hạn rõ. R-155/R-156 xác nhận hành vi owner và cửa sổ lửa; không suy non-owner hoặc timer ms cố định.

Riêng `bomb.flame_ms`, thay fallback BnB bằng hiệu chuẩn V1 **540ms**, R-158 (`design`): ba capture
vùng lửa hết sau khoảng531–562ms từ native return, có đối chứng counter18 trúng/counter19 sống.
Không đổi19update native thành19tick Rust. 540ms làm tròn lên17tick tại30Hz; lửa sống đủ17tick kể cả
tick tạo, `until = created + ticks - 1`. Đây là lựa chọn ánh xạ có chủ ý theo số đo, không đổi nhãn
bằng chứng thành “native chứa hằng540ms”. Phần giảm TTL ngay lúc tạo của R-109 không còn áp dụng;
chain, phá gạch/item và các phần khác giữ nguyên. Fuse vẫn theo R-115; timer nhốt được cập nhật ở bổ sung dưới.

## Bổ sung2026-10-24 — counter bubble VN và hiệu chuẩn nhốt

R-159 nối native receiver/consumer: ngưỡng stage+56C0=200 so với boxed age của đúng bubble,
nhánh716FE1 dẫn subtype13; ba capture native serialize11→13 là6332–6355ms, có đối chứng ngoài lửa.
HELP còn ở+5,2s. Counter có3reset sớm trong phép lấy mẫu; không coi200 là200ms/200tickRust và
không dùng tỉ lệ counter/wall-clock làm quy đổi. Chưa đọc increment writer/seed constructor.

Chọn R-160 `player.trap_to_death_ms=6350` (`design`/design) thay fallback BnB4000; đây là calibration
của vòng đời quan sát, không hằngms native confirmed. 30Hz ceil191tick, tính tick tạo→chếtT+190,
~6333ms hiệu dụng. Native serialize11→13 được chọn làm mốc tương ứng RustTrapped→Dead; nếu13 thuộc
cuối animation/mode khác, mở lại hiệu chuẩn. Fuse/flame/speed/cap/touch-order và warmup giữ nguyên.
[Evidence/range/hash/reset/overhead](../evidence/measurements/vn-trap-timer-2026-10-24/README.md).
Quyết định dựa quyền dùng đo động VN của chủ dự án; cần reviewer độc lập exacthead trước merge,
không nâng gate/phát hành hoặc nhãn native literal từ một giá trị calibration.

## Bổ sung2026-10-25 — logical Attr identity và đại lượng

R180 đã nối tên hàng logic Character/C00/C01/f0 với cùng manager và actual native fieldreader VN;
R181 cùng R173/R175 nối base/max/current và tăng/kẹp. Phần chưa nối identity trong mô tả ngoại lệ
ở trên là baseline lịch sử; không đòi avatar/tên hiển thị để xác nhận logicalrow đã có native source.
Đây cũng không tự xác nhận nativeKRconsumer hoặc cách đổi scalar thành px/s.

Giữ12khóa R117 `kr_static/candidate` và `release=false` trong lượt evidence này. Lượt review tiếp
phải tách **số**, **vai trò field/base/max**, **đại lượng/đơn vị engine**; xác nhận scalar không làm
quy đổi R11424px/s/level thành nativeconfirmed. VNconfirmed được dùng trực tiếp theo ADR0002;
không đặt yêu cầu đo lại KR chỉ vì region. G2 có thể chấp nhận trace và DESIGN đã nghiệm thu theo
SCOPE; không chờ toàn bộ ItemAttr/6root, không gán candidate→confirmed để vượt gate.
