# Evidence — dữ liệu có thể đối chiếu

| Artifact | Nội dung và giới hạn |
|---|---|
| [kr_feature_inventory.json](inventory/kr_feature_inventory.json) | CA.exe + 13 IDX/IDD hash, 51.467 entry; không xác nhận gameplay |
| [kr_feature_candidates.csv](inventory/kr_feature_candidates.csv) | 46 nhóm matcher, số khớp và ví dụ; asset signal, không runtime |
| [kr_asset_groups.csv](inventory/kr_asset_groups.csv) | Toàn bộ nhóm tên tài nguyên cấp đầu |
| [kr_scope_groups.csv](inventory/kr_scope_groups.csv) | Export disposition của 46 nhóm từ registry nguồn |
| [kr_v1_content.json](inventory/kr_v1_content.json) | 103 payload được fingerprint và probe Village00; chưa đóng đủ dependency |
| `images/vn/` | Ảnh client VN được chuyển nguyên file từ docs root |
| [measurements/vn-timing-2026-10-19/README.md](measurements/vn-timing-2026-10-19/README.md) | Hồ sơ đo tick/tốc độ/timer nhốt client VN; log trích + script phân tích, không có asset game |
| [measurements/kr-static-trapped-2026-10-21/README.md](measurements/kr-static-trapped-2026-10-21/README.md) | Định danh build + bảng VA/RVA + evidence_id của phép đọc tĩnh trapped KR (R-126…R-131); metadata, không có binary hay decompile |
| [measurements/kr-static-flame-2026-10-21/README.md](measurements/kr-static-flame-2026-10-21/README.md) | Điều tra nổ/lửa + thời lượng frame KR: mỏ neo, evidence_id, giới hạn công cụ; **chưa có claim**, dùng cho lượt sau |
| [measurements/kr-static-2026-10-21/README.md](measurements/kr-static-2026-10-21/README.md) | Định danh build KR/VN và bảng VA/RVA của phép đọc tĩnh movement/va chạm (R-119…R-125); metadata, không có binary hay decompile. Phạm vi lớp: consumer đã trace là `CGOPet`/AI (R-134/R-135); R-119…R-122 `confirmed` chỉ cho `CMovable`, bomber là `candidate`/chưa chứng minh |
| [measurements/kr-static-item-2026-10-21/README.md](measurements/kr-static-item-2026-10-21/README.md) | Điều tra item/cap KR: bộ nạp `ItemAttr` + cấu trúc bảng, provenance IDA trực tiếp; **chưa có claim** |
| [measurements/kr-static-trapped-speed-2026-10-22/README.md](measurements/kr-static-trapped-speed-2026-10-22/README.md) | Truy vết `(dir,len)` khi bị nhốt: `CGOPet`; R-136 dispatch dir→sub-object `+1412`→reader; R-137 direct caller setter là wrapper NPC; `B479E0` lớp sử dụng `unknown`; quét 5 site slot 95 + packet VN 176/KR 183 dùng slot `+116`, input dir→position player `unknown`; R-138 packet KR 183→`CGOGhost`→`0xB334C0` (40 px), C→S 148/149 candidate; bảng kiểm, `evidence_id`, giới hạn quét cục bộ |
| [measurements/kr-static-flame-chain-2026-10-22/README.md](measurements/kr-static-flame-chain-2026-10-22/README.md) | Đọc tĩnh nổ lan/chặn/dây chuyền KR: `0x00E9C520`/`0x00E9CDC0`, message id 27 cho từng ô, thời gian lửa = số frame `GetAnimCount`; claim R-139…R-141; bảng VA/RVA, `evidence_id`, giới hạn (chưa có handler gói nổ/damage window) |
| [DOCS_MIGRATION.json](DOCS_MIGRATION.json) | Bản đồ đường dẫn cũ→mới, không phải bằng chứng luật chơi |

Không commit client/pack/binary/database hoặc cache ảnh lớn vào repo chỉ để tạo evidence.
Trace/capture mới phải có build/hash, điều kiện đầu, cấu hình server giả lập và giới hạn phép đo.
Tách dữ liệu nguồn, phân tích và kết luận; fixture sinh từ implementation không được gọi là oracle gốc.

Tái tạo từ gốc worktree:

```bash
# Tool đọc pack ở repo l10n (ADR 0003); ghi kết quả vào repo này
cd /home/admin/boom-re-l10n
python3 tools/kr_feature_inventory.py /home/admin/boom-re/boomKR --out /home/admin/boom-rs/docs/evidence/inventory --hash-payloads
python3 tools/kr_v1_content.py /home/admin/boom-re/boomKR /home/admin/boom-rs/docs/evidence/inventory/kr_v1_content.json
cd /home/admin/boom-rs
python3 tools/kr_catalog.py
python3 tools/kr_catalog.py --check
```

Quét inventory không ghi đè scope nguồn ở `docs/specification/features.json`.
