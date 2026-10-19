> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Audit kế hoạch viết lại Rust — 2026-10-19

Kết luận: hướng tách core/asset/render/network và dùng client cũ làm nguồn đối chiếu hợp lý.
Kế hoạch hiện tại **chưa đủ cơ sở để cam kết ~100% gameplay KR hoặc chốt lịch phát hành**.
Vấn đề chính là chất lượng nguồn chuẩn, tiêu chí nghiệm thu và phạm vi, không phải tính khả thi của Rust.

Quyết định đã có: giao thức mới, không tương thích client cũ; desktop Windows/Linux/macOS;
server-authoritative; KR là mục tiêu sản phẩm, VN là nguồn tham chiếu. Không cần quyết lại các điểm này.

## Phạm vi và kiểm chứng

- Bản được audit: `docs/archive/plans/REWRITE_PLAN_2026-10-19.md` tại worktree `boom-re-l10n`, nhánh
  `claude/l10n-kr-vi`, HEAD trước audit `231ac01`.
- Đối chiếu `MASTER_PLAN.md`, `assets.md`, `map.md`, `items.md`, `oc_e-game.md`,
  `features.md`, `progress.md`, docs Việt hóa và source/tool/test liên quan.
- Đối chiếu worktree `boom-re` tại `0e71e8a` và repo `boomkr-re`, nhất là
  `docs/rust-rewrite.md`, `docs/kr-data.md`, `docs/kr-content.md`, `rust/krpack`.
  `REWRITE_PLAN.md` hiện chỉ có ở worktree l10n; server ở main đã đi xa hơn bản server trong worktree này.
- Đã chạy `cargo test --offline` tại **cả hai** thư mục `server/`: mỗi bản **7 test pass**.
  Đây là test codec, packet builder và golden so với fakeserver; không phải test tương đương gameplay.
- Audit source/docs; không chạy lại client Wine, không đo gameplay mới, không kiểm lại toàn bộ asset.
  Những tuyên bố runtime trong docs vẫn là nguồn tài liệu, không được nâng thành phép đo mới.

Mức độ: **P1** ảnh hưởng trực tiếp độ tin cậy của nghiệm thu/kiến trúc; **P2** ảnh hưởng phạm vi,
khả năng tái lập hoặc tổ chức triển khai. Không có lỗi P0 được xác lập trong phạm vi audit này.

## Phát hiện

### A01 — P1: VN là baseline hữu ích, chưa phải oracle đã xác nhận cho KR

**Bằng chứng:** `REWRITE_PLAN.md` §10 khẳng định “Feel Oracle cho KR là client VN”.
`boomkr-re/docs/rust-rewrite.md` §2–3 đánh dấu tái sử dụng luật lõi và suy ra các mode từ tên class/chuỗi;
chính §3 lại thừa nhận KR có thể đổi luật và mode có tên chưa chắc chạy được.
§7 loại Alchemist/Baseball khỏi chuẩn ~100%, trong khi kế hoạch chính đặt full feature dưới mục tiêu fidelity.

**Hệ quả:** replay đúng VN có thể vẫn sai KR. Sự hiện diện của tên class không xác nhận đường chạy,
tham số hoặc hành vi tương ứng. Archive/video cũng không đủ để làm golden pixel chính xác nếu thiếu
build, độ phân giải và điều kiện capture.

**Sửa:** lập ma trận theo build + mode + quy tắc: `measured_vn`, `verified_kr`, `inferred_kr`,
`archive`, `design`. Dùng VN khởi động phát triển; công bố rõ phần nào chỉ cam kết tương đương VN,
phần nào đã xác nhận KR, phần nào được thiết kế lại. Profile phải chứa khác biệt luật/tham số,
không chỉ danh sách pack/map/mode. Không yêu cầu phải mở khóa runtime KR trước khi bắt đầu code.

### A02 — P1: Log server giả lập đang bị dùng làm bằng chứng timing gốc

**Bằng chứng:** `REWRITE_PLAN.md` §3.2 ghi đúng rằng fuse 2,5 s là tạm.
Nhưng `boomkr-re/docs/kr-data.md` §10 lấy chênh lệch opcode 111 → 151 trong `fakeserver.log`
để kết luận đã đo được fuse 2,500 s. `server/src/session.rs` tự hẹn giờ bằng `BOMB_FUSE`
(mặc định 2,5 s). `server/tests/bomb.rs` chỉ kiểm parse/serialize packet, không đo luật nổ.

**Hệ quả:** đó là xác nhận giả lập làm đúng cấu hình của nó, không xác nhận thời gian server gốc.
Client cũ chơi trọn trận cũng không chứng minh các quyết định do server giả lập đưa ra giống bản gốc.

**Sửa:** giữ 2,5 s làm `DESIGN/provisional`, ghi rõ nguồn. Tách các sự kiện client tự quyết khỏi
sự kiện do server giả lập kích hoạt trong trace. Chỉ nâng độ tin cậy khi có đường tính/hằng số gốc
phù hợp hoặc nguồn độc lập đủ điều kiện. Không dùng log chính emu làm oracle cho thuật toán của emu.

### A03 — P1: Chưa chứng minh trace nguồn có thể tái lập

**Bằng chứng:** §4.2 gọi `xdotool` là “input tất định”, nhưng `tools/duo.sh` dùng keydown/keyup
và `sleep` theo thời gian thực. Đây là automation; chưa gắn input vào tick mô phỏng.
Plan chưa quy định seed, trạng thái đầu, đồng hồ, packet schedule và ảnh hưởng khi debugger dừng process.

**Hệ quả:** cùng script có thể được client nhận ở tick khác nhau. Lúc đó tiêu chí 0 px/±1 tick
không phân biệt được lỗi engine mới với nhiễu của quá trình thu nguồn chuẩn.

**Sửa:** trước khi lấy 15 trace, chạy lặp cùng một tình huống trên **client nguồn** và báo sai khác.
Fixture cần build hash, patch list, map/content hash, cấu hình emu, trạng thái đầu/seed nếu lấy được,
input thực tế theo tick/thời điểm nhận, packet và clock tương ứng. Chỗ không kiểm soát được phải
ghi giới hạn hoặc phân loại phép đo; không gọi toàn bộ harness là deterministic.

### A04 — P1: Chốt fixed tick/không float trước khi biết mô hình gốc

**Bằng chứng:** §3.2 chưa biết fixed/variable step; §5 chốt core fixed tick, không float.
§8 đề xuất nếu gốc variable-step thì chạy đúng công thức ở tần số cố định.

**Hệ quả:** đổi chuỗi delta-time có thể đổi tích lũy, làm tròn, thứ tự va chạm và thời điểm sự kiện;
giữ công thức không tự bảo đảm giữ hành vi. Đây là rủi ro đối với chính cam kết 0 px.

**Sửa:** giữ mục tiêu core deterministic nhưng để mô hình thời gian/số học là quyết định có gate sau R1.
Prototype đi thẳng/rẽ góc/va chạm phải chứng minh mô hình được chọn phù hợp nguồn chuẩn.
Nếu cố ý chuẩn hóa khác gốc, ghi thành thay đổi sản phẩm và điều chỉnh tiêu chí tương ứng.

### A05 — P2: Công cụ so ảnh được viện dẫn sai; thước đo chưa đủ bắt lỗi quan trọng

**Bằng chứng:** R3 và §9.3 dùng `tools/snapdiff.py` cho golden ảnh.
Nhưng script này (`tools/snapdiff.py` ở repo `boom-re`/`boom-re-l10n`) đọc `pre.snap`/`post.snap`, so thanh ghi, stack và vùng nhớ VM.
Nó không chụp hoặc so PNG. R0 còn yêu cầu hash file PNG khớp Python, trong khi cùng pixel có thể
được mã hóa thành file PNG khác nhau.

**Hệ quả:** tiêu chí R3 hiện chưa có công cụ thực hiện được như mô tả. Tỉ lệ 99,5% toàn màn hình
còn có thể che lỗi nhỏ nhưng quan trọng ở nhân vật/bom nếu phần nền giống nhau chiếm đa số.

**Sửa:** thêm hạng mục image-diff thực sự: chuẩn hóa RGBA/kích thước, xuất heatmap, đo cả toàn ảnh
và vùng nhân vật/bom/UI, kiểm frame animation theo thời gian. R0 so hash pixel đã decode + metadata
frame, không chỉ byte file PNG. Ghi rõ chính sách màu/alpha, vùng động và môi trường capture.

### A06 — P2: Asset decode chưa đồng nghĩa với asset chạy được trong engine

**Bằng chứng:** `docs/specification/assets/MAPS.md` §3 còn để registration/canvas/pivot ở độ tin cậy trung bình,
f2 thấp; parser map được dẫn ở `/tmp/opencode/parse_map.py`, ngoài repo.
`boomkr-re/rust/krpack/src/main.rs::zlib_inflate` hiện trả `Vec::new()`;
tên entry dùng `String::from_utf8_lossy` dù có tên cp949.
`boomkr-re/docs/kr-data.md` đã có ObjectTable, TileInfo và map script cần được sử dụng.

**Hệ quả:** xuất được ảnh không chứng minh dựng đúng animation, collision, script và UI.
Mốc “100% asset” đang gộp kiểm kê, decode và hiểu semantics thành một việc.

**Sửa:** tái sử dụng phần Rust reader đã có, bổ sung inflate/encoding/validation;
đưa parser và fixture cần thiết vào repo. Tách coverage: entry đọc được → payload decode được
→ metadata hiểu được → object/map render đúng → script/gameplay hoạt động đúng.
Ghi rõ interpreter/runtime cho map script, ObjectTable và các state chuyển động thuộc crate nào.
Không ép toàn bộ semantics chưa rõ hoàn tất trong mốc chuyển đổi asset đầu tiên.

### A07 — P2: Shared core chưa đủ để đặc tả netcode

**Bằng chứng:** §5 nói prediction rồi “reconcile hoặc rollback”, LAN giống offline và
“chống gian lận tự nhiên”. R5 chỉ yêu cầu ping mô phỏng 100 ms và trace giống offline.

**Hệ quả:** thiếu hợp đồng input/tick khiến client/server dễ lệch; rollback có thể phát lại tiếng,
particle hoặc phần thưởng. Server-authoritative vẫn cần kiểm input và mọi thay đổi trạng thái.

**Sửa:** trước R5 chốt thứ tự command, input trễ/mất/trùng, tick/sequence, seed/RNG,
snapshot đầy đủ và hash rules/content. Tách sự kiện mô phỏng khỏi side effect trình diễn;
quy định xác nhận/hủy sự kiện dự đoán. Test thêm jitter/loss/reorder, reconnect và input không hợp lệ.
So trạng thái authoritative sau hội tụ; định nghĩa riêng độ lệch hình ảnh/độ trễ được chấp nhận
trong lúc prediction. Không dùng “trace online = offline” chung cho mọi input và mọi thời điểm.

### A08 — P2: Release 1 và tiêu chí hình ảnh chưa thống nhất với phần Việt hóa

**Bằng chứng:** R5–R6 cần online/phòng chờ nhưng W1 lại để sẵn sàng, đội, nhân vật,
quyền chủ phòng và tùy chọn phòng ở sau Release 1. R4 “mọi map/toàn bộ item” chưa chốt tập nội dung
VN hay KR. `l10n/README.md` chọn font Unicode và bản dịch Việt; quiz cũng được bản địa hóa.
Gallery ảnh đã có, nhưng không đồng nghĩa mọi ảnh đã được dịch/duyệt.

**Hệ quả:** milestone có thể bị coi là xong dù chưa chơi được luồng phòng tối thiểu;
ngược lại, font/chữ Việt đúng yêu cầu lại bị đánh trượt vì khác golden Hàn/VN cũ.

**Sửa:** R1 sản phẩm cần danh sách map/item/mode cụ thể và toàn bộ thao tác phòng tối thiểu.
Tách profile đối chiếu nguyên bản khỏi profile Việt hóa; golden riêng cho phần thay đổi có chủ ý,
cùng test glyph, dấu, wrap, clipping, placeholder và tương tác. Import quyết định ảnh được duyệt
thành dữ liệu đầu vào build, không dựa vào cache trình duyệt.

### A09 — P2: Docs và kế hoạch chuyển workspace chưa có một nguồn chuẩn thống nhất

**Bằng chứng:** `MASTER_PLAN.md` vẫn có các gate/khóa pack KR cũ; `assets.md` còn ghi frame chưa giải
trong khi `map.md` đã giải một phần. `kr-data.md` §3 phủ định bảng drop rồi §6 đính chính có MonInfo.
Plan chính bảo chuyển `server/` sang `crates/boom-server` nhưng giữ server cũ làm oracle;
`tools/duo.sh`, `tools/match_play.sh`, `tools/gen_golden.py` vẫn trỏ đường dẫn `server/`.

**Hệ quả:** người triển khai có thể đọc nhầm kết luận cũ hoặc làm hỏng harness đo trong lúc đổi workspace.
Branch l10n cũng không phải snapshot server mới nhất để lấy làm nền một cách máy móc.

**Sửa:** ghi tài liệu nào supersede mục nào, sửa kết luận đầu tài liệu đã có đính chính,
và chốt baseline commit khi tích hợp. Giữ legacy server/adapter với test và launch command rõ ràng,
tách khỏi server mới. Migration chỉ đạt khi golden cũ, script launch và fixture path vẫn dùng được.

### A10 — P2: Lịch hiện tại là ước lượng thô, chưa phải lịch dựa trên dependency

**Bằng chứng:** §6 có thời lượng từng pha, R0/R1 song song và tổng 6–8 tháng,
nhưng chưa có critical path, phân bổ người, gate nguồn chuẩn KR hoặc thời gian xử lý dữ liệu chưa hiểu.
Các pha còn phụ thuộc lẫn nhau qua collision, animation, script và authority.

**Sửa:** dùng mốc có deliverable thay cho cam kết tháng ngay lúc này. Sau prototype đầu tiên,
đo throughput theo map/item/màn hình và lập lại lịch với dependency, người phụ trách, buffer và
phạm vi fidelity thực tế. Chưa có bằng chứng để kết luận con số 6–8 tháng chắc chắn đúng hoặc sai.

## Thứ tự đề nghị sau audit

1. **Chốt baseline và nhãn bằng chứng:** build mục tiêu KR, nguồn VN, quyết định thiết kế lại,
   tập nội dung Release 1; đồng bộ các kết luận docs đã lỗi thời.
2. **Một vòng đo nguồn có thể lặp:** một map, đi thẳng/rẽ góc, đặt bom; phân biệt timing
   client với timing emu, lưu fixture và báo nhiễu giữa các lần chạy.
3. **Một lát cắt chơi được:** reader Rust + một map + nhân vật + một bom; chưa cần port hết asset.
   Dùng nó để quyết mô hình thời gian/số học, frame metadata, blend và collision.
4. **Hai người qua server authoritative:** command/snapshot/reconciliation, kiểm delay/jitter/loss;
   giữ luồng cũ hoạt động làm công cụ tham chiếu.
5. **Sau đó mới chốt lại roadmap mở rộng** asset, UI, Việt hóa và các mode; cập nhật ước lượng từ số đo.

Các bước này có thể tiến hành song song ở phần độc lập. Không cần đợi RE xong toàn game mới viết Rust;
cần tránh biến giả thuyết hoặc giá trị giả lập thành tiêu chí “đúng bản gốc”.
