# boom-rs — engine Boom KR viết lại bằng Rust

Repo tích hợp của dự án: tài liệu điều hành, catalog/scope và Cargo workspace
([ADR 0003](docs/decisions/0003-engine-repo-and-docs-home.md)).
Bắt đầu ở [AGENTS.md](AGENTS.md), rồi [docs/README.md](docs/README.md).

## Lệnh thường dùng

```bash
cargo xtask setup                   # một lần: bật hook pre-push chạy ci
cargo xtask ci                      # cổng bàn giao: fmt, clippy, test, replay golden, docs/catalog
cargo xtask bless                   # ghi lại golden replay sau khi đổi hành vi core có chủ đích
SCENARIO=explosion cargo test -p boom-scenario -- --nocapture   # chạy scenario của một system
cargo run -p boom-replay -- run fixtures/replays/walk-two-players.toml
cargo run -p boom-assets -- map-info <Village00.map> --build kr
python3 tools/kr_catalog.py         # sinh lại CSV/PROGRESS từ features.json + execution.json
```

Test đọc map thật cần dữ liệu game ngoài repo: đặt `BOOM_KR_VILLAGE00_MAP` và `BOOM_VN_VILLAGE00_MAP`
trỏ tới file `.map` đã giải nén; thiếu thì test tự bỏ qua.

## Cấu trúc

| Đường dẫn | Nội dung |
|---|---|
| `crates/boom-core` | Mô phỏng thuần, xác định: `Mode`, `System`, `SimulationEvent`, state hash |
| `crates/boom-content` | Rules profile có provenance + overlay, map runtime |
| `crates/boom-assets` | Đọc định dạng legacy (`.map` KR/VN) |
| `crates/boom-replay` | Chạy/kiểm replay so golden |
| `crates/boom-scenario` | Định dạng dựng trận chung + harness boundary scenario |
| `crates/boom-room` | Máy trạng thái phòng thuần (chờ task backend) |
| `xtask` | Tác vụ repo |
| `data/rules` | Rules profile (giá trị luật kèm claim) |
| `fixtures` | Profile cố định, replay golden, scenario theo system |
| `docs` | Plan, scope, kiến trúc, ADR, catalog, research |

Docs được chuyển từ `/home/admin/boom-re-l10n` (commit `9bde2ceacd82cd542f871fcd42f18d29e6e5705d`);
lịch sử trước đó tra ở repo nguồn.
