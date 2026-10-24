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

Ghi chú tích hợp: PR #127 đã ghép các sai khác của ADR 0013 vào sổ §10 (tổng hợp ở
[§10.3](#103-nguồn-giá-trị-hiện-hành--adr-0012-và-adr-0013)), giữ riêng số bảng `confirmed` với
ý nghĩa drop chưa xác nhận.

Danh sách đầy đủ các sai khác có chủ ý **và** các khoảng trống fidelity còn chờ đo: [§10](#10-sai-khác-có-chủ-ý-so-với-client-gốc).

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
hiện, flame-item, cap và pickup lúc đã cap). Loading barrier hoàn tất là mốc bắt đầu warmup/countdown
theo rules; đồng hồ thời hạn lượt chỉ chạy sau warmup, nên warmup không ăn vào thời hạn lượt
(R-113: `round.warmup_ms` 3000 ms, `oss`/design — quyết định áp dụng của V1, không phải timing
`confirmed` KR).
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

## 10. Sai khác có chủ ý so với client gốc

Sổ này tách hai loại khác biệt giữa engine V1 và client gốc (KR/VN):

- **Sai khác chấp thuận** — quyết định scope/policy của V1, có claim hoặc ADR dẫn tới; giữ nguyên.
- **Khoảng trống fidelity** — engine V1 đang khác client gốc (hoặc chưa biết) nhưng **chưa có
  bằng chứng đo**; giữ nguyên cho tới khi đo được, không tự coi là đã khớp.

Sổ **không đổi scope** và **không tạo feature**: nó ghi lại điều đã quyết để reviewer đối chiếu
và để báo cáo sai khác (V1-037) có một danh sách chuẩn. Mọi mục "chờ đo" nằm trong
[QUESTIONS](../research/QUESTIONS.md); kế hoạch lấy oracle và phương pháp đo ở **Q-009**.

Mỗi mục có ít nhất một claim trong [RULES_V1](../specification/gameplay/RULES_V1.md) hoặc ADR dẫn
tới; mức bằng chứng của claim ghi theo [AGENTS §4](../../AGENTS.md#4-bằng-chứng). Mục nào cần
thêm dẫn spec (phần đó không có claim riêng) thì ghi spec đó kèm theo ở cột *Nguồn*.

Cột *V1* ghi **giá trị đang dùng** kèm nguồn của nó, theo thứ tự ưu tiên
[ADR 0013](../decisions/0013-rule-source-precedence.md) (KR `confirmed` → VN `confirmed` → BnB
`design` → `design` riêng; chỉ so **cùng đại lượng, đơn vị, ngữ cảnh**). Bằng chứng gốc của
client KR/VN và nhãn của nó **giữ nguyên**; quyết định cũ của
[ADR 0012](../decisions/0012-v1-rules-from-bnb.md) (BnB đè giá trị gốc) chỉ còn hiệu lực ở chỗ
ADR 0013 **không** tìm được nguồn `confirmed` cùng đại lượng. Bảng từng khóa, nhãn và giới hạn áp
dụng nằm ở [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013).
Tổng hợp theo nhóm nguồn: [§10.3](#103-nguồn-giá-trị-hiện-hành--adr-0012-và-adr-0013).

### 10.1 Sai khác chấp thuận cho V1

| # | Sai khác | Client gốc | V1 | Nguồn |
|---|---|---|---|---|
| D-01 | Bảng rơi item của map | `Village00` rơi **6 loại Lucci** (`kr_static` / confirmed); bảng ItemAttr của client VN ghi trọng số Bomb/Fire/Roller **3650/2800/2000** (`vn_static` / confirmed) nhưng **vai trò bảng còn `unknown`** | Profile riêng: chỉ **Bomb/Fire/Roller**, trọng số **3650:2800:2000** (số bảng VN — dùng làm trọng số ba item V1 là quyết định áp dụng, **không** gọi là xác suất drop gốc); xác suất gạch có item **35%** (`oss` / design, xem G-09); giữ nguyên map asset làm bằng chứng | R-055, R-051, R-052, R-112 (lịch sử: R-067 → R-106 → R-112) ([RULES_V1 §5](../specification/gameplay/RULES_V1.md#5-item-rơi-và-nhặt); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)); [ADR 0013](../decisions/0013-rule-source-precedence.md); SCOPE_V1 §2 |
| D-02 | Chủ phòng và ready | Chủ phòng **không** bấm ready | Chủ phòng **cũng phải** ready | R-063 ([RULES_V1 §7](../specification/gameplay/RULES_V1.md#7-vòng-đời-trận-và-phòng-legacy-vn)); SCOPE_V1 §3 |
| D-03 | Ai quyết kết quả và vị trí người chơi | Client tự mô phỏng di chuyển, đồng bộ P2P qua UDP; server gốc thất lạc | Server authoritative; client chỉ dự đoán di chuyển của mình, state đồng bộ bằng snapshot | [ADR 0005](../decisions/0005-authority-and-prediction.md); R-047, R-066; SCOPE_V1 §1 |
| D-04 | Nền màn hình (JPEG) | Nền login/lobby/prepare là JPEG 800×600 có chữ nằm trong ảnh (KR: chữ Hàn; bản VN: nền login **không** có chữ Việt) | Không đưa nền vào atlas; dựng nền bằng màu, chữ lấy từ catalog | R-090 (`kr_static`+`vn_static` / confirmed), R-091 (`design` / design) ([RULES_V1 §12](../specification/gameplay/RULES_V1.md#12-nền-và-ảnh-ui-của-client-v1)); [BUNDLES §8](../specification/assets/BUNDLES.md#8-giới-hạn-đã-biết); [UI_STRINGS §3](../specification/localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1) |
| D-05 | Ảnh UI có chữ | Ảnh nút/hộp thoại có chữ nằm trong ảnh (KR: chữ Hàn ở nút/dialog, chữ Latin ở art kết quả; bản VN: nút/dialog đọc được chữ Việt, art kết quả **chưa đọc rõ**) | Sáu sprite được duyệt nạp từ VN/PNG theo manifest; phần tử chưa duyệt hoặc `excluded` dùng text catalog | R-092 (`kr_static`+`vn_static` / confirmed), R-093 (`design` / design) ([RULES_V1 §12](../specification/gameplay/RULES_V1.md#12-nền-và-ảnh-ui-của-client-v1)); [BUNDLES §3.3](../specification/assets/BUNDLES.md#33-phần-tử-ui); [UI_STRINGS §3](../specification/localization/UI_STRINGS_V1.md#3-ảnh-có-chữ-trong-luồng-v1) |
| D-06 | Mô hình phòng và cách xếp kết quả | 16 slot trạng thái, đội, kết quả theo hạng mỗi slot và cột bắt/cứu/EXP/tiền | FFA 2–4 người, một kết quả mỗi lượt, hòa khi hết giờ | R-062, R-065 ([RULES_V1 §7](../specification/gameplay/RULES_V1.md#7-vòng-đời-trận-và-phòng-legacy-vn)); [VN_ENGINE_DIFF](../research/findings/VN_ENGINE_DIFF.md) |
| D-07 | Số map chơi được | 1.151 map trong pack KR, nhiều mode | Một map cố định `Village00` | R-002, R-070, R-071 ([RULES_V1 §1](../specification/gameplay/RULES_V1.md#1-lưới-và-map)); SCOPE_V1 §1 |
| D-08 | Chỉ số nhân vật (speed/bomb/range) | KR `BomberAttr` C00 f2–f7 = `5,7,1,10,1,7`, C01 = `4,8,2,7,1,9` (`kr_static` / confirmed — **số** đã đọc); **nghĩa trường** (speed/bomb/range) còn `candidate` | Giữ nguyên 12 khóa `characters.c00_*`/`c01_*` theo số KR — ngoại lệ `candidate` có chủ ý; **không** dùng template cân bằng BnB (R-105 đã bị thay) | R-040, R-041, R-042, R-117 (lịch sử: R-105) ([RULES_V1 §4](../specification/gameplay/RULES_V1.md#4-nhân-vật-và-tốc-độ); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)); [ADR 0013](../decisions/0013-rule-source-precedence.md) |

### 10.2 Khoảng trống fidelity — chờ đo

| # | Sai khác | Client gốc (mức bằng chứng) | V1 (giá trị đang dùng) | Nguồn | Câu hỏi |
|---|---|---|---|---|---|
| G-01 | Thời lượng frame animation | Pack **không** mang thời lượng; consumer chưa định vị được (`unknown`) | 100 ms cho mọi clip (`design` — ADR 0013 giữ nhịp này vì BnB không có nhịp clip KR) | R-079, R-085, R-118 ([RULES_V1 §8](../specification/gameplay/RULES_V1.md#8-animation-và-trình-diễn-v1); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)); [BUNDLES §7.1](../specification/assets/BUNDLES.md#71-chọn-clip-và-frame-v1) | Q-003 |
| G-02 | Hình lửa theo hướng | Clip lửa 100 frame gộp thân/đuôi/nhánh (`candidate`) | Mọi ô lửa dùng **dải frame lõi 28..=59** (`design`) | R-078, R-087; [BUNDLES §7.1](../specification/assets/BUNDLES.md#71-chọn-clip-và-frame-v1) | Q-003 |
| G-03 | Chuỗi chết | Clip chết kết thúc bằng ảnh rỗng 0×0 (dữ liệu `confirmed`) | Sổ trình diễn (R-089): chuỗi chết phát **trọn clip**, xong nhân vật giữ **khung cuối có ảnh**; nếu clip không có khung nào có ảnh thì **không vẽ**. Ở chế độ mạng chưa nối event server vào sổ (#114) nên chưa có chuỗi chết để phát | R-077, R-086, R-089 ([RULES_V1 §8.1](../specification/gameplay/RULES_V1.md#81-quyết-định-trình-diễn-của-client-v1-r-085r-089)); [BUNDLES §7.2](../specification/assets/BUNDLES.md#72-sổ-trình-diễn-v1) | Q-003 |
| G-04 | Fuse bom | Chưa biết (`unknown`); emulator tự đặt 2,5 s | 2,5 s (`oss` / design — BnB) | R-021, R-022, R-115 ([RULES_V1 §3](../specification/gameplay/RULES_V1.md#3-bom-lửa-nhốt-chết); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)) | Q-006 |
| G-05 | Nhốt → chết | Native bubble threshold200; serialize11→13 đo6332–6355ms, HELP còn+5,2s; sampler córeset sớm, chưa có literalms | **6350ms** calibrationDESIGN R-160→191tick30Hz/chếtT+190 (~6333ms); thay fallback4s, không đổi200counter native thànhtickRust | R-159/R-160 ([RULES_V1 §25](../specification/gameplay/RULES_V1.md#25-nhốt--hết-hạn-vn-và-hiệu-chuẩn-v1-2026-10-24)); [evidence](../evidence/measurements/vn-trap-timer-2026-10-24/README.md) | Q-006/Q-018 |
| G-06 | Tốc độ nhân vật | Quan sát thô 5→6 px/tick ở 30 ms, checksum chưa xác thực (`candidate`) | **24 px/s mỗi cấp** (`oss` / design — BnB); **không** phải 33 px/s cũ của R-075 (claim đã superseded) | R-043, R-045, R-075 (superseded), R-114 ([RULES_V1 §4](../specification/gameplay/RULES_V1.md#4-nhân-vật-và-tốc-độ); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)) | Q-005 |
| G-07 | Nhịp tick | 30 ms/bước đo được ở **client** (`vn_runtime` / confirmed) — **không** xác nhận tick server | **30 Hz** (`oss` / design — BnB); 33 Hz cũ của R-074 đã superseded | R-013, R-074 (superseded), R-113 ([RULES_V1 §2](../specification/gameplay/RULES_V1.md#2-trận-và-thời-gian); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)) | Q-004 |
| G-08 | Người bị nhốt có di chuyển không | "Đi rất chậm" (`web`/`candidate`), chưa có px/s | Đứng yên (`design` — R-036; BnB cũng đứng yên nên cơ chế này giữ theo ADR 0012) | R-029, R-036, R-108 ([RULES_V1 §3](../specification/gameplay/RULES_V1.md#3-bom-lửa-nhốt-chết); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)) | Q-005 |
| G-09 | Xác suất gạch có item | Chưa biết (`unknown`); OSS dùng 0,35 (`candidate`) | **35%** (350 permille; `oss` / design — BnB) | R-053, R-067 (superseded), R-112 ([RULES_V1 §5](../specification/gameplay/RULES_V1.md#5-item-rơi-và-nhặt); [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013)) | Q-007 |
| G-10 | Âm thanh event→entry | Chưa có trace consumer; A10 của `features.md` còn "chưa điều tra" | Mapping event→entry và policy chồng tiếng là `design` | R-082, R-083 ([RULES_V1 §10](../specification/gameplay/RULES_V1.md#10-âm-thanh)); [AUDIO_V1](../specification/assets/AUDIO_V1.md) | Q-015 |
| G-11 | Ảnh UI đã Việt hóa | Nút/dialog VN có chữ Việt (R-092); ảnh Draw đã ghép do chủ dự án cung cấp và duyệt ngày 2026-10-24; art Win/Lose VN chưa được chọn | 9 hàng image-pending của V1 tương ứng 8 asset đã duyệt; 6 sprite trong closure được tích hợp qua PR #241. Kho ảnh chung còn 3.030 pending, ngoài luồng V1; Win/Lose dùng text. V1-035 đã nghiệm thu, G5 còn chờ runtime UX | R-092/R-093; [UI_STRINGS §3.2](../specification/localization/UI_STRINGS_V1.md#32-bundle-ui-việt-hóa-đã-duyệt-2026-10-24); [IMAGE_REVIEW](../specification/localization/IMAGE_REVIEW.md); `data/assets/ui-vi-v1.toml` | Q-012 |

Các mục "chờ đo" **không** phải lỗi đã biết: phần lớn là chỗ V1 tự chọn giá trị khi client gốc
chưa có bằng chứng. Việc đo client gốc **đang hoãn** (chủ dự án sẽ lấy oracle sau: client BnB
Trung Quốc qua tài khoản test, hoặc thông số công khai trên mạng với nhãn `web`/`candidate`);
nguồn oracle, phương pháp đo độ lặp và cách lưu bằng chứng nằm ở **Q-009**.

### 10.3 Nguồn giá trị hiện hành — ADR 0012 và ADR 0013

Sổ trên ghi **giá trị đang dùng**. Nguồn của từng giá trị sau ADR 0013 chia làm ba nhóm, và
sổ này **không** dùng claim đã superseded làm lựa chọn hiện hành:

- **Đã về gốc KR/VN** (khôi phục nguồn `confirmed` cùng đại lượng): thời hạn vòng **120 s** và
  cỡ ô **40 px** (R-110), **Bomb/Fire +1** (R-111), trọng số rơi **3650:2800:2000** (R-112 —
  *số* bảng VN `confirmed`, vai trò bảng vẫn `unknown` R-052, nên áp làm trọng số ba item V1 là
  quyết định áp dụng chứ không phải xác suất drop gốc), chỉ số C00/C01 theo `BomberAttr` KR
  (R-117 — ngoại lệ `candidate` có chủ ý, xem D-08). Theo ADR 0012/trước ADR 0013 các giá trị này
  đã bị BnB đè (ví dụ vòng 120→180 s); nay về lại giá trị gốc, **bằng chứng gốc không bị hạ nhãn**.
- **Vẫn theo BnB** (`oss` / `design`) vì chưa có nguồn `confirmed` cùng đại lượng: nhịp tick
  **30 Hz** (R-113 — R-013 chỉ đo bước *client*, không xác nhận tick server), warmup **3 s**
  (R-113), tốc độ **24 px/s mỗi cấp** (R-114 — R-043 còn `candidate`), fuse
  **2500ms** (R-115), hộp va chạm
  **26×26 px** / assist **12 px** / khoảng chạm **28 px** (R-116), Roller **+1 cấp** (R-111),
  xác suất gạch có item **35%** (R-112 — R-053 `unknown`).
- **`design` riêng** (BnB **không** cung cấp cùng đại lượng, nên **không** phải lựa chọn từ BnB):
  nhịp clip **100 ms** (R-118 — R-079 `unknown`; ADR 0013 giữ `design` riêng vì BnB không có nhịp
  clip KR).

**Profile đã đổi giá trị, cơ chế thì chưa.** Profile
[`v1-dev`](../../data/rules/v1-dev.toml) đã mang giá trị R-110…R-118, nhưng các **cơ chế** BnB
đi kèm còn chờ task triển khai: **#130** (movement/hitbox/assist, bomb pass — đã merge), **#131**
(drop khi vỡ gạch, lửa và item, nhặt tại cap), **#132** (chạm/nhốt, chuỗi lửa, warmup, thứ tự
kết quả). **#133** không còn là chỉ dẫn chọn template BnB — chỉ số giữ theo KR (R-117). Sổ này
ghi **giá trị đã chọn**, không khẳng định engine đã chạy theo; xem [ADR 0012](../decisions/0012-v1-rules-from-bnb.md)
và [ADR 0013](../decisions/0013-rule-source-precedence.md).

**Số liệu không suy ra từ phần trăm.** Tỷ lệ feature `done` và số claim `confirmed` **không**
phải % fidelity; xem [REWRITE_PLAN §Nguyên tắc fidelity](REWRITE_PLAN.md#nguyên-tắc-fidelity-và-đo-nguồn).

### 10.4 Cửa sổ lửa theo phép đo VN — 2026-10-24

Theo quyết định chủ dự án, dùng phép đo động VN trực tiếp. R-155 xác nhận owner đi ra khỏi bom
rồi không vào lại; R-156 xác nhận cửa sổ lửa, cùng tọa độ x50/y39: counter18 trúng, counter19 sống.
Phạm vi là owner/build/mode được đo, chưa suy non-owner hoặc damage lặp sau khi đã nhốt.

V1 thay riêng lựa chọn lửa của R-115 và boundary TTL của R-109 bằng **R-158**: hiệu chuẩn540ms,
ceil17tick ở30Hz, tính cả tick tạo (`until=created+ticks-1`). Đối chứng native hết vùng hoạt động
sau khoảng531–562ms từ return; 540ms là lựa chọn tích hợp có sai số theo tick, không phải hằng
native hoặc phép đổi19update VN thành19tick Rust. Fuse/trap, chain, phá gạch/item giữ lựa chọn cũ.
Metadata và scalar đối chứng ở [evidence VN](../evidence/measurements/vn-flame-window-2026-10-24/README.md).
Giới hạn animation clip/hướng vẽ lửa trong G-02 vẫn còn; không lấy field lifetime để tuyên bố đã khớp
mọi đuôi hiệu ứng native. Gate/feature progress vẫn theo execution.json/features.json.


### 10.5 Chốt V1 và tiếp tục fidelity — quyết định 2026-10-24

Thực hiện theo [REWRITE_PLAN](REWRITE_PLAN.md), mục điều phối sáu issue RE; nguồn câu hỏi còn mở là
[QUESTIONS](../research/QUESTIONS.md). Native confirmed dùng cho đúng đại lượng/ngữ cảnh theo ADR 0013.
Phần chưa biết có thể được giải bằng phép đo hoặc lựa chọn DESIGN có nguồn, ca nghiệm thu và reviewer;
không đổi candidate/unknown thành confirmed. Ngoại lệ chỉ số C00/C01 hiện hành phải được review rõ ở G2.

G2/G3/G5 có thể nghiệm thu phần V1 khi source/profile/acceptance đã được chấp nhận và thực thi đã kiểm;
phần fidelity còn thiếu giữ link Q/issue, owner và điều kiện làm tiếp. Một issue RE còn mở không tự là
blocker phát hành; một DESIGN chưa được nghiệm thu cũng không được vượt gate. Giữ nguyên 37 feature,
6 gate và requirement G6; không bật profile release hoặc đánh dấu done chỉ vì CI xanh.

UI Việt hóa V1-035 đã được nghiệm thu; G5 còn kiểm runtime focus/input/scale/cửa sổ/âm lượng/SFX.
V1-036 cần kiểm gói cuối trên Windows/Linux/macOS với version/fingerprints; gói QA không phải tuyên bố
release/fidelity. Native KR layout, số voice và các nhánh trình diễn chưa xác nhận vẫn là backlog fidelity.

### 10.6 Hiệu chuẩn vòng đời nhốt theo VN24/10

R-159 xác nhận counter/consumer200 trong đúng ca VN; R-160 chọn6350ms cho Rust,191tick30Hz và
chết ởT+190 khi tính tick tạo. Mốc là native serialize11→13; chưa tách death-animation tail, không
suy hằngtimer server, writer/clock hoặc200tickRust. Sampler early resets giữ trong evidence, không
làm epoch đơn điệu giả. Các giới hạn này có owner/root147 và điều kiện mở lại khi thấy endpoint/mode
khác; không giao lại phép đo owner-grace/lửa đã có. G3 cần sign-off riêng, scope37feature không đổi.
