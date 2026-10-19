# Register nguồn web KR / CN / TW

Cập nhật: 2026-10-19. Nguồn chuẩn về các web source đã được đọc trong phiên research trước;
lượt dọn docs này chỉ hợp nhất ghi chú, **không xác minh lại truy cập hoặc luật**.
Target **KR PC build 00967**. Tất cả nguồn dưới đây có `target_build_match: unconfirmed`;
không claim nào tự trở thành ruleset đã nghiệm thu. Backlog: [QUESTIONS](../QUESTIONS.md).

Ngày truy cập của sáu nguồn: **2026-10-19**, theo ghi nhận research trước.
Chất lượng official và mức phù hợp build là hai trục riêng. Không suy ngày bài từ copyright footer.

| Source ID / khu vực | URL / locator | Ngày xuất bản / phạm vi | Kết luận hỗ trợ và giới hạn |
|---|---|---|---|
| WEB-KR-001 / KR official Nexon | [Patch notice](https://ca.nexon.com/News/Notice/Content/140373), mục 5–6; mục 4 là recipe event | Bài 2023-03-08, áp dụng 2023-03-09; patch lịch sử | Recipe bột cơ bản và đổi xác suất phân giải Magic Market; hạng ★★★ vàng 20% / đỏ 60% / trắng 20%. Chưa kiểm chuỗi patch sau tới 00967; không trộn recipe event vào recipe thường. Ngoài V1. |
| WEB-CN-001 / CN official SDO | [Item guide](https://bnb.web.sdo.com/web3/intro/items.asp), mô tả từng item | Không xác định ngày/build; PC BnB CN | Kim thoát bẫy, phi tiêu kích nổ, khiên 2 giây, spring vượt bom/vật cản. Không đủ cooldown/collision/tick; không xác nhận khiên KR. Các item này ngoài V1. |
| WEB-CN-002 / CN official SDO | [PvP modes](https://bnb.web.sdo.com/web5/gameinfo/gameinfo_2.asp?CategoryID=3018&cid=1&id=121355), đoạn theo mode | Không xác định ngày/build | Điều kiện thắng/cơ chế Zombie, Hunting, Race, Captain, Bomb; Zombie ít nhất 5 người, bắt đầu 2 zombie. Chưa xác nhận mode/rules tương ứng KR; ngoài V1, không dùng suy luật Battle FFA. |
| WEB-CN-003 / CN official SDO | [Controls](https://bnb.web.sdo.com/web5/gameinfo/gameinfo_1.asp?id=121317), ready/start/kick và room/chat | Không xác định ngày/build | Tham khảo luồng ready/cancel, quyền chủ phòng và room/chat commands. Hỗ trợ checklist UI; không chứng minh wire protocol, authority hoặc toàn bộ policy V1. Liên quan Q-008/Q-012. |
| WEB-CN-004 / CN official SDO | [Equipment](https://bnb.web.sdo.com/web5/gameinfo/gameinfo_1.asp?id=121319), equip/use và Hero | Không xác định ngày/build | Cách trang bị/phím dùng item, Hero có trang bị riêng. Checklist UI/backend cho later; không đủ mô hình inventory hoặc xác nhận KR. |
| WEB-TW-001 / TW official Beanfun | [SnowBall event guide](https://tw.beanfun.com/bnb/event/20120116_snow/p1.html), hướng ném/range/snowman/rescue | Event 2012-01-16–2012-03-01; ngày xuất bản chưa xác nhận | Ném hướng nhìn, range cơ bản 3 ô, gặp vật cản/người thì nổ; ba giai đoạn snowman, đồng đội chạm hồi một giai đoạn. Cách đếm range và phiên bản KR chưa xác nhận. Ngoài V1. |

| WEB-KR-002 / KR community wiki (namu mirror) | [크레이지 아케이드/아이템](https://namu.wiki/w/크레이지%20아케이드/아이템) — cào qua `namu.moe` | Cập nhật trang 2026-10-15; access 2026-10-19; build KR cuối | Danh mục item + số cụ thể: +1 bóng/+1 ô, Ultra→max, Red Devil→max+đẩy, Roller +1 speed, Oxygen kéo dài thời gian bị nhốt, Trap 1→2, Drill 3 lần, Power Ginseng 15 s / Super 30 s; **tốc độ phương tiện dùng CÙNG thang với nhân vật** (rùa 1, cú 5, rùa cướp biển 9, UFO 10, goòng 10/2, kart 10/4/11.5). Hỗ trợ Q-005/Q-007. Không có đơn vị px/s; không chứng minh build 00967. |
| WEB-KR-003 / KR community wiki (namu mirror) | [크레이지 아케이드/경험치표](https://namu.wiki/w/크레이지%20아케이드/경험치표) | Trang 2026-08-12; access 2026-10-19 | Bảng EXP 720 cấp đầy đủ (cần/tổng theo cấp, huy hiệu); max 720 = 78.000.000 EXP; hell level mỗi 15 cấp; ladder 1 bậc = 60 điểm. Đã parse → `data/kr/exp_table.json` (boomkr-re). Số liệu tự khai từ trang chủ Nexon; chưa đối chiếu build 00967. |
| WEB-KR-004 / KR community wiki (namu mirror) | [크레이지 아케이드/게임 모드](https://namu.wiki/w/크레이지%20아케이드/게임%20모드) | Access 2026-10-19; build KR cuối | Cơ chế bị nhốt (bong bóng mờ dần → nổ → chết; đi chậm, không đặt bóng; Lodumani ngắn hơn), nổ dây chuyền, respawn 3 s, time bomb 5 s + speed max, host không start 10 s (30 s 슈퍼방장), Manner/Free. **Không có số giây tới lúc chết.** Hỗ trợ Q-004/Q-008. |
| WEB-KR-005 / KR community wiki (namu mirror) | [크레이지 아케이드/전략 및 플레이 팁](https://namu.wiki/w/크레이지%20아케이드/전략%20및%20플레이%20팁) | Access 2026-10-19 | Cơ chế suy ra từ chiến thuật: 걸치기 (đặt bóng lệch tâm ô), 연사 (đặt liên tiếp), 일자; phân loại 노샵/기샵/풀샵/올노; 몰아잡기 dùng Power Ginseng 15 s cuối. Hỗ trợ Q-005 (cornering) và thiết kế rule profile. |
| WEB-KR-006 / KR community wiki (namu mirror) | [크레이지 아케이드/몬스터 모드](https://namu.wiki/w/크레이지%20아케이드/몬스터%20모드) | Access 2026-10-19; build KR cuối | 8 bộ map + 6 boss, thứ tự khó, ngưỡng SS theo thời gian/kill (61–160 s), điểm kill 엔젤코인 +3 / boss +5 / chết −2, cơ chế đá bóng-xuyên, giếng, băng trượt. Ngoài V1 (monster mode) nhưng dùng cho backlog. |
| WEB-KR-007 / KR community wiki (namu mirror) | [크레이지 아케이드/맵](https://namu.wiki/w/크레이지%20아케이드/맵) + [문서 chính](https://namu.wiki/w/크레이지%20아케이드) | Access 2026-10-19 | Danh sách theme/map, tên map nổi tiếng, mô tả vật thể theo map (băng trượt, ray, giếng, gai). Dùng đối chiếu `out/scripts/` (770 XML) — không thay thế dữ liệu pack. |
| WEB-KR-008 / KR community wiki (namu mirror) | [크레이지 아케이드/펫](https://namu.wiki/w/크레이지%20아케이드/펫) | Access 2026-10-19 | Danh mục pet + hiệu ứng. Ngoài V1. |

| WEB-GH-001 / GitHub third-party remake | [tpoisonooo/BnB](https://github.com/tpoisonooo/BnB) — `server/game.js`, `server/map.js`, `server/characters.js` | Cập nhật 2026-10-16; access 2026-10-19; remake CN 盛大 (không phải KR 00967) | Hằng số server: tick 30 Hz, fuse **2.5 s**, lửa **0.5 s**, bị nhốt **4 s**, round 180 s, warmup 3 s, drop rate 0.35 + bảng weight, TILE **40**, 15×13, hộp va chạm 26 px, assist 12 px, kick 360 px/s, **speed 1 cấp = 24 px/s**. Tác giả tự cân bằng lại; nguồn số "official" là 百度百科. Dùng làm candidate cho Q-004/Q-005; không thay pack/spec. |
| WEB-GH-002 / GitHub | [gunheehan/CrazyBoom](https://github.com/gunheehan/CrazyBoom) | Access 2026-10-19; Unity C# | Danh sách tính năng (player property, item, water balloon network, game lifecycle) — **không có hằng số**. Chỉ tham khảo kiến trúc. |
| WEB-GH-003 / GitHub | [ljcduo/CrazyArcade](https://github.com/ljcduo/CrazyArcade) | Access 2026-10-19; C++/DX11 demo | Render/input demo, không có số. |
| WEB-GH-004 / GitHub | [developeSHG/GAME-CrazyAcade](https://github.com/developeSHG/GAME-CrazyAcade) | Access 2026-10-19; C++ WinAPI | Map tile tool + collision, không có số. |
| WEB-GH-005 / GitHub | [weishi-lab/SCNU-426](https://github.com/weishi-lab/SCNU-426) | Access 2026-10-19; Java, 2020 | Bản một phần; ít giá trị. |

| WEB-GH-006 / GitHub | [Ark-Point/crazy-arcade](https://github.com/Ark-Point/crazy-arcade) | Access 2026-10-19; JS | Web game + AI agent; **không có bảng chỉ số gốc**. Chỉ tham khảo kiến trúc agent/benchmark. |
| WEB-GH-007 / GitHub | [Lor3walker/popo-arcade](https://github.com/Lor3walker/popo-arcade) | Access 2026-10-19; JS | Remake "from-scratch"; chỉ số tự thiết kế, không dẫn nguồn gốc. Không dùng làm oracle. |

## Khi bổ sung nguồn hoặc dùng một claim

- Ghi source ID, URL, title/locator, access date, publication date nếu biết, region/build/event,
  claim hỗ trợ và giới hạn. Link source từ findings/spec; không chép một bảng luật sang nhiều plan.
- Claim luật V1 ghi ở [RULES_V1](../../specification/gameplay/RULES_V1.md) với nguồn/trạng thái theo
  [AGENTS §4](../../../AGENTS.md#4-bằng-chứng); nguồn web là `web`, trạng thái `candidate` tới khi đối chiếu.
- Nguồn official khác region (CN/TW) vẫn là `web`/`candidate` cho build KR/VN; chứng minh tương ứng build là task riêng.
  Nguồn trùng nội dung không tính độc lập; search snippet chưa đọc chỉ là đầu mối.
- Một số deep link Nexon từng redirect Home/lỗi tải; không kết luận website mất hết dữ liệu.
  Khi lỗi, ghi URL/thời điểm/phạm vi truy cập, tìm archive hoặc nguồn khác có xuất xứ.
- Chưa có nguồn web trong register xác nhận fuse/tick/collision KR chính xác hoặc toàn bộ recipe
  build 00967. Đây là giới hạn của lượt tìm trước, không phải kết luận Internet không có thông tin.

Không lưu toàn bài có bản quyền vào repo. Bổ sung ghi chú ngắn có locator; nếu cần artifact,
ghi provenance/hash/phạm vi sử dụng ở evidence. Research mới theo task ưu tiên trong QUESTIONS.
