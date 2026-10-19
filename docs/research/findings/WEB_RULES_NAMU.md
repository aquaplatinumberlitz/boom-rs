# WEB-RULES — luật chơi & thông số trích từ namu (mirror), build KR cuối

**Nguồn**: `namu.moe` (mirror truy cập được của `namu.wiki`; namu.wiki trả Cloudflare 403 cho tool).
**Ngày truy cập**: 2026-10-19. **License**: CC BY-NC-SA 2.0 KR — nội bộ, không phát hành lại.
**Bản cào đầy đủ**: `docs/research/sources/raw-namu/*.md` (gitignored, chỉ tồn tại trên máy).
**Source ID**: WEB-KR-002 … WEB-KR-008 trong [register nguồn](../sources/README.md).

**Trạng thái bằng chứng**: wiki cộng đồng, **không phải tài liệu Nexon**; phản ánh build KR đóng cửa
2026-08-13. Mọi claim dưới đây là `candidate`/`unverified` cho target `00967`, phải đối chiếu pack
(`data/kr/`) hoặc đo lại trước khi thành rule production. Không thay thế spec.

Liên quan: Q-004 (tick/numeric), Q-005 (movement/stats — đặc biệt bảng speed-level),
Q-007 (drop/pickup — trọng số ≠ %). Style chơi/giá trị thị trường **không** đưa vào V1.

## 1. Item trong trận (`namu-items.md`)

### 1.1 Tăng chỉ số (2.1)

| Item | Hàn | Tác dụng / con số |
|---|---|---|
| Bubble | 물풍선 | **+1** số bóng đặt được |
| Fluid | 액체 | **+1 ô** tầm nổ |
| Ultra | 울트라 물풍선 | tầm nổ → **max của nhân vật** ngay lập tức (có map thành item bẫy) |
| Roller | 롤러 스케이트 | **+1** tốc độ |
| Red Devil | 붉은 악마 | tốc độ → **max** + **đẩy được bóng** |
| Skate | 스케이트 | map băng: tốc độ → max (thay Red Devil) |
| Glove | 장갑 | **ném** bóng đi xa |
| Shoes | 신발 | **đẩy** bóng (mạnh ở monster mode) |
| Superman | 슈퍼맨 | mọi chỉ số → max **tạm thời**; **không bất tử**; trúng nước thì mất |

### 1.2 Giảm chỉ số / bẫy (2.2–2.3)

| Item | Hàn | Tác dụng |
|---|---|---|
| Green Devil | 초록 악마 | **nhả** 1 item đã ăn → giảm 1 bậc chỉ số tương ứng |
| Push-Pin | 압정 | bóng **chạm là nổ ngay** |
| Banana | 바나나 껍질 | trượt tới vật cản; đặt **1 / 3 / 5** quả |
| Trap | 트랩 | đặt giữa 2 block; ai đi qua → bị nhốt; **1 item = 2 trap**; nước không phá được |
| Bond | 본드 | giảm tốc rất mạnh (chậm hơn rùa); nước rửa trôi |
| Devil | 마귀 | đảo hướng điều khiển / tự đặt bóng; 물약 giải |

### 1.3 Tiện ích (2.4)

| Item | Hàn | Tác dụng |
|---|---|---|
| Ghost | 유령 | tàng hình tạm thời (đồng đội thấy mờ) |
| Disguise | 위장도구 | đổi sang màu đội bạn |
| Drill | 드릴 | phá block trước mặt; **3 lần** |
| **Oxygen** | 산소통 | **kéo dài thời gian bị nhốt trước khi chết**; vô hiệu hoá kim |
| Motor | 모터 | xe goòng nhanh trên mọi địa hình |
| Moonwalk | 문워크 | đi **lùi** |
| Golden Devil | 금빛 악마 | (협공배틀) tốc độ địch → **1** |

### 1.4 Phương tiện (2.5) — **cùng thang tốc độ với nhân vật**

| Phương tiện | Tốc độ | Ghi chú |
|---|---|---|
| 거북이 (rùa) | **1** | chậm nhất; 통조림 → 해적 거북이 |
| 해적 거북이 (rùa cướp biển) | **9** | |
| 부엉이 (cú mèo) | **5** | không bao giờ "quay ra đồ hỏng" |
| 우주선 (UFO) | **10** | mạnh nhất; **không nhặt được item**; qua vật cản (trừ block không phá được) |
| 고장 난 우주선 | **1** | 스패너 sửa thành UFO |
| 문어 / 해적 문어 | (map biển) | thay rùa; 통조림 → 해적 |
| 광석 운반차 | **10** trên ray / **2** ngoài ray | 모터 → luôn 10 |
| 탱크 (xe tăng) | **5** | bắn bóng theo hướng, số bóng = max nhân vật |
| 조이스틱 (kart) | **10** khi xăng ≥1/3 · **4** khi cạn · **11.5** khi nitro | 붐힐오일 nạp |
| 치킨통 | = **10** | như UFO nhưng không hỏng |
| 상어 (cá mập) | — | nuốt **3** bóng, nhả **2** bóng/lần |

### 1.5 Item cash / lượt dùng (2.6–2.7)

| Item | Hàn | Tác dụng / số |
|---|---|---|
| Needle | 바늘 | **Ctrl** thoát bong bóng — item quan trọng nhất |
| Couple Needle | 커플바늘 | cứu 크앤 (partner) |
| 통조림 | — | rùa/mực → bản cướp biển (speed 9) |
| 무선 물폭탄 | — | đặt bom rồi **Space** để nổ; ăn 1 item = 3 quả |
| 다트 | — | bắn xa, nổ bóng ngay |
| 스프링 | — | nhảy qua vật cản, tối đa **3 ô** |
| 파워산삼 | — | buff **15 giây** (không phải max) |
| 슈퍼 만년삼 | — | như trên **30 giây** |
| 실드 | — | **bất tử theo thời gian**; trúng nước giảm thời gian |
| 센서 | — | hiện vùng nước sắp nổ |
| 투시경 | — | xem item trong hộp, xa **2 ô** |
| 엔젤코인 | — | hồi sinh người **chết sau cùng** |
| 위장 도치 | — | hoá nhím (mode 고슴도치) |

## 2. Luật & thời gian (`namu-modes.md`)

### 2.1 Bị nhốt trong bong bóng (cơ chế, **không có số giây**)

- Bị nước trúng → nhốt trong bong bóng, **không đặt được bóng**, **đi rất chậm** (bản rất cũ: không đi được).
- Chạm **đồng đội** → thoát; chạm **địch** → bong bóng nổ, chết.
- Không ai chạm: bong bóng **mờ dần**; khi **không còn thấy nhân vật** → nổ → chết.
- **바늘** (Ctrl) thoát ngay; **산소통** kéo dài thời gian (nhưng vô hiệu kim).
- Thời gian tới lúc mờ **gần như giống nhau mọi nhân vật**, **trừ 로두마니 ngắn hơn** (setting sợ nước).

### 2.2 Bóng nổ dây chuyền

Bóng đặt sau, nếu bị **tia nước** của bóng trước chạm → **nổ theo**.

### 2.3 Phòng / bắt đầu

| Việc | Số |
|---|---|
| Mọi người ready mà host không start | **10 giây** → cảnh báo tự kick (슈퍼방장: **30 giây**) |
| Manner room | đội phải **cân số** (2:2, 3:3, 4:4…); Free room: không cần |
| Số người | tối đa **8** |

### 2.4 Game type (2.4)

| Type | Luật / số |
|---|---|
| 부활대전 (respawn) | chết → hồi sinh sau **3 giây** ở vị trí ngẫu nhiên; địch +1 điểm |
| 고슴도치 (nhím) | bị sặc → thành nhím: phá bóng khi chạm, không bị nhốt, **không di chuyển một lúc**, cứu được người trong bóng, không dùng bom |
| 대장잡기 (captain) | mỗi đội 1 người mang cờ; captain chết → **cả đội chết** |
| 시한폭탄 (time bomb) | người mang bom: tốc độ **cố định max**, có **5 giây** để chạm người khác; hết giờ → nổ |
| 팝케이드 (2024) | 1–4 người, bắn hết "bắp rang" sinh ra từ xô; 15 map chia 1/2/4 người |

### 2.5 Mode khác

- 협공배틀 (co-op battle), 혼자놀기 (solo), 눈싸움 / 슈퍼파이터 (event quá khứ).

## 3. Monster mode (`namu-monster.md`)

- **8 bộ map**: 문어대습격 · 황제의귀환 · 물개의분노 · 로봇대공습 · 거북대격돌 · 골렘대침공 · 크아시티 · 스페셜몬스터 (+ 시즌2: 불의 제왕, 샌드킹, 설원의지배자).
- **Boss**: 대왕문어 (mực) · 황제펭귄 · 황제물개 · 라이트닝킹 (robot) · 대왕거북 · 거대골렘.
- Độ khó boss: **대왕거북 < 대왕문어 < 거대골렘 < 황제펭귄 < 황제물개 < 라이트닝킹**.
- Mỗi map 3 stage (2 stage thường + boss); có map 2 stage cũng có boss.
- **Điểm SS** = thời gian + số kill, ví dụ:
  - 문어대습격 09: **130 s / 11 kill**; 물개의분노 05: **80 s / 8 kill**;
  - 골렘대침공 09: **61 s / 16 kill** (khó nhất); 거북대격돌 01: **65 s / 32 kill**.
- **Điểm kill**: 엔젤코인 **+3**, boss **+5**, chết **−2**.
- Cơ chế map: đá bóng (걸치기) để bắn xuyên, xe goòng/ray, băng trượt, giếng nước, bẫy gai, 유령, boss triệu hồi.
- **Kim/실드는 bắt buộc** ở nhiều map (가시); một số map không cho item.

## 4. EXP 720 cấp (`namu-exp.md` + `data/kr/exp_table.json`)

- Max level **720**; tổng EXP tới 720 = **78.000.000**; EXP lưu tối đa **2.147.483.647**.
- **Hell level mỗi 15 cấp** (3 cấp có 준헬렙); từ 칭호 14 (Master-Lord) chỉ còn hell ở 금메달 + đỏ + Wave.
- 16 칭호: Beginner → Veteran → Expert → Elite → Master → Champion → Hero → King → Lord → God → Star →
  Master-Hero → Master-King → Master-Lord → Master-God → Angel-Green.
- Ví dụ EXP cần: L2 **400** · L10 **2.500** · L50 **4.600** · L100 **8.600** · L225 **77.200** (hell) ·
  L585 **682.000** (hell lớn nhất) · L719→720 **600.000**.
- Bảng đầy đủ 720 dòng: `data/kr/exp_table.json` (parse từ trang này).
- **Ladder**: 1 bậc = **60 điểm**.

## 5. Bản đồ (`namu-maps.md`)

Theme xuất hiện: 빌리지 · 캠프 · 포레스트 · 패트릿 · 무덤 · 공동묘지 · 아쿠아 · 광산 · 사막 · 빙하/설원 ·
팩토리 · 바다 · 켄터키 · 카트 · 좀비 · 클로버 · 바니팜 · 꼬물이 · 크아퐁 · 크아시티.
Một số map có tên riêng nổi tiếng: 캠프 08 · 팩토리 07 · 포레스트 07 · 패트릿 14 · 무덤 12 · 빌리지 10 · 문어대습격 09.

## 6. Chiến thuật → suy ra cơ chế vật lý (`namu-strategy.md`)

- **걸치기 (đứng sát mép)**: đặt bóng lệch nửa ô để tia nước chỉ phủ 1 phía — engine phải cho đặt bóng
  khi nhân vật **lệch tâm ô**, không ép snap tuyệt đối.
- **연사 (bắn liên tiếp)**: đặt nhiều bóng nhanh khi có nhiều bóng — cần hàng đợi đặt bóng + hồi chiêu rất ngắn.
- **일자 (đường thẳng)**: tận dụng tia nước dài theo hàng.
- Kiểu chơi theo item: **노샵** (không item shop) · **기샵** (chỉ item cơ bản) · **풀샵** (full) · **올노**.
- **몰아잡기**: dùng 파워산삼 (15 s) trong **15 giây cuối** trận để bắt hàng loạt.
- Trong monster mode: 신발/붉은악마 + 걸치기 để giết boss nhanh; một số map boss chết trong **~10–30 giây**.

## 7. Vẫn chưa có số (phải đo hoặc RE)

| Thiếu | Tình trạng |
|---|---|
| Thời gian bị nhốt tới khi chết (giây) | mọi nguồn chỉ mô tả cơ chế |
| Quy đổi cấp tốc độ (1…11.5) → ô/giây | chỉ có thang tương đối |
| Fuse bóng đặt thường | ja.wikipedia nói bóng **ném** nổ sau 3 s; đặt thường chưa rõ |
| Công thức Magic Market (KR-only) | không có trên wiki/pack |
| Tỉ lệ rơi item theo từng map | web không có; pack có `ItemAttr` weight + `prob%` |
