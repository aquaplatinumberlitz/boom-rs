> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Inventory artefacts & bảo quản (PRES-001 / PRES-002 / PRES-003)

Tác giả: dsh · 2026-10-18 · theo `docs/archive/plans/MASTER_PLAN_2026-10-18.md` §6.1 và §12.
Tag bằng chứng theo `MASTER_PLAN` §7.

## 1. PRES-003 — IDA DB: **đã backup, nhưng vẫn cần chủ DB lưu**

### Phát hiện **[VERIFIED]**

| File | mtime | size |
|---|---|---|
| `unpacked_ca.exe.i64` | 2026-10-18 **12:10** | 312.377.024 |
| `unpacked_ca.exe.id0` | 2026-10-18 **20:45** | 306.888.704 |
| `unpacked_ca.exe.nam` | 2026-10-18 **20:45** | 499.712 |
| `unpacked_ca.exe.til` | 2026-10-18 14:19 | 25.990 |

→ component (`.id0`, `.nam`) **mới hơn `.i64` gần 8,5 giờ** ⇒ DB đang mở và **có thay đổi chưa ghi vào
`.i64`**. Không có tiến trình `ida*` nào trong namespace này (`ps aux`), nên nhiều khả năng GUI đang chạy ở
session/namespace khác.

### Đã làm

Backup đầy đủ (cả component, không chỉ `.i64`) + checksum:

```
/home/admin/backups/ida-20261018/           630 MB
  unpacked_ca.exe.i64   d7a9874a…
  unpacked_ca.exe.id0   ed10fb09…
  unpacked_ca.exe.nam   b064a2d6…
  unpacked_ca.exe.til   38cf48f8…
  CA.exe.i64 (KR)       c160bad2…
  SHA256SUMS
```

### Còn phải làm (chủ DB)

1. Trong IDA: `Ctrl+S` (hoặc File → Save) để ghi state vào `.i64`.
2. Kiểm lại: `stat -c '%y %n' unpacked_ca.exe.i64 unpacked_ca.exe.id0` — `.i64` phải **mới hơn** `.id0`.
3. Nếu IDA báo DB hỏng/không lưu được: đóng IDA, giữ nguyên `boom-re/` (backup ở trên là bản chụp 20:45).

> Không thể tự lưu từ đây: `mcp ida save_database` trả `no open database instance` — worker không giữ DB
> này, GUI mới là bên giữ.

## 2. PRES-001 — Inventory artefact lớn / bản quyền

| Artefact | Kích thước | Có trong git? | Tái tạo từ đâu |
|---|---|---|---|
| `~/Downloads/Boom/` (client VN gốc) | 1,1 GB | ❌ | **không tái tạo được** — bản cài game |
| `boomKR/` (build KR + `.i64`) | 1,8 GB | ❌ (ignore `8f25060`) | patch Nexon `00967` (đã hết hạn?) |
| `wine/Boom/` | 1,1 GB | ❌ | `cp -a --reflink=auto ~/Downloads/Boom wine/` |
| `thirdparty/` | 803 MB | ❌ | tải Mesa/swd3d |
| `assets/` | 665 MB | ❌ | `tools/idf.py extract ~/Downloads/Boom assets` |
| `logs/` | 2,9 GB | ❌ | chạy client (`snap`, `dumps`, `live`) |
| `unpacked_ca.exe` | 19 MB | ❌ | unpack Themida (README §1) |
| `ca_fixed_patched.exe` | 19 MB | ❌ | `tools/rebuild_wine.sh` |
| `server/target/` | 289 MB | ❌ | `cargo build` |
| `unpacked_ca.exe.i64` | 312 MB | ❌ | IDA + tên từ `rtti.txt`/`ida_names.json` |

Tổng ổ đĩa: **~9,4 GB**. Trong git chỉ ~205 file text (`docs/`, `tools/`, `decomp/`, `server/src/`).

**Quy tắc:** `.gitignore` hiện đã chặn `*.exe`, `assets/`, `logs/`, `wine/`, `thirdparty/`, `boomKR/`,
`*.i64`. Kiểm tra trước khi push:

```bash
git status --porcelain | grep -E "\.(exe|dll|idd|idx|i64)$" && echo "CÓ BINARY — DỪNG" || echo sạch
```

## 3. PRES-002 — SHA-256

| Phạm vi | Trạng thái | File |
|---|---|---|
| build KR (`boomKR/`, 177 file) | ✅ xong | `boomkr-re/out/boomKR.sha256` |
| client VN gốc (`~/Downloads/Boom`, 98 file) | ⬜ chưa | — |

Kiểm lại:

```bash
cd /home/admin/boom-re/boomKR && sha256sum -c /home/admin/boomkr-re/out/boomKR.sha256
```

## 4. Liên quan

- Project con cho KR: `/home/admin/boomkr-re/` (commit `87d9d76`) — pack 4/13, schema `files.nfo2`.
- Việc runtime KR **bị chặn** bởi KR-002 (`MASTER_PLAN` §6.3).
