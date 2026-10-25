"""Bộ công cụ gói phát hành: dựng/mở gói, sinh `LICENSES`, kiểm bộ đóng gói và
smoke chạy binary trong gói (V1-036).

Dùng trong `.github/workflows/release.yml` và `.github/workflows/ci.yml`:

    python3 tools/check_package.py pack --bin-dir target/release --out dist/<gói>
    python3 tools/check_package.py licenses --out dist/<gói>/LICENSES
    python3 tools/check_package.py check dist/<gói>
    python3 tools/check_package.py unpack dist/<gói>.tar.gz --out dist/smoke
    python3 tools/check_package.py smoke dist/smoke/<gói> --log dist/smoke-log.txt
    python3 tools/check_package.py test

Nguyên tắc:

- **Offline.** Không gọi mạng. Toàn văn license lấy **nguyên văn** từ file
  `LICENSE*`/`COPYING*`/`NOTICE*`/`UNLICENSE` trong nguồn crate của cargo registry
  (cargo fetch đã tải về lúc build). Không tự viết lại toàn văn license và không
  suy diễn license mà không có file nguồn: license không tìm thấy toàn văn thì
  phần phát hành báo lỗi, không lấp chỗ trống bằng văn bản tự chế.
- **Smoke là chạy binary thật trong gói.** `smoke` chạy `boom-server`,
  `boom-client`, `boom-assets` lấy từ `bin/` của gói: `--help`, `content`,
  `health --offline`, và các ca **thiếu file phải báo lỗi rõ** (rules/l10n/bundle/
  pack). Build xanh **không** đồng nghĩa gói chạy được (V1-036).
- **Một nguồn chuẩn cho bố cục gói.** `pack` dựng gói theo
  `docs/specification/RELEASE.md` §2 và `check` kiểm đúng danh sách file cấm
  trong cùng tài liệu đó.
- **Xác minh nội dung, không tin tên file.** Một file chỉ là *toàn văn* license khi
  nội dung khớp **mẫu toàn văn chuẩn** của license đó (`LICENSE_TEMPLATES`, xem
  `tools/fixtures/licenses/`): mọi **điều khoản** của mẫu phải có **trọn vẹn**
  trong file, không chỉ nhờ tên file, câu đầu hay tiêu đề mục. File rỗng/cụt, file
  tên khai license nhưng chứa license khác, hay file có tên nhưng nội dung không xác
  minh được → công cụ **báo lỗi và không sinh gói**.
- **Đối chiếu nghiêm, không dùng ngưỡng mờ.** Mỗi điều khoản của mẫu là một cụm từ
  **liên tiếp**; nội dung crate phải chứa đúng cụm từ đó (sau khi chuẩn hoá khoảng
  trắng và dấu câu). Không có "tỉ lệ từ khớp" nào che được việc mất một điều khoản.
- **Chỉ khác biệt đã liệt kê rõ mới được phép.** (1) **Dòng bản quyền** và phần đầu
  (tên, phiên bản, URL) — mỗi crate ghi riêng, nằm ngoài phần điều khoản; (2) **tên
  chủ thể** — cụm từ chung trong điều khoản ("the copyright holder") được crate thay
  bằng tên thật (vd "Xiph.org Foundation"). Điều khoản viết lại phải được **liệt kê
  tường minh** trong `ClauseVariant`; không liệt kê ⇒ file bị bác bỏ.
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
import platform
import re
import shutil
import subprocess
import sys
import unittest
from collections.abc import Callable
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

#: Thư mục chứa toàn văn chuẩn (mẫu đối chiếu) và fixture cho tự kiểm.
FIXTURE_DIR = Path(__file__).resolve().parent / "fixtures" / "licenses"

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
REQUIRED_DIRS = ("bin", "data/rules", "data/maps", "data/l10n", "assets/fonts")

#: File bắt buộc có trong gói phát hành.
#: `data/maps/village00-kr.toml` là bảng phân loại ô server nạp mặc định
#: (`DEFAULT_MAP_PROFILE_PATH`); thiếu thì `boom-server serve` từ gói không khởi động (#305).
REQUIRED_FILES = ("README.md", "LICENSES", "assets/fonts/OFL.txt", "data/maps/village00-kr.toml")

#: Binary bắt buộc trong `bin/` (Windows thêm `.exe`).
REQUIRED_BINARIES = ("boom-client", "boom-server", "boom-assets")

#: Marker đọc phần README chạy nhanh trong `docs/specification/RELEASE.md`.
#: README trong gói **trích nguyên văn** từ đó (một nguồn chuẩn).
QUICKSTART_START = "<!-- QUICKSTART:START -->"
QUICKSTART_END = "<!-- QUICKSTART:END -->"

#: Thư mục nguồn trong repo và mẫu file chép vào gói: `data/rules` và `data/l10n` là
#: **file dữ liệu** của bản phát hành, `assets/fonts` là font OFL (không phải asset
#: game). Không chép pack/bundle/asset game — người dùng tự dựng bundle.
PACK_SOURCES: tuple[tuple[str, str], ...] = (
    ("data/rules", "*.toml"),
    ("data/maps", "*.toml"),
    ("data/l10n", "*.toml"),
    ("assets/fonts", "*"),
)

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


@dataclass(frozen=True)
class ClauseVariant:
    """Biến thể **điều khoản** được chấp nhận, liệt kê rõ kèm nguồn.

    Mẫu chuẩn là văn bản SPDX; crate thật đôi khi viết khác một điều khoản. Chỉ
    những khác biệt **được liệt kê ở đây** mới được chấp nhận — khác biệt chưa
    liệt kê ⇒ file không xác minh được toàn văn ⇒ công cụ báo lỗi, không tính là
    toàn văn.
    """

    clause: str  # điều khoản gốc trong mẫu chuẩn (nguyên văn)
    variant: str  # câu chữ thay thế được chấp nhận
    source: str  # nguồn (crate thật) để đối chiếu


@dataclass(frozen=True)
class LicenseTemplate:
    """Mẫu **toàn văn chuẩn** của một license SPDX, dùng để đối chiếu file license crate.

    - `terms_start`: mốc bắt đầu phần **điều khoản**. Phần trước mốc này là tiêu đề,
      phiên bản, **dòng bản quyền** và URL — không phải điều khoản nên không bắt buộc
      (crate ghi mỗi kiểu khác nhau).
    - `parts`: các phần của mẫu, mỗi phần là `(tên file trong `tools/fixtures/licenses/`,
      mốc kết thúc hoặc `None`) — mốc kết thúc dùng để cắt phần phụ (appendix) không
      thuộc điều khoản.
    - `slots`: các cụm từ trong mẫu được phép thay bằng **tên chủ thể** thật (vd "the
      copyright holder" → "Xiph.org Foundation"). Đây là khác biệt **được liệt kê rõ**.
    - `variants`: các điều khoản được phép viết theo câu chữ khác (liệt kê rõ, kèm nguồn).
    """

    ident: str
    terms_start: str
    parts: tuple[tuple[str, str | None], ...]
    slots: tuple[str, ...] = ()
    variants: tuple[ClauseVariant, ...] = ()


#: Mẫu toàn văn chuẩn của các license SPDX mà tool xác minh được.
#:
#: Một file chỉ là **toàn văn** license khi nội dung nó có **đủ mọi đơn vị điều khoản**
#: của mẫu ứng với — không suy từ tên file, không chỉ nhìn câu đầu, không chỉ nhìn
#: tiêu đề mục. Vì vậy file bị cắt cụt hay file chỉ có tiêu đề mục đều bị bác bỏ.
#:
#: Thứ tự quan trọng: id **cụ thể hơn** đứng trước (`Apache-2.0 WITH LLVM-exception`
#: trước `Apache-2.0`), vì file kèm ngoại lệ LLVM cũng chứa đủ điều khoản Apache-2.0.
LICENSE_TEMPLATES: tuple[LicenseTemplate, ...] = (
    LicenseTemplate(
        "Apache-2.0 WITH LLVM-exception", "TERMS AND CONDITIONS FOR USE",
        (("apache-2.0-spdx.txt", "END OF TERMS AND CONDITIONS"),
         ("apache-2.0-llvm-exception.txt", None)),
    ),
    LicenseTemplate(
        "Apache-2.0", "TERMS AND CONDITIONS FOR USE",
        (("apache-2.0-spdx.txt", "END OF TERMS AND CONDITIONS"),),
    ),
    LicenseTemplate(
        "MIT", "Permission is hereby granted", (("mit-spdx.txt", None),),
        variants=(
            ClauseVariant(
                clause=("The above copyright notice and this permission notice shall be "
                        "included in all copies or substantial portions of the Software."),
                variant=("The above copyright notice and this permission notice (including "
                         "the next paragraph) shall be included in all copies or "
                         "substantial portions of the Software."),
                source="crate `bytemuck 1.25.2` (crates.io), file `LICENSE-MIT`",
            ),
        ),
    ),
    LicenseTemplate(
        "BSD-3-Clause", "Redistribution and use in source and binary forms",
        (("bsd-3-clause.txt", None),),
        slots=("copyright holder",),
    ),
    LicenseTemplate(
        "BSD-2-Clause", "Redistribution and use in source and binary forms",
        (("bsd-2-clause.txt", None),),
        slots=("copyright holder",),
    ),
    LicenseTemplate("0BSD", "Permission to use, copy, modify", (("0bsd.txt", None),)),
    LicenseTemplate("Zlib", "This software is provided", (("zlib.txt", None),)),
    LicenseTemplate(
        "Unlicense", "This is free and unencumbered", (("unlicense.txt", None),),
    ),
    LicenseTemplate("Unicode-3.0", "NOTICE TO USER", (("unicode-3.0.txt", None),)),
    LicenseTemplate(
        "LGPL-2.1-or-later", "The licenses for most software",
        (("lgpl-2.1-or-later.txt", None),),
    ),
)

#: Id này là biến thể **nghiêm hơn** của id kia (văn bản chứa thêm điều khoản): file
#: khớp cả hai thì chỉ tính là toàn văn của id nghiêm hơn — không phát hành toàn văn
#: BSD-3-Clause dưới nhãn BSD-2-Clause (văn bản BSD-3 đã chứa đủ điều khoản BSD-2).
LICENSE_SUPERSEDES: dict[str, str] = {
    "Apache-2.0 WITH LLVM-exception": "Apache-2.0",
    "BSD-3-Clause": "BSD-2-Clause",
}

#: Số ký tự (đã chuẩn hoá) tối thiểu để một đơn vị được tính là **điều khoản**. Đơn vị
#: ngắn hơn là tiêu đề mục ("1. Definitions.") — không mang nghĩa điều khoản.
MIN_UNIT_CHARS = 40

#: Số từ tối đa mà một **tên chủ thể** (slot) được chiếm. Chủ thể là tên riêng
#: ("Xiph.org Foundation") nên ngắn; dài hơn thì không còn là tên nữa.
MAX_SLOT_TOKENS = 6

#: Ký hiệu một đoạn "tên chủ thể" trong pattern điều khoản.
SLOT = "slot"

#: Ký hiệu một đoạn văn bản cố định trong pattern điều khoản.
LITERAL = "literal"

#: Cụm từ bắt buộc của **thông báo** license (file chỉ trỏ tới toàn văn ở nơi khác).
#: File như vậy không phải toàn văn nên không dùng làm nguồn toàn văn, nhưng vẫn hợp lệ.
LICENSE_NOTICE_CLAUSES: dict[str, tuple[str, ...]] = {
    "Apache-2.0": (
        "licensed under the apache license, version 2.0",
        "apache.org/licenses/license-2.0",
    ),
}

#: Dòng bản quyền; miniquad dùng `@ 2019-2020 Tác giả` thay cho ký hiệu ©.
COPYRIGHT_MARKER_RE = re.compile(
    r"^\s*(?:\(c\)|©|copyright\b|@(?=\s+(?:1\d{3}|20\d{2})\b))",
    re.IGNORECASE,
)

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

#: Placeholder bất kỳ trong mẫu chuẩn (`<year>`, `<owner>`, `[yyyy]`): crate điền
#: giá trị thật nên đơn vị điều khoản chứa placeholder không thể đòi khớp.
UNIT_PLACEHOLDER_RE = re.compile(r"<[^<>]{1,60}>|\[[^\[\]]{1,60}\]")

#: Câu trong văn bản license: kết thúc bằng dấu chấm, câu sau bắt đầu bằng chữ hoa.
SENTENCE_RE = re.compile(r"(?<=[.!?])\s+(?=[A-Z\"'(])")


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
    idents: list[str] = field(default_factory=list)  # mọi id mà nội dung là toàn văn


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


def normalize_words(text: str) -> list[str]:
    """Chuẩn hoá văn bản thành danh sách **từ**: bỏ dấu câu, hạ chữ thường.

    So khớp theo từ nên nguồn crate xuống dòng khác nhau vẫn khớp với mẫu chuẩn.
    """
    words: list[str] = []
    for token in text.split():
        word = re.sub(r"[^a-z0-9]+", "", token.lower())
        if word:
            words.append(word)
    return words


def flatten(words: list[str]) -> str:
    """Ghép danh sách từ thành chuỗi để so khớp theo cụm từ liên tiếp."""
    return " ".join(words)


def clauses_present(clauses: tuple[str, ...], flat: str) -> bool:
    """Đủ tất cả điều khoản `clauses` trong văn bản đã chuẩn hoá `flat` không."""
    return all(flatten(normalize_words(clause)) in flat for clause in clauses)


def split_sentences(text: str) -> list[str]:
    """Chia văn bản thành câu (kết thúc bằng dấu chấm, câu sau bắt đầu bằng chữ hoa)."""
    return [part.strip() for part in SENTENCE_RE.split(text) if part.strip()]


@lru_cache(maxsize=None)
def template_text(template: LicenseTemplate) -> str:
    """Ghép các phần của một mẫu toàn văn, cắt bỏ phần phụ (appendix)."""
    parts: list[str] = []
    for filename, end_marker in template.parts:
        text = (FIXTURE_DIR / filename).read_text(encoding="utf-8")
        if end_marker:
            index = text.find(end_marker)
            if index >= 0:
                text = text[:index]
        parts.append(text)
    return "\n\n".join(parts)


@lru_cache(maxsize=None)
def template_clauses(template: LicenseTemplate) -> list[str]:
    """Các **điều khoản** (câu) của một mẫu toàn văn, nguyên văn.

    Mẫu được chia thành đoạn, đoạn thành câu; câu quá ngắn là tiêu đề mục nên bỏ,
    câu chứa placeholder SPDX (`<year>`, `<owner>`) cũng bỏ — crate điền giá trị
    thật của mình nên không thể đòi khớp placeholder.
    """
    text = template_text(template)
    index = text.find(template.terms_start)
    if index < 0:
        return []
    clauses: list[str] = []
    for paragraph in re.split(r"\n\s*\n", text[index:]):
        for sentence in split_sentences(paragraph):
            if UNIT_PLACEHOLDER_RE.search(sentence):
                continue
            if len(flatten(normalize_words(sentence))) < MIN_UNIT_CHARS:
                continue
            clauses.append(sentence)
    return clauses


def find_block(flat: str, block: str, start: int) -> int:
    """Vị trí (ký tự) của cụm từ `block` trong `flat`, đúng ranh giới từ, từ `start`.

    `flat` là văn bản đã chuẩn hoá (các từ nối bằng một dấu cách). Trả về -1 nếu
    không có. Cụm từ phải nằm trọn vẹn giữa hai ranh giới từ ⇒ không khớp nhầm vào
    giữa một từ.
    """
    if not block:
        return start
    # `padded` thêm một dấu cách ở đầu để cụm từ bắt đầu ngay đầu văn bản cũng
    # khớp; chỉ số trả về là chỉ số trong `flat` (nhờ dấu cách đứng trước cụm từ).
    padded = " " + flat + " "
    needle = " " + block + " "
    return padded.find(needle, start)


def clause_pattern(clause: str, slots: tuple[str, ...]) -> list[tuple[str, str]]:
    """Chuyển một điều khoản của mẫu thành **pattern** để đối chiếu.

    Pattern là danh sách các đoạn: đoạn văn bản cố định (`literal`) và các khoảng
    trống dành cho **tên chủ thể** (`slot`). Mọi đoạn cố định phải xuất hiện **trọn
    vẹn, liên tiếp** trong nội dung đối chiếu ⇒ mất một cụm từ trong điều khoản là
    bị bác bỏ (không có ngưỡng mờ nào che được).
    """
    tokens = normalize_words(clause)
    slot_tokens = [normalize_words(slot) for slot in slots]
    pattern: list[tuple[str, str]] = []
    index = 0
    while index < len(tokens):
        hit = None
        for candidate in slot_tokens:
            if candidate and tokens[index:index + len(candidate)] == candidate:
                hit = candidate
                break
        if hit is not None:
            if not pattern or pattern[-1][0] != SLOT:
                pattern.append((SLOT, flatten(hit)))
            index += len(hit)
            continue
        if pattern and pattern[-1][0] == LITERAL:
            pattern[-1] = (LITERAL, f"{pattern[-1][1]} {tokens[index]}")
        else:
            pattern.append((LITERAL, tokens[index]))
        index += 1
    return pattern


def clause_patterns(template: LicenseTemplate, clause: str) -> list[list[tuple[str, str]]]:
    """Các pattern được chấp nhận cho một điều khoản: mẫu chuẩn + biến thể đã liệt kê."""
    patterns = [clause_pattern(clause, template.slots)]
    unit = flatten(normalize_words(clause))
    for variant in template.variants:
        if flatten(normalize_words(variant.clause)) != unit:
            continue
        patterns.append([(LITERAL, flatten(normalize_words(variant.variant)))])
    return patterns


def match_pattern(pattern: list[tuple[str, str]], flat: str, start: int,
                  vocabulary: set[str]) -> int:
    """Vị trí (ký tự) trong `flat` khớp `pattern`, tìm từ `start`; -1 nếu không khớp.

    Khoảng `slot` được điền bằng **tên chủ thể**: một cụm từ ngắn (≤ `MAX_SLOT_TOKENS`
    từ) có ít nhất một từ **không có trong mẫu chuẩn** (tên riêng của crate). Nhờ vậy
    thay "the copyright holder" bằng "Xiph.org Foundation" là hợp lệ, còn xoá hẳn
    cụm từ đó (chỉ còn từ của mẫu) thì bị bác bỏ.
    """
    if not pattern:
        return start
    kind, text = pattern[0]
    if kind == LITERAL:
        index = find_block(flat, text, start)
        if index < 0:
            return -1
        return match_pattern(pattern[1:], flat, index + len(text), vocabulary)
    # Khoảng tên chủ thể: đoạn cố định kế tiếp phải nằm ngay sau tên chủ thể.
    if len(pattern) < 2 or pattern[1][0] != LITERAL:
        return -1
    following = pattern[1][1]
    index = start
    while True:
        index = find_block(flat, following, index)
        if index < 0:
            return -1
        name = flat[start:index].strip()
        if (name == text
                or (1 <= len(name.split()) <= MAX_SLOT_TOKENS
                    and any(token not in vocabulary for token in name.split()))):
            matched = match_pattern(pattern[2:], flat, index + len(following), vocabulary)
            if matched >= 0:
                return matched
        index += 1


def match_template(template: LicenseTemplate, text: str) -> bool:
    """Nội dung `text` có phải **toàn văn** license theo mẫu chuẩn không.

    Đối chiếu **nghiêm**: mọi điều khoản của mẫu phải có trong nội dung, mỗi điều
    khoản là một cụm từ **liên tiếp** khớp tuyệt đối (sau khi chuẩn hoá khoảng trắng
    và dấu câu). Chỉ hai khác biệt được phép, đều **liệt kê rõ** trong mẫu: dòng
    bản quyền ở phần đầu (nằm ngoài phần điều khoản) và *tên chủ thể* thay cho cụm
    từ chung trong điều khoản. Biến thể khác ⇒ file không phải toàn văn.
    """
    flat = flatten(normalize_words(text))
    if flatten(normalize_words(template.terms_start)) not in flat:
        return False
    clauses = template_clauses(template)
    if not clauses:
        return False
    vocabulary = set(normalize_words(template_text(template)))
    cursor = 0
    for clause in clauses:
        matched = -1
        for pattern in clause_patterns(template, clause):
            matched = match_pattern(pattern, flat, cursor, vocabulary)
            if matched >= 0:
                break
        if matched < 0:
            return False
        # Điều khoản sau phải nằm **sau** điều khoản này (giữ đúng thứ tự điều khoản).
        cursor = matched + 1
    return True


def detect_full_by_content(text: str) -> list[str]:
    """Các SPDX id mà nội dung `text` là **toàn văn** (đối chiếu mẫu chuẩn)."""
    matched = [t.ident for t in LICENSE_TEMPLATES if match_template(t, text)]
    superseded = {general for specific, general in LICENSE_SUPERSEDES.items()
                  if specific in matched}
    return [ident for ident in matched if ident not in superseded]


def detect_notice(text: str) -> str | None:
    """SPDX id của thông báo trỏ tới toàn văn license (nếu nội dung là thông báo)."""
    flat = flatten(normalize_words(text))
    for ident, clauses in LICENSE_NOTICE_CLAUSES.items():
        if clauses_present(clauses, flat):
            return ident
    return None


def detect_by_content(text: str) -> tuple[str | None, str]:
    """Nhận diện SPDX id từ **nội dung** file license.

    Trả về `(spdx_id, kind)` với `kind`:
    - `full`: toàn văn license — nội dung khớp **mẫu chuẩn** của license đó, đủ mọi
      điều khoản ([LICENSE_TEMPLATES](tools/check_package.py)), không chỉ câu đầu
      hay tiêu đề mục;
    - `notice`: thông báo "xem license ở đường dẫn khác" (không phải toàn văn);
    - `other`: không nhận ra nội dung này là license nào.
    """
    matched = detect_full_by_content(text)
    if matched:
        return matched[0], "full"
    notice = detect_notice(text)
    if notice:
        return notice, "notice"
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
    matched = detect_full_by_content(text)
    if matched:
        # File có thể chứa nhiều license (vd MIT + Apache): ưu tiên id cùng họ với
        # tên file, không thì id đầu tiên.
        detected = matched[0]
        for ident in matched:
            if claimed and LICENSE_FAMILY.get(ident, ident) == claimed:
                detected = ident
                break
        if claimed and LICENSE_FAMILY.get(detected, detected) != claimed:
            return LicenseFile(path=path, ident=detected, kind="mislabeled",
                               text=text, claimed=claimed, idents=matched)
        return LicenseFile(path=path, ident=detected, kind="full", text=text,
                            claimed=claimed, idents=matched)
    notice = detect_notice(text)
    if notice:
        if claimed and LICENSE_FAMILY.get(notice, notice) != claimed:
            return LicenseFile(path=path, ident=notice, kind="mislabeled",
                               text=text, claimed=claimed)
        return LicenseFile(path=path, ident=notice, kind="notice", text=text, claimed=claimed)
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
    """Crate ngoài workspace, kể cả dependency vendored/path, từ `cargo metadata`."""
    crates: list[Crate] = []
    members = set(metadata.get("workspace_members", []))
    for package in metadata.get("packages", []):
        if package.get("id") in members:
            continue  # Chỉ bỏ crate của workspace, không bỏ dependency path/vendored.
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
            if classified.kind == "full":
                # File chứa nhiều license (vd MIT + Apache trong cùng một file) là
                # nguồn toàn văn cho **mọi** id mà nội dung nó khớp mẫu chuẩn.
                for ident in classified.idents:
                    candidates.setdefault(ident, []).append(LicenseText(
                        ident=ident,
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
    out.append("LICENSE*/COPYING*/NOTICE* trong nguồn crate (registry hoặc dependency path).")
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
    out.append("File license kèm trong nguồn crate (registry hoặc dependency path), và dòng bản quyền trích từ đó.")
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
                label = ", ".join(item.idents) or (item.ident or "không nhận ra")
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

    for name in ("data/rules", "data/maps", "data/l10n"):
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
# Dựng gói / mở gói
# --------------------------------------------------------------------------------------


def quickstart_text(source_root: Path) -> str:
    """Phần README chạy nhanh trong `docs/specification/RELEASE.md` (một nguồn chuẩn)."""
    doc = (source_root / "docs" / "specification" / "RELEASE.md").read_text(encoding="utf-8")
    start = doc.index(QUICKSTART_START) + len(QUICKSTART_START)
    end = doc.index(QUICKSTART_END)
    return doc[start:end].strip() + "\n"


def binary_source(bin_dir: Path, name: str) -> Path | None:
    """Binary `name` trong thư mục build (Windows thêm `.exe`); `None` nếu không có."""
    for candidate in (bin_dir / name, bin_dir / f"{name}.exe"):
        if candidate.is_file():
            return candidate
    return None


def package_binary(package_dir: Path, name: str) -> Path:
    """Đường dẫn binary `name` trong gói phát hành (Windows thêm `.exe`)."""
    candidate = package_dir / "bin" / name
    if candidate.is_file():
        return candidate
    return candidate.with_suffix(".exe")


def pack_package(bin_dir: Path, out_dir: Path, source_root: Path) -> list[str]:
    """Dựng gói phát hành theo bố cục `docs/specification/RELEASE.md` §2.

    Gom `bin/`, `data/rules/`, `data/l10n/`, `assets/fonts/` và `README.md` (trích
    từ §8 của tài liệu đó). **Không** kèm `LICENSES` (sinh bằng lệnh `licenses`),
    **không** kèm pack/bundle/asset game.

    Trả về danh sách lỗi (rỗng = đạt).
    """
    problems: list[str] = []
    binaries = out_dir / "bin"
    binaries.mkdir(parents=True, exist_ok=True)
    for name in REQUIRED_BINARIES:
        source = binary_source(bin_dir, name)
        if source is None:
            problems.append(f"không có binary {name} trong {bin_dir}")
            continue
        shutil.copy2(source, binaries / source.name)
    for relative, pattern in PACK_SOURCES:
        source_dir = source_root / relative
        target = out_dir / relative
        target.mkdir(parents=True, exist_ok=True)
        if not source_dir.is_dir():
            problems.append(f"không có thư mục nguồn trong repo: {relative}")
            continue
        for item in sorted(source_dir.glob(pattern)):
            if item.is_file():
                shutil.copy2(item, target / item.name)
    try:
        text = quickstart_text(source_root)
    except (OSError, ValueError) as error:
        problems.append(f"không trích được README chạy nhanh: {error}")
    else:
        (out_dir / "README.md").write_text(text, encoding="utf-8")
    return problems


def unpack_archive(archive: Path, out_dir: Path) -> list[str]:
    """Giải nén archive phát hành (`.zip`/`.tar.gz`) vào thư mục đích.

    Dùng để **mở gói đã đóng** rồi chạy binary trong đó: kiểm archive thật, không
    chỉ thư mục dựng tạm. Trả về danh sách lỗi (rỗng = đạt).
    """
    if not archive.is_file():
        return [f"không có file archive: {archive}"]
    out_dir.mkdir(parents=True, exist_ok=True)
    try:
        shutil.unpack_archive(str(archive), str(out_dir))
    except (OSError, ValueError) as error:
        return [f"không mở được archive {archive}: {error}"]
    return []


# --------------------------------------------------------------------------------------
# Smoke: chạy binary thật trong gói (V1-036)
# --------------------------------------------------------------------------------------

#: Thời gian tối đa cho một lệnh smoke (giây).
SMOKE_TIMEOUT_S = 120


@dataclass(frozen=True)
class CommandResult:
    """Kết quả chạy một lệnh: mã thoát, stdout và stderr (tách riêng)."""

    code: int
    stdout: str
    stderr: str

    @property
    def output(self) -> str:
        """Output gộp (stdout + stderr) để kiểm nội dung thông điệp lỗi."""
        return self.stdout + ("\n" + self.stderr if self.stderr else "")


@dataclass(frozen=True)
class SmokeCase:
    """Một ca kiểm smoke: lệnh chạy và điều kiện phải đạt.

    - `ok_codes`: mã thoát **được chấp nhận** (vd ca thiếu file phải thoát `2`).
    - `must_contain`: cụm phải có trong output — dùng để kiểm lỗi báo **rõ**
      (đường dẫn + cách sửa), không chỉ "có lỗi".
    - `json_fields`/`json_expect`: stdout phải parse được JSON, có các field này
      và field có đúng giá trị.
    """

    key: str
    name: str
    argv: tuple[str, ...]
    ok_codes: tuple[int, ...]
    must_contain: tuple[str, ...] = ()
    must_not_contain: tuple[str, ...] = ()
    json_fields: tuple[str, ...] = ()
    json_expect: tuple[tuple[str, str], ...] = ()


#: Runner chạy lệnh: nhận ca kiểm + thư mục làm việc, trả về [`CommandResult`].
#: Tự kiểm dùng runner giả nên không cần binary thật.
CommandRunner = Callable[[SmokeCase, Path], CommandResult]


def run_command(case: SmokeCase, cwd: Path) -> CommandResult:
    """Chạy lệnh của ca smoke; giải mã output bằng UTF-8.

    Không dùng `text=True`: console Windows mặc định cp1252 sẽ hỏng thông điệp
    tiếng Việt. Đọc byte rồi tự giải mã UTF-8 ⇒ nội dung kiểm tra giống nhau mọi OS.
    """
    try:
        proc = subprocess.run(
            list(case.argv),
            capture_output=True,
            cwd=str(cwd),
            timeout=SMOKE_TIMEOUT_S,
        )
    except OSError as error:
        return CommandResult(127, "", f"không chạy được lệnh: {error}")
    except subprocess.TimeoutExpired:
        return CommandResult(124, "", f"lệnh không kết thúc sau {SMOKE_TIMEOUT_S}s")
    return CommandResult(
        proc.returncode,
        proc.stdout.decode("utf-8", "replace"),
        proc.stderr.decode("utf-8", "replace"),
    )


def smoke_cases(package_dir: Path, work_dir: Path) -> list[SmokeCase]:
    """Các ca smoke của một gói phát hành (acceptance V1-036).

    Nhóm ca:
    - binary trong gói chạy được (`--help`, `content`, `health --offline`);
    - thiếu rules/l10n/bundle ⇒ báo lỗi **rõ** (đường dẫn + cách sửa), mã thoát `2`;
    - thiếu pack ⇒ `boom-assets` báo lỗi và nêu thư mục pack.
    """
    server = package_binary(package_dir, "boom-server")
    client = package_binary(package_dir, "boom-client")
    assets = package_binary(package_dir, "boom-assets")
    rules = package_dir / "data" / "rules" / "v1-dev.toml"
    l10n = package_dir / "data" / "l10n" / "vi-v1.toml"
    missing_rules = work_dir / "thieu-file" / "rules.toml"
    missing_l10n = work_dir / "thieu-file" / "vi-v1.toml"
    missing_bundle = work_dir / "thieu-bundle"
    missing_pack = work_dir / "thieu-pack"
    bundle_out = work_dir / "bundle-out"
    return [
        SmokeCase(
            key="help-server",
            name="boom-server --help",
            argv=(str(server), "--help"),
            ok_codes=(0,),
            must_contain=("health", "content"),
        ),
        SmokeCase(
            key="help-client",
            name="boom-client --help",
            argv=(str(client), "--help"),
            ok_codes=(0,),
            must_contain=("--bundle",),
        ),
        SmokeCase(
            key="content-ok",
            name="boom-server content (rules + l10n trong gói)",
            argv=(str(server), "content", "--rules", str(rules), "--l10n", str(l10n)),
            ok_codes=(0,),
            must_contain=("rules:", "vi-VN"),
        ),
        SmokeCase(
            key="health-offline",
            name="boom-server health --offline",
            argv=(str(server), "health", "--offline"),
            # 0 = healthy; 1 = unhealthy (gói **không** kèm DB identity — `serve` tạo
            # sau). Mã 3 = lỗi cấu hình ⇒ lỗi thật, không được chấp nhận.
            ok_codes=(0, 1),
            json_fields=("scope", "server_version", "rules_hash", "content_sha256"),
            json_expect=(("scope", "offline"),),
        ),
        SmokeCase(
            key="missing-rules-server",
            name="boom-server content — thiếu file rules",
            argv=(str(server), "content", "--rules", str(missing_rules)),
            ok_codes=(2,),
            must_contain=("file rules", str(missing_rules), "sửa:"),
        ),
        SmokeCase(
            key="missing-rules-client",
            name="boom-client — thiếu file rules",
            argv=(str(client), "--rules", str(missing_rules)),
            ok_codes=(2,),
            must_contain=("file rules", str(missing_rules), "sửa:"),
        ),
        SmokeCase(
            key="missing-l10n-server",
            name="boom-server content — thiếu file chuỗi",
            argv=(str(server), "content", "--rules", str(rules), "--l10n", str(missing_l10n)),
            ok_codes=(2,),
            must_contain=("file chuỗi", str(missing_l10n), "sửa:"),
        ),
        SmokeCase(
            key="missing-bundle-server",
            name="boom-server content — thiếu bundle",
            argv=(str(server), "content", "--rules", str(rules), "--bundle", str(missing_bundle)),
            ok_codes=(2,),
            must_contain=("bundle", str(missing_bundle), "sửa:", "boom-assets bundle"),
        ),
        SmokeCase(
            key="missing-bundle-client",
            name="boom-client — thiếu bundle",
            argv=(str(client), "--bundle", str(missing_bundle)),
            ok_codes=(2,),
            must_contain=("bundle", str(missing_bundle), "sửa:", "pack-dir"),
        ),
        SmokeCase(
            key="missing-pack-assets",
            name="boom-assets bundle — thiếu pack",
            argv=(str(assets), "bundle", "--map", "Village00", "--out", str(bundle_out),
                  "--pack-dir", str(missing_pack)),
            ok_codes=(1,),
            must_contain=(str(missing_pack),),
        ),
    ]


def json_kind(value: object) -> str:
    """Tên kiểu JSON (tiếng Việt) của một giá trị đã parse — dùng cho thông điệp lỗi."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, (int, float)):
        return "số"
    if isinstance(value, str):
        return "chuỗi"
    if isinstance(value, list):
        return "array"
    if isinstance(value, dict):
        return "object"
    return type(value).__name__


def check_smoke_case(case: SmokeCase, result: CommandResult) -> list[str]:
    """Kiểm một ca smoke; trả về danh sách vi phạm (rỗng = đạt)."""
    problems: list[str] = []
    if result.code not in case.ok_codes:
        problems.append(
            f"{case.name}: mã thoát {result.code} (mong đợi {list(case.ok_codes)})")
    output = result.output
    for needle in case.must_contain:
        if needle not in output:
            problems.append(f"{case.name}: output không có {needle!r}")
    for needle in case.must_not_contain:
        if needle in output:
            problems.append(f"{case.name}: output có {needle!r} (không được có)")
    if case.json_fields or case.json_expect:
        try:
            body = json.loads(result.stdout.strip() or "{}")
        except json.JSONDecodeError as error:
            problems.append(f"{case.name}: stdout không phải JSON ({error})")
        else:
            # JSON đúng cú pháp nhưng **không phải object** ⇒ không có field nào để
            # kiểm, nên fingerprint/scope bắt buộc bị bỏ sót. Phải báo lỗi, không
            # im lặng bỏ qua nhánh này (review #168 vòng 1).
            if not isinstance(body, dict):
                problems.append(
                    f"{case.name}: JSON phải là object, thực tế là {json_kind(body)}")
            else:
                for name in case.json_fields:
                    if name not in body:
                        problems.append(f"{case.name}: JSON thiếu field {name!r}")
                for name, expected in case.json_expect:
                    if body.get(name) != expected:
                        problems.append(
                            f"{case.name}: JSON {name!r} = {body.get(name)!r} "
                            f"(mong đợi {expected!r})")
    return problems


def health_fingerprint(result: CommandResult) -> dict:
    """Fingerprint từ output `boom-server health --offline` (rỗng nếu không đọc được)."""
    try:
        body = json.loads(result.stdout.strip() or "{}")
    except json.JSONDecodeError:
        return {}
    if not isinstance(body, dict):
        return {}
    keys = ("scope", "status", "server_version", "wire_schema", "state_schema",
            "rules_profile_id", "rules_hash", "content_sha256", "protocol_limits_version")
    return {key: body[key] for key in keys if key in body}


def content_fingerprint(result: CommandResult) -> dict:
    """Profile rules và hash từ dòng `rules: …` của `boom-server content`.

    Đây là fingerprint của **file rules trong gói** (khác `health`, báo cấu hình
    mặc định của binary).
    """
    for line in result.stdout.splitlines():
        if not line.startswith("rules:"):
            continue
        fields = [part.strip() for part in line.split("·")]
        fingerprint: dict = {}
        if len(fields) >= 2:
            fingerprint["package_rules_profile"] = fields[0].removeprefix("rules:").strip()
            fingerprint["package_rules_hash"] = fields[1].removeprefix("hash").strip()
        return fingerprint
    return {}


@dataclass
class SmokeRun:
    """Kết quả một lượt smoke: các ca đã chạy, output và danh sách vi phạm."""

    package: Path
    version: str
    cases: list[SmokeCase] = field(default_factory=list)
    results: list[CommandResult] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)
    fingerprint: dict = field(default_factory=dict)
    files: list[tuple[str, str]] = field(default_factory=list)


def package_fingerprint(package_dir: Path) -> list[tuple[str, str]]:
    """SHA-256 của binary và file dữ liệu trong gói (ngón tay nội dung gói)."""
    entries: list[tuple[str, str]] = []
    for relative in ("bin", "data/rules", "data/maps", "data/l10n", "assets/fonts"):
        directory = package_dir / relative
        if not directory.is_dir():
            continue
        for path in sorted(directory.rglob("*")):
            if not path.is_file():
                continue
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            entries.append((path.relative_to(package_dir).as_posix(), digest))
    return entries


def smoke_package(
    package_dir: Path,
    runner: CommandRunner | None = None,
    work_dir: Path | None = None,
    version: str = "dev",
) -> SmokeRun:
    """Chạy smoke trên gói phát hành: binary thật trong `bin/` + ca lỗi thiếu file.

    `runner` cho phép tự kiểm không cần binary thật (runner giả). Lệnh chạy trong
    thư mục tạm để không ghi file nào vào gói.
    """
    runner = runner or run_command
    # Đường dẫn tuyệt đối: lệnh chạy trong thư mục tạm nên đường dẫn tương đối
    # (vd `dist/boom-rs-smoke`) sẽ không tìm thấy.
    package_dir = package_dir.resolve()
    run = SmokeRun(package=package_dir, version=version)
    for name in REQUIRED_BINARIES:
        if not package_binary(package_dir, name).is_file():
            run.problems.append(f"thiếu binary trong gói: {name}")
    if run.problems:
        return run
    with _TempDir() as tmp:
        workspace = Path(work_dir) if work_dir else Path(tmp)
        workspace.mkdir(parents=True, exist_ok=True)
        for case in smoke_cases(package_dir, workspace):
            result = runner(case, workspace)
            run.cases.append(case)
            run.results.append(result)
            run.problems.extend(check_smoke_case(case, result))
            if case.key == "health-offline":
                run.fingerprint.update(health_fingerprint(result))
            if case.key == "content-ok":
                run.fingerprint.update(content_fingerprint(result))
    run.files = package_fingerprint(package_dir)
    return run


def git_revision() -> str:
    """SHA commit hiện tại (best effort; không có `git` thì ghi `unknown`)."""
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True,
            cwd=str(ROOT),
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unknown"
    if proc.returncode != 0:
        return "unknown"
    return proc.stdout.decode("utf-8", "replace").strip() or "unknown"


def render_smoke_log(run: SmokeRun) -> str:
    """Log smoke dạng text: phiên bản, fingerprint và từng ca đã chạy."""
    out: list[str] = []
    out.append("=== boom-rs — log smoke package (V1-036) ===")
    out.append(f"package: {run.package}")
    out.append(f"version: {run.version}")
    out.append(f"os: {platform.system()} {platform.machine()}")
    out.append(f"git: {git_revision()}")
    out.append("")
    if run.fingerprint:
        out.append("--- fingerprint (boom-server health --offline) ---")
        out.append(json.dumps(run.fingerprint, ensure_ascii=False, indent=2, sort_keys=True))
        out.append("")
    if run.files:
        out.append("--- sha-256 file trong gói ---")
        for relative, digest in run.files:
            out.append(f"{digest}  {relative}")
        out.append("")
    for case, result in zip(run.cases, run.results):
        out.append(f"--- {case.name} ---")
        out.append(f"$ {' '.join(case.argv)}")
        out.append(f"exit: {result.code}")
        output = result.output.strip()
        out.append(output if output else "(không có output)")
        out.append("")
    out.append("=== kết luận ===")
    if run.problems:
        out.append(f"KHÔNG ĐẠT: {len(run.problems)} vi phạm")
        for problem in run.problems:
            out.append(f"- {problem}")
    else:
        out.append(f"ĐẠT: {len(run.cases)} ca smoke đều đúng kỳ vọng")
    return "\n".join(out) + "\n"


def cmd_pack(args) -> int:
    problems = pack_package(Path(args.bin_dir), Path(args.out), Path(args.source_dir))
    if problems:
        print("LỖI: không dựng được gói:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    files = sum(1 for path in Path(args.out).rglob("*") if path.is_file())
    print(f"Gói đã dựng: {args.out} ({files} file).")
    return 0


def cmd_unpack(args) -> int:
    problems = unpack_archive(Path(args.archive), Path(args.out))
    if problems:
        print("LỖI: không mở được archive:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print(f"Đã mở gói: {args.archive} → {args.out}")
    return 0


def cmd_smoke(args, runner: CommandRunner | None = None) -> int:
    package_dir = Path(args.package_dir)
    if not package_dir.is_dir():
        print(f"LỖI: không có thư mục gói: {package_dir}", file=sys.stderr)
        return 1
    # `runner` cho phép tự kiểm mô phỏng gói lỗi mà không cần binary thật.
    run = smoke_package(package_dir, runner=runner, version=args.version)
    if args.log:
        log = Path(args.log)
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(render_smoke_log(run), encoding="utf-8")
        print(f"Log smoke: {log}")
    if run.problems:
        print("LỖI: smoke package không đạt:", file=sys.stderr)
        for problem in run.problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print(f"Smoke OK: {len(run.cases)} ca · version {run.version} · {package_dir}")
    if run.fingerprint:
        print(json.dumps(run.fingerprint, ensure_ascii=False, sort_keys=True))
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

#: MIT **mất câu bắt buộc** — ca âm của review #128 vòng 2 (trước đây vẫn bị nhận là
#: toàn văn vì chỉ kiểm mảnh câu đầu file).
MIT_MISSING_CLAUSE = MIT_TEXT.replace(
    "shall be included in all copies or substantial portions of the Software.", "")

#: MIT **mất một phần phạm vi áp dụng** của điều khoản (`or substantial portions`) —
#: ca âm của review #128 vòng 3: điều khoản còn đủ từ nên ngưỡng mờ 85% nhận nhầm
#: là toàn văn; nay đối chiếu nghiêm nên bị bác bỏ.
MIT_MISSING_SCOPE = MIT_TEXT.replace(" or substantial portions", "")

#: MIT có điều khoản bị **viết lại** (đủ từ nhưng đổi nghĩa) — phải bị bác bỏ.
MIT_REWRITTEN_CLAUSE = MIT_TEXT.replace(
    "all copies or substantial portions", "some copies of the Software")

#: Apache-2.0 **chỉ có tiêu đề mục**, không có nội dung điều khoản — ca âm còn lại
#: của review #128 vòng 2 (trước đây vẫn bị nhận là toàn văn).
APACHE_HEADINGS_ONLY = """\
                              Apache License
                        Version 2.0, January 2004
                     http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1. Definitions.

2. Grant of Copyright License.

3. Grant of Patent License.

4. Redistribution.

5. Submission of Contributions.

6. Trademarks.

7. Disclaimer of Warranty.

8. Limitation of Liability.

9. Accepting Warranty or Additional Liability.

END OF TERMS AND CONDITIONS
"""


def workflow_step_block(text: str, name: str) -> list[str]:
    """Các dòng của bước workflow có `name` (không kể dòng `- name:`).

    Không cần thư viện YAML: workflow của dự án đặt `- name:` cho mỗi bước và
    các khóa của bước nằm thụt lền sâu hơn. Trả về `[]` nếu không có bước nào
    tên như vậy.
    """
    lines = text.splitlines()
    for index, line in enumerate(lines):
        stripped = line.strip()
        if not stripped.startswith("- name:"):
            continue
        value = stripped[len("- name:"):].strip().strip('"').strip("'")
        if value != name:
            continue
        indent = len(line) - len(line.lstrip())
        block: list[str] = []
        for follow in lines[index + 1:]:
            content = follow.strip()
            if content and not content.startswith("#"):
                if len(follow) - len(follow.lstrip()) <= indent:
                    break
            block.append(follow)
        return block
    return []


class PackageToolTests(unittest.TestCase):
    """Tự kiểm công cụ đóng gói (chạy bằng `python3 tools/check_package.py test`)."""

    def _valid_package(self, root: Path) -> Path:
        package = root / "boom-rs-dev-linux-x86_64"
        (package / "bin").mkdir(parents=True)
        (package / "data" / "rules").mkdir(parents=True)
        (package / "data" / "maps").mkdir(parents=True)
        (package / "data" / "l10n").mkdir(parents=True)
        (package / "assets" / "fonts").mkdir(parents=True)
        (package / "data" / "maps" / "village00-kr.toml").write_text("[map]\n", encoding="utf-8")
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

    def test_check_rejects_missing_map_profile(self):
        """Thiếu bảng phân loại ô map ⇒ `boom-server serve` từ gói không khởi động (#305)."""
        with _TempDir() as root:
            package = self._valid_package(Path(root))
            (package / "data" / "maps" / "village00-kr.toml").unlink()
            joined = " | ".join(check_package(package))
            self.assertIn("data/maps/village00-kr.toml", joined)
            # Thư mục map rỗng cũng bị bắt.
            self.assertIn("data/maps/", joined)

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

    def test_licenses_include_vendored_dependency_but_exclude_workspace(self):
        with _TempDir() as root:
            root = Path(root)
            directory = _make_crate(root, "vendored", "1.0.0", {"LICENSE-MIT": MIT_TEXT})
            crate = Crate("vendored", "1.0.0", "MIT", None, None, directory)
            metadata = _metadata([(crate, directory)])
            dependency = metadata["packages"][0]
            dependency.update(id="path+vendored", source=None)
            workspace = dict(dependency, id="path+boom", name="boom-client", license=None)
            metadata["packages"].append(workspace)
            metadata["workspace_members"] = ["path+boom"]
            crates = crates_from_metadata(metadata)
            self.assertEqual([crate.key for crate in crates], ["vendored 1.0.0"])
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

    # -- Toàn văn: đối chiếu MẪU CHUẨN, không chỉ mảnh câu (review #128 vòng 2) ------

    def test_detect_by_content_rejects_incomplete_full_text(self):
        """Thiếu một điều khoản, hoặc chỉ có tiêu đề mục ⇒ không phải toàn văn."""
        self.assertNotEqual(detect_by_content(MIT_MISSING_CLAUSE)[1], "full")
        self.assertNotEqual(detect_by_content(APACHE_HEADINGS_ONLY)[1], "full")
        self.assertEqual(detect_by_content(MIT_MISSING_CLAUSE), (None, "other"))
        self.assertEqual(detect_by_content(APACHE_HEADINGS_ONLY), (None, "other"))

    def test_removing_any_clause_is_detected(self):
        """Bỏ một điều khoản khỏi mẫu ⇒ không còn được nhận là toàn văn của license đó.

        Câu mà vẫn xuất hiện chỗ khác trong văn bản (vd tiêu đề mục trùng câu định
        nghĩa) thì bỏ đi không làm mất nội dung nào — không tính là sót.
        """
        for template in LICENSE_TEMPLATES:
            text = template_text(template)
            for clause in template_clauses(template):
                variant = text.replace(clause, "")
                unit = flatten(normalize_words(clause))
                if unit in flatten(normalize_words(variant)):
                    continue  # câu vẫn còn trong văn bản (chỗ khác)
                with self.subTest(ident=template.ident, clause=clause[:40]):
                    self.assertNotEqual(detect_by_content(variant), (template.ident, "full"))

    def test_build_report_rejects_incomplete_full_text(self):
        """Ca âm của reviewer: `build_report` phải báo lỗi và không xuất toàn văn."""
        cases = {
            "MIT mất câu bắt buộc": ("MIT", MIT_MISSING_CLAUSE),
            "Apache-2.0 chỉ có tiêu đề mục": ("Apache-2.0", APACHE_HEADINGS_ONLY),
        }
        for label, (ident, content) in cases.items():
            with self.subTest(label=label), _TempDir() as root:
                root = Path(root)
                crate_dir = _make_crate(root, "iota", "1.0.0", {"LICENSE": content})
                crates = [Crate("iota", "1.0.0", ident, None, None, crate_dir)]
                report = build_report(crates)
                self.assertTrue(report.problems, f"phải báo lỗi: {label}")
                self.assertNotIn(ident, report.texts,
                                 f"không được xuất làm toàn văn: {label}")

    # -- Đối chiếu nghiêm: mất một phần điều khoản cũng bị bác bỏ (review #128 vòng 3) --

    def test_detect_by_content_rejects_missing_scope(self):
        """Mất *một phần* điều khoản (không mất cả câu) cũng phải bị bác bỏ.

        Trước đây cơ chế đo "tỉ lệ từ khớp" (85%) chấp nhận file này là toàn văn vì
        còn 17/20 từ khớp — nay đối chiếu nghiêm nên bị bác bỏ.
        """
        self.assertNotEqual(detect_by_content(MIT_MISSING_SCOPE)[1], "full")
        self.assertEqual(detect_by_content(MIT_MISSING_SCOPE), (None, "other"))

    def test_build_report_rejects_missing_scope(self):
        """Ca âm của reviewer (vòng 3): `build_report` phải báo lỗi, không xuất MIT."""
        with _TempDir() as root:
            root = Path(root)
            crate_dir = _make_crate(root, "repro", "1.0.0", {"LICENSE-MIT": MIT_MISSING_SCOPE})
            crates = [Crate("repro", "1.0.0", "MIT", None, None, crate_dir)]
            report = build_report(crates)
            self.assertTrue(report.problems)
            self.assertNotIn("MIT", report.texts)

    def test_detect_by_content_rejects_rewritten_clause(self):
        """Điều khoản bị **viết lại** (đủ từ nhưng đổi nghĩa) phải bị bác bỏ."""
        self.assertEqual(detect_by_content(MIT_REWRITTEN_CLAUSE), (None, "other"))

    def test_subject_name_may_be_replaced_but_not_deleted(self):
        """Tên chủ thể thay được; **xoá hẳn** cụm từ đó thì không được."""
        template = next(t for t in LICENSE_TEMPLATES if t.ident == "BSD-3-Clause")
        text = template_text(template)
        renamed = text.replace("the copyright holder", "the Xiph.org Foundation")
        self.assertEqual(detect_by_content(renamed), ("BSD-3-Clause", "full"))
        deleted = re.sub(r"the copyright holder", "", text, flags=re.IGNORECASE)
        self.assertNotEqual(detect_by_content(deleted)[1], "full")

    def test_templates_detect_their_own_full_text(self):
        """Mỗi mẫu chuẩn phải tự khớp (bảo đảm mẫu trong repo không hỏng)."""
        for template in LICENSE_TEMPLATES:
            with self.subTest(ident=template.ident):
                text = template_text(template)
                self.assertEqual(detect_by_content(text), (template.ident, "full"))

    def test_detect_by_content_accepts_known_variants(self):
        """Biến thể **đã liệt kê rõ** vẫn là toàn văn license.

        - MIT của `bytemuck 1.25.2`: thêm "(including the next paragraph)" — khai báo
          trong `ClauseVariant` của mẫu MIT.
        - BSD-3-Clause của `ogg 0.7.1`: mục 3 thay "the copyright holder" bằng tên
          chủ bản quyền thật — khai báo trong `slots` của mẫu BSD-3-Clause.
        """
        # MIT của `bytemuck 1.25.2`: thêm "(including the next paragraph)".
        self.assertEqual(detect_by_content(MIT_TEXT.replace(
            "The above copyright notice and this permission notice shall be included",
            "The above copyright notice and this permission notice "
            "(including the next paragraph) shall be included")), ("MIT", "full"))
        # BSD-3-Clause của `ogg 0.7.1`: mục 3 thay "the copyright holder" bằng tên
        # chủ bản quyền thật.
        self.assertEqual(detect_by_content(_fixture("bsd-3-clause.txt").replace(
            "Neither the name of the copyright holder nor the names of its contributors",
            "Neither the name of the Xiph.org Foundation nor the names of its contributors")),
            ("BSD-3-Clause", "full"))
        # Apache-2.0: URL tiêu đề ghi `https` và có hậu tố `LICENSE-2.0`.
        self.assertEqual(detect_by_content(APACHE_TEXT.replace(
            "http://www.apache.org/licenses/",
            "https://www.apache.org/licenses/LICENSE-2.0")), ("Apache-2.0", "full"))

    def test_file_with_several_licenses_is_source_for_each(self):
        """File chứa nhiều license (vd MIT + Apache) là nguồn toàn văn của từng id."""
        with _TempDir() as root:
            root = Path(root)
            crate_dir = _make_crate(root, "kappa", "1.0.0", {
                "LICENSE": MIT_TEXT + "\n" + APACHE_TEXT})
            classified = classify_license_file(crate_dir / "LICENSE")
            self.assertEqual(classified.kind, "full")
            self.assertEqual(sorted(classified.idents), ["Apache-2.0", "MIT"])
            crates = [Crate("kappa", "1.0.0", "MIT OR Apache-2.0", None, None, crate_dir)]
            report = build_report(crates)
            self.assertEqual(report.problems, [])
            self.assertIn("MIT", report.texts)
            self.assertIn("Apache-2.0", report.texts)

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

    def test_vendored_miniquad_keeps_at_year_author_notice(self):
        """Thông báo tác giả thực của miniquad phải còn trong LICENSES của gói."""
        directory = ROOT / "vendor" / "miniquad"
        notice = "@ 2019-2020 Fedor Logachev <not.fl3@gmail.com>"
        self.assertIn(notice, (directory / "LICENSE-MIT").read_text(encoding="utf-8"))
        report = build_report([Crate("miniquad", "0.4.11", "MIT OR Apache-2.0",
                                     None, None, directory)])
        self.assertEqual(report.problems, [])
        text = render_licenses(report)
        copyrights = text.split("PHẦN 3 — BẢN QUYỀN TỪNG CRATE", 1)[1]
        self.assertIn("bản quyền: " + notice, copyrights)
        self.assertIn("Permission is hereby granted", text)

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

    # -- Dựng gói / mở gói (V1-036) ---------------------------------------------------

    def test_pack_builds_layout_without_game_assets(self):
        """`pack` dựng đúng bố cục gói và **không** kèm asset game."""
        with _TempDir() as root:
            root = Path(root)
            bin_dir = root / "build"
            bin_dir.mkdir()
            for name in REQUIRED_BINARIES:
                (bin_dir / name).write_text("bin", encoding="utf-8")
            out = root / "pkg"
            self.assertEqual(pack_package(bin_dir, out, ROOT), [])
            for name in REQUIRED_BINARIES:
                self.assertTrue((out / "bin" / name).is_file(), name)
            self.assertTrue((out / "data" / "rules" / "v1-dev.toml").is_file())
            # Bảng phân loại ô map server nạp mặc định phải có trong gói (#305).
            self.assertTrue((out / "data" / "maps" / "village00-kr.toml").is_file())
            self.assertTrue((out / "data" / "l10n" / "vi-v1.toml").is_file())
            self.assertTrue((out / "assets" / "fonts" / "OFL.txt").is_file())
            readme = (out / "README.md").read_text(encoding="utf-8")
            self.assertIn("boom-server content", readme)
            # Không có pack/bundle/asset game trong gói.
            self.assertEqual(
                [problem for problem in check_package(out)
                 if "pack/bundle/asset game" in problem],
                [])

    def test_pack_reports_missing_binary(self):
        """Thiếu binary thì `pack` phải báo lỗi, không dựng gói nửa vời."""
        with _TempDir() as root:
            root = Path(root)
            bin_dir = root / "build"
            bin_dir.mkdir()
            problems = pack_package(bin_dir, root / "pkg", ROOT)
            self.assertTrue(problems)
            joined = " | ".join(problems)
            for name in REQUIRED_BINARIES:
                self.assertIn(name, joined)

    def test_unpack_opens_archive(self):
        """`unpack` mở được archive đã đóng (zip/tar.gz) và trả về thư mục gói."""
        import tarfile
        with _TempDir() as root:
            root = Path(root)
            source = root / "boom-rs-dev-linux-x86_64"
            (source / "bin").mkdir(parents=True)
            (source / "bin" / "boom-server").write_text("bin", encoding="utf-8")
            archive = root / "boom-rs-dev-linux-x86_64.tar.gz"
            with tarfile.open(archive, "w:gz") as tar:
                tar.add(source, arcname=source.name)
            out = root / "smoke"
            self.assertEqual(unpack_archive(archive, out), [])
            self.assertTrue((out / "boom-rs-dev-linux-x86_64" / "bin" / "boom-server").is_file())

    def test_unpack_reports_missing_archive(self):
        """Archive không có thì báo lỗi rõ, không im lặng."""
        with _TempDir() as root:
            problems = unpack_archive(Path(root) / "khong-co.tar.gz", Path(root) / "out")
            self.assertTrue(problems)
            self.assertIn("khong-co.tar.gz", problems[0])

    # -- Smoke: chạy binary trong gói (V1-036) ----------------------------------------

    def test_smoke_accepts_working_package(self):
        """Gói chạy được và lỗi thiếu file báo rõ ⇒ smoke đạt."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            run = smoke_package(package, runner=_FakeRunner(), version="0.1.0")
            self.assertEqual(run.problems, [])
            self.assertEqual(len(run.cases), 10)
            self.assertEqual(run.fingerprint.get("scope"), "offline")
            self.assertIn("server_version", run.fingerprint)
            log = render_smoke_log(run)
            self.assertIn("version: 0.1.0", log)
            self.assertIn("fingerprint", log)

    def test_smoke_reports_wrong_exit_code(self):
        """Lệnh trả mã sai (vd `content` lỗi trong khi file trong gói hợp lệ) ⇒ báo lỗi."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            runner = _FakeRunner(overrides={
                "content-ok": CommandResult(2, "", "không đọc được file rules: /x/rules.toml"),
            })
            run = smoke_package(package, runner=runner)
            self.assertTrue(run.problems)
            self.assertTrue(any("content-ok" in problem or "rules" in problem
                                for problem in run.problems))

    def test_smoke_reports_unclear_error_message(self):
        """Thiếu file mà thông điệp **không** nêu đường dẫn/cách sửa ⇒ smoke phải báo lỗi."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            runner = _FakeRunner(overrides={
                "missing-rules-server": CommandResult(2, "", "lỗi: không đọc được"),
            })
            run = smoke_package(package, runner=runner)
            self.assertTrue(run.problems)
            self.assertTrue(any("file rules" in problem for problem in run.problems))

    def test_smoke_reports_health_not_json(self):
        """`health --offline` phải in JSON có fingerprint ⇒ không JSON là lỗi."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            runner = _FakeRunner(overrides={
                "health-offline": CommandResult(0, "không phải JSON", ""),
            })
            run = smoke_package(package, runner=runner)
            self.assertTrue(run.problems)
            self.assertTrue(any("JSON" in problem for problem in run.problems))

    def test_smoke_reports_health_json_not_object(self):
        """`health --offline` in JSON **không phải object** ⇒ phải báo lỗi.

        Ca âm của review #168 vòng 1: JSON đúng cú pháp nhưng là array/null/scalar
        thì không có field nào để kiểm, nên smoke phải **không** bỏ qua.
        """
        for label, stdout in (("array", "[]"), ("null", "null"), ("chuỗi", '"bad"'),
                              ("số", "42"), ("boolean", "true")):
            with self.subTest(hinh=label), _TempDir() as root:
                root = Path(root)
                package = self._valid_package(root)
                runner = _FakeRunner(overrides={
                    "health-offline": CommandResult(0, stdout, ""),
                })
                run = smoke_package(package, runner=runner)
                self.assertTrue(run.problems, f"JSON {label} phải bị báo lỗi")
                self.assertTrue(
                    any("JSON phải là object" in problem for problem in run.problems),
                    f"thông điệp phải nói JSON phải là object: {run.problems}")

    def test_smoke_accepts_health_json_object(self):
        """JSON object đủ field ⇒ vẫn đạt (kiểm mới không vướng ca hợp lệ)."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            run = smoke_package(package, runner=_FakeRunner())
            self.assertEqual(run.problems, [])

    def test_smoke_log_written_when_smoke_fails(self):
        """Smoke fail vẫn ghi log (bằng chứng để debug) **và** trả mã khác 0.

        Review #168 vòng 1: log là thứ duy nhất còn lại khi gói lỗi trên một OS.
        """
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            log = root / "smoke-log.txt"
            runner = _FakeRunner(overrides={
                "health-offline": CommandResult(0, "[]", ""),
            })
            args = argparse.Namespace(
                package_dir=str(package), log=str(log), version="0.1.0")
            code = cmd_smoke(args, runner=runner)
            self.assertEqual(code, 1, "smoke fail phải trả mã khác 0")
            self.assertTrue(log.is_file(), "phải ghi log kể cả khi smoke fail")
            text = log.read_text(encoding="utf-8")
            self.assertIn("KHÔNG ĐẠT", text)
            self.assertIn("JSON phải là object", text)
            self.assertIn("version: 0.1.0", text)

    def test_workflow_uploads_smoke_log_when_smoke_fails(self):
        """Bước upload log smoke phải chạy **cả khi smoke fail** (review #168 vòng 1).

        Điều kiện mặc định của GitHub Actions là `success()` ⇒ bước bị bỏ qua khi
        smoke trả mã khác 0, mất đúng artifact cần để debug.
        """
        for relative in (".github/workflows/ci.yml", ".github/workflows/release.yml"):
            with self.subTest(workflow=relative):
                text = (ROOT / relative).read_text(encoding="utf-8")
                block = workflow_step_block(text, "Upload log smoke")
                self.assertTrue(block, f"không thấy bước 'Upload log smoke' trong {relative}")
                conditions = [line.strip() for line in block
                              if line.strip().startswith("if:")]
                self.assertTrue(
                    conditions,
                    f"bước upload log phải có `if:` riêng trong {relative}")
                condition = conditions[0]
                self.assertIn("!cancelled()", condition)
                self.assertIn("hashFiles(", condition)
                joined = "\n".join(block)
                self.assertIn("dist/smoke-log.txt", joined)

    def test_workflow_smoke_step_keeps_job_failing(self):
        """Bước smoke không được nuốt lỗi — job phải đỏ khi gói sai."""
        for relative, step_name in (
            (".github/workflows/ci.yml", "Smoke: chạy binary trong gói"),
            (".github/workflows/release.yml", "Mở gói và smoke"),
        ):
            with self.subTest(workflow=relative):
                text = (ROOT / relative).read_text(encoding="utf-8")
                block = workflow_step_block(text, step_name)
                self.assertTrue(block, f"không thấy bước {step_name!r} trong {relative}")
                joined = "\n".join(block)
                self.assertNotIn("continue-on-error: true", joined)
                self.assertNotIn("continue-on-error: true", text)

    def test_smoke_reports_missing_binary(self):
        """Gói thiếu binary thì smoke báo lỗi, không chạy lệnh không tồn tại."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            (package / "bin" / "boom-server").unlink()
            run = smoke_package(package, runner=_FakeRunner())
            self.assertTrue(run.problems)
            self.assertTrue(any("boom-server" in problem for problem in run.problems))

    def test_smoke_resolves_package_path(self):
        """Đường dẫn trong lệnh phải **tuyệt đối**: lệnh chạy ở thư mục tạm."""
        with _TempDir() as root:
            root = Path(root)
            package = self._valid_package(root)
            run = smoke_package(package, runner=_FakeRunner())
            self.assertTrue(run.package.is_absolute())
            for case in run.cases:
                self.assertTrue(Path(case.argv[0]).is_absolute(), case.argv[0])
                for argument in case.argv[1:]:
                    if "/" in argument or "\\" in argument:
                        self.assertTrue(Path(argument).is_absolute(), argument)

    def test_smoke_cases_cover_required_checks(self):
        """Bộ ca smoke phải có đủ: help/content/health và lỗi thiếu rules/l10n/bundle/pack."""
        with _TempDir() as root:
            cases = smoke_cases(Path(root) / "pkg", Path(root) / "work")
            keys = {case.key for case in cases}
            self.assertEqual(keys, {
                "help-server", "help-client", "content-ok", "health-offline",
                "missing-rules-server", "missing-rules-client", "missing-l10n-server",
                "missing-bundle-server", "missing-bundle-client", "missing-pack-assets",
            })


class _FakeRunner:
    """Runner giả cho tự kiểm: mô phỏng output binary theo từng ca.

    Dùng để kiểm **logic kiểm** của công cụ mà không cần binary thật; binary thật
    được kiểm bởi CI ba OS qua lệnh `smoke`.
    """

    def __init__(self, overrides: dict[str, CommandResult] | None = None):
        self.overrides = overrides or {}
        self.calls: list[str] = []

    def __call__(self, case: SmokeCase, cwd: Path) -> CommandResult:
        self.calls.append(case.key)
        if case.key in self.overrides:
            return self.overrides[case.key]
        return self._simulate(case)

    @staticmethod
    def _arg(case: SmokeCase, flag: str) -> str:
        return case.argv[case.argv.index(flag) + 1]

    def _simulate(self, case: SmokeCase) -> CommandResult:
        if case.key in ("help-server", "help-client"):
            return CommandResult(0, "dùng: boom-server health | content | serve\n--bundle DIR\n", "")
        if case.key == "content-ok":
            return CommandResult(
                0, "rules: kr00967-v1-dev · hash 0xf498081887ed064d · 31 rule\nchuỗi: vi-VN · 110 khóa\n", "")
        if case.key == "health-offline":
            return CommandResult(0, json.dumps({
                "status": "unhealthy",
                "scope": "offline",
                "server_version": "0.1.0",
                "wire_schema": "wire-r1",
                "state_schema": "state-r1",
                "rules_profile_id": "kr00967-v1-dev",
                "rules_hash": "0x1",
                "content_sha256": "0x2",
                "protocol_limits_version": "limits-r1",
            }), "")
        if case.key.startswith("missing-rules"):
            path = self._arg(case, "--rules")
            return CommandResult(
                2, "", f"không đọc được file rules: {path} (No such file)\n  sửa: file phải tồn tại\n")
        if case.key == "missing-l10n-server":
            path = self._arg(case, "--l10n")
            return CommandResult(
                2, "", f"không đọc được file chuỗi: {path} (No such file)\n  sửa: file phải tồn tại\n")
        if case.key in ("missing-bundle-server", "missing-bundle-client"):
            path = self._arg(case, "--bundle")
            return CommandResult(
                2, "",
                f"không nạp được bundle: {path}\n  lỗi: thiếu bundle.toml\n  "
                "sửa: dựng lại bundle từ pack của bạn: "
                "`boom-assets bundle --map Village00 --out <dir> --pack-dir <thư mục pack>`\n")
        if case.key == "missing-pack-assets":
            path = self._arg(case, "--pack-dir")
            return CommandResult(
                1, "", f"{path}: lỗi pack: {path}/FxMap.idx: No such file or directory\n")
        raise AssertionError(f"ca smoke lạ: {case.key}")


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


def force_utf8_output() -> None:
    """Ép stdout/stderr dùng UTF-8: console Windows mặc định cp1252, in tiếng Việt sẽ lỗi.

    `UnicodeEncodeError` khi in thông điệp tiếng Việt làm job CI fail dù công cụ
    đã làm đúng. Ép được thì ép; không được thì bỏ qua (không được làm hỏng lệnh).
    """
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is None:
            continue
        try:
            reconfigure(encoding="utf-8", errors="replace")
        except (OSError, ValueError):  # pragma: no cover - stream không ép được
            pass


def main(argv: list[str] | None = None) -> int:
    force_utf8_output()
    parser = argparse.ArgumentParser(
        description="Dựng/mở gói phát hành, sinh LICENSES, kiểm gói và smoke chạy binary (V1-036).")
    sub = parser.add_subparsers(dest="command", required=True)

    licenses = sub.add_parser("licenses", help="sinh file LICENSES từ cargo metadata")
    licenses.add_argument("--out", required=True, help="đường dẫn file LICENSES cần ghi")
    licenses.add_argument("--metadata", help="file JSON `cargo metadata` (mặc định: chạy cargo)")
    licenses.set_defaults(func=cmd_licenses)

    check = sub.add_parser("check", help="kiểm bộ đóng gói (file cấm, thành phần bắt buộc)")
    check.add_argument("package_dir", help="thư mục gói phát hành")
    check.set_defaults(func=cmd_check)

    pack = sub.add_parser("pack", help="dựng gói phát hành (bin/, data/, assets/fonts/, README)")
    pack.add_argument("--bin-dir", required=True, help="thư mục binary build (vd target/release)")
    pack.add_argument("--out", required=True, help="thư mục gói cần dựng")
    pack.add_argument("--source-dir", default=str(ROOT),
                      help="gốc repo lấy data/ và assets/fonts (mặc định: repo hiện tại)")
    pack.set_defaults(func=cmd_pack)

    unpack = sub.add_parser("unpack", help="giải nén archive phát hành (.zip/.tar.gz)")
    unpack.add_argument("archive", help="file archive cần mở")
    unpack.add_argument("--out", required=True, help="thư mục đích giải nén")
    unpack.set_defaults(func=cmd_unpack)

    smoke = sub.add_parser("smoke", help="chạy binary trong gói và kiểm lỗi thiếu file")
    smoke.add_argument("package_dir", help="thư mục gói đã giải nén")
    smoke.add_argument("--log", help="file log smoke (version + fingerprint + từng ca)")
    smoke.add_argument("--version", default="dev", help="phiên bản gói (ghi vào log)")
    smoke.set_defaults(func=cmd_smoke)

    test = sub.add_parser("test", help="tự kiểm công cụ này")
    test.set_defaults(func=cmd_test)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
