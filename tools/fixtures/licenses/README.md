# Fixture toàn văn license (dùng cho tự kiểm `tools/check_package.py`)

Mỗi file là **nguyên văn** một toàn văn license, dùng làm dữ liệu vào cho tự kiểm —
không phải license của dự án, không dùng để phân phối.

| File | Nguồn |
|---|---|
| `mit-spdx.txt` | văn bản MIT chuẩn của SPDX (placeholder `<year> <copyright holders>`) |
| `apache-2.0.txt` | toàn văn Apache-2.0 (bản trong crate `ab_glyph 0.2.32`, SHA-256 `a6cba85bc92e0cff7a450b1d873c0eaa2e9fc96bf472df0247a26bec77bf3ff9`) |
| `windows-link-0.2.1-license-mit.txt` | **nguyên văn** file `license-mit` của crate `windows-link 0.2.1` trên crates.io (SHA-256 `c2cfccb812fe482101a8f04597dfc5a9991a6b2748266c47ac91b6a5aae15383`); `windows-sys 0.61.2` có file giống hệt |

`windows-link-0.2.1-license-mit.txt` giữ nguyên văn để làm regression: thông báo bản quyền
**không ghi năm** (`Copyright (c) Microsoft Corporation.`) phải còn trong `LICENSES`, và file
MIT phải được xác minh đủ điều khoản chứ không phải chỉ nhờ tên file.
