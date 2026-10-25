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


## Amount và XY tại virtual movement thực — R-171

Câu hỏi: **lời gọi movement thực nhận amount bao nhiêu và XY đổi ra sao trước/sau Roller?**
Capture `host-movement-amount-146-r2-caller-observe` giữ cùng fixtureVN bc875…/Factory01index193/
room19, đúngmột117/158kind2, không111/151/120. [4rows](movement-amount.csv) và
[44rawhash/phạm vi](movement-amount.json) giữ cả probe/harness/server thực dùng và lượt guard lỗi trước đó.
Root Codex chạy duy nhất trong namespaceuser/net/PID/mount riêng, loopback và `/tmp`/display riêng;
đã stopcommand2, supervisorDONE/exec99379EXIT0. Không gọi hàm trong inferior hoặc ép stat.

Probe tại **call59D902→actualhero.v+244=59DA6F** và **return59D908**. Mỗi phase chỉ một cặp entry/return,
kiểm sameactor/type/controller, amountpointer=`EBP+12`, target59DA6F và checksumXY. Return kiểm
sameframe/thread và target không đổi. PRE arm từactualherobootstrap, POST arm sau handlernative158
715E90 đãreturn71B717; logpacketphảiđúngmộtkind2. Thời gian debugger chỉ dùng kiểmthứtự,
**không** là timing/vận tốc/FPS. Không suy actor hoặc hànhvi từ `ready`/keydown/serverACK.

| Phase | Native amount trước/sau call | Native XY trước→sau | EAX tại return | Caller return |
|---|---|---|---:|---|
| Trước Roller | 5→0 | (560,519)→(560,514) | 5 | 131E9DF |
| Sau native158 return | 6→0 | (560,284)→(560,278) | 6 | 131E9DF |

Actor449162552/C664C4, controller449164076/C6630C, frameEBP1233148/amountptr1233160 và
thread[42000,312,0] giữ nguyên. Native158return815660557626827ns nằmgiữa hai cặp; không dùng
khoảngđó làm tốc độ item hoặc thời gian mô phỏng. Đây là **amount5 đi5pixel vàamount6 đi6pixel
trong hai đoạn dọc cụ thể**. Không suy mọi hướng/collision/max hoặc native level/FPS/px/s; EAX ghi
đúng giá trị quan sát, không tự đặt nghĩa toànhàm là “consumed” từ hai con số bằng nhau.

### Caller thực khác giả định và nguồn chưa nối

Lượt đầu `host-movement-amount-146-r1` không có consumerpair dùngđược: guard hard-code
outerreturn5A1035 bị từchối trước khi ghi entry. Lỗi giả định caller được giữ riêng, không là kếtquả
âm của gameplay. Lượt r2 đổi phương pháp thành **ghi caller thực** với các guardtype/target/ptr/XY
vẫn giữ; quan sát outerreturn131E9DF ở cảhai cặp. Không retry cùng lỗi hoặc xóa raw thấtbại.

Cache22B131E9D0..131E9E6 có directcall131E9DA→59D894 (return131E9DF), nhưng chưa có đủ thân
producer để đặt tên/ýnghĩa. Không gọi cả vùng131E9 là VM/pet từ địa chỉ hoặc gán nó cho5A0F75.
Cache5A0F75prefix chỉ chứng minh **conditional** cases4..7 dùng amountpointer, gọi59D894 và trừ EAX;
chưa chứngminh nó là caller của hai capture này. Cũng chưa loại trừ nó chạy ở các thời điểm không chụp.
`controller+58=7` của R-170 là protectedfield, **không phải** hero.v+58state; hero+58 là Y.

Staticcache lượt consumer có prefix5A0F75768B, helper59605825B và prefix59D894172B:965B/3units,
vượt grant2body mộtunit đãghi rõ. SlotC66708 `6fda5900→59DA6F` được kiểm từPE và kiểm lại runtime.
Không suy phầnđuôi5A0F75/consumerkhác từ prefix. Boxpayload không được lưu đủ để tự giải mã lại
sau capture; decode live kiểmcookie/checksum, tươngtự giới hạn R-170.

R-171 chốt quan sát **argument→XY trong lời gọi thực** ở hai ca, không nối toànbộ đường
R-170producer hoặc dùng đồnghồ R-169 để suy normalization. Bước tiếp: reuse actualcaller/sourcecache
để truy nguồn amount/cadence hoặc writer/clamp có ca rõ; không đo lại cặp này chỉ để được cùng số.
Profile speed24DESIGN, cap/identityKR và toànbộgate giữ nguyên.

## Plateau C64 sau 12 Roller — R-172

[25 dòng CSV](roller-cap.csv) gồm bootstrap và24entry/return của12native158;
[metadata/getter/đối chứng/93hash](roller-cap.json). FixtureVN VBNB_116bc875…,
Factory01index193/room19, cùngactor449222352/C664C4/controllerC6630C. Quyền dùng đo độngVN
theo ADR0002/0013; giữ giới hạn IAT/devirt/offline, chưa chứng minh vanilla tương đương.

| Mốc native | C64 entry→return | Ghi nhận |
|---|---|---|
| Bootstrap | 5 | Trường protectedcontroller+64, cookie/checksum hợp lệ |
| Pickup1 | 5→6 | Một117/158kind2 |
| Pickup2 | 6→7 | Cùngactor/controller |
| Pickup3…12 | 7→7, mười lần | Cả12ACKnative đã xửlý; không lấy wireACK làm nativecapture |
| Getter cuối | EAX7/C647 | 595CB6→5A64E6, cùngactor/thread, entryESP1233252→return1233260 |

12vịtríitem duy nhất x14/y11…0, không111/151/120. Tổng27traps, khôngconsumer-error.
Sauđó nativeXY vẫn điDown46→193 vàUp193→53, cuối về39. Các khoảng650ms chỉ chọn mẫu
đối chứng; không suy vận tốc/FPS từ keydown hay thời gian debugger. HaiACK5/6 có cùngY285,
haiACK9/10 cùngY131 được giữ nguyên; không bỏ các hàng không đổiXY.

**Kết luận:** protectedC64 plateau7 sau hai lần tăng, qua thêm10ACK, getter cuối7 và đối chứng
di chuyển. Không gọi trường đó là cấp tốc độ, capmọiKR/C00/C01, per-frameamount hoặc maxfield.
Actor được chọn từ bootstrap và đọc ở mốc handler; chưa chứng minh đó là ECXreceiver715E90.
Payload box không lưu đủ để giải mã lại offline; decoderlive đã kiểm cookie/checksum, registerEAX
cuối corroborates7. Writer/clamp/cadence/normalization và ánhxạ nhânvậtKR còn riêng.

### Giữ các lượt chưa đủ quan sát

- Capr1: thiếu instanceHeroBoot dù báoready, không có capture. Bản sửa đăngkýhook được kiểm offline
  oldfalsepositive/newgreen/mutationred và review độc lập trước chạy.
- Capr2: ba cặp actualv244 amount5/6/7 cóYdelta−5/−6/−7; sau native158 thứ3 không quan sát đủ cặp
  trước pickup4, guard failclosed. 12wireACK tổng cộng chỉ có3nativeentry/return; không kết luậncap
  hoặc nhánhamount>=8. Prefix59D894 đãcache không chứa threshold8.
- Capr3: đổi điểm quan sát sang native158entry/return/C64 và getter cuối, không lặp input/probe lỗi.
  Kiểm offline callback/schema/24ACK synthetic/getter/sameactor/failclosed không phải gameplay.
  Giữ rawcả3lượt, chỉ publicmanifest; không publish ảnh/pack/binary/longasm.

Nguồn đã dùng được giữ trong rawrun: frozenharness/probe/server, sources/start/isolation,
registered-hooks, nativeACK/getter, packetlog, XYsamples và supervisor. Máy chỉ có một runtime
owner; stop3 kết thúc namespace riêng, wrapper xácnhận ownerfalse. Profile và các gate không đổi.
Bước tiếp là writer/maxfield/nhánh hoặc normalization có cạnh/ca mới; không đo lại plateau7 này.

## Receiver và phép tăng/kẹp C64 — R-173

[10 dòng event CSV](roller-writer.csv) và [payload/metadata/101hash](roller-writer.json) kết hợp
hai capture độc lập với phân tích tĩnh theo địa chỉ vừa quan sát. Cùng fixtureVN VBNB_116bc875…,
Factory01index193/room19, patchIAT/devirt/offline được ghi rõ. Không yêu cầu đo lại KR theo ADR0002/0013;
không gọi fixture tương đương nguyên bản khi chưa có proof.

| Capture | Native quan sát | Ý nghĩa / giới hạn |
|---|---|---|
| Hardware discovery, actor449290536 | Một715E90→71B717, C64 trước5/sau6; bốn watchstop type7 tạiPC413FDA; tổng7traps | CùngPC không phải bốn writer; PC sau instruction. Cả4payload transient checksumunknown giữ nguyên, không đổi thành0 |
| Dispatch, actor449155584 | Actual89C202 ARG1index2/ARG2selector3/ARG3receiver449157116=hero+5FC, VT C66284; return89CDCB C645→6; tổng5traps | Cùngthread/frame trong mộtACKkind2; không ghép actor/clock với capture hardware |
| Binding tĩnh | Selector3 trỏ89C2D8/callreceiver.v+14, PEslotC66298→599499; COLoffset5FC/typeCGOZzolmanBomber | Khớp offset receiver thật; không phải breakpoint trực tiếp lên virtualtarget599499 |

### Lvalue và phép cập nhật

Method599499 (63B, gồmRET) lấy receiver+5C = hero+658 = controller+64. Child445516 (25B,
gồmRET) đọc box qua414543, tínhold+1, rồi gọi413F32 ghi lại cùngbox. Call445525 córeturn44552A
trong hardwarestack; không suy toàn bộ callgraph từ backtrace đã tối ưu/mất frame.

Sauchild,599499 đọc updatedC64 và maxfield receiver+50 = hero+64C = controller+58; nếuupdated
lớn hơnmax thì ghi max vàoC64. Trong phạm vi số nguyên bình thường của ca đã đo, đây là
**C64=min(old+1,maxfield)**. Không dùng điều kiệnold>max (sai thứ tự), không tuyên bố semantics
overflow. Địa chỉ maxfield đã nối; **giá trị số max chưa đo trên actor này**. R172plateau7 của
capture/actor khác không biến thành literalmax7 của method này.

### Kiểm lại và giới hạn

Cảhai run có đúng một117/158kind2/x14y11, không111/151/120, actor/type riêng được giữ. NativeACK
vàdispatch đã trả về; rootstop2 đóng namespace/process riêng, không ảnh hưởng session khác.
Header12/payload10 đầy đủ tại các mốc cho phép giải mã lại offline: endpoint5/6 hợp lệcookie/checksum,
cả4transient vẫnunknown. Watchrows không lưu nativeclock thì CSV để trống; không tự gán timestamp.

Reader414543 xuất hiện trong bảngitem không tự làC64 reader; actoralias được nối từ actualARG3,
vtable và địa chỉlvalue. Hai tool window yêu cầu768B/in767B, gồm code liền kề vượt giới hạn hai unit; đã dừng, giữ raw.
Có7/9RETsite trong hai window, không gọi chúng là số hàm đã xác minh;88B hai target là phần phân tích ngữ nghĩa. Các cut RET thiếu3B trước đây giữ lại cùng bản sửa, không xóa
raw để giảm số body. Chỉ publicmetadata, không PE/pack/ảnh/rawlongasm.

R173 xác nhận đường tăng1/kẹp trên C64 được chọn. Không suy level, f2/f3, identityC00/C01KR, công thức
px/s hoặc trần mọi nhân vật. Init/current/maxfieldsource và mappingKR vẫn ở Q017/Q020; profile vàG2/G6
không đổi. Bước tiếp phải là cạnh khởi tạo/consumer cụ thể, không lặp12Roller/watch/dispatch đã giải.

## Khởi tạo base, max và current trên actor VN — R-174

[16 mốc quan sát](stat-init.csv), [payload, locator và manifest](stat-init.json).
Hai capture dùng cùng fixture VN `bc875…`, Factory01/room19, mỗi capture chọn một actor
`C664C4` với controller `C6630C` và facet `C66284`. Server offline không tạo item/bom hay chỉ thị
nhốt/chết; lựa chọn nhân vật mặc định của harness không xác nhận identity C00/C01 KR.

| Capture | Các giá trị hợp lệ trên cùng actor | Quan sát ghi |
|---|---|---|
| `host-stat-init-149-r1`, actor449423536 | Tại RET `7BC41E`: current/max `0/0`; tại updater `5B1472`: `5/7` | 4 hardware stop type7 trên current, cùng PC `413FDA`, return trong stack `59BD50`; tổng6 stop |
| `host-stat-init-base-max-149-r1`, actor449224784 | Tại RET: base/max/current `0/0/0`; tại updater: `5/7/5` | 4 stop base khi max/current còn0; tiếp4 stop max khi base đã5/current còn0; tổng10 stop |

Header12/payload10 ở mốc ổn định được giải mã lại và kiểm cookie/checksum. Cả12 trạng thái ghi dở
vẫn `unknown`, giữ đầy đủ raw; không gán số cho ciphertext chưa hợp lệ. Capture đầu lưu raw transient
ở trường top-level `header12`/`payload10`, bên cạnh nested `current` unknown. Payload pointer không
đổi tại các mốc kiểm. Chỉ theo dõi bốn stop đầu mỗi payload, không bao phủ mọi lần ghi hoặc lần ghi
cùng giá trị. Hai actor độc lập; không ghép clock/pointer và không đo thời gian, level hay px/s.

### Nguồn ghi và ba cặp đầu vào

Offset tính từ actor: **base+640** là nguồn để reset **current+658** (controller+64);
**max+64C** là cùng lvalue dùng để kẹp ở R-173. Tên base mô tả vai trò sao chép, chưa là speed level.

Return `59750A` và `597518` trong hai stack khớp các call `413F32` của `597432`: ghi local đầu vào
thứ nhất vào base và thứ hai vào max sau khi kẹp từng cặp trong ba cặp local. Setter `59BD36`
đọc base rồi ghi current; return `59BD50` khớp capture đầu. `5A517F` gọi virtual `+0x1A0` trên actor;
slot PE của vtable đã quan sát trỏ `59BD36`. Đây là binding tĩnh với receiver phù hợp, không phải
breakpoint trực tiếp tại return `5A521A`; backtrace có thể bỏ frame. Tên class trong cache IDA chỉ
là locator, loại actor lấy từ vtable runtime.

Sáu local đi qua virtual `+0xA0→595FD8`, gồm sáu getter riêng trên `global[E60844]`, cùng numeric
key và tham số `-1`. Chúng chưa chứng minh sáu cột liền nhau của một row. Getter đầu `971A0F`
dùng box `manager+0x4BD0+key*0x24` **nếu dương**; nhánh còn lại `971A3B` có điều kiện khác và một box
ở `manager+45C4+key*24` (các offset/stride viết hệ16), phần join/fallback chưa đọc. Getter thứ hai
`971B26` chuyển tiếp `971676(key,0)`, chưa xác nhận row hoặc literal. Nhánh modifier virtual `+0xA4`
có thể sửa local trước khi ghi; nhánh được chọn chưa đo.

R-174 xác nhận lifecycle và pipeline ghi/reset trong các ca này. Nguồn key, fallback, literal5/7,
row BomberAttr, f2–f7 và identity C00/C01 vẫn chưa xác nhận. Max7 của actor mới không đo lại max của
actor R-173. Profile, 12 khóa candidate và G2/G6 giữ nguyên.

### Kiểm lại và điểm dừng

Hai run đóng namespace riêng với exit0, không `consumer-error`/relocation. Toàn bộ114/105 packet
message tương ứng không có opcode111/117/120/151/158. Hook dùng tối đa hai slot hardware, kiểm
type/registration và cùng actor; updater là đối chứng dữ liệu native, marker hook/stage không thay
kết quả. Harness giữ network loopback-only, user/PID/mount riêng và hash nguồn/patch fixture.

Manifest có60 file runtime cùng các capture/cache tĩnh; số file không phải số hàm. Chín instruction
block giữ2.114 byte từ3.264 byte window yêu cầu, có prefix/nhánh chưa đọc và bản sao raw; không gọi
chúng là chín full function. Chỉ public metadata, không game/media/raw decompile. Bước tiếp là
key/fallback `971A59/971A5B`, helper `971676` hoặc modifier có đối chứng, không đo lại reset/plateau.

## Sáu output tới field khởi tạo và key đầu vào — R-175

[8 mốc native](stat-roles.csv), [payload, wire checksum, 77 hash và source](stat-roles.json).
Hai ca chỉ đổi byte lựa chọn nhân vật trong server offline từ0 sang1; fixture `bc875…`,
Factory01/room19 và các nguồn probe/server/harness còn lại giống nhau. Frame S103 và S106 được
giải mã lại, kiểm checksum: cả hai frame mang0 trong ca0 và1 trong ca1. Đây là đầu vào
`emu_choice`, chưa xác nhận identity nhân vật KR.

| Ca | Actor riêng | Key tại call virtual+A0 | Sáu output trước/sau vùng modifier | Field sau reset |
|---|---|---|---|---|
| byte0 | 449551848 | 0 | `[5,7,1,10,1,7]` / cùng giá trị | movement5/7/5; capacity1/10/1; Fire1/7/1 |
| byte1 | 449274328 | 0 | `[5,7,1,10,1,7]` / cùng giá trị | movement5/7/5; capacity1/10/1; Fire1/7/1 |

Mỗi ca có bốn stop trên cùng actor/frame/thread: trước+A0 tại597471, sau return597477,
join5974DB và sau reset5A521A. Vtable thực C664C4, controller C6630C, facet C66284; slot604=0.
Marker trước+A4 tại5974C3 không được chạm; không có phép đo entry/return hay hiệu ứng của+A4.
Đây chỉ là **lần khởi tạo đầu được quan sát**. Kết quả không phân biệt được identity; không suy
mode19 bỏ qua lựa chọn, hai nhân vật giống nhau hoặc những lần khởi tạo sau cũng nhận key0.

### Nối output với consumer

`597432` chuyển bốn output cuối tới5787AE theo thứ tự đối số. Setter ghi các box của facet
actor+510 và kẹp hai cặp capacity/Fire theo counterpart. Reset578979 sao chép hai base tới
field current đã nối consumer trong R-168/R-166:

| Output của595FD8 | Đích khởi tạo | Đích current / consumer đã có |
|---|---|---|
| 0 / 1 | actor+640 / +64C | +640→+658 qua59BD36; R-170/R-173/R-174 |
| 2 / 3 | facet+68 / +74 | +68→+4; getter capacity R-168 |
| 4 / 5 | facet+80 / +8C | +80→+1C; getter Fire R-166 |

Vai trò **base khởi tạo** của output2/4 được nối qua sao chép và consumer, không dựa vào thứ tự
số. Output3/5 là counterpart dùng kẹp khi khởi tạo; vai trò trần sau pickup vẫn `candidate`.
Reset còn ghi box facet+10=1; ý nghĩa giữ opaque. Offset vtable+84/+88 ở phép so R-168 là slot
hàm, không phải offset của các field dữ liệu trong bảng này.

### Fallback có điều kiện, chưa là row identity

Năm block mới giữ335 byte từ976 byte window, được đối chiếu byte với PE cùng SHA; metadata
không gọi chúng là năm full function. Tail971A59/971A5B nối getter đầu tới97163C, helper58B
này dùng override `manager+4BD0+key*24` nếu dương, nếu không đọc base tại
`manager+1D4+key*19C` rồi cộng **giá trị** đối số thứ hai. Maxhelper971676 có cùng dạng nhưng
fallback tại+1E0; wrapper971B26 truyền giá trị cộng0. Không cộng đối số này vào địa chỉ box.
Các offset/stride trên viết hệ16; stride19C không tự chứng minh tên bảng hoặc C00/C01.

Trong cả hai ca, manager pointer giữ nguyên giữa các mốc của chính ca đó; first dword làC83AEC,
chưa xác định lớp. Các box primary/alt/base/max tại key0 lần lượt0/0/5/7, hợp lệ cookie/checksum.
Chúng không thay breakpoint trực tiếp vào mọi nhánh getter, và sáu output vẫn đến từ sáu getter
riêng. Không nâng thành sáu cột liền nhau của BomberAttr KR.

Hai namespace riêng đã đóng exit0; mỗi ca có S191 và nativeXY hợp lệ, không111/117/120/151/158,
không item/bom. Toàn bộ timing debugger bị loại khỏi kết luận. Chỉ công bố metadata/payload nhỏ,
không game/media/asm dài. Bước tiếp là caller đưa đối số key vào5AED95→5A517F→597432;
không lặp hai lựa chọn hoặc phép đo plateau để tìm lại cùng kết quả. Profile và G2/G6 chưa đổi.

## Field bắt đầu trận tới key và sáu output — R-178

[12 mốc của hai ca](match-key.csv), [payload, dataflow và manifest](match-key.json).
Sử dụng cùng fixture VN `bc875…`, Factory01/room19; byte chọn ở S103/S106 giữ1. Ca đối chứng cũ
có key0 và output `[5,7,1,10,1,7]`. Lượt mới chỉ đổi **byte thứ hai của block mỗi slot trong S501**
từ0 sang1 cho câu hỏi gameplay; endpoint channel được pin3838 để sửa lỗi bootstrap riêng, giữ
cùng localhost/mode/map/spawn. Source và cấu hình được fingerprint; server không ép sáu chỉ số.

### Chuỗi ghi và đọc được quan sát

| Điểm | Native quan sát / dataflow | Phạm vi |
|---|---|---|
| Parser S501 | Hai decode byte liên tiếp: slot tại6C4E10, giá trị tại6C4E32; `MOVZX EAX,AL`, push value/slot, call8E76A3 tại6C4E42 | Getter decode có tăng cursor1byte và bounds; không suy type từ tên `pos` trong comment emu cũ |
| Setter entry8E76A3 | Return6C4E47, slot0/value1, context272107560/count100/table277730572; field+60 trước=`FFFFFFFF` | Native plain DWORD, không phải box hoặc ghi u8 |
| Setter return6C4E47 | Cùng context/count/table/thread; field=`01000000`/DWORD1 | Có payload/raw và kiểm field bằng value truyền vào |
| Typed context→key | Descriptor arg0 có VT C20934, pointer+4 bằng **chính context272107560**. Slot+34 trỏ4B82C2→8E76CF, đọc rowstride244/field60 theo slot0 | Class RTTI `CBomberExternalContextFC` là binding kiểu; không tự là tên nhân vật |
| Call khởi tạo | Actual actorVT C664C4+8C→7BC250 chỉ tail-jump5AED95; native return6EB8A3, arg3=1, key tại597471=1 | Native cùng actor395728984/thread; backtrace return6EC22A giữ riêng, không thay return thật |
| Output và reset | `[4,8,2,7,1,9]` trước/sau vùng modifier; final movementbase/max/current4/8/4, capacitybase/counterpart/current2/7/2, Firebase/counterpart/current1/9/1 | Output role qua R-175; marker+A4 không chạm; counterpart chưa là cap sau pickup được đo trong ca này |

Native storage là **DWORD**: setter8E76C7 có opcode`89540860`, ghi EDX; wire u8 đã zero-extend
trước khi gọi. Không dùng so sánh u8 để bỏ qua ba byte còn lại của field. Getter8E76F0 cũng đọc
DWORD; fallback khi slot không hợp lệ là−1, không phải0. Ca mới đo trực tiếp field1 và key1,
không cần suy từ fallback hay nhãn decompile.

Nguồn key trước đây đã nối: virtual+34 trên descriptor trả vào local `[EBP−54]` của producer,
local được push làm arg3 tại6EB89D. Source target6EB788 được tìm từ call6EC225; receiver thật/EDX
được bổ sung bằng register capture, không chỉ dựa vào matching PEslot. Wrapper7BC250 không ép key0.
Vì vậy hai ca đổi byte **trong phòng** của R-175 vẫn first-init key0 không được dùng để kết luận
lựa chọn bị bỏ qua: harness cũ còn gửi0 ở field S501 đi vào context này.

### Đối chiếu output, không gộp identity

| Ca / actor | Byte S103/S106 | Field S501 trong generator | Key native | Sáu output native |
|---|---|---|---|---|
| `host-key-producer-type-149-r1`,449280160 | 1 | 0 | 0 | `[5,7,1,10,1,7]` |
| `host-match-slot-key-149-r3-count100`,395728984 | 1 | 1; setter native được chụp | 1 | `[4,8,2,7,1,9]` |

Hai actor khác nhau, không ghép pointer/clock. Key0/1→output và các destination đã có consumer;
đây chưa tự chứng minh asset/name C00/C01 KR hoặc sáu cột BomberAttr KR liền nhau. Dãy số trùng
R-040/R-041 không thay bằng chứng identity; 12 khóa candidate và profile giữ nguyên.
Autostart S501 của server không log frame hex: proof input là generator/hash/cấu hình và native
setter argument, **không tuyên bố đã kiểm CRC frame S501**. S103/S106 của các ca trước có proof riêng.

### Đại lượng liên quan cùng actor: cửa sổ XY khi current4

Sau khi cả7 stop khởi tạo kết thúc và toàn bộ hook tắt, protocol cửa sổ Up/Down được ghi **trước
khi gửi lệnh**. Trong cùng lối x14, nativeXY current4 có hai ước lượng từ60% giữa của span chuyển
động: **136,799 px/s Up /134,336 px/s Down**. Thời lượng lấy giữa timestamp đọc XY ở hai đầu,
không lấy thời gian keydown/packet; không suy client FPS, server tick hoặc speed level KR.

JSON giữ hai endpoint đọc, reader span, khoảng lớn nhất giữa các mẫu thay đổi và phân tích nhạy
với±8px/hai bước4 cùng sai số thời gian bằng observed change-gap+reader-span. Đây là giả định
nhạy của ước lượng, không phải khoảng tin cậy thống kê hay bound scheduler tuyệt đối. Dữ liệu là
một điểm vận tốc thực ứng với scalar4, bổ sung các ca scalar5/6 của R-169; chưa chọn công thức
normalization mới cho engine hoặc ghép clock giữa những capture khác nhau.

### Lượt hỏng và kiểm lại

| Run được giữ riêng | Kết quả | Cách sử dụng |
|---|---|---|
| `host-match-slot-key-149-r1` | FailedToConnSvr trước stage11/S501; không native setter/key | Setup failure, loại khỏi kết luận gameplay |
| `host-match-slot-key-149-r2-channel-pin` | Đã vào trận; observer đặt count≤32 trong khi native count100, dừng trước lưu setter pair/key | Instrumentation failure; không gọi S501/giá trị1 là lỗi gameplay |
| `host-match-slot-key-149-r3-count100` | Chỉ sửa giới hạn đọc observer≤256, vẫn một field slot0; setter/key/final/S191/XY đầy đủ | Ca hợp lệ; không thay native count, không ghi bộ nhớ game |

R3 có39 file raw,7 stop/8hook, exit0; control0 có42 file raw. Manifest chính123 raw/static hash,
hai run hỏng thêm45 hash ở mục excluded riêng;14 instruction block giữ603/1.439byte window,
không gọi là14 full function. Có RTTI/data hash và source budgets riêng. Hai run hỏng được lưu nguyên,
không rename thành ca thành công hoặc retry mù. Probe guard đã qua mock cả count100/256 hợp lệ,
257/0 lỗi và các ca actor/thread/table/value khác; mock không tự chứng minh giá trị native.

Đã giải mã lại các box/current/XY, đối chiếu raw field plain DWORD, source hashes, pointer alias,
frame/thread và count/table stability. S191/only-lo/namespace riêng; không111/117/120/151/158,
không item/bom hoặc statforcing. Toàn bộ process riêng đã đóng sạch. Câu hỏi key→stat ở ca đã có
câu trả lời; bước tiếp là key→render/row identity có cạnh mới hoặc cap thực của character khác,
không đọc lại setter/getter/context chỉ để tìm lại cùng source.

## Tên hàng Attr tới sáu reader native — R-180

[Các hàng/return CSV](attr-identity.csv), [source, payload tóm tắt và manifest](attr-identity.json).
Câu hỏi đã giải: **key0/1 có trỏ đúng hàng logic Character/C00/C01 và đúng sáu field client đọc hay không?**
Không dùng dãy số trùng nhau làm bằng chứng identity. Hai quan sát native khác nhau được giữ riêng:

- Ca `host-key-render-native-149-r1`: actor448865232/key1, descriptorC20934; đường5A4AA9/v38
  đưa chuỗi `tester` tới5B293F, actualreturn5A4B98/RET8. Đây là chuỗi nickname/context trong ca,
  không phải đường chứng minh assetCharacter/C01. Dừng hướng caption/special-case này; 30hash giữ lại.
- Ca `host-attr-file-consumer-149-r1`: 33rawfile, properWinei386, Factory01/room19, cùng server
  R178 pin3838/fieldS501=1; không item/bom/input di chuyển. Quan sát reader971383 và post-store971517
  nối inputname/f0 tới row thực; 595FD8/597477 nối cùng manager tới sixoutput. Không sửa memory game.

### Chuỗi nguồn và native receiver

Filename `Attr/BomberAttr` tạiC83AD0 được push tại9712CE. Reader success971383 lấy inputname/f0,
rowbase=`EBX+f0*0x19C`, lưu tên tạirow+90 và sáu field protected tại
`1D4/1E0/1EC/1F8/204/210`. Post-store kiểm checksum/cookie và so từng field với input.
Bản xuất VN `BomberAttr.bin` SHAa44ba347… chỉ là reference hai record, không tuyên bố toàn pack trong
runtime giống byte với bản xuất; native input/post-store là bằng chứng nối hàng.

| Logical row | f0 | Manager / rowbase | f2…f7 native đã lưu |
|---|---:|---|---|
| Character/C00 | 0 | 10596C50 / 10596C50 | 5,7,1,10,1,7 |
| Character/C01 | 1 | 10596C50 / 10596DEC | 4,8,2,7,1,9 |

ManagerEBX có vptrC83AEC; globalE60844 tại consumer bằng **chính pointer manager này**, không chỉ
cùng type hoặc cùng giá trị. Consumer actor393578184/C664C4, key1, selectedrow/nameC01 còn sống tại
entry. Sáu actual CALL414543 và returnEAX lần lượt:

| Field trong C01 row | Native return | EAX |
|---|---|---:|
| +1D4 | 97166D | 4 |
| +1E0 | 9716A7 | 8 |
| +1EC | 9716E1 | 2 |
| +1F8 | 97171B | 7 |
| +204 | 971755 | 1 |
| +210 | 97178F | 9 |

Mỗi return kiểm cùng thread/stack/return address/fieldpointer, EAX bằng giải mã field đã lưu;
ECX lúc return là0x37, **caller-saved**, không được dùng thay fieldpointer. out6 tại597477 là
`[4,8,2,7,1,9]`, ESPentry+12. Có27actualstop,15readerentry gồm9unmatched và6matching,
6native reader-return;11hook gồm5fixed+6temporary. S191 và post-returnXY có thật, không111/117/120/151/158.

### Chất lượng và phạm vi

Root kiểm lại63rawSHA của hai ca, 18CSVrow, box checksum/cookie, pointeralias, sáu directcall5byte,
sourcehash và namespaceexit. DSH1 kiểm độc lập37case mock trên chính observerbody và rawdata.
`registered-hooks-final.json` được ghi **trước** `mark_terminal`: đây là snapshot trước disable,
không phải bằng chứng các enabledflag đã false. Giữ nguyên file/SHA; cleanup dựa source-order,
mock kiểm object disabled và wrapperEXIT0/namespace đã kết thúc, không viết lại metadata lịch sử.

Confirmed trong VN: **logical Attr name/f0→typed manager→row→six native returns/out6**.
Chưa quan sát avatar/tên hiển thị hoặc native consumer KR; không dùng avatar làm điều kiện bổ sung
cho logicalrow đã chứng minh. Native unit/time/override mọi mode và profile12khóa là câu hỏi riêng.
R178 không còn thiếu logicalrowbinding; nhận định identity chưa nối trong các mục lịch sử phía trên
được bổ sung bởi R180, không xóa giới hạn riêng của từng capture.

## Tăng và kẹp capacity/Fire theo trần đã nạp — R-181

Câu hỏi source: counterpart output3/5 trong R175 có thật là max khi tăng stat không?
Từ switch89C202 đã cache: selector1→v4, selector2→v8 trên kiểu receiverC66284. Hai thunk
594EDB/594EE9 rebase−EC: hero+5FC→hero+510, rồi tail vA4/vA0. Facet510 cùng class đã có runtime
vptrC663C4 ở R166; data slotA4/A0 trỏ578842/5788E1. **Binding selector1/2 này là static**,
không gọi nó là hai activation runtime mới.

| Stat | Method | Current/max theo hero | Source effect |
|---|---|---|---|
| Capacity | 578842 | +514 / +584 | current=min(old+1,max) |
| Fire | 5788E1 | +52C / +59C | current=min(old+1,max) |

Cả hai gọi cached445516 đọc old, cộng1, store; sau đó đọc current/max và store max nếu current lớn hơn.
Công thức áp trong miền signed32 bình thường; không khẳng định overflow INT_MAX. Return1.
R175 đã nối f4/f5 tới facet+68/+74 và f6/f7 tới+80/+8C; reset copy base→current+4/+1C.
Do đó counterpart+74/+8C là **trần loaded của stat trong hai method**, không còn chỉ là counterpart
kẹp lúc khởi tạo. R173 chứng minh độc lập kiểu tăng/kẹp movementcurrent/max; không gộp actor/capture.

Manifest ghi7instructionblock với2.560byte requested/761retained (3block loader/parser,2thunk,2block effect),
child44551625byte **reuse**,24byte slotdata và30byte directcall kiểm riêng; không gọi7block là7hàm đầy đủ.
Raw/code dài/binary/media giữ private. Không đo lại plateau, không suy globalcap10, các override getter,
nguồn ItemAttr/dropgroup/KR consumer, cấp tốc độ→px/s, hay chọn12profilekeys từ một nhãn mơ hồ.
**Không đổi profile, source/status12khóa, releaseflag hoặc gate trong bổ sung này.** Bước tiếp G2 là
review cùng đại lượng và DESIGN quy đổi V1 từ row/statpoint đã có; không quay lại chuỗi nickname/loader.
