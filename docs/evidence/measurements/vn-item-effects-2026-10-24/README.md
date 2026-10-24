# Hiệu ứng Bomb/Fire VN — đo offline có đối chứng (2026-10-24)

Root [#149](https://github.com/aquaplatinumberlitz/boom-rs/issues/149). Hai ca Bomb/Fire và ba lượt cap bổ sung
bằng chứng cho R-025/R-111; không đổi rules/profile và không suy trần chỉ số KR từ một nhân vật VN.
Media/binary/decompile dài giữ local, repo chỉ nhận metadata.

## Nguồn và phép đo

VN VBNB_116 fixture SHA `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`,
base400000/i386; giới hạn patch như [provenance owner-grace](../vn-player-grace-2026-10-24/README.md).
Server của các ca Ocean01 có SHA `fa65dc31bb8937e0a798c157bf5f4c8a95f99de4db8492402497973005ab5a45`
chỉ cấp item553, xác nhận117→158 và111→152; khi bật BOMB_ECHO thì gửi151 theo lịch **emu_choice**.
Server không mô phỏng stat/cap/va chạm/nhốt: hiệu ứng được thực hiện ở client VN.\nFactory01 dùng bản sao đổi tuple map đầu/index193; SHA từng server-used.py được ghi riêng trong\nmanifest, không gán SHA của server Ocean cho các run Factory.

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
`docs/vm_6EFD4A_inputs.md` §5/callsite5B1FB5 và `docs/oc_e-game.md` §5a-5d.
152được server dựng từ chính record111 nên không phải bằng chứng độc lập thứhai về getter/stat.
Việc hiểu vai trò tầm của r18 dùng trace/đối chiếu legacy đã có R-025; phép đo này ghi đúng scalar.

## Phạm vi và bước tiếp

- Xác nhận hành vi Bomb1→2 và trườngr18sauFire trong đúngbuild/actor; bổ sungR-025/R-111.
- Chưa tìm được nativegetter/writer tăngstat/kẹpmax, Rollerlevel, nghĩa f2..f7 hay12khóaC00/C01KR.
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

[controls.csv](controls.csv) nối chín pickup cuối (ba mỗi run) với KEY_PRESS Space trong\nsupervisor.log và cửa sổ không có111 trong packets.jsonl; thêm phép thử ô trống khác là đối chứng\nthứtư, không thay ba lần thử saupickup. Command cuối đã là stop, nên dùng action log lịch sử.\n\nBa lượt đều đặt quả đầu ở(14,12), rồi đi lên12 ô mới, mỗi ô có một Bombkind0. Đủ12 lần
117→158 khác index, đủ các vị trí native; server xác nhận mọi111, không tự giới hạn số bom.

| Lượt | Kết quả trước bất kỳ151 | Đối chứng cuối |
|---|---|---|
| cap-r1 (BOMB_ECHO0) | 10 quả tại y12..3, sau đó ba ACK mới ởy2/1/0 và Space không có111 | Ô trống khác nativeX510/cell12,0, Space vẫn không111; không151/120 |
| cap-r2 (BOMB_ECHO45) | Lặp lại10 quả và12 pickup, ba lần ở trần không111 | Gate12s hết trước lịch emu45s; plateau vẫn hợp lệ nhưng **chưa thực hiện** phép giải phóng chỗ |
| cap-r3 (BOMB_ECHO25) | Cùng plateau10 và ba pickup dư không111; ô trống nativeX515/cell13,0 vẫn không111 | 151id1 lúc18:23:44.526,111serial200/id11 lúc18:23:45.010, **trước**151id2 lúc18:23:45.616; không120 |

R3 sửa lịch emu để đối chứng nằm trong gate, không tăng timeout hoặc lấy25s làm fuse native.
Các151 sau đó vẫn giữ trong log; đối chứng chỉ xét111id11 giữa151 đầu và thứhai.
Không khẳng định chính xác chỉ một slot được giải phóng: chain nội bộ chưa instrument trong ca này.

R-163 xác nhận **trần hữu hiệu10 bom còn tồn tại trong đúng fixture/actor** và đặt lại thành công
sau151. Chưa phân biệt stat cap/pool cấp phát, chưa tìm getter/writer hoặc nối C00/C01KR.
Không nâng12khóa candidate target hoặc đổi profile từ kết quả riêng này.
