# VN — endpoint nhốt/chết và counter của bong bóng (2026-10-24)

Task[#257](https://github.com/aquaplatinumberlitz/boom-rs/issues/257), root[#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147).
Nguồn **vn_static/vn_runtime**; các quan sát đúng build/case được dùng theo quyết định chủ dự án
và ADR0002/0013. Native counter200 không phải hằngms. Calibration6350ms của Rust là **DESIGN**.
Không tuyên bố RE toàn bộ timer/input/touch của client hoặc server gốc đã xong.

## Build, fixture và phép đo

- VN VBNB_116 `ca_fixed_patched.exe`, SHA
  `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, base0x400000.
  [Chuỗi rebuild/patch và giới hạn](../vn-player-grace-2026-10-24/README.md#rebuild--patch-scope).
- Ocean01 bản VN, map-index672; room type19, một người, spawn(1,0), nativeXY(40,39).
  Chưa biết tương ứng nhân vật C00/C01 KR; không suy tất cả mode/character hoặc server gốc.
- Server giả riêng chỉ echo152 và **một151 cho đúng bomb id1 sau8s** (`emu_choice`), không tự
  nhốt/giết; `MATCH_END=0`, `DEATH_END=0`. Bootstrap501→189mode1→190→189mode0→191.
  `ROUND_TIME=600` được truyền env nhưng UI vẫn countdown khoảng120s; không khẳng định thời hạn600s.
  Phép đo diễn ra trước countdown hết, không lấy timeout của server làm nguyên nhân chết.
- Proper `winedbg --gdb` i386; verify16byte tại7324C0 trước hook và bytecall71734E.
  PID/user/net namespace riêng, chỉ loopback, prefix/game copy riêng; dừng supervisor của từngrun,
  không kill Wine/Xvfb/server dùng chung. Không sửa giá trị gameplay hoặc gọi hàm trong inferior;
  software breakpoints phục vụ quan sát được ghi rõ.
- Clock là `time.monotonic_ns()` khi debugger bắt **serialize entry/return**, không phải giờ logpacket
  hoặc trường `t` trong gói120. Mỗi run so cùng con trỏ hero/vtableC664C4/XY; không so địa chỉ giữa run.

[CSV endpoints](endpoints.csv), [counter observation](counter-observation.json),
[manifest139file hash/size](manifest.json). Raw/ảnh/probe script giữ local ở clone
`/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/host-trap-native-147-*`; repo chỉ metadata,
không game binary/ảnh/audio/DB/decompile dài.

## Ba lần baseline và đối chứng

Cả ba dùng cùng sparse hooks: sender5AF165 chỉ ghi subtype11/13; site71734E; temporary return hooks.
Không có breakpoint mỗi frame; không dùng phím F2 hoặc ép state/death từ server.

| Run | entry11→entry13(ms) | return11→entry13(ms) |
|---|---:|---:|
| r1 | 6332,232297 | 6328,904704 |
| r2 | 6355,412588 | 6352,137645 |
| r3 | 6335,516850 | 6331,871757 |

Giữ cả ba; range6332,232–6355,413ms, median6335,517ms. Độ phân giải clock không phải độ chính xác
cơ chế: mỗi hook dừng debugger vài ms, tick native/host scheduling và pha serialization còn ảnh hưởng.
Entry11→return11 khoảng3,2–3,6ms. Không trừ overhead theo cách đoán để tạo hằng timer.

R2 ảnh HELP còn tại snapshot-begin **+3800,453ms** và **+5200,963ms** sauentry11; ảnh dead
ở+8360,685ms (sau13). Đây là xác nhận nhìn thấy, không khẳng định frame bắt đầu/kết thúc animation.
Native13 là mốc gửi sự kiện chết; **death-animation tail chưa tách**.

Negative: đặt bom(0,0), ra(2,0) ngoài range1; cùngone151, theo dõi đến hơn7,8s sau151.
Ảnh vẫn alive, khôngnative11/13, khôngpacket120. Đủ đối chứng sự kiện không tự phát ở case này;
không phải phép đo non-owner/touch và không nâng case đó thành mọi mode.

## Consumer/counter được nối

1. `71734E` gọi hero slot+0x1C0; runtime đọc đúngtarget **5AF165**, cùnghero trướcđó đãserialize11.
   Đã córecord entry và return riêng, không dùng marker PROBE_READY như bằng chứng hook đãhit.
2. Stage dispatcher716B75 so giá trị từ **controller tại hero+0x5F4** slot+0x40 với boxed
   stage+0x56B4 và stage+0x56C0. Nhánh716FE1 đặtflag rồi tới7172A1→71734E→serialize13.
   Nhánh717251 khác có thể do tương tác khác; chưa gán chung thành timeout/touch.
3. Runtimecontroller vtableC6630C/slot40=5960B4: loadmember+0x378 rồi tail-callmember.vtable+0x88.
   Member được quan sát vtableC2E21C/slot88=57450F→58D507; getter58D507 đọc boxed **member+0x40**
   qua414543. Tên IDA `CGOBubble__vf34` chỉ giúp định hướng; pointer/table/bytes/giá trị có raw riêng.
4. Run`threshold` bắt đúng716FE1 trước13, stagefields16/200. Run`bubble-age` bắt cùnghero/member:
   **value0 tạiserialize11**, **value200 tại716FE1**, stage+56C0=200; sauđóserialize13.
   `hero+26E0=0` ở các hook, không dùngcounter cũ đó làm nguồn timer.

**Giữ đủ giới hạn của chuỗi lấy mẫu:** runbubble-age có212row trướcthreshold, nominal5ms,
read-only boxed decoder kiểmchecksum/cookie. Value0..200 nhưng có **ba reset sớm3→0** tạiindices3/7/11;
không phải epoch đơn điệu. Nguyên nhân reset/generation/object reuse chưa chứng minh, không tự giải
thích là lỗi sampler/animation. Mười ba row sauthreshold/death bị loại khỏi lifetime analysis và
được ghi số lượng; object/age reset sau13 không được dùng để kéo dài lifetime trướcchết.
Không suy200update native kéo dài đúng200tickRust hoặc tính slope sau khi âm thầm bỏ reset.

Các run`fields`, `threshold`, `bubble-age` có instrumentation bổ sung; CSV giữ tất cả riêng
(6329,513/6375,400/6353,815ms), **không gộp** chúng vào range ba baseline để giả cùng điều kiện.

## R-159 và lựa chọn R-160

- R-159 xác nhận **counter/consumer threshold200 trong đúng ca VN** và các endpoint quan sát.
  Chưa biết writer tăng/reset, clock native, mode khác, non-owner hoặc thứ tự touch/timeout.
- R-160 chọn **6350ms** (`design`/design) làm calibration vòng đời Rust, dựa khoảng native11→13
  và ảnh HELP còn sau5,2s. Không gọi6350làliteral native hoặc lấy packet6,359s làm timer.
- Rust30Hz: ceil(6350×30/1000)=**191tick**. `life` tính tick nhốt là lần giảm đầu tiên:
  từTrapped tickT tớiDead tickT+190, **6333,333ms hiệu dụng**. Giữ cách tính tick tạo/timeout trướctouch;
  chỉ đổi giá trị profile. Fuse/flame/speed/cap/warmup không đổi.
- Mốc native serialize11→serialize13 được chọn tương ứng RustTrapped→Dead. Nếu sau này chứng minh
  subtype13 chỉ là cuối animation hoặc phụ thuộc mode, mở lạicalibration; không coi sự lựa chọn này
  lànative state-transition đãđọc đủmọiphase. Internal counter200 vẫn mang đơn vị riêng.

## Coverage và kiểm tái tạo

Dsh2 dùngcache/PE-copy: vf1507172E7 đủbytesret tại7173A2; senderret5AF1CD=`C20800`.
Tên718C5A(vf48) kết thúc718CF0, **không** phải hàm chứa return718D51/59: đó làfn718CF3.
Root đọc phần còn lại cùngcallee716B75 để giải help, không quét toàn image. Round4worker đã đọc
1full+2prefix, **vượt budget2body một phần**, đã ghi/dừng. Các thunk liền kề lỡ xuất cùng range của
root được ghi là collateral, không dùng để mở rộng kết luận; không đếm một lệnh objdump thành1body.
Leafbytes/sourceSHA giữ local`/home/admin/boom-team/tmp/vn147-leaf-getters-bytes.json`.

Tái tính CSV bằng `(entry13_ns-entry11_ns)/1e6`; kiểm samehero/vtable, stage13target5AF165.
Manifest: SHA-256 từngfile gốc, không hash ảnh đã sửa. Counter chỉ dùngrows có
`begin_ns>=entry11_ns` **và** `end_ns<=threshold_ns`; giữ3earlydrops trongJSON.
Rust regression đọc profile thật: trước6350 thìred120!=191; sau6350 qua, người bị nhốt còn ởtick189,
chết đúngtick190. FullCI/scenario/replay và source-precedence check là regression, không oracle.
Không nâng gate/G3/root147 chỉ từ kết quả scoped này.

Oracle O-013 hiện chạy profile fixture lịch sử riêng và vẫn in6300ms; không sửa fixture/golden để
làm nó xanh theo calibration mới. Kết quả đó không phải đo profiledev hiện6350ms. Gói QA28dad28
cũng giữ đúng profile/head lịch sử của nó; trước nghiệm thu release phải dựng lại từhead luật cuối,
không suy smoke cũ đã kiểm profile mới. Không rerun matrix mạng sáu lượt cho thay đổi metadata này.
