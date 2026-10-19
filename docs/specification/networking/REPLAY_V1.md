# Replay V1 — header, content hash và golden

Tài liệu task **V1-026 phần 1 / issue #70**. `boom-replay` chạy replay xác định theo tick; golden là
**regression baseline**, không phải oracle client gốc (AGENTS §4). Phần ghi replay từ trận
`boom-server` thật thuộc V1-026 phần 2 (sau #60).

## 1. Header bắt buộc

File replay là TOML, dùng các khóa dựng trận của `boom_scenario::setup` cộng:

| Khóa | Ý nghĩa | Kiểm |
|---|---|---|
| `schema` | Revision file replay; hiện `1` | Khác → lỗi, exit ≠ 0 |
| `hash_algo` | Thuật toán state hash; hiện `fnv1a64-v0` | Khác → lỗi, exit ≠ 0 |
| `profile_hash` | Định danh rules profile (`0x…`, FNV-1a 64) | `load_profile` tính lại; khác → lỗi |
| `content_sha256` | SHA-256 hex của **Terrain đã phân loại** | `run` tính lại; khác → lỗi |
| `ticks`, `checkpoints` | Số tick chạy và các tick ghi state hash | Golden so toàn văn |

`rules` là bắt buộc. Map có đúng một trong hai dạng:

- `map = [...]` — ASCII fixture (`#` cứng, `+` mềm, `.` nền, chữ số spawn).
- `[map_pack]` — map thật từ pack legacy:
  ```toml
  [map_pack]
  env = "BOOM_KR_PACK"                 # biến môi trường trỏ thư mục chứa FxMap.idx/.idd
  entry = "Map/Village00"              # mặc định Map/Village00
  build = "kr"                         # kr | vn
  profile = "../../data/maps/village00-kr.toml"
  ```
  `boom-replay` đọc entry qua `boom-assets`, parse `.map`, rồi phân loại ô qua `boom-content`
  (`MapProfile`). `boom-scenario` chỉ nhận `Terrain` đã dựng, không mang decoder pack.

## 2. Content hash

`content_sha256` băm encoding chuẩn của `Terrain` sau phân loại: tag `boom-terrain-v1`, width/height,
số ô, từng `CellKind` (Floor/Solid/Breakable/Unknown), số spawn và tọa độ spawn. Vì vậy:

- ASCII và map pack cho cùng lưới/spawn/loại ô → cùng hash;
- đổi bất kỳ ô, spawn, kích thước hoặc cách phân loại → hash khác;
- hash không phụ thuộc đường dẫn pack, thứ tự file hay byte gốc `.map`.

Tính hash khi viết replay:

```sh
cargo run -p boom-replay -- content-hash fixtures/replays/full-ffa.toml
```

## 3. Pack và CI

CI không có dữ liệu game. Nếu `[map_pack]` có nhưng biến `env` chưa đặt (hoặc rỗng),
`run`/`verify`/`bless` **bỏ qua** với thông báo ở stderr và exit 0. Nếu biến có nhưng pack, entry
hoặc profile hỏng thì là lỗi (exit ≠ 0), không che cấu hình sai.

Chạy tay:

```sh
BOOM_KR_PACK=/home/admin/boomkr-full \
  cargo run -p boom-replay -- verify fixtures/replays/village00-pack.toml
```

## 4. Golden hiện có

| File | Nội dung | CI |
|---|---|---|
| `walk-two-players.toml` | Đi/vòng, hết giờ hòa | Chạy |
| `full-ffa.toml` | Đặt bom, chain, phá 3 khối + drop, nhặt Bomb/Fire/Roller, bị nhốt, chạm làm chết, thắng `winner:0` | Chạy |
| `village00-pack.toml` | Village00 KR từ pack, 2 người, 10 tick | Bỏ qua nếu thiếu `BOOM_KR_PACK` |

`full-ffa` dùng prestate có chú thích trong file để chạm đủ system; input viết tay theo ý định.
Golden chỉ khóa regression: đổi hành vi core có chủ đích phải chạy `cargo xtask bless` và giải thích
trong PR, không coi golden là bằng chứng fidelity KR.

## 5. Từ chối

Các ca sau trả lỗi rõ và exit ≠ 0 (test tự động ở `crates/boom-replay/tests/reject.rs`):

- `schema` khác `1`;
- `hash_algo` khác `fnv1a64-v0`;
- `content_sha256` tính lại khác;
- `profile_hash`/rules khác (cùng cơ chế `profile_hash` cũ).

## 6. Chưa làm

- Chưa ghi replay từ `boom-server`/trận thật; chưa có manifest content bundle (V1-026 phần 2).
- Chưa replay snapshot/delta; replay vẫn dựng trận từ setup + input theo tick.
- Chưa chốt hash cho toàn bundle (chỉ map Terrain); `content_sha256` sẽ mở rộng khi có bundle.
- `boom-replay` dùng `boom-assets` để đọc pack; edge dependency này cần integrator thêm vào
  ARCHITECTURE §1 khi merge (file ARCHITECTURE ngoài scope task).
