# Catalog chức năng KR — baseline 1

Ngày: 2026-10-19. Baseline: **`KR-00967-V1-scope-1`**.
Target: `/home/admin/boom-re/boomKR/`, build `00967` theo tài liệu hiện có.

**Đã chốt phạm vi sản phẩm V1; chưa xác nhận runtime toàn bộ client KR.** Đây là catalog của dự án,
không phải danh sách do Nexon phát hành, cũng không phải tuyên bố đã RE hết executable.

## Nguồn chuẩn và kết quả

- [Registry JSON](features.json) là nguồn chuẩn được review bằng tay: **114 mục**,
  gồm **37 V1**, **75 later**, **2 drop_legacy**. Mọi mục đều đã có quyết định scope;
  trạng thái implementation/proof/review được cập nhật tại registry, xem [PROGRESS](../planning/PROGRESS.md).
- [CSV chức năng](features.csv) xuất từ registry: mỗi mục có ID, owner theo luồng,
  server responsibility, acceptance, open questions và dependency.
- [Đặc tả Release 1](../planning/SCOPE_V1.md) chốt tập nội dung, hành vi và gate phát hành.
- [46 nhóm bằng chứng](../evidence/inventory/kr_feature_candidates.csv),
  [quyết định theo nhóm](../evidence/inventory/kr_scope_groups.csv),
  [toàn bộ nhóm tài nguyên cấp đầu](../evidence/inventory/kr_asset_groups.csv).
- [Manifest client](../evidence/inventory/kr_feature_inventory.json): SHA-256 của CA.exe, cả 13 `.idx`
  và 13 `.idd`; 51.467 entry, mọi index đọc hết và offset/size nằm trong pack.
- [Nội dung V1](../evidence/inventory/kr_v1_content.json): 103 tài nguyên đã giải payload và fingerprint,
  map header/spawn/drop và danh sách entry bắt buộc. Chưa phải bundle đóng đủ dependency.

46 nhóm mở rộng từ 32 nhóm ban đầu, bổ sung exchange, cosmetics/emote, PC benefits,
DTT, custom maps, MonWars, Partners, options, kết quả và các nhóm nội dung chưa phân loại.
Các pattern khớp 25.828 đường dẫn, còn 25.639 đường dẫn chưa khớp. Đây là coverage bộ lọc tên,
**không phải phần trăm chức năng**. Nhóm có thể chồng lặp; một chức năng có thể có nhiều asset;
asset tồn tại không chứng minh đang được bật, reachable hoặc còn được server hỗ trợ.

## Phân biệt scope, bằng chứng và tiến độ

Catalog được nối trực tiếp với tài liệu điều hành:

```text
REWRITE_PLAN → SCOPE_V1 → FEATURE_CATALOG / features.json
                                   ↓ implementation + proof/review
                PROGRESS ← tools/kr_catalog.py ← execution.json (phase/gates)
```

[REWRITE_PLAN](../planning/REWRITE_PLAN.md) là kế hoạch thực hiện;
[PROGRESS](../planning/PROGRESS.md) cho biết thực tế đã nghiệm thu bao nhiêu so với catalog.
Không tính % từ số dòng mô tả, số asset hoặc số claim RE. Baseline hiện tại:
**V1 = done/37**, **toàn catalog = done/112**;2drop legacy hiển thị riêng, không tính hoàn thành.
Đây là tỷ lệ số mục; chưa có ước lượng effort thì không đổi thành % công sức/thời gian hoặc % giống KR.
Scope mới phải version hóa; membership/scope signature ngăn đổi mẫu số âm thầm trong báo cáo.

Ba trục độc lập:

- `release`: `V1`, `later`, `drop_legacy` là quyết định sản phẩm.
- `evidence_status`: `asset_signal` chỉ xác nhận tài nguyên phù hợp tên; `design_requirement`
  là yêu cầu engine/backend mới. Chưa nâng mục nào lên `runtime_confirmed_kr`.
- `implementation`: `not_started`, `in_progress`, `blocked`, `in_review`, `done`; không chuyển sang done
  chỉ vì có asset hoặc spec. Báo cáo đếm riêng các trạng thái, chỉ done mới có credit.

37 mục V1 được tách đủ để chia việc. Mỗi mục có `tracking`: assignee/reviewer, accepted baseline/spec,
integration commit, evidence, verification, review, blocker và next_action.
Done cần độc lập review approved, verification passed có command/result, commit đã nằm trong HEAD checkout tích hợp, evidence tồn tại,
không blocker và dependency done; validator kiểm tính nhất quán, không tự chứng minh acceptance đúng.
Phase và gate đọc riêng từ [execution.json](../planning/execution.json); hoàn thành plan/spec không tự hoàn thành feature.

Các mục later là backlog nghiên cứu có tên, nguồn và câu hỏi;
acceptance chi tiết của chúng phải được viết trước khi mở triển khai. Không coi suy luận về tên
SandBox/DIYBomb/Partners… là hành vi đã xác nhận. Hai mục bỏ tích hợp legacy là auth Nexon/NGM
và GreenPC legacy; auth/session/validation của dự án vẫn bắt buộc V1.

## Danh mục theo nhóm

`V1 subset` nghĩa là chỉ các hành vi được liệt kê trong registry thuộc V1, không phải toàn bộ nhóm.
Những item/effect thuộc Battle nhưng chưa hỗ trợ vẫn ở later, không được tự lọt vào random drop.

| Nhóm | Tên | Quyết định | Entry khớp |
|---|---|---|---:|
| KR-F001 | Đăng nhập/chọn server | V1 subset + backlog | 41 |
| KR-F002 | Sảnh/quảng trường | V1 subset + backlog | 1021 |
| KR-F003 | Ghép trận/phòng | V1 subset + backlog | 318 |
| KR-F004 | Trận Battle/bom | V1 subset + backlog | 82 |
| KR-F005 | Chọn nhân vật | V1 subset + backlog | 67 |
| KR-F006 | Shop | Later | 412 |
| KR-F007 | Hồ sơ/túi đồ | Later | 237 |
| KR-F008 | Chat/whisper | V1 subset + backlog | 17 |
| KR-F009 | Guild | Later | 26 |
| KR-F010 | Couple/bạn bè | Later | 51 |
| KR-F011 | Quest/nhiệm vụ ngày | Later | 81 |
| KR-F012 | CAPass | Later | 9 |
| KR-F013 | Ladder/rating | Later | 82 |
| KR-F014 | Magic Market | Later | 88 |
| KR-F015 | Câu cá | Later | 24 |
| KR-F016 | Pet | Later | 1356 |
| KR-F017 | Tutorial | Later | 74 |
| KR-F018 | HiddenCatch | Later | 303 |
| KR-F019 | AI Battle | Later | 11 |
| KR-F020 | Airplane | Later | 10 |
| KR-F021 | Baseball | Later | 37 |
| KR-F022 | BigBattle | Later | 47 |
| KR-F023 | RPG/CAHero | Later | 256 |
| KR-F024 | Monster/campaign | Later | 1801 |
| KR-F025 | SandBox | Later | 79 |
| KR-F026 | DIYBomb | Later | 9 |
| KR-F027 | ConfusedFight | Later | 50 |
| KR-F028 | IamBoss | Later | 4 |
| KR-F029 | SnowyChamp/Antarctic | Later | 21 |
| KR-F030 | MineExplore/IamWriggle | Later | 15 |
| KR-F031 | ItemCollection | Later | 27 |
| KR-F032 | Event/reward | Later | 54 |
| KR-F033 | Trang phục/trang trí | Later | 17469 |
| KR-F034 | Emote/fanfare | Later | 111 |
| KR-F035 | Exchange | Later | 13 |
| KR-F036 | DTT/season shop | Later | 43 |
| KR-F037 | CustomMap/TileCustom | Later | 281 |
| KR-F038 | MonWars | Later | 7 |
| KR-F039 | Partners | Later | 27 |
| KR-F040 | GreenPC | Drop legacy | 10 |
| KR-F041 | PC-room benefits | Later | 16 |
| KR-F042 | MissionPong/CAMission/GrowSprout | Later | 73 |
| KR-F043 | Tùy chọn/help/notice | V1 subset + backlog | 81 |
| KR-F044 | Kết quả/vòng đời trận | V1 subset + backlog | 100 |
| KR-F045 | Mount/trạng thái đặc biệt | Later | 848 |
| KR-F046 | Nội dung chiến dịch khác | Later | 70 |

Các yêu cầu nền tảng không có group KR (protocol mới, authority, replay, lưu tài khoản, mạng,
build/QA) vẫn nằm trong registry V1 dưới nhãn `design_requirement`.

## Quy trình bổ sung bằng chứng

Đọc UI/chuỗi → truy class/handler/consumer → ghi state/input/output → runtime nếu chạy được.
Ghi nguồn và trạng thái theo AGENTS §4: VN và KR ngang nhau (ADR 0002); archive CN/TW, web và OSS là candidate.
Các chức năng chưa tìm ra phải có mục mới với ID ổn định; baseline scope hiện tại không được hiểu
là chứng minh client không còn chức năng khác. Phát hiện mới cập nhật backlog, không tự mở rộng V1.

Các câu hỏi then chốt V1: timing/speed/hitbox/cornering, bomb/chain/trap,
map object/collision và dependency rendering. Xem gate có owner trong Release 1.

## Tái tạo và kiểm tra

```bash
# Tool đọc pack ở repo l10n (ADR 0003); ghi kết quả vào repo này
cd /home/admin/boom-re-l10n
python3 tools/kr_feature_inventory.py /home/admin/boom-re/boomKR --out /home/admin/boom-rs/docs/evidence/inventory --hash-payloads
python3 tools/kr_v1_content.py /home/admin/boom-re/boomKR /home/admin/boom-rs/docs/evidence/inventory/kr_v1_content.json
cd /home/admin/boom-rs
python3 tools/kr_catalog.py
python3 tools/kr_catalog.py --check
```

Inventory tool chỉ ghi dữ liệu quét; không sửa `docs/specification/features.json` hoặc tự đổi scope.
CSV chức năng và quyết định nhóm là export; sửa JSON rồi xuất lại.
Validator kiểm unique ID, đầy đủ disposition cho46nhóm, dependency tồn tại/không cycle,
V1 không phụ thuộc later, proof/review/status nhất quán, baseline mẫu số và export/progress không stale.
Tool cũng sinh trạng thái phase trong REWRITE_PLAN từ execution.json. Không sửa khối GENERATED/PHASE_STATUS tay.
Nó không xác nhận luật KR, nội dung proof hoặc gameplay parity.

Khảo sát lần này dùng parser/file trực tiếp và đối chiếu docs; không chạy client hoặc decompile
mới. Header map KR dài hơn VN 4 byte (spawn +80 so với +76), xem RULES_V1 R-003. `docs/research/findings/VN_FEATURES_REFERENCE.md` vẫn là checklist VN độc lập.

Workflow nhóm: [TEAM_WORKFLOW](../planning/TEAM_WORKFLOW.md).
