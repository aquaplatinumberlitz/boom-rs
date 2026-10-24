# Hiệu ứng Bomb/Fire/Roller VN — đo offline có đối chứng (2026-10-24)

Root [#149](https://github.com/aquaplatinumberlitz/boom-rs/issues/149). Hai ca Bomb/Fire và ba lượt cap bổ sung
bằng chứng cho R-025/R-111; không đổi rules/profile và không suy trần chỉ số KR từ một nhân vật VN.
Media/binary/decompile dài giữ local, repo chỉ nhận metadata.

## Nguồn và phép đo

VN VBNB_116 fixture SHA `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`,
base400000/i386; giới hạn patch như [provenance owner-grace](../vn-player-grace-2026-10-24/README.md).
Server của các ca Ocean01 có SHA `fa65dc31bb8937e0a798c157bf5f4c8a95f99de4db8492402497973005ab5a45`
chỉ cấp item553, xác nhận117→158 và111→152; khi bật BOMB_ECHO thì gửi151 theo lịch **emu_choice**.
Server không mô phỏng stat/cap/va chạm/nhốt: hiệu ứng được thực hiện ở client VN.
Factory01 dùng bản sao đổi tuple map đầu/index193; SHA từng server-used.py được ghi riêng trong
manifest, không gán SHA của server Ocean cho các run Factory.

Một runtime owner Codex, namespace mạng chỉloopback/PID riêng, privateprefix/game, WineGDBi386.
Ocean01VNindex672/room19, heroC664C4; X/Y boxed+4C/+58 đọc nonmutating/checksum, one-shotgetter58D56D.
Input đi tới X native0/40/80, tọa độ thực ở control40 có thể45 nên giữ actual trong CSV.
`harness-used.py` của từngrun là bản sao đúng script đã chạy, hash cùng rawlog trong
[manifest.json](manifest.json): **210 file size/SHA, gồm96PNG**; repo không chứa nội dung các raw này.
[CSV ban đầu](cases.csv) giữ ba ca; [capacity.csv](capacity.csv) ghi pickup/bomb/nổ của ba lượt Factory. Không gộp lỗi phương pháp vào số liệu đạt.

## Kết quả có đối chứng

| Ca | Trình tự thực | Kết luận trong ca |
|---|---|---|
| Bomb kind0, `host-item-bomb-149-r1` | 111ởx0→152id1; Space ởnative45/cell1,0 **không111**; nhặt kind0 ởx80 qua117/158; Space→111ởcell2,0/serial20/id2; vềnative40/cell1,0 thử quả3 **không111**. Không có151, hai quả trước còn tồn tại | Sau một Bomb, sức chứa đồng thời tăng1→2, chưa phải maxcap |
| Fire kind1, `host-item-fire-149-r2` | 111x0/r18=1; controlSpaceở45 không111; nhặtFire117/158; thửđặt trước151 không111;151id1 làm quảđầu hết;111kế tiếp ởx2 có **r18=2** | Kind1 làm trường tầm r18 tăng1→2; sức chứa vẫn1 trong ca. Không dùng thời gian lịch151 làm fuse gốc |
| Fire1 lỗi, `host-item-fire-149-r1` | Helperđã ởtarget80 nhưng vẫn chọnLeft; client trôi80→40 trước151 và bịnhốt ở40; không có111thứhai | **Loại khỏi phép so Fire/range**; không kết luận Fire đổi tầm quảđãđặt |

Lần Fire2,151quảthứhai xảy ra sau khi ghi nhận r18 và gây nhốt owner; phần đó ngoài phép so item.
Không biến một lần nhặt/thử đặt bịtừchối thành giới hạn max cho mọi nhânvật/mode.

### Cẩn thận tên field trong tool

Parser `parse_bomb_record` đặt tên một field là **power**, nhưng fieldđó luôn1 trong caFire này.
Không ghi `power1→2`: field thực đổi là **r18**, qua vf31 theo legacy
`docs/vm_6EFD4A_inputs.md` §5; callsite native thực là5B1F92, còn5B1FB5 là directcall6EFD24 và `docs/oc_e-game.md` §5a-5d.
152được server dựng từ chính record111 nên không phải bằng chứng độc lập thứhai về getter/stat.
Việc hiểu vai trò tầm của r18 dùng trace/đối chiếu legacy đã có R-025; phép đo này ghi đúng scalar.

## Phạm vi và bước tiếp

- Xác nhận hành vi Bomb1→2 và trườngr18sauFire trong đúngbuild/actor; bổ sungR-025/R-111.
- Đã nối getter tầm với111 trong ca bổ sung bên dưới; writer tăngstat/kẹpmax, Rollerlevel, nghĩa f2..f7
  và12khóaC00/C01KR vẫn chưa nối.
  Bảng ItemAttr không tự chứng minh client/server chọn drop. Root149giữPARTIAL.
- Cap cần nhiều lần nhặt đãxácnhận tại trần hoặc actualconsumer/getter; không suy cap từ lần3bịtừchối
  sau mộtBomb. Đườngđi/cellống lấy từ map+ObjectTable chỉlà data candidate trước khi clientchạy qua.
- R1Fire giữ như đối chứng phươngpháp lỗi; r2sửa helper `origin==target` thànhnoop và nhả phím trước
  lệnhmới. Không có lỗi mới được xóa hoặc dùng để tăng sốcase đạt.

## Sức chứa hữu hiệu — Factory01

Root đọc layout từ SOURCE legacy_map.rs/closure::parse_object_table, giải Factory01.map và
ObjectTable VN. Hành lang dữ liệu x14,y12→0 không có Static/Box; **client thực sự đi qua toàn bộ**
mới xác nhận đường thử. Map index193 trong fx.idx, SHA trong manifest; cùng room19/heroC664C4.
Getter xác minh spawn native(560,519); không dùng map đã sửa hay ép teleport.

[controls.csv](controls.csv) nối chín pickup cuối (ba mỗi run) với KEY_PRESS Space trong
supervisor.log và cửa sổ không có111 trong packets.jsonl; thêm phép thử ô trống khác là đối chứng
thứtư, không thay ba lần thử saupickup. Command cuối đã là stop, nên dùng action log lịch sử.

Phân biệt hai loại xác nhận: S158 xác nhận nhặt item (C117), còn S152 xác nhận đặt bom (C111).
Cả9 pickup cuối (ba mỗi run) đều có S158 **trước** lúc bấmSpace; chỉ không có C111/S152 mới.
Controls.csv giữ riêng timestamp C117→S158→Space và cửa sổ không C111.

Ba lượt đều đặt quả đầu ở(14,12), rồi đi lên12 ô mới, mỗi ô có một Bombkind0. Đủ12 lần
117→158 khác index, đủ các vị trí native; server xác nhận mọi111, không tự giới hạn số bom.

| Lượt | Kết quả trước bất kỳ151 | Đối chứng cuối |
|---|---|---|
| cap-r1 (BOMB_ECHO0) | Sau9 pickup đầu có10 quả tại y12..3;3 pickup còn lại (index9/10/11, trong tổng12) đều được xác nhận S158; mỗi lần bấmSpace không C111 | Ô trống khác nativeX510/cell12,0, Space vẫn không111; không151/120 |
| cap-r2 (BOMB_ECHO45) | Lặp lại10 quả và12 pickup có S158; Space sau3 pickup cuối không C111 | Gate12s hết trước lịch emu45s; plateau vẫn hợp lệ nhưng **chưa thực hiện** phép giải phóng chỗ |
| cap-r3 (BOMB_ECHO25) | Cùng plateau10;3 pickup cuối trong tổng12 vẫn có S158, mỗi lần bấmSpace không C111; ô trống nativeX515/cell13,0 vẫn không111 | 151id1 lúc18:23:44.526,111serial200/id11 lúc18:23:45.010, **trước**151id2 lúc18:23:45.616; không120 |

R3 sửa lịch emu để đối chứng nằm trong gate, không tăng timeout hoặc lấy25s làm fuse native.
Các151 sau đó vẫn giữ trong log; đối chứng chỉ xét111id11 giữa151 đầu và thứhai.
Không khẳng định chính xác chỉ một slot được giải phóng: chain nội bộ chưa instrument trong ca này.

R-163 xác nhận **trần hữu hiệu10 bom còn tồn tại trong đúng fixture/actor** và đặt lại thành công
sau151. Ba ca này chưa phân biệt stat cap/pool cấp phát; R-168 bên dưới bổ sung phép so sức chứa riêng.
Writer/clamp và ánh xạ C00/C01KR vẫn chưa nối.
Không nâng12khóa candidate target hoặc đổi profile từ kết quả riêng này.


## Receiver thật của getter tầm — R-166

Ca `host-item-range-receiver-149-r1` dùng cùng fixture/Ocean01, một Firekind1 tại(2,0).
[range-getter.csv](range-getter.csv) nối **hai native call** với hai bản ghi111;
[range-getter.json](range-getter.json) giữ50hash runtime/static, gồm các ảnh và raw chỉ có local.
Bytegate PE/archi386 được kiểm trước khi gắn probe; thời gian debugger không dùng để đo fuse/latency.
Server vẫn chỉ xác nhận pickup/đặt/nổ theo lịch8s emu, không tự tính stat.

Tại5B1F92, `call [eax+0x7C]` nhận ECX=EDI−0xE4. Runtime khóa được trên cùng hero449159016:
ECX449160312=hero+0x510, EDI449160540=hero+0x5F4, vtableC663C4, target5AE518.
Trước Fire trả EAX1 rồi111r18=1; sau117→158 trả EAX2 rồi111r18=2. Parserpower vẫn1.
Sau đặt quả thứhai, phần nổ/nhốt trong raw nằm ngoài phép so getter.

Probe ghi **2 call/3 return rows**: hai return rows của lần thứhai có cùng receiver/target/value và
nằm trước cùng111. CSV giữ số hàng trả về nhưng chỉ tính một lần gọi; không coi đây là ba lần lặp.
Callsite5B1FB5 trước đây trích theo legacy **không phải virtualgetter**: đó là directcall6EFD24 sau
khi ECX đã nạp lại từ[EBP−0x14]; vai trò đối tượng này chưa được xác nhận trong bundle50hash. Phân biệt facet+0x510 với vtable chính của heroC664C4.

Đọc tĩnh cùng SHA: wrapper5AE518 có gate byte receiver+0x2C14, khác0 trả hằng1, bằng0 tail5A021B.
Thân5A021B đầy đủ gọi578963 lấy giá trị nền rồi có các override có điều kiện qua virtual+0x90 và
4AD113. Hàm578963 gọi virtual+0xBC(arg1), sau đó đọc **boxed receiver+0x1C** qua reader414543
đã biết. Với delta receiver thực, vị trí box là hero+0x52C theo phép cộng; chưa chứng minh writer hoặc
runtime nội dung box. Không gọi đây là DWORDstat trực tiếp hoặc giá trị cuối luôn lấy nguyên từ box.

R-166 xác nhận getter thực sự tiêu thụ khi tạo111 và scalar1→2 trong đúng ca, không xác nhận toàn bộ
ngữ nghĩa gate/override, statmax, Roller hay ánh xạ C00/C01KR. Trần hữu hiệu10 của R-163 giữ phạm vi
cũ. Bước tiếp là allocation virtual+0x48 trên **cùng facet** và writer/clamp của box, tránh lặp nhầm
directcallsite cũ hoặc bảy KRlookupcaller đã cache.


## Nhánh chọn phần tử bom — R-167

Cùng caller trước getter tầm,5B1EC5 gọi virtual+0x48 trên facet+0x510. TableC663C4+48 trỏ59F493;
hàm này gọi579DC0 với facet nguyên. **579DC0** mới trừ8, đưa container=hero+0x508 tới578A16.
Đây là cạnh đọc tĩnh trên cùng SHA, chưa phải một lần đo runtime giá trị list/count.

578A16 duyệt list tại container+0x4C, count tại+0x50; phần tử được chọn khi hai giá trị boxed
entry+0x1C vàentry+0xE4 cùng0. Hai getter8/11byte58D4DA/59111A tail tới reader414543 đã cache.
Nếu không có phần tử đạt điều kiện, hàm trả0. Nếu có, hàm điền descriptor và trả pointer phần tử
đã chọn qua579DC0/59F493. Toàn thân397byte578A16 đã đọc, không lấy prefix96byte làm toàn hàm.

Kết quả giải được **cơ chế từ chối khi không tìm được phần tử**, chưa giải writer số lượng phần tử
hoặc giới hạn stat. R-168 bên dưới bổ sung phép đo phân biệt giới hạn getter với số phần tử pool. Trong những thân đã đọc không thấy literal10 hoặc writer count+50. Không gọi
hai boxedfield là cờ active/id khi chưa có consumer; số10 vẫn chỉ là R-163 runtime. Cạnh tiếp cần đo
count/predicate trong cùng actor trước/sau pickup hoặc tìm writer có địa chỉ cụ thể; không quét lại
bảy KRlookupcaller và không đọc các hàm cleanup chỉ để tăng số body.


## So sức chứa trước cấp phát — R-168

Ca mới `host-item-capacity-compare-149-r1` giữ Factory01 thật/index193, room19, cùng loại actor
vtableC664C4. Chỉ một runtime owner trong namespace mạng/PID riêng; fake cấp12 Bombkind0 ở x14/y11..0,
xác nhận117→158 và111→152, **không gửi151 hoặc ép nhốt**. Không sửa stat/memory trong client.
[capacity-getter.csv](capacity-getter.csv) ghi14 lần so; [capacity-getter.json](capacity-getter.json)
giữ35hash raw/static/patch. Ngay trước lần đặt tại ô khác cuối cùng, XY thực là(505,39), không ghi
nhầm target510 là vị trí đã tới.

Tại5B1E58, trước allocator5B1EC5, source so kết quả virtual+88 với virtual+84 trên **cùng facet**
hero+510/C663C4. +88→5AEC77, +84→5A0274. Nếu giá trị thứnhất≥thứhai, `jge5B1FF7` bỏ qua allocator.
Probe đọc register/stack ngay trướccmp, không gọi hàm trong inferior, không biến thời gian dừng debugger
thành phép đo timing. Không gắn thêm breakpoint ở đích nhảy; điều kiện nhánh được suy trực tiếp từ
operand đã chụp và lệnhJGE, đối chiếu kết quả có/không111.

| Mốc | Giá trị +88 / +84 | List count | Phần tử thỏa cảhai predicate0 | Kết quả |
|---|---:|---:|---:|---|
| Trước quảđầu | 0 / 1 | 25 | 25 | Có111/152 |
| Sau1..8 pickup | 1/2 đến8/9 | 25 | 24 đến17 | Mỗi lần có111/152 |
| Saupickup9 | 9 / 10 | 25 | 16 | Cóquả10 |
| Saupickup10/11/12 | 10 / 10 | 25 | 15 | Mỗi lần có158 trướcSpace nhưng không111 |
| Ô trống khác cuối | 10 / 10 | 25 | 15 | Không111; không151/120 trong run |

Cả14 mẫu đọc đủ25 entries, không lỗi checksum/read; mọi entry+E4 bằng0, số entry+1C khác0 khớp
operand+88 trong ca. Tổng12 pickup/ACK và10 placement/ACK. Như vậy **đã tách được lý do plateau10**:
giá trị giới hạn của getter dừng10 và phép so phía trước từ chối đặt, dù list vẫn có15 phần tử đạt
điều kiện allocator. Không phải hết pool10. Chưa thấy writer/literal clamp10, chưa khẳng định max10
cho mọi nhân vật/mode; đây là phép đo cuối getter trong đúng ca.

### Giới hạn patch của base getter

5A0274 gọi base5788B8 nhưng có các override có điều kiện; không suy giá trị box nền luôn bằng kết quả
cuối. Thân5788B8 **22byte** gọi virtual+C8(1), truyềnfacet+4 tới tail16B62F0. Địa chỉ này **nằm trong
section.devirt có file backing**, không phải thiếu memory/ngoài PE. Đúng5byte tại đó là JMP414543;
source legacy `tools/devirt.py:341–348` ghi patchsite5788C9 là `EXPERIMENT: plain skip`.

Đã đối chiếu byte/source và giữ hash/provenance trong manifest; stub này không tự đặt hằng10.
Tương đương toàn bộ với vanilla chưa được chứng minh. Theo quyết định dùng runtime VN của chủ dự án,
kết quả vẫn dùng được trong phạm vi fixture đã công khai patch; không nâng thành literal native/server
hoặc coi boxfacet+4 (hero+514 theo phép cộng) đã được đo. Không đọc lại VM hay thay patch để đạt số mong muốn.

Root149 còn writer/clamp, Roller, ItemAttr/BomberAttr và C00/C01KR. Profile và các khóa nhân vật
không đổi từ một actor VN chưa nối identity; R-163/R-167 được bổ sung, không xóa giới hạn cũ.


## Roller và vận tốc thực — R-169

Câu hỏi đã trả lời: **một Rollerkind2 có làm đổi vận tốc native của cùng actor trong ca kiểm không?**
Câu hỏi cấp tốc độ/normalization và identity C00/C01KR vẫn chưa trả lời, không gộp vào kết luận đạt.
Một capture mới `host-item-roller-149-r1`, cùng fixturebc875…/Factory01index193/room19;
DSH1 là runtimeowner riêng trong namespaceuser/net/PID chỉloopback, privategame/prefix. Đã kết thúc
EXIT0/dọn namespace trước phân tích; không đổi binary/pack/DB/memory hoặc gọi hàm trong inferior.

Server chỉ cấp **một itemkind2** tại(14,6) vàACK117→158. BOMB_PLACE0/BOMB_ECHO0, khôngSpace;
log có đúngmột pickup/ACK vàkhông111/151/120. Nativeactor448364544/vtableC664C4 giữ nguyên, X560;
đi lên/xuống hànhlangx14 đã được kiểm từ ca Factory trước. Hai cặp Up/Down trước item, transitqua
pickup, hai cặp sau vàidle. Không dùng thời gian giữ phím/packet làm vận tốc.

[roller-speed.csv](roller-speed.csv) ghi9cửa sổ từ nativeXY cóchecksum/readbegin/end;
[roller-speed.json](roller-speed.json) giữ35hash (25runtimefile/10static captures) và giới hạn.
Mỗi cửa sổ trung tâm bỏhai mẫu **đổiY** ở mỗi đầu, dùng ΔY/Δbegin_ns; giữ riêng đầyđủfull/central
windows vàtransit trongraw. Root tính lại cả9cửa sổ từ raw, không lấy kết quả do worker ghi làm oracle.

| Phase | Up trung tâm, hai cửa sổ (px/s) | Down trung tâm, hai cửa sổ (px/s) | Mẫu dịch chuyển |
|---|---:|---:|---|
| Trướcpickup | 165.149;170.311 | 167.349;169.018 | 5px giữa các mẫu đổiY |
| Sau117/158kind2 | 203.614;203.787 | 197.861;202.928 | Chủ yếu6px;hai bước12px vẫn giữ |

Đây là tăng vận tốc cửa sổ hữu hạn trên **cùng actor**, không phải cấp5→6 hoặc hàm24px/s/level.
Transitqua pickup có lẫn trước/sau được loại **khỏi phép so**, không xóa raw. Hai hướng/lượt lặp
trong cùng mộtrun là đối chứng, không gọi là nhiều thử nghiệm nhân vật độc lập.

### Sai số, probe không thành công và giới hạn

Sampler nominal5ms, nhưng khoảng các mẫu đổiY còn có cadence native/hostjitter/quantization;
gap lớn tới64.4ms trong các cửa sổ sạch vẫn giữ. Readspan ghi nhận toànrun tốiđa1.139ms,
trong các cửa sổ trungtâm tốiđa0.468ms; đây không phải sai số tuyệt đối của vận tốc. Chưa có timestamp
update native, không bịa confidencebound hoặc chuyển medianinterval thành cấp/tickserver.
Sampler bỏ qua checksum/readfailXY mà không đếm tổng; số mẫu bị bỏ chưa biết, đã công khai trongmanifest.
Idle khoảng29.2s bao gồm setup trước command, không coi đó là nhiều ca idle; các waitcontrols nằm tronglog.

Đọc thử sáu box ở vùng đã có mỏneo facet: +514/+520/+52C/+538/+544 giữ nguyên; +550 lỗi read/checksum
ở755/755mẫu, được ghi riêng, không bỏXY. Không từ đó kết luận mọi stat bất biến hoặc đặt tên +550 làspeed.
Tại captureR-169 nativelevel/getter/writer chưa nối; getter thực được bổ sung riêng R-170 bên dưới, level/writer vẫnunknown. Đọc tĩnh inputcontroller thựcC6630C+0C→5AE857→59C1BC và
heroC664C4+DC/+218→592DE6/596DB7 chỉ nối state/dir/effect; helper chưa mở. Getter59A263 trảboxed
controller+20B8 nhưng chưa có consumer tốcđộ. Không gán body cậpnhật21 của facet thànhmover hoặc
lấy±4targetcorrection cũ làm normalization. Coverage/hashes giữ cả11byte/bodyoverrun, không đếmthunk là0body.

R-169 xác nhận hiệu ứng tăng vận tốc trong caVN, **chưa nâng Roller+1level R-045 hoặc speed24DESIGN**,
chưa đổi profile. Bước tiếp tạiR-169 là source actualXY-delta/consumer đúngreceiver; R-170 bên dưới đã bổ sung actualgetter5→6, nhưng ý nghĩa cấp/writer vẫn mở. Không đo lại mộtRoller chỉ để có số đẹp hơn.


## Giá trị getter đưa vào movement trước/sau Roller — R-170

Câu hỏi: **getter thực sự được actor đang chơi gọi có đổi giá trị sau một Roller không?**
Ca `host-producer-146-r7-event-phase` trả lời được câu hỏi này, bổ sung R-169. Không đồng nhất
scalar với tên “level”, không hiệu chuẩn px/s từ lượt đặt breakpoint này và không thay profile Rust.

Cùng fixture VN VBNB_116 SHA `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`,
Factory01/index193/room19; root Codex là runtime owner duy nhất. Namespace user/net/PID/mount riêng,
chỉ loopback, `/tmp` riêng, Xvfb cấp display tự động. Chỉ gửi input/nhận item qua giao thức; không ép
stat hoặc gọi hàm trong inferior. Giữ giới hạn patch đã công khai ở R-166/R-168/R-169, chưa chứng minh
vanilla-equivalence. Theo ADR0002/0013 và quyết định chủ dự án, dùng trực tiếp quan sát VN trong phạm vi này.

[CSV bốn lần trả về](movement-producer.csv) và [manifest52hash](movement-producer.json) giữ source,
raw runtime, harness/probe/server thực dùng và chẩn đoán setup lỗi. Root tính lại số dòng/operand,
kiểm packet và thứ tự native, đối chiếu18 vùng byte cache với đúng fixture PE. Không có analysis provider
hoặc thân native mới được mở khi xuất metadata.

### Điểm quan sát và đối chứng

Actor bootstrap một lần ở updater `5B1472`, sau khi kiểm vtable thật `C664C4`, controller
`hero+5F4`/`C6630C` và XY checksum. Hai breakpoint tại **địa chỉ trả về**, tự tắt sau mỗi getter:

- Controller `C6630C+38 → 59EC93` trả tại `595CCE` trong `595CB6`.
- Hero `C664C4+12C → 595CB6` trả tại `5A64E6` trong base updater `5A6244`.
  Source lưu EAX vào local amount rồi, sau các modifier có điều kiện, truyền con trỏ local qua
  virtual+128 tại `5A65C8 → 5A0F75`; thân consumer này chưa đọc.

PRE được arm ngay tại actor bootstrap. POST được arm sau **handler native158 `715E90` trả về
`71B717`**, cùng thread; không dùng timestamp gửi packet làm mốc hoàn thành hành vi, không interrupt
Wine đang chạy. Có đúngmột C117/S158 kind2, không C111/S151/C120 trong toàn capture. Đối chứng chuyển
động đọc XY thật: Y519→394→519 trước, qua394→274 tới item, sau đó112; X560. Target399/279/119
khác actual394/274/112, không dùng target thay vị trí đo.

| Phase | Actor / controller | Controller return | Hero amount return | Box controller+64 | XY tại hai lần chụp |
|---|---|---:|---:|---:|---|
| Trước item | 449413520 /449415044 | 5 | 5 | 5 | 560,519 |
| Sau native158 return | Cùng hai pointer/vtable | 6 | 6 | 6 | 560,274 |

Hai PRE tại813003597711378/813003603853014ns; native158 entry813008051545103 và
return813008066092487ns; hai POST tại813008777273531/813008779984453ns. Chỉ dùng clock để kiểm
**thứ tự**, mọi timing/vận tốc của capture này bị loại do debugger. PREcapturecount2 trước khi armPOST;
không coi `probe-ready` hoặc ACKcommand là kết quả getter.

### Phần đã nối và phần vẫn chưa biết

`59EC93`223B chọn giữa các protected field theo predicate: +2F8/+304, fallback+58 hoặc+64 với
clamp+20FC. Runtime +64 đổi5→6; +58=7, +2F8/+304/+20FC=0 giữ nguyên. **Chưa chứng minh branch đã chọn
+64** chỉ từ giá trị bằng nhau, chưa nối writer/clamp của pickup. Reader kiểm cookie/checksum trực tiếp;
raw12byte box header không chứa đủ10byte payload để tự giải mã lại độc lập sau run, giới hạn này được giữ.
Hero raw+2468=255/+246C=0 nên guard bonus của `595CB6` không bật trong ca; raw+2688=11079/11251
chỉ là counter quan sát, không quy ra ms. `local_ebp_m14_old_context` ghi trước store, không phải lượng
vừa trả về hoặc level. +550 của sampler vẫn lỗi read/checksum riêng, không loại XY vì lỗi probe stat này.

Có hai cạnh tĩnh đã lưu: producer→local amount→virtual+128, và arithmetic/writer XY ở
`59DA6F` prefix→virtual+68→`5A55EC`→`58D42B`. Chưa đọc đủ cầu nối `5A0F75`, không tuyên bố đã nối
mọi nhánh, collision hoặc speed normalization. Các round cache cũ có tiny accessor/body vượt budget
đã ghi trong checkpoint; không đếm một lệnh batch là một body hoặc xóa phần đã đọc để giảm count.

R-170 xác nhận **return scalar5→6 và protected field5→6 trong đúng actor/case**. R-169 xác nhận
hiệu ứng vận tốc ở **capture khác**; không lấy hai capture làm một phép đo native level/FPS, không
suy24px/s/level, max/identity C00/C01KR. Bước tiếp: reuse receiver/consumer đã tìm, giải nghĩa nhánh
và writer/clamp hoặc kiểm amount→XY trên ca có đối chứng; không đo lại capacity hoặc lặp bootstrap cũ.

### Các lượt không dùng làm kết quả

Sáu setup trước r7 được giữ nguyên cùng diagnostic hashes: lỗi Python/path, thiếu actor bootstrap,
bootstrap UI quá rộng, lỗi preflight/display, phase arm không ACK, và signal debugger dừng sai context.
Không có getter evidence dùng được từ các lượt đó. Root tiếp quản và sửa theo nguyên nhân, cuối cùng
đổi sang actor bootstrap/native158 event để arm tự động; không retry cùng đường lỗi vô hạn. Ca writer
đặt breakpoint rộng trước đó cũng không dùng làm timing. Lượt r7 đã stop bằng command2, supervisorDONE;
không còn client/namespace riêng của phép đo. Không sửa/kill phiên dùng chung.
