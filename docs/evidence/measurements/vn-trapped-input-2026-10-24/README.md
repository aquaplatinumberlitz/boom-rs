# Di chuyển khi bị nhốt — VN offline 2026-10-24

Task [#259](https://github.com/aquaplatinumberlitz/boom-rs/issues/259), root [#147](https://github.com/aquaplatinumberlitz/boom-rs/issues/147).
Baseline `f5ffb03e80ef28dce3fb946af60391d42b7edae9`; nguồn `vn_runtime` theo ADR0002/0013 và quyết định chủ dự án24/10.
Fixture VBNB_116 SHA `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`,
base400000/i386. Giới hạn patch/rebuild như [owner-grace](../vn-player-grace-2026-10-24/README.md);
không cần giả định server gốc hay đồng nhất class VN với C00/C01 KR.

## Phép đo và kết quả

Ocean01 VNindex672, room19, một người. Server offline chỉ gửi152/151 (fuse8s **emu_choice**),
không mô phỏng di chuyển/nhốt/chết. Namespace mạng chỉloopback, PIDnamespace riêng, privateprefix/game,
Wine32 `winedbg --gdb`; một runtime owner Codex. Không đụng DB/binary gốc, không kết nối server thật.

Hook thưa getter X58D56D và sender5AF165/11,13. Đọc X/Y boxed+4C/+58 bằng decoder checksum
nonmutating từ process thực, không gọi getter trong inferior. R1 hero449298672, R3hero449555808;
vtable cảhaiC664C4, sender11/13 cùngptr trong từngrun. R3 getter xác minh trả40 **sau11/trước13**,
không phải chuỗi thời gian GetX→11→13. R1 có getter trước11 và control di chuyển khi sống.

| Case | Điều khiển và tọa độ native | Kết luận |
|---|---|---|
| r1 alive | Right40→80, Left80→35, mỗi bước5px; sau thả phím giữ35 | Positive control phím/đườngtrống/sampler cùngactor |
| r1 trapped | Sau native11, Left35→26, Right26→35, mỗi bước1px; idle không đổiXY | Trapped vẫn di chuyển theoinput, chậm hơn control |
| r3 trapped | Sau11, Right40→61, idle giữ61, Left61→40, idle giữ40; native13 cùngactor | Repeat hai chiều; breakpoint threshold/deathsite đã bỏ, chỉhook thưa |
| r2 excluded | Có111/152/151, không ghi native11/13; protocol dừng vì timeout | Không dùng làm negative hay số đo tốc độ; nguyên nhân chưa xác định |

Native-event/command/XY và **93 file raw** có size/SHA trong [manifest.json](manifest.json),
raw ở private root được ghi trong manifest. Media/PE/decompile không đưa lên repo.
[CSV](movement.csv) tách **XY transitions** khỏi các bản ghi chỉđổi bubble_age.
Clock là CLOCK_MONOTONIC begin/end của lần đọc; fit displacement giữa **first/last transition**,
không chia displacement cho thời gian giữ phím hay packetwallclock.
Bốn đoạn trapped: **17,007375;16,318188;16,879114;16,837315px/s**.
Hai đoạn alive quá ngắn chỉlàm positive control, không hiệu chuẩn cấp tốc độ.

Sampler sleep5ms nhưng chỉghi khi XY/age thay đổi; khoảng giữa các bản ghi không phải pollinterval.
CSV ghi thêm khoảng vận tốc bảo thủ từ previousrecord.begin→currentrecord.end ở haiendpoint;
đây chỉlà bracket của dữ liệu đãghi, không chặn được mọi scheduling/debugger overhead.
Idle trước/sau vàgiữa haihướng không đổiXY. Hai positivecapture không tạo statisticalCI hay
xác nhận mọi character/mode/cấp/tìnhhuống. Không đổi counter bubble thành tickRust.

## Áp dụng V1 và phần chưa biết

- R-161 xác nhận **có slow movement theo phím ngang** trong đúngcase VN; thay giảđịnh đứngyên R-036.
- R-162 chọn **17px/s DESIGN calibration**, gần các fit first/last; không phải literalnative17.
  Tại30Hz số nguyên566mpx/tick, tức16,98px/s;60Hz283mpx/tick. Không nhân speedlevel vì chưa đo quan hệ.
- V1 áp cùng tốcđộ cho hai trục và dùng hộp13px/bámgóc12px hiệnhành: **DESIGN**, chưa đo hìnhhọc bubble
  hay phím dọc native. Dead không đi; trapped không pickup/đặtbom theo policy hiệnhành.
- R1 bấmSpace2lần, R3 mộtlần sau11 không có111 mới; chỉquan sát **không đặt được trongcase**.
  Chưa tách gate/cap/cell nên không nâng thành nativebomb-inputconfirmed.
- Timer6350ms/fuse2500ms/flame540ms/cap/hitbox/touch-order không thay. Root147 vàG2/G3 chưa done.

Kiểm code: targeted movement test với gate cũ `life!=Alive` red (x60000≠65660), gate mới green;
slow input haihướng độc lập speedlevel, Dead đứngyên, va chạm vẫn chặn.
Scenario `life/trapped_movement_before_touch.toml` chốt DESIGN movement trước touch: tạiđúng28px,
Trapped đi vào283mpx thì bị pop ngaytickđó; không gọi thứtự này là nativeconfirmed. Predictor dùng chung coremovement.
Scenario trapped slow chạy qua profile fixture mới; profileidentity/replay thay theo rulekey mới,
ba header replay đổi profilehash12ef614d5751da78→40e42ec9c27c59bf;
re-bless hai ASCIIreplay không đổi bytegolden (event/outcome/checkpoint giữ nguyên).
Replaypack Village00 đãverify với BOOM_KR_PACK=/home/admin/boomkr-full, EXIT0;
goldenpack cũnggiữ nguyên. O-011 giữ webexpectedunknown, không thay bằng17 đểtựxácnhậnnativeoracle.
