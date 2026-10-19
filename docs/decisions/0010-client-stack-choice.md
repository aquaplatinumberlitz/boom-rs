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
| Code game | 591 dòng | 798 dòng |

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
| Binary Linux release | **2.631.328 B** (2,51 MiB) | 6.216.616 B (5,93 MiB) |
| Build Windows GNU release | 15,19 s | **13,50 s** |
| `.exe` Windows | 9.156.705 B | **7.966.962 B** |
| Crate transitive (Linux) | **53** | 117 |
| Dòng code game | **591** | 798 |

Thời gian build và số Windows/macOS giữ từ lần đo head `66e91d7`; vòng sửa frame/audio chỉ đo lại
pixel, audio, binary và LOC Linux ở head hiện tại, không build lại Windows/macOS trên host này.

Ảnh: [`build-size.png`](../../spikes/client-macroquad/measurements/build-size.png).

Linux release nghiêng rõ về macroquad (½ thời gian, nhỏ hơn ~2,4 lần, ít hơn 64 crate). Trên Windows,
phương án B build nhanh hơn và `.exe` nhỏ hơn, nhưng chênh lệch nhỏ và không đổi kết luận về
chi phí bảo trì/độ phức tạp.

### 4.2 So sánh pixel (regression)

Cùng scene, so snapshot 1× và 2× (bỏ HUD):

| Vùng | Pixel | Khác | Tỉ lệ | Max Δ |
|---|---|---|---|---|
| Scene 1× (600×520) | 312.000 | 143 | **0,046%** | 17/255 |
| Scene 2× (1200×1040) | 1.248.000 | 572 | **0,046%** | 17/255 |
| HUD chữ Việt 1× | 24.000 | 2.611 | 10,9% | 144/255 |

- Toàn bộ 143 pixel khác ở scene 1× nằm trong vùng khói nổ bán trong suốt (bbox x 352–367,
  y 278–292); nguyên nhân là alpha blend trên GPU (macroquad) so với blit CPU thẳng alpha. Không có
  pixel khác ở nền, tile, object, nhân vật, bom, hay biên sprite.
- Scene 2× có 572 pixel khác; **kiểm trực tiếp** `compare.py --check-2x-from` cho cả hai stack cho
  `mismatch = 0`, tức ảnh 2× là nearest expansion của đúng ảnh 1×. Đây chỉ là kiểm scale, không
  chứng minh pivot/alpha/z-order đúng.
- HUD khác do khử răng cưa/vị trí glyph của GPU so với blit CPU; ảnh so cho thấy đủ dấu
  tiếng Việt, không thiếu glyph/clipping. Đây là khác biệt trình bày, không phải lỗi mapping.
- Ảnh nhiệt: [`pixel-diff-scene-1x.png`](../../spikes/client-macroquad/measurements/pixel-diff-scene-1x.png).

Kết luận pixel: đây là **so sánh/regression giữa hai implementation**, không phải oracle fidelity KR.
Hai renderer dùng chung decoder, quy ước anchor/sort và chuyển additive→alpha, nên lỗi chung về
pivot/layer/blend không bị phát hiện. Chưa có phép so với client gốc hoặc decoder độc lập, nên các
tiêu chí pivot/alpha/z-order là **chưa kiểm chứng độc lập** (xem §4.5).

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
| Sprite RLE alpha | đo trên scene | decode + render; chưa có oracle pixel độc lập |
| Pivot/canvas (f0/f1) | **chưa kiểm chứng độc lập** | hai renderer dùng chung công thức; cần fixture pivot bất đối xứng + ảnh client gốc/decoded bằng `i16.py` (chỉ so cục bộ, không commit ảnh game) |
| z-order | **chưa kiểm chứng độc lập** | hai renderer sort cùng key; cần fixture nhiều lớp chồng với expected pixel |
| Alpha/blend | **chưa kiểm chứng độc lập** | cả hai chuyển additive→alpha giống nhau; cần expected pixel trên fixture alpha |
| Integer scale 1×/2× | đo | `--check-2x-from` mismatch 0 cho cả hai; chỉ chứng minh scale, không chứng minh pivot |
| Chữ Việt | đo một phần | HUD 2.611/24.000 pixel khác, đủ glyph; chưa shaping/wrap/fallback |
| BGM/SFX | load/decode pass | `--audio-smoke` pass cả BGM+SFX; playback/timing chưa đo vì không có soundcard |
| Input-held/focus | **chưa đo** | code chỉ xử lý cạnh `B/S/R/Esc`; C00 tự đổi hướng theo thời gian; cần test giữ phím/mất focus |
| Build+run 3 OS | một phần | Linux build+run; Windows chỉ build; macOS chỉ `check`; gate 3 OS chưa đạt |
| Render tách tick | **chưa kiểm** | spike chưa có tick mô phỏng; presentation contract ở P6 |
| GPU thật | **chưa đo** | chỉ llvmpipe/Xvfb; frame time GPU thật chưa có |
| Frame animation | đã sửa | sắp theo số; 300 ms = explosion `#4`; snapshot/heatmap tái tạo |

Vì ma trận còn nhiều mục **chưa đo/chưa kiểm chứng độc lập**, ADR không tuyên bố đã đạt đầy đủ
acceptance của ADR 0007. Quyết định chọn macroquad là **có điều kiện**, dựa trên chi phí/độ phức tạp
và tính nhất quán pixel; integrator/Claude duyệt ngoại lệ, và P6 phải hoàn tất các phép đo trên
(đặc biệt fixture pivot/alpha/z-order và build+run 3 OS) trước khi coi 0007 đã đóng.

## 5. Quyết định

**Chọn phương án A — macroquad 0.4.16 + quad-snd — làm client stack khởi điểm cho V1.**
Phương án B (winit + softbuffer + fontdue) được giữ làm tham chiếu kỹ thuật/regression pixel
(không phải oracle fidelity), không đưa vào `crates/`.

Lý do:

1. **Tính nhất quán pixel giữa hai stack** trên scene đục; khác biệt chỉ ở alpha blend và khử răng cưa
   chữ. Đây là comparison/regression, **không** phải bằng chứng đạt pivot/alpha/z-order (các mục đó
   chưa có oracle độc lập, §4.5).
2. **Chi phí thấp hơn rõ rệt**: Linux release nhanh gấp ~2, binary nhỏ hơn ~2,4 lần, ít hơn 64
   crate, ít hơn ~200 dòng code game; text/audio đã tích hợp, không phải tự viết layout/blit.
3. **Ít rủi ro tích hợp P6**: một dependency framework thay vì tự ghép winit + softbuffer +
   fontdue + layout + blend; vòng render vẫn tách được khỏi tick mô phỏng qua presentation contract.
4. **Đủ kiểm soát cho V1**: integer scale bằng render target + nearest đã kiểm expansion trực tiếp,
   PNG decode sẵn; framework có API input/focus nhưng **chưa đo** input-held/focus (§4.5); chưa cần
   custom shader cho scene V1.
5. **Có đường lùi**: phương án B đã chứng minh scene/pixel có thể dựng bằng CPU; nếu macroquad
   chặn effect/blend hoặc API churn, có thể thay renderer sau presentation contract mà không đổi core.

Quyết định chọn A **không phụ thuộc** vào việc đã đạt đầy đủ acceptance pixel/pivot/z-order hay đã
build+run 3 OS: các lý do chi phí/dependency/đường lùi vẫn đứng. Đây là quyết định **có điều kiện**;
integrator/Claude duyệt ngoại lệ, và P6 phải thực hiện các phép đo còn thiếu ở §4.5 trước khi coi
ADR 0007 đã đóng.

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
- Trước khi coi ADR 0007 đã đóng, P6 phải hoàn tất các mục §4.5: fixture pivot/alpha/z-order với
  expected pixel độc lập (hoặc so ảnh client gốc/decoded `i16.py` cục bộ, không commit ảnh game),
  test giữ phím/mất focus, build+run 3 OS thật. Nếu sai khác vượt ngưỡng, mở ADR mới thay vì sửa
  lén spike.
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
- Pivot/alpha/z-order **chưa kiểm chứng độc lập**: hai renderer dùng chung anchor/sort/additive→alpha;
  chưa có fixture expected pixel hoặc ảnh client gốc để so.
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
  exit 2; chi tiết tái hiện ở `measurements/README.md`.
- Scale: `compare.py --check-2x-from` kiểm nearest expansion trực tiếp (mismatch 0 cho cả hai).
- Ảnh: [`build-size.png`](../../spikes/client-macroquad/measurements/build-size.png),
  [`pixel-diff-scene-1x.png`](../../spikes/client-macroquad/measurements/pixel-diff-scene-1x.png).
- Không commit pack/asset game; snapshot gốc nằm trong `local/` (gitignored).
