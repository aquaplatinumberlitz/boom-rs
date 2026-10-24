# Kiểm UX client Rust — 2026-10-24

Task [#253](https://github.com/aquaplatinumberlitz/boom-rs/issues/253). Đây là kiểm thực thi
**DESIGN của Rust V1**, không phải oracle hay claim native VN/KR. Không nâng gate G5/G6 hoặc V1-036.
Code được kiểm: `00c9305c25d9831c0b6481c14bb2714684d8acdc`; binary debug Linux SHA-256
`cda316bcacdc585c45bee7ace2f26749fc8a375c1f06c2a7ef67d897d75955f3`.
[Manifest](manifest.json) ghi hash/size raw. Raw giữ local tại worktree
`/home/admin/boom-rs-wt/v1-window-ux-253/local/qa-ux-253/`, không đẩy ảnh/audio game lên repo.

## Phép kiểm và kết quả

| Ca | Đầu vào/quan sát | Kết quả và giới hạn |
|---|---|---|
| Đối chứng lỗi UI scale | Binary baseline `73529f8ed3c64e667e37e9ed741f03c553db7f146023ebe1b63c31db1bc180f6`, `--ui-shot room --scale 2` | Trước sửa ra 800×600, không phải 1600×1200 |
| UI nguyên pixel | Cùng bundle Việt hóa, chụp room 1×/2× trên Xvfb 1920×1440, llvmpipe | Sau sửa 800×600/1600×1200; RGBA 2× **bằng chính xác** ảnh 1× nhân nearest. Đã xem chữ/ảnh và sidebar; không gọi llvmpipe là GPU thật |
| GPU thật/fullscreen | X11 `:0`, 1920×1080; NVIDIA RTX3060/driver615.78.08. `/proc/<ownpid>/maps` có thư viện NVIDIA; placeholder offline, F11 | Cửa sổ 600×560 → 1920×1080, `_NET_WM_STATE_FULLSCREEN` có thật. Canvas vẫn 600×560 nguyên pixel, có viền. Đây là vận hành cơ bản, **không** benchmark FPS/latency/cấu hình tối thiểu |
| Ra khỏi fullscreen | F11 lần hai trên cùng desktop | Desktop trả về cửa sổ 1920×1006 thay vì kích thước yêu cầu 600×560; renderer giữ đủ canvas ở giữa. **Giới hạn còn lại:** WM X11 quyết định frame/maximize; chưa đảm bảo khôi phục đúng kích thước cũ. Phải kiểm thêm khi nghiệm thu G5 |
| Mất focus thật | Giữ Right, chuyển sang cửa sổ QA riêng, nhả Right ở đó, chờ, quay lại; không dùng F2 | Tâm sprite vàng di chuyển 719,5→724,5 px trước chuyển focus; sau chuyển dừng tại728,5 px trong hai capture cách nhau0,5s và khi quay lại. Có đối chứng đã đi được; focus loss nhả phím, không kết luận từ keyup gửi lại client |
| Rebind không bị shortcut chiếm | Xvfb; lần lượt right=S, right=F10, right=F11, giữ0,25s sau warmup | Cả ba đi từ x59,5→65,5 px, cửa sổ giữ600×560. F10 chưa gán đổi600×560→1200×1120. Đây là vị trí pixel sprite placeholder, không phải oracle vận tốc |
| BGM/volume/mute | ALSA `pulse` qua file config riêng, null sink sở hữu riêng, monitor float32 stereo44100Hz; cùng bundle, sfx=0 | BGM100 RMS0,174333; master50 RMS0,090992 (tỉ lệ0,52194; cửa sổ capture không phase-lock nên không khẳng định đúng0,5 từng sample). Mute RMS/peak=0. Có đúng một stream của từng PID vào sink riêng |
| SFX | BGM=0, demo core phát event, capture giống trên | RMS0,020748/peak0,652083: có SFX thực qua backend. Không xác nhận từng cue hoặc native voice count; dedup dựa test event identity hiện có, không suy từ waveform |

Bundle local đã validate: `8b762ac8f04b2c4c8710b6ca25c8762b504373c2b566898196c940b9633978f3`,
68 sprite/297 image/7 atlas/15 audio. Các ảnh UI/text đã duyệt theo V1-035; không duyệt lại23 câu legacy.
Chỉ tạo/dọn PID, Xvfb, stream và null-sink riêng; giữ default sink/master volume và app khác.

## Tái tạo

```bash
cargo build -p boom-client
xvfb-run -a -s '-screen 0 1920x1440x24' env LIBGL_ALWAYS_SOFTWARE=1 \
  target/debug/boom-client --ui-shot room --scale 2 \
  --bundle /path/to/approved-bundle --snapshot local/room-2x.png
```

Lặp với `--scale 1`, dùng Pillow nearest nhân2 và so toàn bộ RGBA; không sửa ảnh bằng tay.
Kiểm focus dùng cửa sổ đích riêng và capture riêng cửa sổ game; so vị trí cùng sprite trước/sau.
Audio dùng `ALSA_CONFIG_PATH` riêng (`pcm.!default { type pulse }`), `PULSE_SINK` riêng,
`parec --device <own-sink>.monitor --format=float32le --rate=44100 --channels=2`;
không đổi cấu hình âm thanh toàn máy. Phép kiểm ghi nhận **đường phát số**, chưa kiểm nghe/DAC/soundcard.

## Chưa nghiệm thu

- GUI tương tác/network room→match trên Windows/macOS, DPI/desktop/backend khác.
- Khôi phục đúng kích thước cửa sổ trên WM X11, và hành vi dưới kích thước canvas tối thiểu.
- So đủ cue/BGM vòng lặp qua nhiều round; nghe qua phần cứng, performance và minimum hardware.
- G2/G3 profile/fidelity và nghiệm thu toàn bộ V1-036 còn mở; kết quả gói QA ba OS ở mục dưới.

Các lỗi fixture tìm cửa sổ trước khi map và capture đầu không có đối chứng movement bị loại khỏi
kết luận focus. Lượt thử defer resize không sửa được WM restore; code đã bỏ workaround đó.
Không dùng những lượt setup lỗi để chứng minh chức năng chạy.

## Gói QA ba OS từ đúng code đã review

[Workflow40069259503](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40069259503)
đã thành công (check + ba job package). Ref `fix/v1-window-ux-253`, head
`28dad286c467a870c3047ff21b26ce82c654f2fc`, phiên bản `qa-20261024-28dad28`; code đã review độc lập
và tích hợp qua PR255 (`796284d422baf14f2c0ab0c27a78100bc335c2f1`). Không tạo tag/GitHub Release.

Đã tải ba archive và ba smoke-log, mở archive thật và chạy `check_package.py check` trên từng gói;
đối chiếu9hash trong từng log với file đã mở (tổng27), version/head/fingerprint, kết luận10case/OS.
[Metadata đầy đủ](package-qa.json). Archive/log giữ local tại worktree
`/home/admin/boom-rs-wt/qa-checkpoints-253/local/package-qa-40069259503/`.

| Archive | Byte | SHA-256 archive (không phải wrapper artifact GitHub) | Smoke/hash |
|---|---:|---|---|
| `boom-rs-qa-20261024-28dad28-linux-x86_64.tar.gz` | 5,229,858 | `1e348edd8b19fd23278c3b12e9d37788eed6a5a5220d0a14633c1c4b773d6d50` | 10/10, 9hashfile khớp |
| `boom-rs-qa-20261024-28dad28-macos-aarch64.tar.gz` | 4,617,642 | `43cf1be236de3e4b36588ad4776abd39a5b6b0cafb5f68dde2287851edf1e5da` | 10/10, 9hashfile khớp |
| `boom-rs-qa-20261024-28dad28-windows-x86_64.zip` | 4,368,500 | `ecd4e6d6a3e47ddc32eead7802acd1766dc3fff25f10b5194017f331f322bc28` | 10/10, 9hashfile khớp |

Cả ba cùng profile `kr00967-v1-dev`, package-rules-hash `0xb357d66b1d761f89`, rules file SHA
`3344e6b07451c79be36b4ffd78e7fc7b70be15342f2921d4fee2967da90dc2a6`, l10n SHA
`fb914d12c9c85f6996a3b31df08cf1cb47fba45b46a0aa6659b728e809593960`.
`health --offline` trả scopeoffline/statusunhealthy/contenthash0 là ngữ cảnh không chạy server,
**không** phải bằng chứng live server healthy hoặc content hash của bundle game.

Binary Linux **lấy từ archive** đã mở thêm snapshot room2× trên Xvfb/llvmpipe,1600×1200;
RGBA bằng chính xác snapshot debug đã kiểm. Binary SHA và ảnh hash ở metadata. Không gọi lần này
là GPU thật; không dùng Wine để chạy gói Windows hoặc giả GUI macOS đã kiểm.

Đây là **gói QA/dev**: rules còn `release=false`/candidate theo ADR0013, chưa chọn minimum hardware,
chưa nghiệm thu G2/G3/G5/G6 hoặc toàn bộ V1-036. Gói chỉ có Rust binaries/data/font/license;
check chặn game pack/binary/media. Bằng chứng smoke ba OS đã có, không tiếp tục ghi “chưa smoke gói cuối”
như blocker cũ; các giới hạn về GUI/hardware/profile vẫn phải ghi rõ khi chốt bản phát hành.
