# Kiểm UX client Rust — 2026-10-24

Task [#253](https://github.com/aquaplatinumberlitz/boom-rs/issues/253). Đây là kiểm thực thi
**DESIGN của Rust V1**, không phải oracle hay claim native VN/KR. Hồ sơ ban đầu không tự nâng gate; quyết định G5 riêng ở mục cuối, G6/V1-036 vẫn mở.
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
| Ra khỏi fullscreen | F11 lần hai trên cùng desktop | Desktop trả về cửa sổ 1920×1006 thay vì kích thước yêu cầu 600×560; renderer giữ đủ canvas ở giữa. **Lịch sử trước PR271:** chưa khôi phục frame; KWin/X11 đã sửa và kiểm lại ở task270/273 bên dưới. Không còn là blocker hiện hành trên WM đã kiểm |
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

## Giới hạn platform/phát hành và fidelity còn lại

- GUI tương tác/network room→match trên Windows/macOS, DPI/desktop/backend khác.
- KWin/X11 restore đã có sửa và đối chứng ở task270/273 bên dưới; WM khác và hành vi dưới canvas tối thiểu chưa xác nhận.
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
tại lúc thu QA chưa nghiệm thu G2/G3/G5/G6 hoặc toàn bộ V1-036; quyết định G5 về sau ở mục cuối. Gói chỉ có Rust binaries/data/font/license;
check chặn game pack/binary/media. Bằng chứng smoke ba OS đã có, không tiếp tục ghi “chưa smoke gói cuối”
như blocker cũ; các giới hạn về GUI/hardware/profile vẫn phải ghi rõ khi chốt bản phát hành.


## Gói QA đúng profile hiện hành — task265

[Workflow40091603701](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40091603701)
đã success4/4job; version`qa-20261024-641f99a`, head`641f99aa997813eca77ecd1d0ef554b3fd55c272`
đã review/mergePR264 và có cùng code/luật với integration4fa1588. Đây là lượt cần thiết vì gói28dad ở
trên có trap4000/trapped trướcPR260; không giả bằng chứng gói cũ đã kiểm luật6350/17 hiện hành.

[Record lịch sử task265](https://github.com/aquaplatinumberlitz/boom-rs/blob/265432d8b5d24df26daea74df8d37855a292a3d8/docs/evidence/measurements/rust-client-ux-2026-10-24/package-current-qa.json) giữ archivebytes/SHA/log/fingerprint và27hashfile
được đối chiếu trên baarchive sau tải/mở. Mỗi OS có10ca smoke đúng kỳvọng từ chính archive trên runner
của OS đó; local chỉ unpack/check/hash, không chạy macOS/Windows qua Linux rồi gọi là GUI.

| Archive OS/arch | Byte | SHA-256 |
|---|---:|---|
| linux-x86_64 | 5231024 | `7b0bfb7812f72af735d70ed968b107c25a845be8f6c2f5f07fd381c0641bee9e` |
| macos-aarch64 | 4616823 | `d587a42624df407f7df7e02033dc2ab690503f4aa4ab61694964396f65f40d8c` |
| windows-x86_64 | 4367734 | `d2d93f049bfe7debca846e025e63cba3ace8cbc30c0ae05bf1f1a387be541554` |

Cảba có package-rules-hash`0x322a09ca1c8453f7`, rulesfileSHA
`83700b3a87fec16efaf539922e18e2f13157b55702349c88e8e04f7bc555b323`; l10nSHA vẫn
`fb914d12c9c85f6996a3b31df08cf1cb47fba45b46a0aa6659b728e809593960`.
TOML thực cùng trap6350/trapped17/flame540 vàreleasefalse; so file byte-identical với currentprofile.
Package-rules-hash không đồng nhất world state/replay hash. Offlinehealthunhealthy/content0 vẫn là
ngữ cảnh offline, không dùng làm bằng chứng livehealthy/contentbundlehash.

Linux binary lấy từ archive mở snapshotroom2×1600×1200 với bundleVi đã duyệt, trên Xvfb/llvmpipe.
Hashbinary/ảnh/log nằm trong metadata; ảnh RGBA bằng snapshotroom trước đã review. Đây không thay
focus/audio/realGPU hoặc GUIWindows/macOS, không benchmark minimumhardware. Không có media vàoGit.
Tại lúc thu QA, G2/G3/G5 in_review/G6blocked/V1-036in_review; đây chỉ cập nhật evidence đúng profile, không ký gate
thay người review, không gỡ candidate hoặc tự chọn license phát hành.


## Khôi phục cửa sổ sau fullscreen — issue270

Lỗi KWin ở bảng kiểm ban đầu đã có đối chứng mới trên source baseline `c8375f2`:
cửa sổ600×560 vào fullscreen1920×1080, thoát còn maximized1920×1006; resize trễ cũng không sửa.
Bản sửa miniquad0.4.11 dùng EWMH add/remove, giữ state của mapped window. Hai vòng vào/ra
khôi phục600×560; đối chứng cửa sổ vốn maximized vẫn giữ maximized1920×1006, không xóa lựa chọn người dùng.
Các ca rebindF10/F11 giữ shortcut ưu tiên gameplay; mở bằng `--fullscreen` rồi thoát trả600×560.
[Metadata, SHA binary/raw và bốn ca](window-restore.json). Screenshot/raw giữ local, không đưa media vào repo.

Source vendor khớp package SHA trong lockfile cũ, trừ đúng hàm X11; MIT/Apache giữ nguyên.
License collector đã sửa để dependency path ngoài workspace không bị bỏ qua:41test pass, report135crate
có miniquad. Không suy đây là GUIWindows/macOS, mọi WM, minimumhardware hoặc nghiệm thu G5.
Gói QA task265 giữ lịch sử trước patch cửa sổ; task273 bên dưới kiểm archive sau sửa, chưa tag/Release.


## Archive sau sửa notice/fullscreen — task273 (2026-10-25)

[Workflow40107097457](https://github.com/aquaplatinumberlitz/boom-rs/actions/runs/40107097457),
reviewed codehead95985fe941be5199568b012482c8eac88b33629f/versionqa-20261025-95985fe,
đã success4/4jobs. [Record hiện hành](package-current-qa.json) ghi ba archive/check/27hashfile/10smokeOS,
135crate và notice`@ 2019-2020 Fedor Logachev` của miniquad trong từng LICENSES. Gói trước sửa
40105903687 build/smoke xanh nhưng thiếu notice này, giữ local để đối chiếu regression đỏ→xanh42test.
Không đổi nguồn/vendor/license; collector nhận thêm marker@year và giữ nguyên dòng thật.

| OS/arch | Byte archive | SHA-256 |
|---|---:|---|
| linux-x86_64 | 5230671 | `dd3f38908e601b5cf1e6bea75f9e33f09b746b8994d32516fd3992d83fa2cfa0` |
| macos-aarch64 | 4616138 | `42c5fca2a32e363d8b1f4565751f20432aa8d082534c1637d00abf9b83e1582e` |
| windows-x86_64 | 4367566 | `0c671c1b88e59ce4777e9e832d11e3087b5aebcec8632145a409896641588094` |

Linux archive thực đã chạy KWin/X11/NVIDIA: normal600×560 và maximized1920×1006 đều khôi phục
đúng state/kích thước sau2vòng fullscreen; rebindF10/F11 không đổi cửa sổ. Phép thử đầu dùng pulse
ngắn không thấy FULLSCREENflag; raw giữ nguyên, nguyên nhân chưa chốt. Lượt có kiểm focus và
gửi keydown150ms/keyup đạt4case; không xóa failure hoặc gọi nó là baseline bug đã chứng minh.
Hashbinary/matrix/logs ở metadata; screenshots/privateoptions không đưaGit. Cảhai probe đã dọn đúng
process riêng/khôi phục focus, không kill app dùng chung hoặc sửa options người dùng.

Luật6350/17/540 vàL10N vẫn cùng byte/hash; không lấy fingerprintofflineunhealthy/content0 làm livehealth.
Đây là QA/devreleasefalse, chưa GUIWindows/macOS/minimumHW/FPS/physicalaudio; lúc thu QA chưa sign-offG2/G3/G5/G6.
Không tự tăng feature% từ góiQA, không tag/Release. Codeheadđónggói được giữ riêng với commitmetadata cuối.

## Nghiệm thu chức năng G5 — 2026-10-25

Quyết định tại [issue278](https://github.com/aquaplatinumberlitz/boom-rs/issues/278), theo
[SCOPE_V1 §6/§7.9](../../../planning/SCOPE_V1.md) và acceptance V1-032/033/034/035.
Reviewer độc lập kiểm các nguồn đã tích hợp; trạng thái chuẩn nằm ở
[execution.json](../../../planning/execution.json), không suy gate từ một phép chụp hoặc CI xanh.

| Tiêu chí G5 | Bằng chứng đã tích hợp | Phạm vi nghiệm thu |
|---|---|---|
| Chữ, ảnh, font, glyph/wrap/placeholder/golden của luồng V1 | PR241/V1-035; [UI_STRINGS_V1](../../../specification/localization/UI_STRINGS_V1.md); 126 khóa, 6 sprite đã duyệt | Functional V1; không dùng 23 legacy review làm text chốt |
| Scale/input/focus/rebind | Hồ sơ đầu trang: RGBA1×/2×; NVIDIA focus có movement positive control và nhả phím; S/F10/F11 rebind | GUI Linux đã kiểm; regression/golden ba OS không chứng minh GUI Win/mac |
| Fullscreen/khôi phục frame | [window-restore.json](window-restore.json), [actual archive QA](package-current-qa.json), PR271/274 | KWin normal/maximized hai vòng và rebind; giữ lỗi fixture pulse ngắn chưa rõ nguyên nhân |
| Audio/volume/mute, không phát trùng khi reconcile | Capture BGM/SFX qua sink riêng ở trên; V1-034; [AUDIO_V1](../../../specification/assets/AUDIO_V1.md), regression identity/reconcile | Đường phát số và policy V1 đã review; không phải phép nghe physical DAC hoặc xác nhận mọi audio KR |

Các mục này đủ cho nghiệm thu **functional G5 của V1**, có reviewer độc lập và PR exact-head approval.
GUI Windows/macOS, backend/DPI/WM khác, minimum hardware và kiểm platform toàn V1 vẫn thuộc
V1-036/G6 hoặc giới hạn fidelity được giữ rõ; không gọi smoke CLI là kiểm GUI. Không tuyên bố
physical DAC, mọi cue qua mọi mode hoặc native layout/voice count đã đo. Không đổi SCOPE, số37feature,
nhãn candidate hoặc nâng G2/G3/G6 theo quyết định này. V1 vẫn36/37; chưa tag/Release.
