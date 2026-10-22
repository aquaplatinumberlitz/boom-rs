# Đo tĩnh KR 00967 — consumer âm thanh theo event (2026-10-23)

Metadata cho lượt điều tra [#196](https://github.com/aquaplatinumberlitz/boom-rs/issues/196) / Q-015,
V1-034 (mục 8 hàng đợi RE: event→entry, biến thể, BGM Village, chồng tiếng). Claim tương ứng:
[RULES_V1 § (đánh số khi merge)](../../../specification/gameplay/RULES_V1.md#-đánh-số-khi-merge--re-kr-00967--âm-thanh-theo-event-r-203r-208)
(R-203…R-208); suy luận, đề xuất AUDIO_V1 và giới hạn ở [KR_RE_AUDIO](../../../research/findings/KR_RE_AUDIO.md).

## Định danh build

| Build | File | Byte | SHA-256 | Image base | RVA |
|---|---|---|---|---|---|
| KR `00967` (module đã unpack) | `kr-full.exe` (bản sao `.re-tmp/kr-full.exe`) | 26.074.624 | `cf72da112308d9f1ca256df8089594c5f787875310933073bf6bd5ba6f70e63b` | `0x00400000` | `VA − 0x00400000` |

Không commit binary/DB/âm thanh. Bản sao ngoài repo trong `.re-tmp/` (đã `git/info/exclude`).

## Công cụ và provenance

| Bước | Tool | Ghi chú |
|---|---|---|
| Bind + decompile | REA `open_binary` `provider_id: "ida"` trên bản sao | REA spawn worker mới; lần gọi đầu trong phiên lỗi schema `data/module must be string` (upstream ida-pro-mcp), đóng/mở lại binary thì thành công — không dùng kết quả lần lỗi |
| Bảng id→tên | IDA MCP trực tiếp trên bản sao `.i64` | parse `0x41C980` (mỗi entry 0x20 B, id tại `base+0x20n`, tên do `push "Wave/..."` ngay sau) |
| Quét site gọi | IDA MCP | 597 xref tới `0xD1CD10`, lọc `push imm` làm đối số id |
| Vtable/RTTI | IDA MCP | slot 146 `CGOBomber` `0x1744964` = `0xB53C80`; `B8B680` trong vtable `0x1746AAC` (lớp item) |
| Chạy client/âm thanh | không | chỉ tĩnh/offline |

## Bằng chứng REA trong phiên

| `evidence_id` | operation | Đối tượng | Dùng cho |
|---|---|---|---|
| `ev_794131143dc920b7c1363d664cf204769039c306ccee15ee8c23cc9c64f6b2d6` | `procedure_pseudo_code` | `0xD1CD10` | R-203/R-205 (dispatcher, random variant, dedup 90/200 ms, 7↔8) |
| `ev_003bb7d0c32710a4ad1a34941670471050fd4d2288da48f8b1e3e4a4b3a03fa9` | `procedure_pseudo_code` | `0x14BF4D0` | R-207 (pool voice, dedup theo id, bỏ khi hết) |
| `ev_6ac4a4d1ed4b0d5302a30c6a79e93cb7a7e92dcbe6e710a0f4d576cbfaa4a443` | `procedure_pseudo_code` | `0xEB3720` | R-204 (nổ: 7 / 343 tuyết) |
| `ev_8cb8b610131ce11c72b663eac6b01223d924d36185e7548015d1256e0b9e0fa7` | `procedure_pseudo_code` | `0xB53C80` | R-204 (chết: 42) |
| `ev_e53b55a542c18b49046fa0120ccb757c411f9944feea9da58b5dad6ca3b74026` | `procedure_pseudo_code` | `0xE71D70` | R-206 (BGM: `Mp3/%s`, record map, Xmas Carol random) |

Bundle: `.re-tmp/evidence-audio-196.json` (5 record, không commit).

## Mỏ neo chính (VA → RVA)

| Vai trò | VA | RVA |
|---|---|---|
| Bảng SFX 649 entry (0x20 B) | `0x19E40D8` | `0x15A40D8` |
| Khởi tạo bảng SFX | `0x41C980` | `0x1C980` |
| Manager singleton `CSoundManRenew` | `0x19D8A88` / getter `0x54EAF0` | `0x15D8A88` / `0x14EAF0` |
| Dispatcher phát SFX | `0xD1CD10` | `0x91CD10` |
| Play thấp (pool voice) | `0x14BF4D0` | `0x10BF4D0` |
| Đếm voice cùng id/age | `0x14BEA70` | `0x10BEA70` |
| RNG kiểu MSVC | `0x15625C2` | `0x11625C2` |
| Nổ: chọn 7/343 | `0xEB3720` | `0xAB3720` |
| Lọc theo khung nhìn trước khi phát nổ | `0x114A1B0` | `0xD4A1B0` |
| Đặt bom: 9/344 | `0xAF1140`; `0x71F490` | `0x6F1140`; `0x31F490` |
| Domino 10 | `0x8326F0` | `0x4326F0` |
| Nhốt: 444/37/77 | `0xB42D60` | `0x742D60` |
| Chết 42 | `0xB53C80` | `0x753C80` |
| Item ăn 68 / lộ 69 / trap 174 | `0xB4B410` / `0xB8B680` / `0xB9DEF0` | `0x74B410` / `0x78B680` / `0x79DEF0` |
| Bảng BGM 117 entry | `0x19E9258` | `0x15A9258` |
| `Mp3/Village` (id 48) | chuỗi `0x1761FE8`; entry `0x19E9858` | `0x1361FE8` |
| Phát BGM | `0xE71D70` → `0xD1D710` | `0xA71D70` → `0x91D710` |
| Tra record map / tên BGM | `0xE04F00` / `0xE02110` | `0xA04F00` / `0xA02110` |

## Bảng id → entry (trích; 649 entry)

| Id | Entry | Id | Entry |
|---:|---|---:|---|
| 7 | `Wave/Cha_BombExplode` | 68 | `Wave/Itm_ItemEaten` |
| 8 | `Wave/Cha_BombExplodeMulti` | 69 | `Wave/Itm_ItemPop` |
| 9 | `Wave/Cha_BombIgnite` | 77 | `Wave/Cha_LockedIn` |
| 10 | `Wave/Cha_DominoBombIgnite` | 78 | `Wave/Ntc_Lose` |
| 37 | `Wave/Cha_CoupleLockedIn` | 164 | `Wave/Ntc_TimebombExplode` |
| 42 | `Wave/Cha_Die` | 174 | `Wave/Itm_UseTrap` |
| 43 | `Wave/Ntc_Draw` | 175 | `Wave/Ntc_Win` |
| 47 | `Wave/Ntc_GameEnd` | 343 | `Wave/Cha_BombExplode_SnowBall` |
| 444 | `Wave/Ntc_Help2` | 344 | `Wave/Cha_BombIgnite_SnowBall` |
| 471 | `Wave/BombExplosion001` | 428..457 | `Ntc_Draw1..2`, `GameEnd1..6`, `GameReady1..4`, `GameStart1..4`, `HurryUp1..2`, `Lose1..6`, `Win1..5` |

## Biến thể random trong `0xD1CD10`

`v20 = rand()` (`0x15625C2`: `state = 214013*state + 2531011`, trả `(state>>16)&0x7FFF`), rồi
`id = v20 % N + base`:

| Id gốc | N | base | Dải |
|---:|---:|---:|---|
| 18, 128 | 4 | 436 | 436..439 |
| 21 | 4 | 440 | 440..443 |
| 43 | 2 | 428 | 428..429 |
| 47 | 6 | 430 | 430..435 |
| 64 | 2 | 445 | 445..446 |
| 78 | 6 | 447 | 447..452 |
| 175 | 5 | 453 | 453..457 |

Nổ 7: `count(7,200ms) > 0` → stop 7+8, play 8; ngược lại `count(8,200ms) != 0` → play 7; còn lại play 8.

## Voice

`0x14BF4D0`: mảng `manager+16`, count `manager+12`; `a3 != 0` + cùng id → trả handle cũ (không phát
thêm); tìm voice `state == 2` (rảnh); không có → `return 0` (bỏ). Volume: `v27 * *(float*)(manager+48) / const`.
Số voice của pool (hằng khởi tạo) chưa định vị — không claim "8".

## Giới hạn

- Chỉ tĩnh; không chạy client/không nghe, không có packet capture.
- Bảng id→entry parse từ bản dump đã init; 649 entry, id 0..651; có id trống.
- Chưa đọc record map trong pack để chốt map Village → tên BGM (R-081 phủ entry/hash).
- "Không có consumer" cho 471 và gạch vỡ là bounded negative trong 649 entry + 597 site `0xD1CD10`;
  đường script `PlayWaveEffect` theo tên chưa bị loại trừ.
- Không sửa `data/assets/audio-v1.toml` hay bảng AUDIO_V1; đề xuất ở findings §7.
