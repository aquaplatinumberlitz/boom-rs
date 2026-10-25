# Luật trận và runtime VN được chấp nhận cho bản viết lại

Ngày nhập: 2026-10-19. [Chính sách trusted sources](../../decisions/0002-vn-kr-trusted-sources.md):
VN được tin cậy ngang KR, dùng ngay cho luật chung; không cần chờ tái kiểm trên KR.

Nguồn: repo `/home/admin/boom-re`, commit `4ccc8904cd907ce627f8a2cf6cf84bed190fd00d`,
`docs/oc_e-game.md` mục “Cứu đồng đội / chọc vỡ bong bóng”, “Vật phẩm khi 2 người”,
“Nhiều map và loại phòng”. Đây là kết quả runtime do phiên Opus thực hiện; lượt nhập docs này
đọc artifact và lịch sử commit, không tự chạy lại client.
Ảnh và script gốc lưu trong Git tại các commit bên dưới; không phụ thuộc việc đã push.
Tra lại bằng `git -C /home/admin/boom-re show <commit>:<path>`.

| Claim | Kết quả được dùng | Bằng chứng / commit | Giới hạn còn lại |
|---|---|---|---|
| VN-B01 | Chạm đồng đội bị nhốt thì cứu; chạm địch bị nhốt thì địch chết | `34e0910`: `docs/rescue_freed.png`, `docs/rescue_enemy_pop.png`; runtime 2 client `logs/duo/r4`, `r5` | Thời lượng trapped/fuse chính xác và xử lý đồng thời chưa chốt |
| VN-B02 | Legacy client tự báo C→S120/12 cứu, /13 chết, server không cần đáp | `34e0910`, `docs/oc_e-game.md` | Chưa rõ đuôi packet có actor cứu/chọc; BẮT/CỨU kết quả vẫn00 |
| VN-B03 | Item nền, item lộ sau phá gạch và pickup hiển thị đồng bộ hai máy | `0cce915`: `docs/duo_items_A_view.png`, `docs/duo_items_brick.png`; `logs/duo/i1` | Chưa thử hai người cùng nhặt; emulator hiện xác nhận mọi117, chưa chống trùng |
| VN-B04 | Chạy được Village01, Ice01, Desert01, Tomb01, Factory01, Patrit01, Mine01, Ocean01, Pharaoh01 | `e0f9a49`: `docs/map_*.png`, `tools/map_loop.sh` | Mỗi map thử một người vào trận/đặt bom/bị nhốt; chưa test toàn map hay đủ mode |
| VN-B05 | Loại phòng14/16/22/32/33 vào trận thường;17 có UI RPG và gửi120/21 khi trúng bom | `e0f9a49`: `docs/roomtype17_rpg.png`, `docs/roomtype32.png`, `tools/roomtype_loop.sh` | Chạy một người Village01; semantics mất HP của21 chưa chắc, server RPG chưa làm |
| VN-B06 | C→S88 đầu khi vào phòng là lựa chọn tự gửi; không được ghi đè map host | `0e71e8a`: thay đổi server và docs | Bài học lifecycle, không bắt giao thức mới sao chép opcode cũ |

## Grace của owner sau đặt bom — confirmed owner-only runtime 2026-10-24

Theo **quyết định user 2026-10-24**, kết quả đo động VN dùng được làm nguồn luật gần ngang tĩnh KR;
không bắt re-equivalence. Đây là **confirmed owner-only physical observation**, chỉ owner-only; chưa
non-owner/TTL. Nguồn: hai run host private (#240) tại `.re-work/vn-grace-240/host-run-{2,3}/`, raw local; evidence
công khai [`docs/evidence/measurements/vn-player-grace-2026-10-24/README.md`](../../evidence/measurements/vn-player-grace-2026-10-24/README.md) (PR #242).

| Case | Fixture / quan sát | Kết quả owner-only |
|---|---|---|
| run2 | VN `VBNB_116`, `ca_fixed_patched.exe` SHA `bc875ba5…b1`, `Village01` SHA `1ca7e03e…1b28`, spawn `(14,0)`; fakeserver gửi 152/151 theo timer, **không tự mô phỏng collision/damage**; fuse60 emu_choice | Space→152 tại `(14,0)`; owner rời Left sang `(13,0)`, Right 0.45 s vẫn ở `(13,0)` face right; run2 stop trước nổ, không 151 |
| run3 | cùng fixture; đứng yên trên bomb **5 s** sau 152 rồi mới Left/Right | 5 s không làm mất quyền rời bomb; Left→`(13,0)`, Right vẫn `(13,0)`; S151 10:42:07.367 → client C→S120/11 10:42:07.389 (trap confirmed) → 120/13 10:42:13.751 (death); server không ép trap; S151 gửi ids0..7 nhưng chỉ 1 bom được đặt |

- Đây là **owner-only**, hai run; không suy TTL ms, không suy grace toàn hệ thống/all non-owner.
  Không claim timer ≤/>5 s: chưa đo thời điểm nhận 152/tick.
- Fixture rebuild có **devirt gameplay reconstruction** (bomb msg-cache `0x6EFD4A`, CBombOwner
  `0x5788C9` experiment, random guards), không chỉ offline/localhost. Đã đối chiếu 25 patch +10 IP
  +15 devirt sites; không manifest nên semantic equivalence chưa chứng minh → provenance ghi để truy vết,
  không hạ cấp theo quyết định user; không tuyên bố vanilla equivalence.
- Bomb cell `(14,0)` từ packet 111; screen chỉ approximate (owner visual bounds x≈760→719, y center
  ≈62, feet≈78; không phải native coords). Không dùng hold 0.45 s/5 s làm timer; `monotonic_ns`
  trong supervisor chỉ là command clock, không phải fuse.
- Map VN `Village01` W15×H13; `(13,0)` tile1005/obj996 floor; col12 tree, row1col13 brick. Không suy
  spawn `(13,1)` của KR.
- Run3 có trap/death sau 151, nhưng owner đã rời bomb trước đó; không phải bằng chứng timing grace.
- Gap: non-owner đứng chồng ô lúc 152 chưa đo. Không dùng getter KR `0xB335C0/0xB335E0` cho VN khi
  chưa xác minh; `tools/gdb/spawn.gdb` chỉ in spawn, không phải player; chấp nhận screen approx +
  packet cell `(14,0)`.

## Cửa sổ sát thương lửa — confirmed owner-only 2026-10-24

**Quyết định user 2026-10-24:** kết quả đo động client VN dùng được làm nguồn luật, gần ngang phân
tích tĩnh KR; không cần chờ tái kiểm/re-equivalence. Phần dưới là **confirmed owner-only physical
observation** theo nghĩa đó, không phải claim toàn hệ thống.

Proper Wine `winedbg --gdb` (i386), byte gate 16B `6a50…8b83` @`0x7324c0`; entry/return `0x6e9aee`
trước Left. Owner-only, map Ocean01, một case mỗi mốc:

| Case | Tọa độ native tại capture | N/frame | N0 sau RET | frame18 start | Kết quả |
|---|---|---|---:|---:|---|
| good200 | x50,y39 (sample) | 1 / 14 | 531.084441 ms | 504.432797 ms | trapped |
| phase12 | x50,y39 | 1 / 18 | 535.176555 ms | 492.810326 ms | trapped |
| phase13 | x50,y39 | 0 / 19 | 561.943179 ms | 530.836641 ms | alive, no trap/no120 |

- **Frame 18 harm / frame 19 expired** ⇒ cửa sổ tồn tại thật owner-only, không chỉ animation. Native XY
  decode+checksum đối chiếu getter `0x58D56D` EAX=40 lúc spawn `(40,39)`; **không dùng getter KR**.
- Animation lifetime/frame scalar ≠ damage TTL; R-158 chọn540ms cho V1 tại30Hz và tính đủ số tick; đây là hiệu chuẩn, không phải hằng timer native.
- Evidence/CSV: [`vn-flame-window-2026-10-24`](../../evidence/measurements/vn-flame-window-2026-10-24/README.md),
  [samples.csv](../../evidence/measurements/vn-flame-window-2026-10-24/samples.csv).
- Giới hạn: owner-only; chưa non-owner, chưa repeated, chưa exact TTL/range; fixture provenance đã biết.

## Đưa vào công việc hiện hành

- **V1-020 / Q-006:** luật chạm địch làm vỡ bong bóng được chốt từ VN-B01; không RE lại chỉ vì khác region.
  Đồng đội cứu nhau thuộc backlog Battle đội, không tự đưa vào V1 FFA.
- **V1-016 / Q-007:** dùng VN-B03 làm scenario đồng bộ hiện/lộ/nhặt item. Vẫn phải chốt drop weight,
  cap và race pickup; không lấy emulator hiện tại làm thiết kế chống trùng.
- **V1-024/027:** server mới tự quyết damage/rescue/death/pickup, kiểm command; hành vi client tự báo
  của legacy là nguồn hiểu cơ chế, không chuyển quyền quyết định sang client mới.
- **V1-030:** tái dùng hiểu biết loader/theme từ chín map; vẫn giữ KR Village00 đã chọn.
  Index map VN không phải ID map KR; map VN chạy được không tự hoàn thành content closure Village00.
- **Later RPG:** giữ boundary mode/strategy và state HP/EXP/skill riêng; chưa có luật RPG đầy đủ.
- **Q-009/Q-013:** dùng main mới làm mốc khảo sát legacy; không chép server snapshot cũ từ l10n đè main.

## Hạ tầng và unknown

Server thử đã sửa kết thúc phòng một đội: chỉ hết khi cả đội chết; đây là sửa emulator,
không phải bằng chứng riêng về thuật toán server gốc. Bộ đếm bắt/cứu và actor attribution còn thiếu.

Batch thử từng gặp server cũ giữ cổng sau timeout và Wine kẹt; lỗi đó có thể làm sai map/kết luận crash.
Fixture sau này cần lưu PID/port, commit/config, kiểm server đúng lần chạy và cleanup theo phiên.
Không chạy `wineserver -k` hoặc kill tiến trình chung trong phiên có người khác đang dùng Wine.
Log runtime trong `logs/` là dữ liệu local có thể không đi cùng Git; ảnh/script/docs đã commit là
artifact bền vững, chưa thay thế trace đầy đủ cho phép đo timing.

## Movement/trap: checkpoint bounded24/10 (#146/#147)

Nguồn VN fixture `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1` như
[provenance owner-grace](../../evidence/measurements/vn-player-grace-2026-10-24/README.md).
Các tọa độ/thời gian quan sát trong đúng run/cấu hình là `vn_runtime` / confirmed; nối cơ chế/level
vẫn candidate/unknown. Phân tích cạnh máy là `vn_static` trong fixture cùng SHA. Không đổi luật
chỉ từ các cạnh bên dưới.

### Vận tốc một cấu hình, chưa biết speed level

Phân tích lại nativeXY đã có của `host-flame-coordinate-good200`, `phase12`, `phase13`; không
chạy lại client. `begin_ns` đầu→cuối, y giữ nguyên, dx liên tiếp±5, chọn cửa sổ theo command sau
khi thu dữ liệu. Độ phân giải bị giới hạn bởi sampler nominal5ms; số lẻ không phải độ chính xácµs.

| Run/đoạn | dx(px) | Thời lượng(ms) | Trung bình(px/s) |
|---|---:|---:|---:|
| good200 trái | -35 | 207,548528 | 168,635 |
| good200 phải | 75 | 448,227236 | 167,326 |
| phase12 trái | -35 | 212,266142 | 164,887 |
| phase12 phải | 75 | 431,524711 | 173,802 |
| phase13 trái | -35 | 209,681500 | 166,920 |
| phase13 phải | 75 | 441,022350 | 170,059 |
| phase13 trái trước trap | -60 | 350,343348 | 171,261 |

Giữ toàn bộ khoảng164,887–173,802px/s; đoạn phase12-phải có interval5,201ms chưa rõ nguyên nhân,
không tự loại như lỗi clock. Không suy C00/C01, native speed-level, cap, hoặc server30Hz từ bảng.
Raw `position-samples.jsonl` SHA: good200`fee84a6eebe499606fbcf299fd91a6ffe0324d54e5eaf04e5fddcfa755fb0c92`,
phase12`5348c898063de1cd783335637460fd7fe4f64e20a208c50ef3fcde67720f111f`,
phase13`931667bd018af9f9f14a38e95433b68b881fcb083769c4aca8f84668f6b7b58a`.
Script tái tính/raw giữ trong clone `re-kr/.re-work/speed-146/`; flame evidence đã lưu provenance của các run.

Actor vtableC664C4 slot21/+0x54 (84byte)→5B1472; wrapper gọi5A6244@5B1646. Cache wrapper SHA
`c2559f3d25fd11c9592fac362bfad365781b28c7b0d3f812b7c77e11ef685858`.
Base cache `5A6244–5A7005` SHA`54c558a6a728dc5de6f3d97b341249536c4e36a0d0ee58e42fd91a3cda599a46`
là **range chưa trọn hàm** (hàm còn đi tiếp), semantics chỉ đọc một phần; ±4 ở path target/current
chưa nối với5px của actor đo. Không nâng thành speed consumer hoặc đổi24px/s/level của profile.

### Timer nhốt: sender chưa nối được seed/expiry

- `900462` là notifier so getter5A6128 với arg rồi có thể emit0xB; chưa phải setter. Getter+2C0
  cho mã0/1/2/default3; không tự đặt tên3=trapped.
- Counter+26E0 giảm ở5A62B1; call+0x54@5A62BF có receiver **memberthis+D18**, chưa resolve target,
  không gán sang hero5B1472. Hero+0x28C→5AF9AB clear counter nhưng packet trực tiếp là162, không120/13.
- Sender5AF165 (đủ range5AF165–5AF1CE) xây packet120 với subtypearg. Stage7172E7 chỉ đọc prefix
  tới717500; site717348 truyền13 vào virtual+0x1C0 trên arg0. **Receiver chưa chứng minh cùng actor**,
  trigger/seed timer chưa nối; chỉ là candidate producer C120/13.

Raw static/cache private ở `re-kr2/.re-work/trap-timer-147/`; không đọc lại15literalhit/VM cũ.
Bước tiếp #147: receiver/trigger của site717348 và watch seed có đối chứng; không lấy khoảng packet
6359ms hay counter162 làm timer nhốt. #146 cần level/setter nối đúng actor trước đo mới.

## Native bubble counter và endpoint nhốt/chết24/10 (R-159/R-160)

[Evidence/CSV/hash/reset](../../evidence/measurements/vn-trap-timer-2026-10-24/README.md): same-runhero
C664C4 vàmemberC2E21C,controllerhero+5F4 slot40→5960B4/member378/slot88→57450F→58D507 đọcboxed+40.
Nhánh716FE1 bắtvalue200/stage56C0=200 rồi71734E→5AF165subtype13;value0đãquansátở11.
Ba baseline6332–6355ms giữaentry11/entry13;negative ngoài lửa không11/13,HELP còn+5,2s.
Sampler có3reset3→0 sớm;không gọi wholetrace epoch đơn điệu,200counter không phải200ms/tickRust.
R-160 chọn6350msDESIGN→191tickRust30Hz,chếtT+190(~6333ms);khôngliteralnative timer.
Receiver71734E/5AF165 đã khóa trongcase này, thay blocker“chưa biết cùnghero”củacheckpointtrước.
Increment/reset writer/clocknative,deathanimationtail,non-owner,input/touch-order vẫn Q018.

## Input khi bị nhốt (R-161/R-162)

Hai capture VN24/10 cùngfixture/sameactor xác nhận sau native11 nhânvật vẫn đi chậm theo
Left/Right; idle không đổiXY. R1alive±5px/bước, trapped±1px/bước; R3repeat haihướng±1px.
[Evidence/manifest/CSV](../../evidence/measurements/vn-trapped-input-2026-10-24/README.md) giữ93rawhash
vàr2failed/excluded. **17px/s là calibrationDESIGN R-162**, khôngliteralnative; thay đứngyênR-036.
Space sau11 không có111 mới chưa tách gate/cap/cell; không gọi là confirmedbombgate.
Trụcdọc/hìnhhọc bubble/level/crosscharacter vàtouchorder còn #147; không đóng root từ kếtquả này.

## Nhặt Bomb/Fire — đối chứng bổ sung24/10 (#149)

[Metadata/92rawhash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md): trênOcean01VN,
trướcnhặtBomb chỉđặt được1quả, sau117/158kind0 đặtthêmquả2, quả3bịtừchối trongkhi cảhai cònnguyên
(không151). Firekind1 khôngtăngcapacity, nhưng111quảkế tiếp saupickup cótrườngr18 từ1→2.
Parserfieldnamedpowervẫn1, khôngnhầmnóvớir18; mộtFirecasehelpertrôi vịtrí bịloại.
BổsungR025/R111trongđúngcase, khôngđổi giátrịprofile/capKR/roller; getter được nối riêng ở R-166 bên dưới.

Factory01 bổ sung R-163: ba lượt12 pickup Bomb/ACK ở các ô khác nhau, bom chỉ tăng tới10;
thử đặt ở ô trống khi đủ10 không111, sau151 đầu lại gửi111 trước151 thứhai.
Trần hữu hiệu của fixture/actor chưa phân biệt stat cap/pool; không áp cho mọi nhân vật KR.
210rawhash giữ cả Fire lỗi và đối chứng giải phóng chỗ chưa chạy do lịch emu45s/gate12s.


### Getter tầm nối native receiver→111 (R-166)

Ca bổ sung cùng fixture đã khóa virtual5B1F92/+7C trên facet hero+510/C663C4→5AE518:
trước Fire trả1, sau117/158 trả2, khớp đúng111r18. Có2call/3returnrows, hàng duplicate không phải ca3.
[50hash và CSV](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#receiver-thật-của-getter-tầm--r-166).
Call5B1FB5 là directcall6EFD24 sau đổi receiver; vai trò đối tượng mới chưa xác nhận trong bundle,
không dùng nó làm địa chỉ virtualgetter.
Base578963 đọc boxfacet+1C (hero+52C theo số học), qua virtual+BC/414543; wrapper và5A021B có
nhánh override chưa đo. Writer/cap và ánh xạ KR vẫn mở, không suy toàn bộ stat từ offset này.


### Sức chứa10 được kiểm trước pool25 (R-168)

[Ca Factory bổ sung/14mẫu/35hash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#so-sức-chứa-trước-cấp-phát--r-168)
đo so5B1E58 trên cùng facet: usage/limit0/1→9/10 rồi10/10 dù tiếp tụcpickup và thử ô khác. Pool25,
còn15 entry thỏa hai predicate0; không phải hết pool10. Không151/120;12pickup/10placement có ACK.
5AEC77/+88 và5A0274/+84 đã định vị; writer/literalclamp và identityKR vẫn mở. Basefacet+4 đi qua
patchstub.devirt→414543,22Bgetter và5Bstub đã giữprovenance; không đo basebox hoặc khẳng định vanillaequivalence.


### Roller tăng vận tốc nhưng cấp chưa nối (R-169)

[One-run/9windows/35hash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#roller-và-vận-tốc-thực--r-169):
DSH1 đo hai cặpUp/Down trước165–170/sau198–204px/s trên cùngactor448364544/C664C4, đúngmột
117/158kind2/khôngbom/nhốt. Observedsampleincrement5→6px không phải cấp5→6. Nativelevel vẫnunknown; R-170 dướiđây bổsungactualgetterreturn5→6.
Không đổi24px/s/level hoặc suy C00/C01KR. Sampler/readjitter/gaps/+550failure/cachedstate-onlypaths
được giữ trongevidence; không lặpWine khi chưa có consumer cấp mới.


### Getter movement đã đo cùng actor trước/sau Roller (R-170)

[4native returns/52rawhash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#giá-trị-getter-đưa-vào-movement-trướcsau-roller--r-170):
controller59EC93→595CCE và hero595CB6→5A64E6 đều trả5 trước/6 sau đúngmột Roller. Cùngactor449413520/
C664C4, controllerhero+5F4/C6630C; native158handler715E90 đãreturn71B717 trướcPOST. Boxcontroller+64
đổi5→6, nhưng branch chọn/writer/clamp chưa chứng minh. Không xem số này là cấp chỉ vì bằng sampledelta.
Source producer→local amount→virtual+128 đã nối; consumer5A0F75 chưa đọc. Timinginstrumented bị loại;
R-169 là capture vận tốc riêng. Không đổi speed24/profile/KRidentity. Bootstrapactor vànativeeventphase
đã thay setup interrupt lỗi; giữ sáu diagnostic thấtbại, không lấy chúng làm phép đo hoặc lặp lại.


### Amount→XY tại lời gọi thực (R-171)

[4rows/44hash](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#amount-và-xy-tại-virtual-movement-thực--r-171):
actualvirtual24459D902→59DA6F/return59D908 trên samehero449162552/C664C4 nhận5→0 và
Y519→514 trước item;6→0 vàY284→278 sau native158, X560. Actualoutercallerreturn131E9DF,
không phải5A1035 giả định. Source5A0F75conditional/static vàR170getter là bằng chứng riêng, không
kết luận actualcase4; controllerfield+58khônglàherostate. Run đầuguardfailkhôngphysicsresult.
Hai cặp là nativeamount/XY observation, khônglevel/FPS/pxs/normalization. Giữ profile hiện hành và
truy nguồn/cadence từ đúngcaller cócache; không đo lại hai cặp này.

### Plateau C64 sau 12 Roller (R-172)

[CSV/manifest và giới hạn](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#plateau-c64-sau-12-roller--r-172):
cùngactor449222352/C664C4, controllerhero+5F4/C6630C, C64 banđầu5; các return native158 cho
6,7 rồi mười lần7. Có24entry/return của12kind2, cuốigetter595CB6→5A64E6 trảEAX7/C647.
Đối chứng nativeXY cuối điDown46→193/Up193→53; khôngbom/nổ/nhốt. Đây là field/getter plateau7
trong ca VN, không phải cap toàn KR, cấp tốc độ, px/s hoặc chứng minh clampwriter.

Bootstrap actor được đọc ở các mốc handler, chưa chứng minh đó là ECXreceiver handler715E90.
Capr1 bootstrap không đăngký/khôngdata; capr2 chỉ3movementpairs5/6/7 rồiobserverabort; capr3 đổiđiểmđọc
sang ACKnative, giữ failedraw. Không đo lại plateau này; chỉ mở lại khi có build/actor/consumer khác hoặc
cạnh writer/maxfield mới. Source/cadence/normalization/capKR vẫn cần bằng chứng riêng.

### Receiver và phép tăng/kẹp C64 (R-173)

[Payload/CSV/provenance](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#receiver-và-phép-tăngkẹp-c64--r-173):
sau mộtkind2, actualdispatch89C202 cóselectorARG2=3, indexARG1=2, ARG3=hero+5FC/C66284.
C64 native5 trước/6 sau89CDCB. PEslot+14→599499 và nhánhselector3 nối tới method tăng/kẹp:
receiver+5C=controller+64, child445516 đọcold/cộng1/ghi cùngbox, rồikẹp theo receiver+50=controller+58.
COLoffset5FC và typeCGOZzolmanBomber khớp receiver, không đo trực tiếp target599499 bằng breakpoint.

Watchdiscovery và dispatch là hai capture/actor khác nhau; bốn transientcipherunknown giữ riêng.
Các payloadcomplete có thể giải mã lại5/6. Maxfield chưa có số trong actor mới; R172plateau7 không
thay việc đo nó. Không suy level/C00C01KR/F2F3 hoặc px/s; bước tiếp là init/current/maxfield consumer,
không lặp getter/plateau hoặc đọc toàn bộ switch item.

### Khởi tạo base/max/current (R-174)

[Hai capture và nguồn getter](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#khởi-tạo-base-max-và-current-trên-actor-vn--r-174):
actor449423536 có current/max0/0 tại RET7BC41E và5/7 tại updater5B1472. Capture đổi target sang
base/max trên actor449224784 cho base/max/current0/0/0→5/7/5, cùng kiểu C664C4/C6630C/C66284.
Tám watch stop của capture sau nối base/max tới return59750A/597518 trong597432; current reset
ở59BD36 từ base+640 sang current+658, max ở+64C. Hai actor độc lập, không ghép clock/pointer.

Sáu output qua595FD8 chưa là sáu cột Attr: getter đầu có override có điều kiện, getter max chuyển
tiếp971676, modifier virtual+A4 và nguồn key/fallback chưa xác nhận. Mười hai cipher transient giữ
unknown/raw; không suy literal5/7, level hoặc C00/C01. Hai run đã đóng sạch; chỉ mở lượt tiếp ở
nguồn key/fallback/modifier mới, không lặp reset.

### Sáu output khởi tạo và hai đầu vào lựa chọn (R-175)

[Wire checksum, source và hai ca](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#sáu-output-tới-field-khởi-tạo-và-key-đầu-vào--r-175)
nối output2/4 qua5787AE→578979 tới capacitycurrent facet+4 và Firecurrent+1C. Output3/5 là
counterpart kẹp khi khởi tạo; vai trò trần sau pickup chưa được xác nhận. Movementbase/max/current
giữ field640/64C/658, không phải speed level được định danh.

Hai lựa chọn byte0/1 được xác minh trong frame S103/S106. Actor riêng449551848/449274328 đều có
key0, output `[5,7,1,10,1,7]` trước/sau vùng modifier và endpoint movement5/7/5,
capacity1/10/1, Fire1/7/1. Chỉ lần khởi tạo đầu được chụp; marker+A4 không chạm. Không suy hai
nhân vật có cùng chỉ số hoặc mode19 bỏ qua lựa chọn. Key/identity còn thiếu; bước tiếp là
caller cung cấp arg3 của5AED95→5A517F, không lặp hai ca. Hai run sạch, không item/bom, timingexcluded.

### Player mover và hai ca rẽ (R-176)

[Argument, nativeXY, control và giới hạn](../../evidence/measurements/vn-player-movement-2026-10-25/README.md)
trên actor449278600/C664C4 nối slotv244 tới59DA6F, nhánh59DABE→594B68/return59DAD9.
Out16=`[560,480,40,40]` tại nativeXY560/519; ý nghĩa downstream/hurtbox chưa đủ.
Hai mốc trướcDownX540/X535 tạiY39 ổn định, rồi chỉ điều khiểnDown cho sửa ngang tớiX560 và
Y109/Y104. Đây là dữ liệu player thật, hỗ trợ candidate corner assistance, chưa là ngưỡngmax.
Opposite-direction controls/nativeXY và mọi overshoot được giữ; không suy tốc độ từ host clock.
Run sạch,6stop/2.952XYrows/60hash, không item/bom; không đổi profile hay identityC00/C01.

### Offset30/35 và aligned control (R-177)

[Run riêng, cùng actor trong mỗi control](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#đối-chứng-lệch30-và35-trên-cùng-actor--r-177):
actor449280160/current5, actualX530/Y39→DownX560/Y139, actualX525/Y39→Down giữ525/39;
alignedX560Down→Y164/Up→39. Hai offset30/35 có phản ứng khác nhau, không chốt literalmax34.
Requested535/530 khác actual530/525; giữ overshoot,9endpoint/42hash/4.387XYrows.
Không movement breakpoint trong lượt này,6hook/5stop chỉ khởi tạo; không item/bom/timing-derived
velocity hoặc native direction flag. Corner-assistance interpretation còn candidate; các response
native đã có đối chứng, không lặp để tìm lại cùng kết quả.

### S501 field→contextkey→stat pairs (R-178)

[Một field đổi, native setter và output](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#field-bắt-đầu-trận-tới-key-và-sáu-output--r-178):
actor395728984, S501 slot0/value1, plaincontextfield DWORDFFFFFFFF→1. Descriptor vC20934 pointer+4
bằng context thật, virtual34 đọc+60; key1 ở initializer cho `[4,8,2,7,1,9]`, finalmovement4/8/4,
capacity2/7/2, Fire1/9/1. Roomcharacterbyte giữ1 như control key0; khác biệt nằm ở field S501.

R1 lỗi channel vàR2 guard đọc count32 thay vì native100 không có endpoint dùng được; giữ45hash
excluded. R3 sửa observer, không native memory/gameplay, đủ7stop/39rawfiles; mọi process riêng
đã đóng. Cùng actor current4 có Up/DownXY estimates136,799/134,336px/s sau hook tắt, không nativelevel
hoặc FPS. Chưa suy C00/C01 KR identity chỉ từ số; bước tiếp là binding render/row có sourcepoint.

## Consumer tile passable trên đường player — R-179

Lượt25/10 chỉ đọc PE VN cùng SHA `bc875ba5…`; không mở client hoặc lấy thêm timing. Từ anchor
playerR176, source59DA6F gọi7293B4 để lấy/cast tile passable theo ô rồi5C0E98 đọc byte hướng.
Return0 đặt flag chặn trong caller; [metadata32hash/case](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#predicate-ô-passable-của-player--r-179).
Code path confirmed không có nghĩa đã quan sát branch chạy; property/nativehurtbox/maxcorner/mọi
mode vẫn unknown. Các đo R176/R177 giữ nguyên, không dùng kết quả PET hoặc hold hữu hạn để lấp chỗ thiếu.

### Logical Attr row và source tăng/kẹp — R-180/R-181

[Native row/reader và metadata](../../evidence/measurements/vn-item-effects-2026-10-24/README.md#tên-hàng-attr-tới-sáu-reader-native--r-180)
đã nối Character/C00 f0=0, C01 f0=1 tới cùng typedmanager10596C50/C83AEC. Trong ca key1,
actualglobalE60844 alias manager; sáu protectedfield reader/EAX và out6 là4/8/2/7/1/9.
Logicalrowidentity đã có source, không chờ avatar/tên hiển thị hoặc chỉ dựa số trùng KR. Nhánh
caption5A4AA9 ở ca khác trả tester, dừng làm cầu asset. 27stop/11hook,63rawSHA hai ca giữ riêng;
inventoryfinal ghi trước disable, cleanup chứng minh bằng source/mocks/wrapperexit, không sửa raw.

Source cùng class của R181 còn nối selector1/2→facet510 hai method578842/5788E1:
capacitycurrent+514/max+584 và Firecurrent+52C/max+59C tăng1 rồi kẹp theo loadedmax.
Cached445516 xác nhận phép cộng, R175 xác nhận nguồn base/max/reset; không cần đo lại plateau.
Đây là staticactivationpath, không hai native activation mới; nativeFPS/pxs/engineunit và profile
candidate được review riêng. Không source/gate/releaseflag nào tự đổi từ dãy số này.

### Sameactor C01 speedcurve và calibration — R183/R184/R185

[Method/12window/40rawhash/R1 riêng](../../evidence/measurements/vn-player-movement-2026-10-25/README.md#curve-tốc-độ-cùng-actor--r-183r-184r-185)
đã kết hợp source R173/R180/R182 với900nativeXYrows cùngactor449284224/C664C4. Fourkind2ACK
đưa4→5→6→7→8; nativeMeanOLS133,70/166,08/200,70/233,23/267,05px/s. Clock là HOSTmemoryread
bracket, windows không chồng9debugstop, không gọi là nativeclock/FPS. R1 actor khác fullcurve thiếu
vì controller/lastACKtimeout, validpoint4/5 và34hash giữ riêng. R2stop2/exit0/cleanup hoàn tất.
R184 chọn33DESIGN từ normalized~33,36 thay24; R185+1logicalpointVNconfirmed, không+1px/s/
capmọimode. Sai sốmodel bảo thủ±31–39px/s khác tinyreadband; không distinguish33/34/nativeconstant.
Profile12statvalues/trapped17/hurtbox/assist/time model/releasefalse giữ nguyên, G2 cần acceptance mới.
