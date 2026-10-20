"""Bộ công cụ gói phát hành: sinh file `LICENSES` và kiểm bộ đóng gói (V1-036 phần 2).

Dùng trong `.github/workflows/release.yml`:

    python3 tools/check_package.py licenses --out dist/<gói>/LICENSES
    python3 tools/check_package.py check dist/<gói>
    python3 tools/check_package.py test

Nguyên tắc:

- **Offline.** Không gọi mạng. Toàn văn license lấy **nguyên văn** từ file
  `LICENSE*`/`COPYING*`/`NOTICE*`/`UNLICENSE` trong nguồn crate của cargo registry
  (cargo fetch đã tải về lúc build). Không tự viết lại toàn văn license và không
  suy diễn license mà không có file nguồn: license không tìm thấy toàn văn thì
  phần phát hành báo lỗi, không lấp chỗ trống bằng văn bản tự chế.
- **Xác minh nội dung, không tin tên file.** Một file chỉ là *toàn văn* license khi
  nội dung có **đủ điều khoản** của license đó (`LICENSE_CLAUSES`, trích từ văn bản
  SPDX chuẩn), không chỉ nhờ tên file hay câu đầu. File rỗng/cụt, file tên khai
  license nhưng chứa license khác, hay file có tên nhưng nội dung không xác minh
  được → công cụ **báo lỗi và không sinh gói**.
- **Một nguồn chuẩn.** `docs/specification/RELEASE.md` là nơi mô tả bố cục gói;
  danh sách file/thư mục cấm trong script phải khớp với mô tả đó.
- **Không asset game trong gói.** `check` chặn pack/bundle/atlas/ảnh/âm thanh theo
  danh sách đuôi file và thư mục cấm. Font OFL (`assets/fonts/`) là ngoại lệ có chủ
  ý: không phải asset game, được phân phối kèm `OFL.txt`.

Mã thoát: `0` thành công · `1` có vi phạm/ lỗi (in rõ từng mục).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import unittest
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# --------------------------------------------------------------------------------------
# Hằng số dùng chung cho sinh LICENSES và kiểm bộ đóng gói
# --------------------------------------------------------------------------------------

#: Tiêu đề từng phần của file LICENSES (marker để `check` xác nhận file đủ cấu trúc).
PART_TITLES = (
    "PHẦN 1 — TOÀN VĂN LICENSE (SPDX)",
    "PHẦN 2 — CRATE PHỤ THUỘC",
    "PHẦN 3 — BẢN QUYỀN TỪNG CRATE",
    "PHẦN 4 — GHI CHÚ",
)

#: Thư mục bắt buộc có trong gói phát hành.
REQUIRED_DIRS = ("bin", "data/rules", "data/l10n", "assets/fonts")

#: File bắt buộc có trong gói phát hành.
REQUIRED_FILES = ("README.md", "LICENSES", "assets/fonts/OFL.txt")

#: Binary bắt buộc trong `bin/` (Windows thêm `.exe`).
REQUIRED_BINARIES = ("boom-client", "boom-server", "boom-assets")

#: Thư mục cấm: pack/bundle/atlas/asset game (không được kèm vào gói).
FORBIDDEN_DIRS = frozenset({
    "bundle", "bundles", "atlas", "atlases", "pack", "packs",
    "sprites", "textures", "tilesets",
})

#: Đuôi file cấm: dữ liệu pack/bundle và asset game (ảnh, âm thanh).
#: Font `.ttf`/`.otf` **không** nằm trong danh sách này (OFL, được kèm cùng `OFL.txt`).
FORBIDDEN_EXT = frozenset({
    # dữ liệu pack/bundle
    ".map", ".idx", ".idd", ".pak", ".bin", ".atlas",
    # ảnh/atlas
    ".png", ".jpg", ".jpeg", ".bmp", ".gif", ".webp", ".tga",
    # âm thanh
    ".wav", ".ogg", ".mp3", ".flac", ".opus",
})

#: File license trong nguồn crate (không phân biệt hoa thường).
LICENSE_FILE_RE = re.compile(r"^(licen[sc]e|copying|notice|unlicense)", re.IGNORECASE)

#: Đuôi file cần bỏ khi đối chiếu tên file license với SPDX id.
LICENSE_STEM_RE = re.compile(r"^(license|licence|copying|unlicense|notice)(.*)$", re.IGNORECASE)

#: Họ license (SPDX id → họ). File tên theo họ (`LICENSE-BSD`) không sai nhãn khi nội dung
#: là biến thể khác cùng họ (`BSD-2-Clause` trong file `LICENSE-BSD`).
LICENSE_FAMILY = {
    "MIT": "MIT",
    "Apache-2.0": "Apache",
    "Apache-2.0 WITH LLVM-exception": "Apache",
    "BSD-2-Clause": "BSD",
    "BSD-3-Clause": "BSD",
    "0BSD": "0BSD",
    "Zlib": "Zlib",
    "Unlicense": "Unlicense",
    "Unicode-3.0": "Unicode",
    "LGPL-2.1-or-later": "LGPL",
}

#: Tên file license → **họ** license mà tên đó khai (định nghĩa chuẩn của crate, không suy
#: từ nội dung). Chỉ dùng để *đối chiếu*, không đủ để kết luận file là toàn văn license.
FILE_NAME_FAMILIES = {
    "mit": "MIT",
    "apache": "Apache",
    "apache-2.0": "Apache",
    "apache-2.0_with_llvm-exception": "Apache",
    "0bsd": "0BSD",
    "zlib": "Zlib",
    "unicode": "Unicode",
    "unlicense": "Unlicense",
    "bsd": "BSD",
    "lgpl": "LGPL",
}

#: Điều khoản bắt buộc của từng license SPDX (trích từ văn bản SPDX chuẩn).
#:
#: Một file chỉ được coi là **toàn văn** license khi nội dung nó có **đủ** các điều khoản
#: dưới đây — không suy từ tên file và không chỉ nhìn câu đầu. So khớp bỏ qua khác biệt
#: xuống dòng và hoa/thường (nguồn crate trong registry xuống dòng khác nhau), nên một
#: file bị cắt cụt sẽ thiếu điều khoản và bị bác bỏ.
LICENSE_CLAUSES: dict[str, tuple[str, ...]] = {
    "MIT": (
        "permission is hereby granted, free of charge, to any person obtaining a copy "
        "of this software and associated documentation files",
        "to deal in the software without restriction",
        "the above copyright notice and this permission notice",
        "without warranty of any kind",
        "in no event shall the authors or copyright holders be liable for any claim",
    ),
    "0BSD": (
        "permission to use, copy, modify, and/or distribute this software for any purpose "
        "with or without fee is hereby granted",
        "the author disclaims all warranties with regard to this software including all "
        "implied warranties of merchantability and fitness",
        "in no event shall the author be liable for any special, direct, indirect, or "
        "consequential damages",
    ),
    "Zlib": (
        "without any express or implied warranty",
        "permission is granted to anyone to use this software for any purpose",
        "the origin of this software must not be misrepresented",
        "altered source versions must be plainly marked as such",
        "this notice may not be removed or altered from any source distribution",
    ),
    "Unlicense": (
        "this is free and unencumbered software released into the public domain",
        "anyone is free to copy, modify, publish, use, compile, sell, or distribute this "
        "software",
        "without warranty of any kind",
        "for more information, please refer to",
    ),
    "BSD-3-Clause": (
        "redistribution and use in source and binary forms, with or without modification, "
        "are permitted provided that the following conditions are met",
        "redistributions of source code must retain the above copyright notice",
        "neither the name of",
        "this software is provided by the copyright holders and contributors",
    ),
    "BSD-2-Clause": (
        "redistribution and use in source and binary forms, with or without modification, "
        "are permitted provided that the following conditions are met",
        "redistributions of source code must retain the above copyright notice",
        "this software is provided by the copyright holders",
    ),
    "Unicode-3.0": (
        "unicode license v3",
        "copyright and permission notice",
        "permission is hereby granted, free of charge, to any person obtaining a copy of "
        "data files",
        "without warranty of any kind",
        "except as contained in this notice, the name of a copyright holder shall not be "
        "used in advertising",
    ),
    "Apache-2.0 WITH LLVM-exception": (
        "apache license",
        "version 2.0, january 2004",
        "llvm exceptions to the apache 2.0 license",
        "1. definitions.",
        "4. redistribution.",
        "7. disclaimer of warranty.",
        "end of terms and conditions",
    ),
    "Apache-2.0": (
        "apache license",
        "version 2.0, january 2004",
        "terms and conditions for use, reproduction, and distribution",
        "1. definitions.",
        "2. grant of copyright license.",
        "3. grant of patent license.",
        "4. redistribution.",
        "5. submission of contributions.",
        "6. trademarks.",
        "7. disclaimer of warranty.",
        "8. limitation of liability.",
        "9. accepting warranty or additional liability.",
        "end of terms and conditions",
    ),
    "LGPL-2.1-or-later": (
        "gnu lesser general public license",
        "version 2.1, february 1999",
        "additional definitions",
        "exception to section 3",
        "granting of further permissions",
        "granting the library license",
        "restrictions on modification and distribution",
        "restrictions on linked applications",
        "distributions in object form",
        "modifications as exception",
        "end of terms and conditions",
    ),
}

#: Cụm từ bắt buộc của **thông báo** license (file chỉ trỏ tới toàn văn ở nơi khác).
#: File như vậy không phải toàn văn nên không dùng làm nguồn toàn văn, nhưng vẫn hợp lệ.
LICENSE_NOTICE_CLAUSES: dict[str, tuple[str, ...]] = {
    "Apache-2.0": (
        "licensed under the apache license, version 2.0",
        "apache.org/licenses/license-2.0",
    ),
}

#: Dòng bắt đầu bằng dấu bản quyền (`Copyright`, `(c)`, `©`).
COPYRIGHT_MARKER_RE = re.compile(r"^\s*(?:\(c\)|©|copyright\b)", re.IGNORECASE)

#: Năm trong dòng bản quyền (`2019`, `1993,2004`, `2016--2023`).
COPYRIGHT_YEAR_RE = re.compile(r"\b(?:1\d{3}|20\d{2})\b")

#: Chủ bản quyền ghi rõ bằng email hoặc tên trong ngoặc nhọn (`<a@b.c>`).
COPYRIGHT_HOLDER_RE = re.compile(r"@|<[^>]+>")

#: Tên riêng viết hoa ở đầu dòng bản quyền (`Microsoft Corporation`, `Individual
#: contributors`); tên viết HOA ĐỦ (`AND PERMISSION NOTICE`) không phải tên riêng.
COPYRIGHT_NAME_RE = re.compile(r"[A-Z][a-z]")

#: Dòng bản quyền còn là placeholder của SPDX (`<year>`, `[yyyy]`, `{yyyy}`,
#: `[name of copyright owner]`).
PLACEHOLDER_RE = re.compile(
    r"<\s*(?:year|copyright [a-z ]*|name of copyright [a-z ]*|owner)\s*>"
    r"|\[\s*(?:yyyy|yyyy-mm-dd|name of copyright owner|copyright owner)\s*\]"
    r"|\{\s*(?:yyyy|name of copyright owner|copyright owner)\s*\}",
    re.IGNORECASE)


# --------------------------------------------------------------------------------------
# Kiểu dữ liệu
# --------------------------------------------------------------------------------------


@dataclass
class Crate:
    """Một crate phụ thuộc (không phải crate của chính dự án)."""

    name: str
    version: str
    license: str  # biểu thức license gốc trong Cargo.toml
    license_file: str | None  # trường `license-file` nếu có
    repository: str | None
    directory: Path  # thư mục nguồn crate trong cargo registry

    @property
    def key(self) -> str:
        return f"{self.name} {self.version}"


@dataclass
class LicenseFile:
    """Một file license tìm thấy trong nguồn crate."""

    path: Path
    ident: str | None  # SPDX id (None nếu không nhận ra)
    kind: str  # "full" | "notice" | "mislabeled" | "invalid" | "other"
    text: str
    claimed: str | None = None  # họ license mà *tên file* khai


@dataclass
class LicenseText:
    """Toàn văn một license SPDX, lấy từ file license của crate nguồn."""

    ident: str
    crate: Crate
    path: Path
    sha256: str
    text: str
    filled_copyright: bool  # toàn văn có dòng bản quyền đã điền (không phải placeholder)


@dataclass
class Choice:
    """License dùng khi phân phối một crate."""

    crate: Crate
    expression: str
    options: list[str]  # mọi id trong biểu thức
    chosen: list[str]  # id thực sự kèm toàn văn trong gói
    required: bool  # biểu thức có AND ⇒ mọi id đều bắt buộc
    options_without_text: list[str] = field(default_factory=list)


@dataclass
class Report:
    """Kết quả phân tích đồ thị phụ thuộc (dùng cho LICENSES và cho `check`)."""

    crates: list[Crate] = field(default_factory=list)
    texts: dict[str, LicenseText] = field(default_factory=dict)
    choices: list[Choice] = field(default_factory=list)
    crate_files: dict[str, list[LicenseFile]] = field(default_factory=dict)
    problems: list[str] = field(default_factory=list)


# --------------------------------------------------------------------------------------
# Biểu thức license SPDX
# --------------------------------------------------------------------------------------

TOKEN_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9.+_-]*|[()]")


def normalize_expression(expression: str) -> str:
    """Chuẩn hoá biểu thức license cũ của Cargo (`MIT/Apache-2.0`) thành dạng SPDX."""
    # `A/B` (dạng cũ) nghĩa là `A OR B`; không đụng dấu gạch chéo trong tên khác.
    expr = re.sub(r"(?<=[A-Za-z0-9])\s*/\s*(?=[A-Za-z0-9])", " OR ", expression)
    return " ".join(expr.split())


def _tokenize(expression: str) -> list[str]:
    return TOKEN_RE.findall(normalize_expression(expression))


class _ExprParser:
    """Cây biểu thức license: `id`, `id WITH exception`, `AND`, `OR`."""

    def __init__(self, tokens: list[str]):
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> str | None:
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def take(self) -> str:
        token = self.peek()
        if token is None:
            raise ValueError("biểu thức license kết thúc sớm")
        self.pos += 1
        return token

    def parse(self):
        node = self.parse_or()
        if self.peek() is not None:
            raise ValueError(f"token lạ trong biểu thức license: {self.peek()}")
        return node

    def parse_or(self):
        node = self.parse_and()
        while self.peek() == "OR":
            self.take()
            node = ("or", node, self.parse_and())
        return node

    def parse_and(self):
        node = self.parse_factor()
        while self.peek() == "AND":
            self.take()
            node = ("and", node, self.parse_factor())
        return node

    def parse_factor(self):
        if self.peek() == "(":
            self.take()
            node = self.parse_or()
            if self.peek() != ")":
                raise ValueError("thiếu dấu ) trong biểu thức license")
            self.take()
            return node
        ident = self.take()
        if ident in ("OR", "AND", "WITH"):
            raise ValueError(f"toán tử license ở sai chỗ: {ident}")
        if self.peek() == "WITH":
            self.take()
            exception = self.take()
            return ("with", ident, exception)
        return ("id", ident)


def parse_expression(expression: str):
    """Phân tích biểu thức license; trả về cây nút."""
    return _ExprParser(_tokenize(expression)).parse()


def expression_ids(node) -> list[str]:
    """Danh sách SPDX id trong biểu thức (giữ thứ tự, bỏ trùng)."""
    ids: list[str] = []

    def walk(item):
        kind = item[0]
        if kind == "id":
            ident = item[1]
        elif kind == "with":
            ident = f"{item[1]} WITH {item[2]}"
        elif kind == "or":
            walk(item[1])
            walk(item[2])
            return
        elif kind == "and":
            walk(item[1])
            walk(item[2])
            return
        else:  # pragma: no cover - không xảy ra
            raise ValueError(f"node license lạ: {item}")
        if ident not in ids:
            ids.append(ident)

    walk(node)
    return ids


def expression_is_or_only(node) -> bool:
    """Biểu thức chỉ có phép OR (người phân phối được chọn một nhánh) hay không."""
    kind = node[0]
    if kind in ("id", "with"):
        return True
    if kind == "or":
        return expression_is_or_only(node[1]) and expression_is_or_only(node[2])
    return False


def expression_or_options(node) -> list[str]:
    """Các nhánh của biểu thức OR thuần, theo thứ tự trái → phải."""
    if node[0] == "or":
        return expression_or_options(node[1]) + expression_or_options(node[2])
    return expression_ids(node)


# --------------------------------------------------------------------------------------
# Nhận diện file license
# --------------------------------------------------------------------------------------


def normalize_for_match(text: str) -> str:
    """Chuẩn hoá văn bản để so khớp: gộp khoảng trắng và hạ chữ thường.

    Nguồn crate trong registry xuống dòng khác nhau; điều khoản license thì không.
    """
    return " ".join(text.split()).lower()


def clauses_present(clauses: tuple[str, ...], flat: str) -> bool:
    """Đủ tất cả điều khoản `clauses` trong văn bản đã chuẩn hoá `flat` không."""
    return all(normalize_for_match(clause) in flat for clause in clauses)


def detect_by_content(text: str) -> tuple[str | None, str]:
    """Nhận diện SPDX id từ **nội dung** file license.

    Trả về `(spdx_id, kind)` với `kind`:
    - `full`: toàn văn license — nội dung có **đủ** điều khoản của license đó
      ([LICENSE_CLAUSES](tools/check_package.py)), không chỉ câu đầu;
    - `notice`: thông báo "xem license ở đường dẫn khác" (không phải toàn văn);
    - `other`: không nhận ra nội dung này là license nào.
    """
    flat = normalize_for_match(text)
    for ident, clauses in LICENSE_CLAUSES.items():
        if clauses_present(clauses, flat):
            return ident, "full"
    for ident, clauses in LICENSE_NOTICE_CLAUSES.items():
        if clauses_present(clauses, flat):
            return ident, "notice"
    return None, "other"


def claimed_family(path: Path) -> str | None:
    """Họ license mà **tên file** khai (không suy từ nội dung); `None` nếu tên không nói."""
    stem = path.stem.lower()
    if stem in FILE_NAME_FAMILIES:
        return FILE_NAME_FAMILIES[stem]
    match = LICENSE_STEM_RE.match(stem)
    if not match:
        return None
    suffix = match.group(2).strip("-_ ")
    return FILE_NAME_FAMILIES.get(suffix)


def classify_license_file(path: Path) -> LicenseFile:
    """Đọc một file license và phân loại theo **nội dung**, đối chiếu tên file.

    - nội dung là toàn văn license → `full` (tên file sai họ thì `mislabeled`);
    - nội dung là thông báo trỏ tới license → `notice`;
    - tên file khai là license nhưng nội dung không xác minh được (rỗng, cụt, license
      khác) → `invalid` (không dùng làm nguồn toàn văn, và báo lỗi khi sinh LICENSES);
    - không nhận ra → `other`.
    """
    raw = path.read_bytes()
    text = raw.decode("utf-8", errors="replace")
    claimed = claimed_family(path)
    detected, kind = detect_by_content(text)
    if kind in ("full", "notice"):
        detected_family = LICENSE_FAMILY.get(detected or "", detected or "")
        if claimed and claimed != detected_family:
            return LicenseFile(path=path, ident=detected, kind="mislabeled",
                               text=text, claimed=claimed)
        return LicenseFile(path=path, ident=detected, kind=kind, text=text, claimed=claimed)
    if claimed:
        return LicenseFile(path=path, ident=None, kind="invalid", text=text, claimed=claimed)
    return LicenseFile(path=path, ident=None, kind="other", text=text, claimed=claimed)


def license_files(directory: Path) -> list[Path]:
    """Các file license trong thư mục nguồn crate (bỏ qua thư mục con)."""
    if not directory.is_dir():
        return []
    found = []
    for entry in sorted(directory.iterdir()):
        if entry.is_file() and LICENSE_FILE_RE.match(entry.name):
            found.append(entry)
    return found


def sha256_of(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def copyright_lines(text: str) -> list[str]:
    """Các dòng bản quyền trong file license (giữ thứ tự, bỏ trùng).

    Dòng bị ngắt giữa chừng trong file (văn bản wrap) được nối lại; dòng còn là
    placeholder của SPDX thì bỏ (không phải ghi bản quyền thật).

    Dòng bản quyền phải có **chủ** thật: có năm, có email/tên trong ngoặc nhọn, hoặc
    (với dòng bắt đầu bằng chữ `Copyright`) tên riêng viết hoa. Nhờ vậy thông báo
    bản quyền **không ghi năm** — vd `Copyright (c) Microsoft Corporation.` — vẫn được
    giữ, còn câu văn trong điều khoản license (vd `copyright notice that is included
    in or attached to the work`) thì không bị lấy nhầm.
    """
    result: list[str] = []
    lines = text.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        match = COPYRIGHT_MARKER_RE.match(line)
        if not match:
            index += 1
            continue
        rest = line[match.end():].strip()
        # Bỏ dấu `(c)`/`©` đứng sau marker ("Copyright (c) …").
        rest = re.sub(r"^(?:\(c\)|©)\s*", "", rest, flags=re.IGNORECASE).strip()
        if not _has_copyright_holder(rest, line):
            index += 1
            continue
        collected = " ".join(line.split())
        # Nối dòng kế tiếp khi dòng hiện tại bị cắt giữa câu.
        while collected.endswith((" or", " and", ",", "of")) and index + 1 < len(lines):
            following = lines[index + 1].strip()
            if not following or COPYRIGHT_MARKER_RE.match(following):
                break
            collected += " " + " ".join(following.split())
            index += 1
        index += 1
        if not collected or PLACEHOLDER_RE.search(collected):
            continue
        if collected not in result:
            result.append(collected)
    return result


def _has_copyright_holder(rest: str, line: str) -> bool:
    """Phần sau marker có đủ dấu hiệu của chủ bản quyền thật không.

    - có năm (`2019`), email/tên trong ngoặc nhọn (`<a@b.c>`) → chắc chắn là thông báo
      bản quyền, kể cả không ghi năm;
    - dòng bắt đầu bằng chữ `Copyright` và phần còn lại bắt đầu bằng tên riêng viết
      hoa (`Microsoft Corporation`, `Individual contributors`) → thông báo bản quyền;
    - còn lại (vd `(c) You must retain, …` — mục điều khoản, không phải bản quyền) → không.
    """
    if COPYRIGHT_YEAR_RE.search(rest) or COPYRIGHT_HOLDER_RE.search(rest):
        return True
    if not line.lstrip().lower().startswith("copyright"):
        return False
    return bool(COPYRIGHT_NAME_RE.match(rest))


def has_filled_copyright(text: str) -> bool:
    """Toàn văn có dòng bản quyền đã điền (không còn placeholder của SPDX) không."""
    return bool(copyright_lines(text))


# --------------------------------------------------------------------------------------
# Thu thập dữ liệu crate
# --------------------------------------------------------------------------------------


def cargo_metadata(cwd: Path | None = None) -> dict:
    """Đọc `cargo metadata` (byte rồi tự giải mã UTF-8: `text=True` hỏng trên Windows)."""
    raw = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        capture_output=True, check=True, cwd=str(cwd or ROOT),
    ).stdout
    return json.loads(raw.decode("utf-8"))


def crates_from_metadata(metadata: dict) -> list[Crate]:
    """Crate phụ thuộc (có `source`) từ `cargo metadata`, sắp xếp theo tên rồi phiên bản."""
    crates: list[Crate] = []
    for package in metadata.get("packages", []):
        if not package.get("source"):
            continue  # crate của chính workspace
        manifest = Path(package["manifest_path"])
        crates.append(Crate(
            name=package["name"],
            version=package["version"],
            license=package.get("license") or "",
            license_file=package.get("license_file"),
            repository=package.get("repository"),
            directory=manifest.parent,
        ))
    crates.sort(key=lambda c: (c.name, c.version))
    return crates


def build_report(crates: list[Crate]) -> Report:
    """Dựng báo cáo license cho đồ thị phụ thuộc: toàn văn, lựa chọn, file theo crate."""
    report = Report(crates=list(crates))
    candidates: dict[str, list[LicenseText]] = {}

    for crate in crates:
        if not crate.directory.is_dir():
            # Nguồn crate chưa tải về (ví dụ crate của nền tảng khác): không âm thầm bỏ qua,
            # vì LICENSES sẽ thiếu bản quyền của crate đó.
            report.problems.append(
                f"crate {crate.key}: chưa có nguồn crate trong cargo registry "
                f"({crate.directory}); chạy `cargo fetch` trước khi sinh LICENSES")
            continue
        files = []
        for path in license_files(crate.directory):
            try:
                classified = classify_license_file(path)
            except OSError as error:  # pragma: no cover - lỗi hệ thống
                report.problems.append(f"không đọc được file license {path}: {error}")
                continue
            files.append(classified)
            if classified.kind == "invalid":
                report.problems.append(
                    f"crate {crate.key}: file license `{classified.path.name}` khai "
                    f"{classified.claimed} nhưng nội dung không xác minh được toàn văn "
                    f"license (rỗng/cụt/không đúng license) — không dùng làm nguồn")
                continue
            if classified.kind == "mislabeled":
                report.problems.append(
                    f"crate {crate.key}: file license `{classified.path.name}` tên khai "
                    f"{classified.claimed} nhưng nội dung là {classified.ident} (sai nhãn)")
                continue
            if classified.ident and classified.kind == "full":
                candidates.setdefault(classified.ident, []).append(LicenseText(
                    ident=classified.ident,
                    crate=crate,
                    path=path,
                    sha256=sha256_of(classified.text),
                    text=classified.text,
                    filled_copyright=has_filled_copyright(classified.text),
                ))
        report.crate_files[crate.key] = files

    # Chọn toàn văn đại diện cho mỗi SPDX id: ưu tiên bản không có dòng bản quyền đã điền
    # (gần văn bản SPDX chuẩn), sau đó theo tên crate cho ổn định.
    for ident, options in candidates.items():
        options.sort(key=lambda t: (t.filled_copyright, t.crate.name, t.crate.version, t.path.name))
        report.texts[ident] = options[0]

    for crate in crates:
        if not crate.license:
            report.problems.append(f"crate {crate.key} không khai license trong Cargo.toml")
            continue
        try:
            node = parse_expression(crate.license)
        except ValueError as error:
            report.problems.append(f"crate {crate.key}: biểu thức license không đọc được ({error})")
            continue
        ids = expression_ids(node)
        if expression_is_or_only(node):
            options = expression_or_options(node)
            chosen = [i for i in options if i in report.texts]
            if not chosen:
                report.problems.append(
                    f"crate {crate.key}: không có toàn văn license nào trong nguồn crate cho "
                    f"biểu thức '{crate.license}' ({', '.join(options)})")
                continue
            report.choices.append(Choice(
                crate=crate, expression=crate.license, options=options,
                chosen=chosen[:1], required=False,
                options_without_text=[i for i in options if i not in report.texts]))
        else:
            missing = [i for i in ids if i not in report.texts]
            if missing:
                report.problems.append(
                    f"crate {crate.key}: thiếu toàn văn license bắt buộc "
                    f"({', '.join(missing)}) cho biểu thức '{crate.license}'")
                continue
            report.choices.append(Choice(
                crate=crate, expression=crate.license, options=ids,
                chosen=ids, required=True))

    return report


# --------------------------------------------------------------------------------------
# Sinh file LICENSES
# --------------------------------------------------------------------------------------

RULE = "-" * 80


def render_licenses(report: Report) -> str:
    """Dựng nội dung file LICENSES từ báo cáo license."""
    out: list[str] = []
    out.append("LICENSES — boom-rs (V1)")
    out.append(RULE)
    out.append("")
    out.append("Gói phát hành này gồm binary của boom-rs và các crate phụ thuộc ở PHẦN 2.")
    out.append("Tệp do `python3 tools/check_package.py licenses` sinh lúc build: đọc")
    out.append("`cargo metadata` cho đồ thị phụ thuộc và đọc nguyên văn các file")
    out.append("LICENSE*/COPYING*/NOTICE* trong nguồn crate của cargo registry.")
    out.append("")
    out.append(f"Crate phụ thuộc: {len(report.crates)} · license SPDX dùng trong gói: "
               f"{len(report.texts)}")
    out.append("")

    # PHẦN 1 — toàn văn license
    out.append("PHẦN 1 — TOÀN VĂN LICENSE (SPDX)")
    out.append(RULE)
    out.append("")
    out.append("Mỗi mục là toàn văn một license SPDX của crate phụ thuộc, lấy nguyên văn từ")
    out.append("file license của crate nguồn (nguồn ghi ở đầu mục); file đó đã được xác")
    out.append("minh đủ điều khoản của license, không chỉ tin tên file. Dòng \"Copyright (c) …\"")
    out.append("nếu có thuộc crate nguồn đó; theo SPDX, dòng ấy là \"<year> <copyright")
    out.append("holders>\", nhưng crate nhiều khi ghi không có năm. Bản quyền từng crate nằm ở")
    out.append("PHẦN 3.")
    out.append("")
    for ident in sorted(report.texts):
        text = report.texts[ident]
        out.append(f"[{ident}]")
        out.append(f"Nguồn: crate `{text.crate.key}` · file `{text.path.name}` · "
                   f"SHA-256: {text.sha256}")
        out.append("")
        out.append(text.text.strip("\n"))
        out.append("")
        out.append(RULE)
        out.append("")

    # PHẦN 2 — crate phụ thuộc
    out.append("PHẦN 2 — CRATE PHỤ THUỘC")
    out.append(RULE)
    out.append("")
    out.append("Tên · phiên bản · license (SPDX) · license kèm khi phân phối · kho mã nguồn")
    out.append("")
    by_key = {c.crate.key: c for c in report.choices}
    for crate in report.crates:
        choice = by_key.get(crate.key)
        expression = crate.license or "(không khai)"
        if choice is None:
            chosen = "(không xác định)"
        elif choice.required:
            chosen = " VÀ ".join(choice.chosen)
        else:
            chosen = choice.chosen[0]
        repository = crate.repository or "không có"
        out.append(f"  {crate.key} — {expression} — phân phối theo: {chosen} — {repository}")
    out.append("")

    # PHẦN 3 — bản quyền từng crate
    out.append("PHẦN 3 — BẢN QUYỀN TỪNG CRATE")
    out.append(RULE)
    out.append("")
    out.append("File license crate đó kèm trong nguồn registry, và dòng bản quyền trích từ đó.")
    out.append("")
    for crate in report.crates:
        out.append(f"  {crate.key}")
        files = report.crate_files.get(crate.key, [])
        if not files:
            if crate.directory.is_dir():
                out.append("    file license trong nguồn: (không có)")
            else:
                out.append("    file license trong nguồn: (nguồn crate chưa có trong registry)")
        for item in files:
            if item.kind == "full":
                label = item.ident or "không nhận ra"
            elif item.kind == "notice":
                label = f"{item.ident} — thông báo trỏ tới toàn văn (không phải toàn văn)"
            else:
                label = "không nhận ra"
            out.append(f"    file license trong nguồn: `{item.path.name}` ({label})")
        lines: list[str] = []
        for item in files:
            for line in copyright_lines(item.text):
                if line not in lines:
                    lines.append(line)
        if not lines:
            out.append("    bản quyền: (không có dòng bản quyền trong file license)")
        for line in lines:
            out.append(f"    bản quyền: {line}")
    out.append("")

    # PHẦN 4 — ghi chú
    out.append("PHẦN 4 — GHI CHÚ")
    out.append(RULE)
    out.append("")
    notes: list[str] = []

    no_file = [c.key for c in report.crates if not report.crate_files.get(c.key)]
    if no_file:
        notes.append(
            "Crate không kèm file license trong nguồn registry (dùng vẫn được theo biểu thức "
            "license khai trong Cargo.toml): " + ", ".join(f"`{k}`" for k in no_file) + ".")

    legacy = sorted({c.key for c in report.crates
                     if "/" in (c.license or "") and c.key in by_key})
    if legacy:
        notes.append(
            "Biểu thức license dạng cũ của Cargo (`A/B`) được hiểu là `A OR B`: "
            + ", ".join(f"`{k}`" for k in legacy) + ".")

    alternatives = [c for c in report.choices if not c.required and len(c.options) > 1]
    if alternatives:
        notes.append(
            f"Vế OR: {len(alternatives)} crate có biểu thức nhiều nhánh; gói phân phối theo "
            "một nhánh (được phép theo điều khoản OR của từng license) và nhánh đã chọn ghi ở "
            "cột \"phân phối theo\" của PHẦN 2.")
    without_text = [(c.crate.key, c.options_without_text) for c in report.choices
                     if c.options_without_text]
    if without_text:
        notes.append(
            "License có trong biểu thức nhưng gói **không** kèm toàn văn (vế OR nên không bắt "
            "buộc; nếu biểu thức có AND thì công cụ báo lỗi và không sinh gói):")
        for key, ids in without_text:
            notes.append(f"  - `{key}`: {', '.join(ids)}.")

    used_ids = {i for c in report.choices for i in c.chosen}
    unused = sorted(set(report.texts) - used_ids)
    if unused:
        notes.append(
            "Toàn văn có trong gói nhưng không phải nhánh được chọn của crate nào: "
            + ", ".join(unused) + ".")

    notes.append(
        "Toàn văn lấy từ file license của crate nguồn tại lúc build (không tải mạng), nên "
        "phần đầu/bản quyền có thể khác câu chữ so với bản SPDX chuẩn; nội dung điều khoản "
        "không đổi.")
    notes.append(
        "Crate của chính dự án (boom-*) không có trường `license` trong Cargo.toml nên "
        "không nằm trong bảng này; xem README và kho mã nguồn của dự án.")
    for note in notes:
        out.append(f"- {note}")
    out.append("")
    return "\n".join(out)


def cmd_licenses(args) -> int:
    if args.metadata:
        metadata = json.loads(Path(args.metadata).read_text(encoding="utf-8"))
    else:
        try:
            metadata = cargo_metadata()
        except (subprocess.CalledProcessError, json.JSONDecodeError) as error:
            print(f"LỖI: đọc `cargo metadata` không thành công: {error}", file=sys.stderr)
            return 1
    crates = crates_from_metadata(metadata)
    report = build_report(crates)
    if report.problems:
        print("LỖI: không sinh được LICENSES đầy đủ:", file=sys.stderr)
        for problem in report.problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    text = render_licenses(report)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8")
    if not out.stat().st_size:
        print(f"LỖI: file LICENSES rỗng: {out}", file=sys.stderr)
        return 1
    print(f"LICENSES: {len(report.crates)} crate, {len(report.texts)} license SPDX, "
          f"{out.stat().st_size} byte → {out}")
    return 0


# --------------------------------------------------------------------------------------
# Kiểm bộ đóng gói
# --------------------------------------------------------------------------------------


def check_package(package_dir: Path) -> list[str]:
    """Kiểm bộ đóng gói phát hành; trả về danh sách vi phạm (rỗng = đạt)."""
    problems: list[str] = []
    if not package_dir.is_dir():
        return [f"không có thư mục gói: {package_dir}"]

    for name in REQUIRED_DIRS:
        if not (package_dir / name).is_dir():
            problems.append(f"thiếu thư mục bắt buộc: {name}/")
    for name in REQUIRED_FILES:
        path = package_dir / name
        if not path.is_file():
            problems.append(f"thiếu file bắt buộc: {name}")
        elif path.stat().st_size == 0:
            problems.append(f"file rỗng: {name}")

    for binary in REQUIRED_BINARIES:
        stem = package_dir / "bin" / binary
        if not stem.is_file() and not stem.with_suffix(".exe").is_file():
            problems.append(f"thiếu binary trong bin/: {binary}")

    for name in ("data/rules", "data/l10n"):
        directory = package_dir / name
        if directory.is_dir() and not any(directory.glob("*.toml")):
            problems.append(f"không có file .toml nào trong {name}/")

    fonts = package_dir / "assets" / "fonts"
    if fonts.is_dir() and not any(
            item.is_file() and item.suffix.lower() in (".ttf", ".otf")
            for item in fonts.iterdir()):
        problems.append("không có file font (.ttf/.otf) nào trong assets/fonts/")

    licenses = package_dir / "LICENSES"
    if licenses.is_file() and licenses.stat().st_size:
        text = licenses.read_text(encoding="utf-8", errors="replace")
        for title in PART_TITLES:
            if title not in text:
                problems.append(f"LICENSES thiếu phần: {title}")

    for path in sorted(package_dir.rglob("*")):
        relative = path.relative_to(package_dir).as_posix()
        if path.is_dir():
            if path.name.lower() in FORBIDDEN_DIRS:
                problems.append(f"thư mục cấm (pack/bundle/asset game): {relative}/")
            continue
        if path.name.lower() == "bundle.toml":
            problems.append(f"file cấm (metadata bundle): {relative}")
        extension = path.suffix.lower()
        if extension in FORBIDDEN_EXT:
            problems.append(f"file cấm (pack/bundle/asset game): {relative}")

    return problems


def cmd_check(args) -> int:
    package_dir = Path(args.package_dir)
    problems = check_package(package_dir)
    if problems:
        print(f"LỖI: bộ đóng gói không đạt ({package_dir}):", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    files = sum(1 for path in package_dir.rglob("*") if path.is_file())
    print(f"Gói OK: {package_dir} ({files} file, không có pack/bundle/asset game).")
    return 0


# --------------------------------------------------------------------------------------
# Tự kiểm
# --------------------------------------------------------------------------------------


def _make_crate(root: Path, name: str, version: str, files: dict[str, str]) -> Path:
    directory = root / f"{name}-{version}"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "Cargo.toml").write_text(
        f'[package]\nname = "{name}"\nversion = "{version}"\n', encoding="utf-8")
    for filename, content in files.items():
        (directory / filename).write_text(content, encoding="utf-8")
    return directory


def _metadata(crates: list[tuple[Crate, Path]]) -> dict:
    packages = []
    for crate, directory in crates:
        packages.append({
            "name": crate.name,
            "version": crate.version,
            "license": crate.license,
            "license_file": crate.license_file,
            "repository": crate.repository,
            "manifest_path": str(directory / "Cargo.toml"),
            "source": "registry+https://github.com/rust-lang/crates.io-index",
        })
    return {"packages": packages}


FIXTURE_DIR = Path(__file__).resolve().parent / "fixtures" / "licenses"


def _fixture(name: str) -> str:
    """Đọc một fixture toàn văn license (nguyên văn, xem `tools/fixtures/licenses/`)."""
    return (FIXTURE_DIR / name).read_text(encoding="utf-8")


#: Toàn văn MIT chuẩn của SPDX (có placeholder `<year> <copyright holders>`).
MIT_TEXT = _fixture("mit-spdx.txt")

#: Toàn văn Apache-2.0 (nguyên văn trong crate `ab_glyph 0.2.32`).
APACHE_TEXT = _fixture("apache-2.0.txt")

#: File `license-mit` nguyên văn của crate `windows-link 0.2.1` (thông báo bản quyền
#: **không ghi năm**); `windows-sys 0.61.2` có file giống hệt.
WINDOWS_MIT_TEXT = _fixture("windows-link-0.2.1-license-mit.txt")


class PackageToolTests(unittest.TestCase):
    """Tự kiểm công cụ đóng gói (chạy bằng `python3 tools/check_package.py test`)."""

    def _valid_package(self, root: Path) -> Path:
        package = root / "boom-rs-dev-linux-x86_64"
        (package / "bin").mkdir(parents=True)
        (package / "data" / "rules").mkdir(parents=True)
        (package / "data" / "l10n").mkdir(parents=True)
        (package / "assets" / "fonts").mkdir(parents=True)
        (package / "bin" / "boom-client").write_text("bin", encoding="utf-8")
        (package / "bin" / "boom-server").write_text("bin", encoding="utf-8")
        (package / "bin" / "boom-assets").write_text("bin", encoding="utf-8")
        (package / "data" / "rules" / "v1-dev.toml").write_text("[profile]\n", encoding="utf-8")
        (package / "data" / "l10n" / "vi-v1.toml").write_text("[strings]\n", encoding="utf-8")
        (package / "README.md").write_text("# boom-rs\n", encoding="utf-8")
        (package / "assets" / "fonts" / "OFL.txt").write_text("OFL 1.1\n", encoding="utf-8")
        (package / "assets" / "fonts" / "BeVietnamPro-Regular.ttf").write_bytes(b"font")
        (package / "LICENSES").write_text(
            "\n".join(f"# {title}" for title in PART_TITLES), encoding="utf-8")
        return package

    def test_check_accepts_valid_package(self):
        with _TempDir() as root:
            package = self._valid_package(Path(root))
            self.assertEqual(check_package(package), [])

    def test_check_rejects_game_assets(self):
        cases = {
            "atlas/page-000.png": "atlas/page-000.png",
            "data/Village00.map": "data/Village00.map",
            "bundle/bundle.toml": "bundle/bundle.toml",
            "data/rules/../../sound.ogg": "sound.ogg",
        }
        for relative, expected in cases.items():
            with self.subTest(relative=relative), _TempDir() as root:
                package = self._valid_package(Path(root))
                target = package / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("x", encoding="utf-8")
                problems = check_package(package)
                self.assertTrue(problems, "phải bắt được file cấm")
                self.assertTrue(any(expected in problem for problem in problems),
                                f"thông điệp phải nêu {expected}: {problems}")

    def test_check_rejects_missing_parts(self):
        with _TempDir() as root:
            package = self._valid_package(Path(root))
            (package / "LICENSES").unlink()
            (package / "assets" / "fonts" / "OFL.txt").unlink()
            (package / "assets" / "fonts" / "BeVietnamPro-Regular.ttf").unlink()
            (package / "bin" / "boom-server").unlink()
            problems = check_package(package)
            joined = " | ".join(problems)
            self.assertIn("LICENSES", joined)
            self.assertIn("assets/fonts/OFL.txt", joined)
            self.assertIn("font", joined)
            self.assertIn("boom-server", joined)

    def test_licenses_have_full_text_and_copyright(self):
        with _TempDir() as root:
            root = Path(root)
            alpha = _make_crate(root, "alpha", "1.0.0", {
                "LICENSE-MIT": "Copyright (c) 2020 Alpha\n\n" + MIT_TEXT})
            beta = _make_crate(root, "beta", "2.0.0", {
                "LICENSE-APACHE": APACHE_TEXT})
            crates = [
                Crate("alpha", "1.0.0", "MIT", None, "https://example.invalid/alpha", alpha),
                Crate("beta", "2.0.0", "Apache-2.0", None, "https://example.invalid/beta", beta),
            ]
            report = build_report(crates)
            self.assertEqual(report.problems, [])
            self.assertEqual(sorted(report.texts), ["Apache-2.0", "MIT"])
            text = render_licenses(report)
            self.assertIn("Permission is hereby granted, free of charge", text)
            self.assertIn("TERMS AND CONDITIONS FOR USE", text)
            self.assertIn("Copyright (c) 2020 Alpha", text)
            self.assertIn("alpha 1.0.0", text)
            for title in PART_TITLES:
                self.assertIn(title, text)

    def test_licenses_pick_or_branch_with_text(self):
        with _TempDir() as root:
            root = Path(root)
            crate_dir = _make_crate(root, "gamma", "0.1.0", {"LICENSE-MIT": MIT_TEXT})
            crates = [Crate("gamma", "0.1.0", "MIT OR LGPL-2.1-or-later", None, None, crate_dir)]
            report = build_report(crates)
            self.assertEqual(report.problems, [])
            self.assertEqual(report.choices[0].chosen, ["MIT"])
            self.assertIn("LGPL-2.1-or-later", render_licenses(report))

    def test_licenses_fail_without_any_text(self):
        with _TempDir() as root:
            root = Path(root)
            crate_dir = _make_crate(root, "delta", "0.1.0", {"README.md": "không có license"})
            crates = [Crate("delta", "0.1.0", "LGPL-2.1-or-later", None, None, crate_dir)]
            report = build_report(crates)
            self.assertTrue(report.problems)
            self.assertIn("LGPL-2.1-or-later", report.problems[0])

    def test_licenses_fail_when_crate_source_missing(self):
        with _TempDir() as root:
            root = Path(root)
            crates = [Crate("zeta", "1.0.0", "MIT", None, None, root / "khong-co")]
            report = build_report(crates)
            self.assertTrue(report.problems)
            self.assertIn("cargo fetch", report.problems[0])

    def test_licenses_metadata_roundtrip(self):
        with _TempDir() as root:
            root = Path(root)
            crate_dir = _make_crate(root, "eps", "1.2.3", {"LICENSE-MIT": MIT_TEXT})
            crate = Crate("eps", "1.2.3", "MIT", None, None, crate_dir)
            metadata = _metadata([(crate, crate_dir)])
            crates = crates_from_metadata(metadata)
            self.assertEqual(len(crates), 1)
            self.assertEqual(crates[0].key, "eps 1.2.3")
            self.assertEqual(build_report(crates).problems, [])

    # -- Nhận diện toàn văn license: đủ điều khoản, không tin tên file ------------------

    def test_detect_by_content_needs_all_clauses(self):
        """Chỉ file **đủ điều khoản** mới là toàn văn license."""
        self.assertEqual(detect_by_content(MIT_TEXT), ("MIT", "full"))
        self.assertEqual(detect_by_content(APACHE_TEXT), ("Apache-2.0", "full"))
        self.assertEqual(detect_by_content(WINDOWS_MIT_TEXT), ("MIT", "full"))
        # File cụt (chỉ còn câu đầu của MIT) không phải toàn văn.
        truncated = MIT_TEXT.split("\n\n")[2]
        self.assertNotEqual(detect_by_content(truncated)[1], "full")
        # File rỗng không phải license nào.
        self.assertEqual(detect_by_content(""), (None, "other"))
        # Thông báo trỏ tới toàn văn thì không phải toàn văn.
        notice = (
            'Licensed under the Apache License, Version 2.0 (the "License");\n'
            "you may not use this file except in compliance with the License.\n"
            "You may obtain a copy of the License at\n\n"
            "    http://www.apache.org/licenses/LICENSE-2.0\n"
        )
        self.assertEqual(detect_by_content(notice), ("Apache-2.0", "notice"))

    def test_classify_rejects_empty_truncated_and_mislabeled(self):
        """File rỗng/cụt/sai nhãn: không được coi là toàn văn, và phải báo lỗi."""
        cases = {
            "file rỗng": "",
            "file cụt": "truncated license",
            "thiếu điều khoản miễn trừ bảo đảm": MIT_TEXT.split("THE SOFTWARE IS PROVIDED")[0],
            "sai nhãn (Apache trong file MIT)": APACHE_TEXT,
        }
        for label, content in cases.items():
            with self.subTest(label=label), _TempDir() as root:
                root = Path(root)
                crate_dir = _make_crate(root, "eta", "1.0.0", {"LICENSE-MIT": content})
                crates = [Crate("eta", "1.0.0", "MIT", None, None, crate_dir)]
                report = build_report(crates)
                self.assertTrue(report.problems, f"phải báo lỗi: {label}")
                self.assertNotIn("MIT", report.texts,
                                 f"không được dùng làm toàn văn: {label}")
                self.assertNotIn("Apache-2.0", report.texts,
                                 f"không được dùng làm toàn văn: {label}")

    def test_classify_accepts_full_text_by_content(self):
        """Ca dương: file toàn văn hợp lệ thì nhận đúng license, không cần tên file."""
        with _TempDir() as root:
            root = Path(root)
            crate_dir = _make_crate(root, "theta", "1.0.0", {"LICENSE": MIT_TEXT})
            classified = classify_license_file(crate_dir / "LICENSE")
            self.assertEqual(classified.kind, "full")
            self.assertEqual(classified.ident, "MIT")

    # -- Dòng bản quyền: giữ cả thông báo không ghi năm ------------------------------

    def test_copyright_keeps_notice_without_year(self):
        """Thông báo bản quyền không ghi năm vẫn phải được giữ (review #128)."""
        self.assertEqual(copyright_lines(WINDOWS_MIT_TEXT),
                         ["Copyright (c) Microsoft Corporation."])

    def test_copyright_ignores_license_terms(self):
        """Câu trong điều khoản license không phải dòng bản quyền."""
        self.assertEqual(copyright_lines(APACHE_TEXT), [])
        lines = copyright_lines(APACHE_TEXT + "\n(c) You must retain, in the Source form.\n")
        self.assertEqual(lines, [])

    def test_licenses_keep_copyright_without_year(self):
        """Regression review #128: windows-link/windows-sys giữ dòng bản quyền không năm."""
        with _TempDir() as root:
            root = Path(root)
            crates = []
            for name, version in (("windows-link", "0.2.1"), ("windows-sys", "0.61.2")):
                directory = _make_crate(root, name, version, {
                    "license-mit": WINDOWS_MIT_TEXT,
                    "license-apache-2.0": APACHE_TEXT,
                })
                crates.append(Crate(name, version, "MIT OR Apache-2.0", None,
                                    f"https://example.invalid/{name}", directory))
            report = build_report(crates)
            self.assertEqual(report.problems, [])
            text = render_licenses(report)
            self.assertIn("Copyright (c) Microsoft Corporation.", text)
            self.assertIn("windows-link 0.2.1", text)
            self.assertIn("windows-sys 0.61.2", text)


class _TempDir:
    """Thư mục tạm (tự xoá khi thoát) — không dùng `tempfile` để tránh phụ thuộc ngoài."""

    def __enter__(self) -> str:
        import tempfile
        self._dir = tempfile.TemporaryDirectory(prefix="boom-check-package-")
        return self._dir.name

    def __exit__(self, *exc) -> None:
        self._dir.cleanup()


def cmd_test(args) -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(PackageToolTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


# --------------------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Sinh LICENSES và kiểm bộ đóng gói phát hành (V1-036 phần 2).")
    sub = parser.add_subparsers(dest="command", required=True)

    licenses = sub.add_parser("licenses", help="sinh file LICENSES từ cargo metadata")
    licenses.add_argument("--out", required=True, help="đường dẫn file LICENSES cần ghi")
    licenses.add_argument("--metadata", help="file JSON `cargo metadata` (mặc định: chạy cargo)")
    licenses.set_defaults(func=cmd_licenses)

    check = sub.add_parser("check", help="kiểm bộ đóng gói (file cấm, thành phần bắt buộc)")
    check.add_argument("package_dir", help="thư mục gói phát hành")
    check.set_defaults(func=cmd_check)

    test = sub.add_parser("test", help="tự kiểm công cụ này")
    test.set_defaults(func=cmd_test)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
