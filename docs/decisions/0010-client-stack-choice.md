# 0010 — Chọn client stack: macroquad cho scene V1

Status: **Accepted (có điều kiện)**, 2026-10-19. Thay [0007](0007-client-stack-spike.md) (Proposed).
Quyết định chọn macroquad dựa trên chi phí build/dependency/code và tính nhất quán pixel; các tiêu chí
ADR 0007 chưa đo đủ được liệt kê ở §4.5, cần integrator/Claude duyệt ngoại lệ và P6 hoàn tất phép đo
trước khi coi 0007 đã đóng.
Owner: client. Reviewer: integrator/Codex. Task: GitHub issue #12, nhánh `issue-12`,
baseline `main @ e50dfb4` (contracts-r3). Spike: `spikes/client-macroquad/`,
`spikes/client-winit-softbuffer/`; **không** merge code spike vào `crates/`.

> 0007 vẫn giữ nguyên làm lịch sử. Index `docs/decisions/README.md` và status của 0007 do
> integrator cập nhật khi merge ADR này (index ADR thuộc integrator theo AGENTS §5).

## 1. Bối cảnh

[ADR 0007](0007-client-stack-spike.md) chốt cách chọn client stack bằng một scene V1:

- sprite RLE alpha, pivot/canvas, z-order, integer scale 1×/2×;
- chữ Việt Unicode có dấu;
- BGM + SFX theo event;
- input bàn phím, mất focus không giữ phím;
- build Windows/Linux/macOS; vòng render tách khỏi tick mô phỏng;
- tối đa hai phương án, mỗi phương án time-box 3 ngày; kết quả là ADR Accepted kèm ảnh/số đo.

Spike chỉ dùng **ảnh đã decode**; không chờ content closure đầy đủ (START-A/Q-002), không
dựng gameplay/mạng/collision.

## 2. Hai phương án đã spike

| | A — macroquad | B — winit + softbuffer + fontdue |
|---|---|---|
| Render | framework 2D nhẹ, GPU (OpenGL/GLES/Metal backend của miniquad) | cửa sổ `winit` + framebuffer CPU `softbuffer`, tự viết blit/text |
| Text | `macroquad::text` (fontdue atlas) | fontdue rasterize + layout thủ công |
| Audio | feature `audio` → `quad-snd` 0.2.8 | `quad-snd` 0.2.8 (dùng chung để cô lập phần render/text) |
| Ảnh | `image` 0.25 (PNG) | `image` 0.25 (PNG) |
| Version | macroquad 0.4.16 | winit 0.30, softbuffer 0.4, fontdue 0.9 |
| Code game | 691 dòng | 823 dòng |

Lý do chỉ hai phương án:

- ADR 0007 giới hạn **tối đa hai**. Chọn một framework nhẹ (A) và một phương án tự dựng
  window/renderer/UI (B) để đo đúng câu hỏi "chi phí tự viết renderer/UI so với độ chính xác pixel".
- Phương án **window + wgpu** trong bảng 0007 không chạy được đầy đủ trong môi trường spike:
  máy không có Vulkan/GPU thật (chỉ llvmpipe qua Xvfb), nên số đo GPU và pixel sẽ không đại diện.
  Phương án B dùng CPU framebuffer để có tham chiếu pixel tất định, vẫn đo chi phí tự dựng renderer/text.
  Đây là **điểm chưa kiểm chứng** của ADR, ghi ở §7.
- Bevy bị loại khỏi lượt này vì bề mặt dependency/ECS lớn hơn nhiều so với nhu cầu scene 2D và
  thời gian build; SDL3 binding bị loại vì thêm thư viện C và ma trận đóng gói 3 OS, trong khi
  mục tiêu lượt này là đo pixel/text/audio tối thiểu. Hai phương án này vẫn có thể được xem xét
  nếu điều kiện đảo ở §7 xảy ra.

## 3. Scene và cách đo

Asset decode bằng `spikes/client-macroquad/tools/prepare-assets`:

- đọc pack KR bằng `boom-assets` (PR #28), không copy pack/asset vào repo;
- giải mã `CDataI16Alpha` v2, frame record 48 byte (`f0/f1` registration, `f3` ảnh,
  `f4..f7` source rect), ghi PNG từng frame;
- lấy tên tile/object từ `data/maps/village00-kr.toml` (#9);
- ghi `scene.json` + PNG + OGG vào `local/asset-spike/` (gitignored).

Scene dùng chung cho cả hai stack, logic 600×560:

- HUD 40 px, map Village00 15×13 ô, tile 40 px (600×520);
- nền + object tĩnh (cây, nhà, hộp, fountain) sort theo `y`;
- C00 `WalkRight` (frame theo `--time-ms 300`; đủ 4 hướng trong `scene.json`);
- `Bomb00` + `BombExplosion004` (frame theo `--time-ms 300`, khói nổ frame **#4**); frame animation
  được sắp theo **số** (`#0,#1,…,#10,…`), không theo thứ tự chuỗi của `BTreeMap`;
- một dòng chữ Việt có dấu:
  `Village00 · C00 đi 4 hướng · Bom nổ · Nhặt Bom, Lửa, Roller · 1×/2×`;
- BGM `MP3/Village`, SFX `Wave/BombExplosion001` (cùng OGG);
- scale 1×/2× bằng nearest-neighbor (render target 600×560 rồi phóng to).

Quy ước trình diễn (không phải luật game):

- object tĩnh: đặt theo `x = cell_x*40 - f0`, `y = cell_y*40 + 40 - (f1+1)`;
- nhân vật/bom/nổ: canh giữa ngang theo ô, đáy theo `f1`;
- sheet nổ gốc dùng additive/nền đen; spike chuyển sang alpha theo độ sáng
  (`alpha = max(r,g,b)`) vì macroquad không có blend per-draw đa nền tảng.

Ý nghĩa `f0/f1` vẫn là **candidate** (Q-003); scene này chỉ để so hai stack trên cùng dữ liệu,
không chứng minh cách client KR đặt pivot.

Snapshot xác định: `--time-ms 300 --no-audio`; macroquad chạy Xvfb + Mesa llvmpipe,
phương án B render offscreen. So pixel bằng
`spikes/client-winit-softbuffer/tools/compare.py`, bỏ vùng HUD. Kiểm trực tiếp scale bằng
`--check-2x-from`: ảnh 2× phải bằng nearest expansion của đúng ảnh 1× (không suy từ tỉ lệ 4× diện tích).

## 4. Kết quả

Máy: AMD Ryzen 5 3600 (6 core/12 thread), Linux, Rust 1.99. Build release target sạch,
registry đã warm. Chi tiết + lệnh tái hiện:
[`measurements/README.md`](../../spikes/client-macroquad/measurements/README.md).

### 4.1 Build, kích thước, dependency

| Chỉ số | macroquad | winit+softbuffer |
|---|---|---|
| Build Linux release | **16,95 s** | 33,99 s |
| Binary Linux release | **2.634.304 B** (2,51 MiB) | 6.216.808 B (5,93 MiB) |
| Build Windows GNU release | 15,19 s | **13,50 s** |
| `.exe` Windows | 9.156.705 B | **7.966.962 B** |
| Crate transitive (Linux) | **53** | 117 |
| Dòng code game | **691** | 823 |

Thời gian build và số Windows/macOS giữ từ lần đo head `66e91d7`; vòng sửa fixture/copy shader chỉ đo lại
pixel, audio, binary và LOC Linux ở head hiện tại, không build lại Windows/macOS trên host này.

Ảnh: [`build-size.png`](../../spikes/client-macroquad/measurements/build-size.png).

Linux release nghiêng rõ về macroquad (½ thời gian, nhỏ hơn ~2,4 lần, ít hơn 64 crate). Trên Windows,
phương án B build nhanh hơn và `.exe` nhỏ hơn, nhưng chênh lệch nhỏ và không đổi kết luận về
chi phí bảo trì/độ phức tạp.

### 4.2 So sánh pixel (regression)

Cùng scene, so snapshot 1× và 2× (bỏ HUD):

| Vùng | Pixel | Khác | Tỉ lệ | Max Δ |
|---|---|---|---|---|
| Scene 1× (600×520) | 312.000 | 69 | **0,022%** | 1/255 |
| Scene 2× (1200×1040) | 1.248.000 | 276 | **0,022%** | 1/255 |
| HUD chữ Việt 1× | 24.000 | 2.560 | 10,7% | 132/255 |

- Toàn bộ 69 pixel khác ở scene 1× nằm trong vùng khói nổ bán trong suốt (bbox x 352–367,
  y 279–292), chỉ lệch 1 LSB do làm tròn blend 8-bit. Không có pixel khác ở nền, tile, object,
  nhân vật, bom, hay biên sprite.
- Scene 2× có 276 pixel khác; **kiểm trực tiếp** `compare.py --check-2x-from` cho `mismatch = 0`,
  tức ảnh 2× là nearest expansion của đúng ảnh 1×. Đây chỉ là kiểm scale, không chứng minh
  pivot/alpha/z-order; phần đó do fixture tổng hợp ở §4.6 kiểm tra.
- HUD khác do khử răng cưa/vị trí glyph của GPU so với blit CPU; ảnh so cho thấy đủ dấu
  tiếng Việt, không thiếu glyph/clipping. Đây là khác biệt trình bày, không phải lỗi mapping.
- Ảnh nhiệt: [`pixel-diff-scene-1x.png`](../../spikes/client-macroquad/measurements/pixel-diff-scene-1x.png).

Kết luận pixel scene: đây là **so sánh/regression giữa hai implementation**, không phải oracle
fidelity KR. Hai renderer dùng chung decoder, quy ước anchor/sort và chuyển additive→alpha, nên
scene này không tự bắt được lỗi chung; fixture tổng hợp §4.6 là phép kiểm độc lập riêng.

Trong lúc làm fixture, phát hiện macroquad làm tối vùng bán trong suốt khi đưa render target ra
cửa sổ: alpha của target bị nhân đôi bởi blend mặc định. Spike đã sửa bằng copy shader tắt blend ở
bước composite cuối (target RGB đã đúng); đây là chi tiết triển khai P6, không phải thay đổi luật.

### 4.3 Renderer CPU (phương án B)

Release, cùng scene, không tính cửa sổ:

| | 1× (600×560) | 2× (1200×1120) |
|---|---|---|
| Thời gian/frame | 2,782 ms | 7,926 ms |

Vẫn dưới ngân sách 16,7 ms/frame cho 60 Hz, nhưng đây là renderer CPU tự viết; chi phí bảo trì
và mở rộng (blend, effect, animation, UI) cao hơn đáng kể so với framework.

### 4.4 3 OS

- **Linux**: build + chạy + snapshot cả hai stack.
- **Windows GNU**: build release **thành công** cả hai (`.exe` ở bảng trên); chưa chạy trên Windows.
- **macOS**: `cargo check --target x86_64-apple-darwin` **pass** cả hai; `cargo build` bị chặn
  trên host Linux vì thiếu SDK macOS (`cc: unrecognized command-line option '-arch'`).
  Cần runner macOS thật để link/chạy — đây là giới hạn của spike, không phải kết luận code hỏng.

### 4.5 Ma trận tiêu chí ADR 0007

| Tiêu chí | Trạng thái | Bằng chứng / phép đo còn thiếu |
|---|---|---|
| Sprite RLE alpha | đo trên scene + fixture | decode + render; fixture §4.6 kiểm alpha độc lập |
| Pivot/canvas (f0/f1) | **đạt fixture tổng hợp** | anchor bất đối xứng [12,37]/[50,58]/[88,60]; expected độc lập; macroquad Δ≤1, winit 0; Q-003 với asset KR vẫn candidate |
| z-order | **đạt fixture tổng hợp** | 3 lớp chồng, thứ tự y; expected độc lập; macroquad Δ≤1, winit 0 |
| Alpha/blend | **đạt fixture tổng hợp** | alpha 64…255; macroquad lệch ≤1 LSB, winit 0; additive không có per-draw, quy đổi trước |
| Integer scale 1×/2× | đo | `--check-2x-from` mismatch 0 cho cả hai; fixture 2× so trực tiếp expected |
| Chữ Việt | đo một phần | HUD 2.560/24.000 pixel khác, đủ glyph; chưa shaping/wrap/fallback |
| BGM/SFX | load/decode pass | `--audio-smoke` pass cả BGM+SFX; playback/timing chưa đo vì không có soundcard |
| Input-held/focus | **chưa đo** | code chỉ xử lý cạnh `B/S/R/Esc`; C00 tự đổi hướng theo thời gian; cần test giữ phím/mất focus |
| Build+run 3 OS | một phần | Linux build+run; Windows chỉ build; macOS chỉ `check`; gate 3 OS chưa đạt |
| Render tách tick | **chưa kiểm** | spike chưa có tick mô phỏng; presentation contract ở P6 |
| GPU thật | **chưa đo** | chỉ llvmpipe/Xvfb; frame time GPU thật chưa có |
| Frame animation | đã sửa | sắp theo số; 300 ms = explosion `#4`; snapshot/heatmap tái tạo |

Vì ma trận còn mục **chưa đo** (input-held/focus, build+run 3 OS, GPU thật), ADR không tuyên bố
đã đạt đầy đủ acceptance của ADR 0007. Quyết định chọn macroquad là **có điều kiện**, dựa trên chi
phí/độ phức tạp và tính nhất quán pixel; pivot/alpha/z-order đã có phép kiểm độc lập ở §4.6.
P6 vẫn phải hoàn tất các phép đo còn lại trước khi coi 0007 đã đóng.

### 4.6 Fixture pixel tổng hợp, expected độc lập

`spikes/client-macroquad/measurements/pixel_fixture.py` sinh sprite **bằng code** (không dùng asset
game): bốn ảnh 100×80 có pattern màu và alpha 64/96/128/160/200/255; scene 4×4 với ba object chồng
nhau theo y (z-order) và pivot bất đối xứng:

| sprite | cell | anchor | vị trí 1× |
|---|---|---|---|
| `layer_a` | (1,1) | [12,37] | (28,82) |
| `layer_b` | (2,2) | [50,58] | (30,101) |
| `layer_c` | (3,3) | [88,60] | (32,139) |

Expected được tính bởi compositor Python riêng, không gọi code renderer/decoder:
`out = (src·a + dst·(255-a) + 127) // 255`, vị trí
`x = origin_x + cell_x·tile - anchor_x`, `y = origin_y + cell_y·tile + tile - (anchor_y+1)`.
Additive: hai spike không có blend additive per-draw; additive được quy đổi sang alpha trước khi
render, nên fixture chỉ kiểm straight alpha (đã ghi rõ trong script/manifest).

`run-pixel-fixture.sh` của từng spike render fixture rồi so `expected-{1,2}x.png` với output thật:

| Stack | 1× khác | Max Δ | 2× khác | Max Δ |
|---|---:|---:|---:|---:|
| macroquad | 3.431/312.000 (1,10%) | 1 | 13.724/1.248.000 (1,10%) | 1 |
| winit+softbuffer | 0/312.000 | 0 | 0/1.248.000 | 0 |

Ngưỡng chấp nhận: **max Δ ≤ 1** và **số pixel lệch ≤ 2%**. Lý do: compositor dùng làm tròn
integer `+127/255`, GPU có thể lệch 1 LSB khi blend; Δ≥2 hoặc lệch >2% là lỗi hệ thống (pivot,
layer, alpha), không phải làm tròn. Macroquad đạt ngưỡng với 1,10% pixel lệch 1 LSB; winit khớp
tuyệt đối. Đây là kiểm **implementation** trên dữ liệu tổng hợp, không phải oracle fidelity KR
(asset/map/pivot Q-003 của client gốc vẫn chưa đối chiếu).

## 5. Quyết định

**Chọn phương án A — macroquad 0.4.16 + quad-snd — làm client stack khởi điểm cho V1.**
Phương án B (winit + softbuffer + fontdue) được giữ làm tham chiếu kỹ thuật/regression pixel
(không phải oracle fidelity), không đưa vào `crates/`.

Lý do:

1. **Tính nhất quán pixel giữa hai stack** trên scene; khác biệt còn lại chỉ 69 pixel lệch 1 LSB
   ở khói nổ và khử răng cưa chữ. Fixture tổng hợp §4.6 kiểm độc lập pivot/alpha/z-order: macroquad
   lệch ≤1 LSB trên 1,10% pixel, winit khớp tuyệt đối. Đây là bằng chứng implementation, không phải
   oracle fidelity KR.
2. **Chi phí thấp hơn rõ rệt**: Linux release nhanh gấp ~2, binary nhỏ hơn ~2,4 lần, ít hơn 64
   crate, ít hơn ~200 dòng code game; text/audio đã tích hợp, không phải tự viết layout/blit.
3. **Ít rủi ro tích hợp P6**: một dependency framework thay vì tự ghép winit + softbuffer +
   fontdue + layout + blend; vòng render vẫn tách được khỏi tick mô phỏng qua presentation contract.
4. **Đủ kiểm soát cho V1**: integer scale bằng render target + nearest đã kiểm expansion trực tiếp,
   PNG decode sẵn; cần **copy shader tối thiểu** cho bước composite render target (đã có trong spike,
   phát hiện nhờ fixture), không cần shader effect phức tạp. Input/focus có API nhưng **chưa đo**.
5. **Có đường lùi**: phương án B đã chứng minh scene/pixel có thể dựng bằng CPU; nếu macroquad
   chặn effect/blend hoặc API churn, có thể thay renderer sau presentation contract mà không đổi core.

Quyết định chọn A **không phụ thuộc** vào việc đã build+run 3 OS hay đã đo input-held/focus: các lý do
chi phí/dependency/đường lùi vẫn đứng. Đây là quyết định **có điều kiện**; P6 phải hoàn tất các phép
đo còn thiếu ở §4.5 trước khi coi ADR 0007 đã đóng.

Không chọn B vì chi phí tự viết renderer/UI và dependency cao hơn, dù Windows build nhanh hơn
một chút. Không chọn Bevy/SDL3 trong lượt này vì ngoài hai phương án của 0007 và chưa có bằng chứng
đo tương ứng; xem điều kiện đảo §7.

## 6. Hệ quả

- P6 tạo `crates/boom-client` với macroquad (pin 0.4.x) và `quad-snd`; **không** copy code spike,
  không đưa `spikes/` vào workspace gốc. Dependency nền/feature ghi theo ARCHITECTURE khi mở task.
- Renderer nhận presentation snapshot + metadata animation; không để type macroquad rò vào
  `boom-core`/`boom-proto`/replay. Client không mang pack decoder; chỉ tiêu thụ bundle đã validate.
- Integer scale 1×/2× dùng render target logic 600×560 (hoặc theo scene thật) + nearest; không
  scale từng sprite bằng float.
- Effect additive (nổ/particle) phải có quyết định riêng ở P6: hoặc asset pipeline tiền xử lý
  sang alpha, hoặc custom material chỉ trên backend hỗ trợ. Spike dùng tiền xử lý alpha; không
  coi đây là cách render cuối cùng.
- Font: V1 nên bundle TTF có license (subset tiếng Việt) thay vì dựa font hệ thống như spike;
  cần test glyph/wrap/focus theo SCOPE_V1 G5.
- Audio: bundle OGG BGM/SFX; `quad-snd` đủ cho V1, nhưng cần đo lại trên máy có soundcard và
  kiểm event identity/dedup theo ADR 0008.
- `scene.json`/`prepare-assets` là định dạng **spike**, không phải bundle schema P4; không commit
  asset game, không dùng làm nguồn content.
- Trước khi coi ADR 0007 đã đóng, P6 phải hoàn tất các mục còn **chưa đo** ở §4.5: input-held/focus,
  build+run 3 OS thật, GPU thật. Fixture §4.6 đã kiểm pivot/alpha/z-order ở mức implementation; nếu
  muốn đối chiếu KR cần thêm ảnh client gốc/decoded `i16.py` cục bộ (không commit ảnh game). Nếu sai
  khác vượt ngưỡng, mở ADR mới thay vì sửa lén spike.
- ADR 0007 được thay; integrator cập nhật index/status. Issue #12 đóng sau review/merge.

## 7. Điều kiện đảo và rủi ro còn lại

Đảo quyết định (mở ADR mới) nếu một trong các điều kiện sau xảy ra:

- macroquad không đáp ứng effect/blend/text shaping cần cho V1 sau khi đo trên GPU thật;
- build/run 3 OS thật (đặc biệt macOS link, Windows runtime) thất bại hoặc lệch pixel quá ngưỡng
  đã định;
- API macroquad thay đổi không tương thích trong thời gian P6/P7 hoặc không còn được bảo trì;
- yêu cầu mới (3D, editor, UI phức tạp) vượt khả năng framework.

Rủi ro đã biết:

- Chưa đo GPU thật: macroquad chạy llvmpipe trong Xvfb; frame time/GPU driver trên Windows/macOS
  chưa có. Số build Windows có, nhưng chưa chạy runtime Windows.
- macOS mới `check`; chưa link/chạy. Gate build+run 3 OS **chưa đạt**.
- Chưa có soundcard: `--audio-smoke` chỉ chứng minh **load/decode** BGM+SFX pass (và fail khi thiếu
  file); playback/timing/âm lượng chưa đo.
- Pivot/alpha/z-order **đạt fixture tổng hợp** (macroquad lệch ≤1 LSB, winit 0), nhưng đây không
  phải oracle KR: asset/map/pivot Q-003 của client gốc vẫn chưa đối chiếu.
- Macroquad cần **copy shader tắt blend** khi đưa render target ra cửa sổ; nếu quên, alpha target bị
  nhân đôi và làm tối vùng bán trong suốt (đã phát hiện và sửa trong spike).
- Input-held/focus **chưa đo**: spike chỉ xử lý cạnh `B/S/R/Esc`, C00 tự đổi hướng theo thời gian.
- `f0/f1` (registration/pivot) là candidate Q-003; scene dùng quy ước trình diễn, chưa phải
  cách client KR đặt sprite.
- Chữ Việt mới test một dòng precomposed; chưa test shaping/fallback/độ dài/wrap đầy đủ.
- Phương án B dùng CPU thay vì wgpu; kết luận "tự dựng renderer" có thể khác nếu dùng GPU thật.

## 8. Bằng chứng

- Spike: [`spikes/client-macroquad/`](../../spikes/client-macroquad/README.md),
  [`spikes/client-winit-softbuffer/`](../../spikes/client-winit-softbuffer/README.md).
- Asset decode: `spikes/client-macroquad/tools/prepare-assets/` (dùng `boom-assets` PR #28).
- Snapshot/diff: `spikes/client-winit-softbuffer/tools/compare.py`; số đo
  [`metrics.json`](../../spikes/client-macroquad/measurements/metrics.json).
- Audio smoke: `--audio-smoke` cho cả hai spike (đọc/kiểm Ogg/decode BGM+SFX); ca thiếu SFX phải
  exit khác 0 (macroquad 2 + `audio smoke FAIL`, winit 1 + tên file/lỗi); chi tiết tái hiện ở
  `measurements/README.md`.
- Fixture độc lập: `spikes/client-macroquad/measurements/pixel_fixture.py` sinh sprite/scene/expected;
  `run-pixel-fixture.sh` của từng spike render + so 1×/2×. Kết quả ở §4.6 và `metrics.json`.
- Scale: `compare.py --check-2x-from` kiểm nearest expansion trực tiếp (mismatch 0 cho cả hai).
- Ảnh: [`build-size.png`](../../spikes/client-macroquad/measurements/build-size.png),
  [`pixel-diff-scene-1x.png`](../../spikes/client-macroquad/measurements/pixel-diff-scene-1x.png).
- Không commit pack/asset game; snapshot gốc nằm trong `local/` (gitignored).
