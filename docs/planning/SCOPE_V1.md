# KR Rust — phạm vi Release 1 (V1)

Baseline **`KR-00967-V1-scope-1`**, ngày 2026-10-19. **Scope đã chốt**;
trạng thái thực hiện/đã nghiệm thu theo [PROGRESS](PROGRESS.md), không sửa trạng thái tại spec này.
`V1` là bản phát hành, khác `R1` là pha đo Oracle trong roadmap cũ.
Tài liệu này cùng [registry](../specification/features.json) thay thế các mô tả phạm vi V1 rộng hơn hoặc
mâu thuẫn trong [kế hoạch cũ](../archive/plans/REWRITE_PLAN_2026-10-19.md). Không tự thay đổi toàn bộ mục tiêu full feature dài hạn.

## 1. Sản phẩm V1

V1 là bản chơi thử desktop có server riêng: đăng nhập tài khoản dự án → một kênh/sảnh danh sách
→ tạo/vào phòng → ready → Battle cá nhân → kết quả → quay lại phòng và chơi tiếp.
Không thương mại hóa trong V1; không ví/tiền thưởng/shop hoặc thanh toán.

| Hạng mục | Quyết định |
|---|---|
| Mode/người chơi | Battle cá nhân (FFA), **2–4 người thật**, không bot/đội/spectator |
| Map | Một map cố định **`FxMap/Map/Village00`**, 15×13, bốn tọa độ spawn trong payload |
| Nhân vật | **C00/C01**, cấp mặc định, không yêu cầu inventory; tên hiển thị/chỉ số phải nối đúng Attr |
| Drop trong trận | Chỉ **Bomb / Fire / Roller**: capacity / tầm / speed; không tiền/mount/debuff/consumable |
| Luật | Di chuyển/va chạm, bom/fuse/nổ/chain, phá gạch, drop/pickup, trapped/chết, thắng/hòa/hết giờ |
| Phòng | Công khai, tối đa4; create/join/leave/kick/host transfer, chọn nhân vật, ready/start, chat chờ |
| Mạng | Giao thức mới, server-authoritative, client prediction/reconciliation; không P2P hoặc client cũ |
| Lưu trữ | Identity tài khoản và tùy chọn client; room/trận không cần phục hồi sau server restart |
| Ngôn ngữ | Tiếng Việt cho mọi luồng V1, font Unicode; dữ liệu pending review không coi là đã chốt |
| Nền tảng | Windows/Linux/macOS; không Web/mobile trong V1 |

Các giới hạn scope và policy của V1 là **DESIGN**, không phải tuyên bố server KR gốc làm như vậy.
Hai nhân vật dùng template riêng; không tự cân bằng thành cùng chỉ số hoặc gán tên VN khi chưa nối dữ liệu.
Khác biệt chỉ số được giữ nếu có bằng chứng; quan hệ speed-level→px/time còn phải hiệu chuẩn.

## 2. Nội dung đã chọn và khác biệt có chủ ý

Nguồn entry và SHA-256 payload: [kr_v1_content.json](../evidence/inventory/kr_v1_content.json).
Đã xác nhận entry map, minimap, theme Village, BGM Village, Attr, sprite item/bomb,
animation walk bốn hướng của C00/C01 và tài nguyên UI create/ready/start.
103 tài nguyên được fingerprint; chưa giải đủ dependency tile/object/animation để gọi là bundle hoàn chỉnh.

Village00 có payload SHA-256
`bd34c418ecc33f126f2fee4c21549e78cdc5c31d7805a2af82bff0e635b67a07`.
Metadata probe đọc được bốn cặp tại offset80:
`(13,11), (1,11), (1,1), (13,1)`; tất cả khác nhau và nằm trong lưới.
Không có marker `<mapscript`; đuôi metadata đã đọc là `[100,0,0,0,0]`.
Đây chưa phải chứng minh toàn bộ map loader/consumer không có script hoặc đã hiểu collision.
Entry `Map/Village01` trong kế hoạch VN cũ **không có** trong index KR này.

**Drop profile V1 thay đổi dữ liệu gốc có chủ ý:** bảng drop Village00 đọc được chỉ có sáu loại Lucci.
V1 giữ nguyên map asset làm bằng chứng, nhưng runtime dùng profile riêng chỉ sinh Bomb/Fire/Roller.
Không ghi đè pack hoặc gọi profile này là native KR. Tỷ lệ, cap và các thông số gameplay phải nằm trong
rules có version/provenance; giá trị tạm chưa được duyệt chỉ dùng ở development, không vượt gate phát hành.
Không mặc định trọng số là phần trăm hoặc lấy log giả lập làm xác suất/timing gốc.

Luật gameplay theo [ADR 0013](../decisions/0013-rule-source-precedence.md):
KR 00967 confirmed → VN VBNB_116 confirmed → BnB design → design riêng, chỉ so cùng đại lượng.
Khôi phục round 120 s, trọng số VN và Bomb/Fire +1; giữ chỉ số KR candidate theo ngoại lệ có lý do.
Bảng từng khóa và giới hạn ở RULES_V1 §11; BnB lấp chỗ trống, warmup 3 s giữ design.
Không mở scope map/item/nhân vật ngoài V1; policy vận hành §3 giữ nguyên.

Ghi chú tích hợp: PR #127 chưa merge tại thời điểm soạn; sau #127, ghép các sai khác của
ADR 0013 vào sổ §10, giữ riêng số bảng confirmed với ý nghĩa drop chưa xác nhận.

## 3. Policy phòng và vòng đời

- Một tài khoản có một phiên hoạt động; đăng nhập phiên thứ hai bị từ chối rõ ràng.
- Join chỉ lúc chờ và còn slot; server xử lý atomic. Không private/password/quick-match trong V1.
- Host cũng phải ready. Đủ 2–4 người và tất cả ready mới start. Roster hoặc lựa chọn nhân vật thay đổi
  hủy toàn bộ ready. Một lệnh start bị gửi lại không tạo thêm round.
- Host rời phòng: người vào sớm nhất còn lại làm host. Host không phải máy chạy mô phỏng trận.
- Khi start, server chốt map/rules/content/roster/seed; chọn một hoán vị seed của bốn spawn,
  lấy N tọa độ đầu cho N người, lưu assignment vào replay. Đây là DESIGN, chưa phải thuật toán spawn KR.
- Loading barrier tối đa **15 giây**. Thiếu ack thì hủy start, về waiting và hủy ready.
- Mất kết nối trong trận: quá **10 giây** loại người đó một lần; không nhập lại giữa vòng.
  Trong khoảng chờ timeout, input mới của người mất kết nối là neutral, bom đã đặt vẫn mô phỏng;
  damage/death vẫn xảy ra. Reconnect ứng dụng cần auth mới; về sảnh rồi chờ vòng kết thúc.
- Rời trận chủ động bị loại ngay; rời/mất kết nối khi loading hủy start và hủy ready.
  Room hết thành viên bị xóa; nếu tất cả rời thì hủy round, không tạo người thắng vắng mặt.
- Kết quả chỉ server quyết định; mọi người nhận cùng round/result ID. Sau ack của tất cả hoặc **5 giây**,
  về phòng, reset ready và state trận. Tiền/EXP/rank không thay đổi.

Các số 15/10/5 là policy dự án V1 có thể điều chỉnh qua revision, không gắn nhãn thông số gốc.
Round duration, fuse và trapped timeout không được suy từ những timeout vận hành này.

## 4. Hợp đồng gameplay phải hoàn thành trước coding task phụ thuộc

- Trạng thái tối thiểu: spawning/alive/trapped/dead, trạng thái bom/flame/item; xác định hitbox theo state.
- Đầu vào được server chấp nhận theo tick/sequence; seed, RNG, thứ tự entity/event và làm tròn có spec.
- BOM đặt chụp power tại lúc đặt (đã xác nhận trên client VN, RULES_V1 R-032); grace chỉ cho người đang đứng trên bom đi ra,
  không cho bước ngược lại. Các điểm này phải ghi là DESIGN nếu chưa có đối chiếu đủ nguồn.
- Trapped vẫn là người chưa chết; trong FFA, chạm địch hoặc hết trapped timer dẫn tới dead theo rule.
  Không có rescue đồng đội, kim hoặc khiên trong V1.
- Giải quyết hết event của một tick rồi xét kết quả: còn1 người chưa chết thì thắng;
  không còn ai thì hòa; hết giờ mà còn nhiều người thì hòa. Tick hết giờ xử lý gameplay trước rồi
  xét timeout. Không chọn người thắng dựa vào thứ tự packet đến.
- Pickup và drop là atomic/idempotent; bomb/chain không gây chết, phá gạch hoặc drop lặp.
- Effect/audio trình diễn có event identity, không phát lại khi reconcile/replay dự đoán.

Gate spec không đòi khôi phục toàn server cũ. Một unknown có thể giải bằng RE/đo, hoặc bằng quyết định
DESIGN có nguồn gốc rõ và acceptance. Không đưa hằng số chưa có provenance vào bản phát hành.

## 5. 37 mục triển khai

Danh sách chi tiết, dependency và acceptance từng mục nằm trong [CSV](../specification/features.csv).

| ID | Hạng mục |
|---|---|
| V1-001–003 | Auth dự án, handshake version/rules/content, một kênh/sảnh |
| V1-004–010 | Phòng/host/ready, C00/C01, chat chờ và start barrier |
| V1-011–015 | Di chuyển/collision, bom/fuse/flame/chain |
| V1-016–019 | Drop/pickup, Bomb/Fire/Roller |
| V1-020–023 | Trapped/dead, kết quả, vòng tiếp và disconnect |
| V1-024–027 | Authority/input/snapshot, reconcile, replay/state hash, QA mạng |
| V1-028–029 | Lưu trữ và log/vận hành |
| V1-030–035 | Content closure, animation, UI, input/scale, audio và Việt hóa |
| V1-036–037 | Build 3 OS và báo cáo đối chiếu/sai khác |

## 6. Gate còn mở, owner và bằng chứng nghiệm thu

| Gate | Owner | Phải có trước khi qua gate |
|---|---|---|
| G1 content/map | assets + RE | Decode dependency tile/object và collision; resolve toàn state render; không skip unknown âm thầm |
| G2 movement/stats | RE + core | C00/C01→Attr; tốc độ/hitbox/cornering/time model có trace hoặc DESIGN đã ghi nhận |
| G3 bomb/damage/items | RE + core | Fuse/chain/flame/trap/cap và profile drop V1 có nguồn/version cùng boundary scenarios |
| G4 protocol/state | core + net | Command window/sequence, snapshot, RNG/state hash, authority và reconciliation contract |
| G5 UX/localization | client + QA | Chốt UI/SFX/font/ảnh cho luồng V1; glyph/wrap/focus/golden đúng profile Việt hóa |
| G6 release | QA + integrator | Tất cả37 mục qua acceptance; 3 OS; các sai khác được ghi nhận và chấp nhận |

Scope đã quyết không có nghĩa gate đã xanh. Trạng thái G1–G6 lấy từ [execution.json](execution.json)
và hiện ở [PROGRESS](PROGRESS.md); spec này định nghĩa gate, không giữ một bảng trạng thái thủ công khác.
G1 còn phải kiểm spawn không bị block và có đường thoát; tọa độ trong header chưa chứng minh điều đó.
G3 bao gồm match duration/clock start và toàn bộ drop profile (xác suất có drop, trọng số, thời điểm
hiện, flame-item, cap và pickup lúc đã cap). Clock bắt đầu khi loading barrier hoàn tất.
G6 yêu cầu toàn bộ37mục, kể cả persistence/log/audio/đối chiếu; một build chạy được chưa đủ để phát hành.
Không cần đợi RE các mode later. Bắt đầu bằng asset closure, phép đo nhỏ và prototype offline trên nội dung đã chọn.

## 7. Bộ nghiệm thu tích hợp

1. Hai client auth→sảnh→phòng→ready→trận→kết quả→phòng→trận thứ hai; không state rò qua round.
2. Bốn client join đồng thời; không quá4slot, spawn không trùng; đổi nhân vật/roster hủy ready.
3. Non-host start/kick bị từ chối; host rời/chết/mất mạng không dừng server hoặc phá kết quả.
4. Đi/rẽ/chạm tường/thoát bom; đặt quá cap; chain nhiều bom; pickup cùng lúc; chết cùng tick; timeout.
5. Phiên bản/rules/content sai bị chặn; packet/input trùng, trễ, out-of-window không nhân state.
6. RTT100ms, jitter0–30ms, loss1%, duplicate/reorder theo fixture: state cuối hội tụ, không lặp audio/effect.
   Nếu transport xử lý loss/reorder nội bộ, fixture kiểm cả đứt kết nối và application dedup.
7. Mất mạng và loading timeout đúng policy; login lại về sảnh, không chiếm thêm slot giữa trận.
8. Replay trên Windows/Linux/macOS có cùng state hash; đây là determinism mới, không tự chứng minh giống KR.
9. Client mất focus không giữ phím; fullscreen/window scale/input và âm lượng hoạt động;
   mọi chuỗi Việt/ảnh được dùng có quyết định review, không clipping hoặc placeholder vỡ.
10. Báo cáo riêng: phần đã đối chiếu VN/KR, phần khác nguyên bản có chủ ý và phần còn chưa xác nhận.

## 8. Ngoài V1

Battle đội/cứu đồng đội, 5–8 người, bot/AI, nhiều map/random/quickmatch, spectator/private room,
mount/push/kick/throw/debuff/consumable; mọi mode phụ (HiddenCatch, Baseball, Monster, RPG…);
quảng trường đi bộ; shop/inventory/economy/gacha; progression/ranking/quest/pass/event;
friends/whisper/guild/couple và công cụ tạo nội dung đều ở backlog later.
Auth Nexon/NGM và trust/GreenPC cũ không được port; auth, validation và bảo mật server mới vẫn phải làm.

Mở rộng V1 cần cập nhật registry/baseline, acceptance và dependency; phát hiện thêm asset không tự mở scope.
Các luồng worker/agent làm theo [workflow nhóm](TEAM_WORKFLOW.md), một owner mỗi task,
reviewer khác owner khi có thể; integrator giữ interface và branch tích hợp chạy được.

## 9. Ba task mở đầu có thể chạy song song

| Task | Owner / reviewer | Bàn giao |
|---|---|---|
| START-A, thuộc V1-030/031 | assets / RE | Resolve Village00→tile/object/animation, kiểm bốn spawn và đường thoát; danh sách dependency/unknown; kiểm bằng viewer sẵn có nếu cần |
| START-B, thuộc G2/G3 | RE / core | C00/C01→Attr và phép đo lặp đi/rẽ/đặt bom trên VN; tách timing client/emu; bảng rule có nguồn và câu hỏi chưa giải |
| START-C, thuộc V1-024/026 | core+net / QA | Spec revision đầu cho InputCommand/SimulationState/Event/Snapshot/Replay, thứ tự tick/sequence, seed/hash và authority |

Walking skeleton (`crates/`, ADR 0004) đã có trong repo `boom-rs`; START-C mở rộng kiểu của nó thành contracts-r1.
START-A khảo sát dữ liệu/fixture và xuất bảng phân loại ô vào `data/`; chưa dựng renderer (chờ spike ADR 0007).
START-B ghi claim vào [RULES_V1](../specification/gameplay/RULES_V1.md) và giá trị vào profile.
Coding task có feature ID chỉ mở sau khi hợp đồng tương ứng được chốt và P3 kết luận.
Đổi chính sách hoặc content của V1 phải sửa registry và docs trước khi giao tiếp task.
