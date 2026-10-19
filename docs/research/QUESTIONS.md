# Câu hỏi nghiên cứu ưu tiên V1

Cập nhật: 2026-10-19. Baseline **KR-00967-V1-scope-1**. Nguồn chuẩn về backlog nghiên cứu.
Claim đã có nằm ở [RULES_V1](../specification/gameplay/RULES_V1.md); ở đây chỉ ghi cái còn thiếu và cách đóng.
Hạn chung của P1: **2026-11-02** ([REWRITE_PLAN](../planning/REWRITE_PLAN.md#hạn-của-pha-chuẩn-bị)).

P1: chặn interface hoặc task V1 phụ thuộc; P2: cần trước tích hợp/phát hành.
Trạng thái: `open`, `partial` (đã có quyết định tạm, còn hiệu chuẩn), `proposed-close` (chờ reviewer), `done`.
`done` nghĩa câu hỏi được trả lời có kiểm chứng **hoặc** có quyết định `design`/giới hạn được reviewer chấp nhận.

## Backlog

| ID / ưu tiên | Trạng thái | Owner / reviewer | Đã biết | Còn thiếu | Cách đóng |
|---|---|---|---|---|---|
| Q-001 / P1 khác biệt VN↔KR | partial | research / RE | ADR 0002: VN ngang KR. Khác biệt đã thấy: header map +4 byte, ID tile/obj lệch +937/+938 (R-003, R-004) | Khác biệt về mode/item/Attr cho phần V1 | Mỗi claim V1 có nhãn trong RULES_V1; khác biệt mới ghi thành claim theo build |
| Q-002 / P1 content closure G1 | partial | assets/RE / core | Village00 KR: lưới, spawn, viền, bảng rơi; bố cục trùng VN (R-002…R-007); item ẩn chỉ ở `CGOTileFixedBox` (R-008) | Bảng thuộc tính tile/obj của theme Village: ID nào cứng/mềm/đi được; dependency render | START-A: đọc theme (KR, đối chiếu VN; quy tắc tra theme ở [legacy server](findings/VN_LEGACY_SERVER_REFERENCE.md) §1), xuất bảng phân loại vào `data/`; R-006/R-007 → confirmed |
| Q-003 / P1 state trình diễn G1/G5 | open | assets/client / RE | Frame 48 byte: rect/canvas/pivot (MAPS §3) | Animation walk/idle/trapped/dead của C00/C01, timing frame, layer/blend | Trace consumer render hoặc đối chiếu ảnh chạy VN; mapping state → animation |
| Q-004 / P1 time model G2 | partial | RE / core+QA | Mô hình đã chọn: fixed step, số nguyên (ADR 0004). Client VN có bước `vf159` theo đồng hồ `timeGetTime` | Tần số tick gốc; cách tích lũy chuyển động mỗi tick | Đo trên VN: đếm lần gọi `vf159`/giây và độ dịch mỗi tick; cập nhật `sim.ticks_per_second` |
| Q-005 / P1 movement/stats G2 | partial | RE / core | BomberAttr C00/C01 (R-040, R-041); nghĩa field là candidate (R-042) | Speed-level → px/s (R-043); hitbox; bám góc; Roller (R-045) | Đo VN: thời gian đi 10 ô ở 2 mức speed; tìm consumer field 2–7; profile dùng 24 px/s (design) tới khi có số |
| Q-006 / P1 bom/damage G3 | partial | RE / core | Fuse do server quyết (R-020); nhốt → chết ≈ 6,3 s (R-026); địch chạm → chết (R-028); tầm/số bom (R-025) | Fuse gốc (R-021, không khôi phục được); lửa tồn tại; chain; thứ tự sự kiện cùng tick; hằng số timer nhốt chính xác | Fuse/lửa: chốt `design` ở hạn P1; timer nhốt: tìm hằng số trong client VN; thứ tự tick: boundary scenario trong START-C |
| Q-007 / P1 drop/pickup G3 | partial | core/design / RE+QA | Item ẩn quyết trước trận (R-050); trọng số Bomb/Fire/Roller (R-051); nhặt có xác nhận (R-054) | Ai dùng bảng trọng số (R-052); xác suất gạch có item (R-053); nhặt cùng tick; cap | Tìm consumer bảng `this+436056`; nếu không tìm ra trước hạn → `design` 35% có claim OSS |
| Q-008 / P1 state/protocol G4 | partial | core+net / QA | contracts-r3 trong code (state, event V1, pipeline, quyền ghi, bất biến; ARCHITECTURE §5) | InputCommand/Snapshot/codec (#14) | contracts-r3 đã approve (#2); còn ADR codec |
| Q-009 / P1 oracle/fixture | open | QA/RE / core+client | `snapdiff.py` so bộ nhớ VM; test server là packet golden | Fixture độc lập nào dùng được, điều kiện capture, sai số đo | Spec fixture manifest + độ lặp; tách regression/fidelity |
| Q-010 / P1 transport/reconciliation | partial | net / core+QA | Mặc định TCP sau trait, input dư thừa (ADR 0006) | Ngân sách correction/độ trễ p95, reconnect | V1-027: chạy fixture mạng, so ngân sách; đảo theo ADR 0006 nếu vượt |
| Q-011 / P2 identity/persistence | open | backend / integrator+QA | V1 chỉ identity/options; một phiên/tài khoản | Credential/session, schema, migration, log | ADR backend + acceptance duplicate login/restart |
| Q-012 / P2 UI/Việt hóa G5 | open | client/l10n / QA | Catalog text ở `boom-re-l10n/l10n/`; 23 mục chờ review; ~150 khóa UI tiếng Việt tự viết trong `boom-re/server/src/strings.rs` (ứng viên) | Tập chuỗi/ảnh V1 thực dùng, font/license/wrap/scale/focus | Lập subset UI V1 + quyết định review; golden profile Việt hóa |
| Q-013 / P1 baseline/migration | proposed-close | integrator / reviewer | ADR 0003: repo `boom-rs`; legacy ở `boom-re` main; docs import từ l10n@9bde2ce | Review chuyển repo (#1); chạy client Wine từ main chưa kiểm lại | `cargo test` của `boom-re/server` đã xanh ở 9ac2075; đóng khi #1 approve |
| Q-014 / P2 readiness/build matrix | partial | integrator/QA / architecture | Toolchain ghim; CI ba OS xanh trên GitHub; hook pre-push; task đợt 1 #1–#15. Quyết định chủ dự án 2026-10-19: **không khóa nhánh**, giữ kỷ luật PR + hook | Toolchain đóng gói client | Đóng gói theo ADR client (#12) |

## Cách nhận và đóng câu hỏi

1. Chọn câu hỏi, ghi owner/reviewer thực tế, baseline, phạm vi file, dependency.
2. Dùng [source register](sources/README.md), RULES_V1 và findings trước khi mở nghiên cứu mới.
   Không RE mode later.
3. Kết quả vào RULES_V1 (claim) và `data/rules/` (giá trị); ở đây cập nhật trạng thái và lý do.
4. Unknown ảnh hưởng interface phải báo integrator trước khi giao task code phụ thuộc.

Mode/item ngoài V1 trong nguồn web giữ làm đầu mối later.
