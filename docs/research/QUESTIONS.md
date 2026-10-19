# Câu hỏi nghiên cứu ưu tiên V1

Cập nhật: 2026-10-19. Baseline **KR-00967-V1-scope-1**; tất cả câu hỏi dưới đây **open**.
Nguồn chuẩn về backlog nghiên cứu, không phải danh sách implementation đã xong.
Đọc [AGENTS](../../AGENTS.md), [REWRITE_PLAN](../planning/REWRITE_PLAN.md),
[SCOPE V1](../planning/SCOPE_V1.md), [ARCHITECTURE](../planning/ARCHITECTURE.md).

Owner/reviewer là vai trò; integrator gán người/agent và phạm vi file khi mở task.
P1: ảnh hưởng boundary/interface hoặc chặn task V1 phụ thuộc; P2: cần trước tích hợp/phát hành,
không chặn toàn bộ planning. `Done` nghĩa câu hỏi được trả lời có kiểm chứng **hoặc** có quyết định
DESIGN/giới hạn/thí nghiệm tiếp theo được reviewer chấp nhận; không có nghĩa engine đã triển khai.
Nguồn bên dưới là tài liệu đã có, chưa được đo/web lại trong lượt tổ chức docs này.

## Backlog đang mở

| ID / ưu tiên, ảnh hưởng | Owner / reviewer | Đã biết và nguồn | Chưa biết | Phương pháp / đầu ra / điều kiện done | Dependency kiến trúc |
|---|---|---|---|---|---|
| Q-001 / P1: mọi claim KR | research / RE | Target 00967; VN chạy được; [audit A01](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | VN đã được chấp nhận ngang KR theo ADR0002; còn khác biệt cụ thể nào về content/ID/mode/event? | Lập claim matrix theo build/feature; mỗi claim có nguồn, locator, trạng thái và reviewer; done khi mọi rule V1 được phân loại, unknown có hướng xử lý | Build/content profile, provenance rules; đi trước Q-004–007 |
| Q-002 / P1: content closure G1 | assets/RE / core | Village00, 103 fingerprint và 4 tọa độ đã có ở [manifest](../evidence/inventory/kr_v1_content.json) | Tile/object/collision, spawn có bị block, đường thoát, dependency/script còn thiếu? | Resolve map→payload→consumer, ghi dependency manifest/unknown; done khi closure có thể kiểm và bốn spawn được kiểm, không skip metadata | Asset API, map/collision model; Q-003, Q-004 |
| Q-003 / P1: state trình diễn G1/G5 | assets/client / RE | Sprite decode có; frame pivot/canvas một phần còn suy luận, [audit A06](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | C00/C01 walk/idle/trapped/dead, timing frame, layer/blend, font/UI dependency? | Trace consumer hoặc đối chiếu có ngữ cảnh; xuất mapping state→animation và scene fixtures; done khi field đã hiểu và giới hạn còn lại rõ | Asset/render/audio ownership, coordinate/unit contract; Q-002 |
| Q-004 / P1: time/numeric model G2 | RE / core+QA | VN có thể đo, xdotool/sleep không bảo đảm input theo tick; [audit A03–04](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | Fixed/variable step, độ lặp input/clock, rounding, sai khác do debugger? | Thiết kế phép đo lặp đi thẳng/rẽ/chạm tường, metadata build/patch/emu/input/clock; done khi có repeatability report và ADR chọn mô hình hoặc thí nghiệm phân biệt tiếp | Core tick, numeric types, input/replay; Q-001, Q-009 |
| Q-005 / P1: movement/stats G2 | RE / core | C00/C01 được chọn; speed-level chưa phải px/s | Nối Attr đúng nhân vật, speed/hitbox/cornering, block theo state, tăng Roller/cap? | Đối chiếu Attr/consumer và fixture vận động; bảng rule có đơn vị/provenance; done khi task movement có contract kiểm được hoặc DESIGN rõ | Character/rules schema, collision query; Q-002, Q-004 |
| Q-006 / P1: bomb/damage G3 | RE / core | Luật cứu/chọc đã chốt từ [VN-B01](findings/VN_BATTLE_RUNTIME_REFERENCE.md); fuse 2,5 s trong emu là provisional; [audit A02](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | Fuse/flame/chain/trap, tầm/rào cản, bomb grace, duration/clock start và thứ tự sự kiện đồng thời? | Tách event client/emu; static/source khác build chỉ là hỗ trợ; bảng rule + boundary scenarios; done khi từng rule có nguồn hoặc DESIGN và acceptance | Tick event ordering, damage/bomb state, match end; Q-004–005 |
| Q-007 / P1: drop/pickup G3 | core/design / RE+QA | [VN-B03](findings/VN_BATTLE_RUNTIME_REFERENCE.md) xác nhận đồng bộ hiện/lộ/nhặt item hai client; Village00 gốc chỉ Lucci; V1 đổi sang Bomb/Fire/Roller có chủ ý | Drop chance/weights/cap, thời điểm hiện, flame-item, pickup cùng tick hoặc khi đã cap? | Đối chiếu consumer và đề xuất profile có version/seed; done khi profile DESIGN và idempotence scenarios được review; không gọi weight là % | Rule profile, RNG, entity order; Q-006, Q-008 |
| Q-008 / P1: state/protocol G4 | core+net / QA | Authority mới, không client cũ; lifecycle policy đã chốt ở SCOPE V1 | Command window/sequence, snapshot đầy đủ, RNG/hash, dedup, prediction event identity? | Viết contract revision đầu + ví dụ input trùng/trễ/chain/chết cùng tick; done khi core/client/server dùng cùng định nghĩa ownership và state | InputCommand/SimulationState/Event/Snapshot/Replay; Q-004, Q-006–007 |
| Q-009 / P1: oracle/fixture | QA/RE / core+client | snapdiff.py so bộ nhớ, không so PNG; test server là codec/packet golden; [audit A05](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | Fixture nào độc lập, điều kiện capture/color/ROI, sai số phép đo và profile Việt hóa? | Spec fixture manifest, repeatability và RGBA/ROI diff; done khi xác định provenance, metric theo đối tượng và giới hạn, phân biệt regression/fidelity | Oracle adapter, render/audio events; Q-003–004, Q-012 |
| Q-010 / P1: transport/reconciliation | net / core+QA | RTT100 ms/jitter/loss fixture là acceptance V1, chưa chọn transport | Reliable/unreliable channels, ack/input backlog, stall/reconnect, correction policy? | So phương án theo command/state contract; ADR + kế hoạch thử loss/reorder/dup/timeout; done khi có lựa chọn và rủi ro đo được | Network boundary và dependency Rust; Q-008 |
| Q-011 / P2: identity/persistence | backend / integrator+QA | V1 chỉ identity/options; room không phục hồi; một active session/account | Account credentials/session lifecycle, storage schema, migrations/logging, giới hạn lưu? | Thiết kế data ownership/threat cases tối thiểu; ADR + acceptance duplicate login/restart; done khi chọn giải pháp phù hợp V1 | Backend/storage adapter, secrets/config; Q-008 |
| Q-012 / P2: UI/Việt hóa G5 | client/l10n / QA | Catalog text có 23 review; gallery ảnh là dữ liệu review, không chứng minh đã duyệt | Chuỗi/ảnh nào thực dùng V1, fallback/font/license/wrap/scale/focus/audio nào cần? | Lập UI/content subset và quyết định review; golden profile Việt hóa + glyph/placeholder/focus cases; done khi subset và unknown có owner, export review là input build | Content bundle/UI/text/audio interface; Q-003, Q-009 |
| Q-013 / P1: baseline/migration | integrator / reviewer | Main server mới hơn l10n; tools legacy dùng server/; [audit A09](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | Baseline commit tích hợp, layout mới, lệnh oracle/fixture sau chuyển? | ADR workspace/migration map; done khi chọn commit và checklist giữ legacy build/launch/fixture, không copy snapshot cũ vào server mới | Workspace boundaries, tool paths; độc lập Q-002–012 |
| Q-014 / P2: readiness/build matrix | integrator/QA / architecture | V1 ba OS, chưa có cam kết thời gian; [audit A10](../archive/reports/REWRITE_PLAN_AUDIT_2026-10-19.md) | Dependency/toolchain/CI hỗ trợ, artifact nào dùng chung, task đầu đủ độc lập? | Review ADR/contract theo REWRITE_PLAN P3; task có owner/reviewer/acceptance; done khi blocking assumptions hiện rõ và từng task có đầu vào đã chốt | Toàn bộ architecture, Q-008/Q-010/Q-013; không cần mọi gate release xanh |

## Cách nhận task và đóng câu hỏi

1. Chọn câu hỏi nhỏ, ghi owner/reviewer thực tế, baseline, phạm vi file và dependency revision.
2. Dùng [source register](sources/README.md) và spec/findings hiện có trước khi mở nghiên cứu mới.
   Ưu tiên Q-001/002/004/008/013 để kiến trúc và phép đo đi song song; không RE mode later.
3. Bàn giao claim/fixture/spec/ADR tại nguồn chuẩn; ở đây thêm link kết quả, status và lý do done.
   Nếu chỉ có nguồn khác region hoặc emu, giữ nhãn tương ứng; không nâng thành verified_kr.
4. Unknown có ảnh hưởng interface phải báo integrator trước khi giao coding task phụ thuộc.
   Không tạo một plan mới, không coi deadline hoặc biểu quyết agent là bằng chứng.

Các mode/item ngoài V1 trong nguồn web giữ làm đầu mối later; chỉ mở task nghiên cứu khi có ưu tiên.

## Nguồn mới đã nhận

[ADR0002](../decisions/0002-vn-kr-trusted-sources.md) chấp nhận VN ngang KR; không yêu cầu tái đo KR chỉ do region.
[Runtime trận VN](findings/VN_BATTLE_RUNTIME_REFERENCE.md) trả lời một phần Q-006/007 và bổ sung Q-009/013
(lỗi server cũ giữ cổng, Wine, main `4ccc890`). Các câu hỏi còn mở do phần timing/race/contract chưa giải,
không phải do bằng chứng VN bị xem là kém tin cậy. Tiến độ engine và scope giữ nguyên.
