# Toàn văn license chuẩn (mẫu đối chiếu) và fixture cho tự kiểm

Mỗi file là **nguyên văn** một toàn văn license, dùng làm **mẫu đối chiếu** lúc
`tools/check_package.py` xác minh file license của crate và làm dữ liệu vào cho tự
kiểm. Không phải license của dự án, không dùng để phân phối, không phải asset game.

Vì sao cần mẫu: một file chỉ là **toàn văn** license khi nội dung nó khớp mẫu ở
**mọi đơn vị điều khoản** (câu) của mẫu — không chỉ nhờ tên file, không chỉ khớp câu
đầu hay tiêu đề mục. Nhờ vậy file bị cắt cụt hoặc file chỉ có tiêu đề mục đều bị bác
bỏ (xem `LICENSE_TEMPLATES` trong `tools/check_package.py`).

| File | SPDX id | Nguồn (nguyên văn) | SHA-256 |
|---|---|---|---|
| `mit-spdx.txt` | MIT | văn bản MIT chuẩn của SPDX (placeholder `<year> <copyright holders>`) | `b85dcd3e453d05982552c52b5fc9e0bdd6d23c6f8e844b984a88af32570b0cc0` |
| `apache-2.0-spdx.txt` | Apache-2.0 | phần **điều khoản** của văn bản Apache-2.0 trong SPDX license list (`/usr/share/licenses/spdx/Apache-2.0.txt`), cắt tại `END OF TERMS AND CONDITIONS` | `601338a5c6aba07bc5151720fc2f099babdeac3e84fff02b207de1e1e74bdaa1` |
| `apache-2.0-llvm-exception.txt` | Apache-2.0 WITH LLVM-exception | phần `--- LLVM Exceptions to the Apache 2.0 License ----` nguyên văn trong file `LICENSE-Apache-2.0_WITH_LLVM-exception` của crate `rustix 1.1.5` (crates.io) | `fa552acb0c6ff2d3baaf84fa500fbe74ab11e032f16fcefa6d064f04a28f321f` |
| `bsd-2-clause.txt` | BSD-2-Clause | văn bản BSD-2-Clause (`/usr/share/licenses/util-linux-libs/COPYING.BSD-2-Clause`) | `527f738966ca396cd5a68c1509390de2a780c6b614d9ee57f7544a6161938ed1` |
| `bsd-3-clause.txt` | BSD-3-Clause | văn bản BSD-3-Clause chuẩn SPDX (`/usr/share/licenses/man-pages/BSD-3-Clause.txt`), placeholder `<year> <owner>` | `3cf06aba3588c41c514f6946bb2d757b413ff6491647d474800f55edca75dcb4` |
| `0bsd.txt` | 0BSD | file `LICENSE-0BSD` nguyên văn của crate `adler2 2.0.1` (crates.io) | `861399f8c21c042b110517e76dc6b63a2b334276c8cf17412fc3c8908ca8dc17` |
| `zlib.txt` | Zlib | file `LICENSE-ZLIB` nguyên văn của crate `bytemuck 1.25.2` (crates.io) | `84b34dd7608f7fb9b17bd588a6bf392bf7de504e2716f024a77d89f1b145a151` |
| `unlicense.txt` | Unlicense | văn bản Unlicense trong SPDX license list (`/usr/share/licenses/spdx/Unlicense.txt`) | `0bdebfeda07d45dada625ae1317c6f833186e798b171d0db640bcf32e92a8240` |
| `unicode-3.0.txt` | Unicode-3.0 | văn bản Unicode-3.0 trong SPDX license list (`/usr/share/licenses/qt6-base/Unicode-3.0.txt`) | `80bb6294022d8764adb36a9d65ebb6bd94efb1865918a71e31e6c247607d7634` |
| `lgpl-2.1-or-later.txt` | LGPL-2.1-or-later | phần **điều khoản** của LGPL-2.1-or-later trong SPDX license list (`/usr/share/licenses/spdx/LGPL-2.1-or-later.txt`), cắt tại `END OF TERMS AND CONDITIONS` | `7f5f53b65d9c876b9f599204adf2258b61ed4565c6e54b2e24c150a01fc4a4e2` |
| `apache-2.0.txt` | (fixture test) | toàn văn Apache-2.0 nguyên văn trong crate `ab_glyph 0.2.32` (crates.io), gồm cả phần `APPENDIX` | `a6cba85bc92e0cff7a450b1d873c0eaa2e9fc96bf472df0247a26bec77bf3ff9` |
| `windows-link-0.2.1-license-mit.txt` | (fixture test) | file `license-mit` nguyên văn của crate `windows-link 0.2.1` (crates.io); `windows-sys 0.61.2` có file giống hệt | `c2cfccb812fe482101a8f04597dfc5a9991a6b2748266c47ac91b6a5aae15383` |

Ghi chú:

- Phần **điều khoản** của `apache-2.0-spdx.txt` đã đối chiếu với bản crate
  (`apache-2.0.txt`): giống nhau, chỉ khác cách xuống dòng và phần `APPENDIX` phụ.
- `windows-link-0.2.1-license-mit.txt` giữ nguyên văn để làm regression: thông báo
  bản quyền **không ghi năm** (`Copyright (c) Microsoft Corporation.`) phải còn trong
  `LICENSES`, và file MIT phải được xác minh đủ điều khoản chứ không phải chỉ nhờ
  tên file.
