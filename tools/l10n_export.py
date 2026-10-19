#!/usr/bin/env python3
"""Sinh `data/l10n/vi-v1.toml` từ danh mục khóa V1 + catalog Việt hóa.

Nguồn (không dịch lại, không sửa bản dịch):

* khóa V1, locator, trạng thái nguồn, yêu cầu hiển thị:
  `docs/specification/localization/UI_STRINGS_V1.md` §2;
* text đã Việt hóa: cột `vi` của `/home/admin/boom-re-l10n/l10n/catalog.csv`.

Cách chạy:

```sh
python3 tools/l10n_export.py            # xuất data/l10n/vi-v1.toml
python3 tools/l10n_export.py --check    # kiểm file đã xuất
```

`--check` chạy được khi **không có** repo `boom-re-l10n` (CI không cần repo đó): khi
thiếu catalog nó chỉ kiểm schema + đối chiếu với §2 của spec; khi có catalog thì so
từng byte `text` với `catalog.vi` và so `catalog_sha256`.

Quy tắc (giữ nguyên với `boom-content::l10n`, xem `crates/boom-content/src/l10n.rs`):

* `text` = `catalog.vi` **nguyên văn**, chỉ trim biên (khoảng trắng và xuống dòng thừa
  ở đầu/cuối); dấu `%s`, `{0}`, `<font …>`, CRLF và `|` giữ nguyên;
* khóa `review`/`image-pending`/`unknown` **không có text**, chỉ có `status` + lý do;
* placeholder của `text` phải khớp số lượng với placeholder của `ko` (nguồn Hàn) sau khi
  trim biên cả hai — lệch thì không xuất;
* mọi `text` phải ở dạng NFC (chuẩn hóa trước khi shape, UI_STRINGS_V1 §5.1).

Định dạng đầu ra là **design** của dự án (bản dịch chuẩn vẫn là `catalog.csv`), không
phải định dạng của client gốc.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import re
import sys
import tomllib
import unicodedata
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "docs/specification/localization/UI_STRINGS_V1.md"
OUT_PATH = ROOT / "data/l10n/vi-v1.toml"
DEFAULT_CATALOG = Path("/home/admin/boom-re-l10n/l10n/catalog.csv")

SCHEMA = 1
LOCALE = "vi-VN"

#: Trạng thái nguồn (UI_STRINGS_V1 §1) và trạng thái nào đã có text chốt.
STATUS_TEXT = "catalog-translated"
STATUS_REUSED = "catalog-reused"
STATUS_REVIEW = "review"
STATUS_IMAGE = "image-pending"
STATUS_UNKNOWN = "unknown"
STATUSES = (STATUS_TEXT, STATUS_REUSED, STATUS_REVIEW, STATUS_IMAGE, STATUS_UNKNOWN)
TEXT_STATUSES = (STATUS_TEXT, STATUS_REUSED)

#: Mã lý do cho khóa chưa có text; client hiện placeholder `⟨khóa⟩` theo mã này.
REASONS = {
    STATUS_REVIEW: "needs-review",
    STATUS_IMAGE: "image-pending-review",
    STATUS_UNKNOWN: "no-locator",
}

KEY_RE = re.compile(r"^[a-z0-9_]+(\.[a-z0-9_]+)+$")
IMAGE_PREFIX = "ảnh "
CATALOG_PREFIX = "catalog:"
NO_LOCATOR = "—"


class ExportError(Exception):
    """Lỗi có thông điệp rõ, in ra và thoát với mã 1."""


@dataclass(frozen=True)
class SpecRow:
    """Một dòng khóa V1 trong §2 của UI_STRINGS_V1."""

    key: str
    locator: str
    status: str
    note: str


@dataclass(frozen=True)
class Entry:
    """Một khóa trong file xuất."""

    key: str
    status: str
    catalog_id: str | None = None
    text: str | None = None
    reason: str | None = None
    note: str = ""
    image: str | None = None


# --------------------------------------------------------------------------- đặc tả


def parse_spec(path: Path) -> list[SpecRow]:
    """Đọc bảng khóa V1 ở §2 của UI_STRINGS_V1."""
    lines = path.read_text(encoding="utf-8").split("\n")
    start = end = None
    for index, line in enumerate(lines):
        if start is None and line.startswith("## 2. "):
            start = index
        elif start is not None and end is None and line.startswith("## 3. "):
            end = index
    if start is None or end is None:
        raise ExportError(f"{path}: không tìm thấy mục §2/§3 để đọc bảng khóa V1")

    rows: list[SpecRow] = []
    for number, line in enumerate(lines[start:end], start=start + 1):
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 5:
            raise ExportError(
                f"{path}:{number + 1}: bảng §2 phải có 5 cột, thấy {len(cells)}"
            )
        key = cells[0].strip("`")
        if not KEY_RE.match(key):
            continue  # hàng tiêu đề và hàng đường kẻ
        status = cells[3].strip("`")
        if status not in STATUSES:
            raise ExportError(
                f"{path}:{number + 1}: khóa `{key}` có trạng thái lạ `{status}`"
            )
        rows.append(SpecRow(key=key, locator=cells[2].strip("`"), status=status, note=cells[4]))

    if not rows:
        raise ExportError(f"{path}: §2 không có dòng khóa V1 nào")
    seen: dict[str, int] = {}
    for row in rows:
        if row.key in seen:
            raise ExportError(f"{path}: khóa V1 trùng `{row.key}`")
        seen[row.key] = 1
    return rows


def spec_summary(path: Path) -> dict[str, int]:
    """Số đếm ghi trong phần tổng hợp của §1, dùng để bắt lệch với bảng §2.

    Trả về rỗng nếu câu chữ đã đổi (kiểm tra tuỳ biến, không phải điều kiện cứng).
    """
    lines = path.read_text(encoding="utf-8").split("\n")
    start = end = None
    for index, line in enumerate(lines):
        if start is None and line.startswith("## 1. "):
            start = index
        elif start is not None and end is None and line.startswith("## 2. "):
            end = index
    if start is None or end is None:
        return {}
    text = "\n".join(lines[start:end])
    summary: dict[str, int] = {}
    patterns = {
        "total": r"Tổng hợp\s*(\d+)\s*khóa",
        "catalog": r"\*\*(\d+)\s*dòng có locator catalog\*\*",
        STATUS_TEXT: r"(\d+)\s*`catalog-translated`",
        STATUS_REUSED: r"(\d+)\s*`catalog-reused`",
        STATUS_REVIEW: r"\*\*(\d+)\s*dòng `review`\*\*",
        STATUS_IMAGE: r"\*\*(\d+)\s*dòng `image-pending`\*\*",
        STATUS_UNKNOWN: r"\*\*(\d+)\s*dòng `unknown`\*\*",
    }
    for name, pattern in patterns.items():
        found = re.search(pattern, text)
        if found:
            summary[name] = int(found.group(1))
    return summary


def read_catalog(path: Path) -> dict[str, list[dict[str, str]]]:
    """Đọc `catalog.csv` (nguồn chuẩn của bản dịch) theo `id`.

    Một `id` có thể có nhiều dòng (catalog tách dòng Lua bị ngắt); chỉ khi các dòng
    đó cho cùng bản dịch thì mới dùng được, ngược lại phải báo lỗi rõ.
    """
    rows: dict[str, list[dict[str, str]]] = {}
    with path.open(encoding="utf-8", newline="") as stream:
        for row in csv.DictReader(stream):
            rows.setdefault(row["id"], []).append(row)
    if not rows:
        raise ExportError(f"{path}: catalog rỗng")
    return rows


def pick_catalog_row(rows: dict[str, list[dict[str, str]]], catalog_id: str) -> dict[str, str]:
    """Dòng catalog dùng cho một `id`; các dòng trùng phải cho cùng bản dịch."""
    found = rows.get(catalog_id)
    if not found:
        raise ExportError(f"catalog không có id `{catalog_id}`")
    first = found[0]
    for other in found[1:]:
        if other["vi"] != first["vi"] or other["status"] != first["status"]:
            raise ExportError(
                f"catalog `{catalog_id}` có nhiều dòng khác nhau, không chọn được bản dịch"
            )
    return first


# ------------------------------------------------------------------- placeholder


def is_ascii_digits(text: str) -> bool:
    """Chỉ số ASCII; giống `char::is_ascii_digit` của loader để hai bên cùng đếm."""
    return bool(text) and all("0" <= char <= "9" for char in text)


def placeholder_tokens(text: str) -> list[str]:
    """Danh sách placeholder của một chuỗi, đã chuẩn hoá.

    Nhận: printf (`%s`, `%d`, `%02d`, `%.1f%%`), `{0}`, `{ComponentType|N}`, thẻ
    `<font …>`, CRLF thật và `|`. Quy tắc này phải giống hệt `boom-content::l10n`
    (`crates/boom-content/src/l10n.rs`) để hai bên cùng đếm.
    """
    tokens: list[str] = []
    chars = list(text)
    index = 0
    size = len(chars)
    while index < size:
        char = chars[index]
        if char == "%":
            end = index + 1
            while end < size and chars[end] in "-+ #0123456789.*":
                end += 1
            if end < size:
                spec = "".join(chars[index:end + 1])
                tokens.append(spec)
                index = end + 1
                continue
            index += 1
            continue
        if char == "{":
            end = text.find("}", index)
            if end != -1:
                body = text[index + 1:end]
                if is_ascii_digits(body) or (
                    "|" in body and is_ascii_digits(body.rsplit("|", 1)[1])
                ):
                    tokens.append("{" + body + "}")
                    index = end + 1
                    continue
            index += 1
            continue
        if char == "<" and text[index:index + 5].lower() == "<font":
            end = text.find(">", index)
            if end != -1:
                tokens.append("<font>")
                index = end + 1
                continue
            index += 1
            continue
        if char == "\r" and index + 1 < size and chars[index + 1] == "\n":
            tokens.append("\r\n")
            index += 2
            continue
        if char == "|":
            tokens.append("|")
            index += 1
            continue
        index += 1
    return tokens


def check_placeholders(catalog_id: str, ko: str, vi: str) -> None:
    """Placeholder của bản dịch phải khớp số lượng với bản Hàn gốc."""
    left = placeholder_tokens(ko.strip())
    right = placeholder_tokens(vi.strip())
    if sorted(left) != sorted(right):
        raise ExportError(
            f"catalog `{catalog_id}`: placeholder lệch giữa ko {sorted(left)} và vi {sorted(right)}"
        )


# ------------------------------------------------------------------------ xuất


def build_entries(rows: list[SpecRow], catalog: dict[str, list[dict[str, str]]]) -> list[Entry]:
    """Dựng danh sách khóa để xuất, kiểm tra mọi lệch so với §2 và catalog."""
    entries: list[Entry] = []
    for row in rows:
        locator = row.locator
        if locator.startswith(CATALOG_PREFIX):
            catalog_id = locator[len(CATALOG_PREFIX):]
            if row.status not in TEXT_STATUSES + (STATUS_REVIEW,):
                raise ExportError(
                    f"khóa `{row.key}`: locator catalog nhưng trạng thái `{row.status}`"
                )
            source = pick_catalog_row(catalog, catalog_id)
            if row.status == STATUS_REVIEW:
                entries.append(
                    Entry(
                        key=row.key,
                        status=row.status,
                        catalog_id=catalog_id,
                        reason=REASONS[STATUS_REVIEW],
                        note=row.note,
                    )
                )
                continue
            expected = "translated" if row.status == STATUS_TEXT else "reused"
            if source["status"] != expected:
                raise ExportError(
                    f"khóa `{row.key}`: catalog `{catalog_id}` có status "
                    f"`{source['status']}`, §2 ghi `{row.status}`"
                )
            text = source["vi"].strip()
            if not text:
                raise ExportError(f"catalog `{catalog_id}`: bản dịch rỗng sau khi trim")
            if unicodedata.normalize("NFC", text) != text:
                raise ExportError(f"catalog `{catalog_id}`: bản dịch chưa ở dạng NFC")
            check_placeholders(catalog_id, source["ko"], text)
            entries.append(
                Entry(
                    key=row.key,
                    status=row.status,
                    catalog_id=catalog_id,
                    text=text,
                    note=row.note,
                )
            )
            continue
        if locator.startswith(IMAGE_PREFIX):
            if row.status != STATUS_IMAGE:
                raise ExportError(
                    f"khóa `{row.key}`: locator ảnh nhưng trạng thái `{row.status}`"
                )
            image = locator[len(IMAGE_PREFIX):].strip()
            if not image:
                raise ExportError(f"khóa `{row.key}`: locator ảnh rỗng")
            entries.append(
                Entry(
                    key=row.key,
                    status=row.status,
                    reason=REASONS[STATUS_IMAGE],
                    note=row.note,
                    image=image,
                )
            )
            continue
        if locator == NO_LOCATOR:
            if row.status != STATUS_UNKNOWN:
                raise ExportError(
                    f"khóa `{row.key}`: không có locator nhưng trạng thái `{row.status}`"
                )
            entries.append(
                Entry(
                    key=row.key,
                    status=row.status,
                    reason=REASONS[STATUS_UNKNOWN],
                    note=row.note,
                )
            )
            continue
        raise ExportError(f"khóa `{row.key}`: locator lạ `{locator}`")
    return entries


def toml_string(value: str) -> str:
    """Chuỗi TOML cơ bản, escape đủ để đọc lại ra đúng byte."""
    out = ['"']
    for char in value:
        if char == '"':
            out.append('\\"')
        elif char == "\\":
            out.append("\\\\")
        elif char == "\n":
            out.append("\\n")
        elif char == "\r":
            out.append("\\r")
        elif char == "\t":
            out.append("\\t")
        elif ord(char) < 0x20 or ord(char) == 0x7F:
            out.append(f"\\u{ord(char):04X}")
        else:
            out.append(char)
    out.append('"')
    return "".join(out)


def render(entries: list[Entry], catalog_sha256: str, catalog_source: str) -> str:
    """Chuỗi TOML của file xuất; thứ tự cố định nên chạy hai lần ra file giống hệt."""
    lines = [
        "# Chuỗi UI V1 đã Việt hóa — sinh bằng `python3 tools/l10n_export.py`.",
        "# Nguồn: UI_STRINGS_V1 §2 (khóa, trạng thái) + catalog Việt hóa của boom-re-l10n (text).",
        "# Sửa file này bằng tool, không sửa tay; text lấy nguyên văn `catalog.vi`.",
        f"schema = {SCHEMA}",
        f'locale = {toml_string(LOCALE)}',
        'generated_by = "tools/l10n_export.py"',
        f"catalog_source = {toml_string(catalog_source)}",
        f"catalog_sha256 = {toml_string(catalog_sha256)}",
        'spec_source = "docs/specification/localization/UI_STRINGS_V1.md"',
    ]
    for entry in sorted(entries, key=lambda item: item.key):
        lines.append("")
        lines.append(f"[strings.{toml_string(entry.key)}]")
        if entry.catalog_id is not None:
            lines.append(f"catalog_id = {toml_string(entry.catalog_id)}")
        lines.append(f"status = {toml_string(entry.status)}")
        if entry.text is not None:
            lines.append(f"text = {toml_string(entry.text)}")
        if entry.reason is not None:
            lines.append(f"reason = {toml_string(entry.reason)}")
        if entry.image is not None:
            lines.append(f"image = {toml_string(entry.image)}")
        if entry.note:
            lines.append(f"note = {toml_string(entry.note)}")
    return "\n".join(lines) + "\n"


# -------------------------------------------------------------------------- kiểm


def parse_exported(text: str) -> tuple[dict | None, list[str]]:
    """Parse TOML đã xuất; trả về (document, lỗi)."""
    try:
        return tomllib.loads(text), []
    except tomllib.TOMLDecodeError as error:
        return None, [f"TOML sai ({error})"]


def check_exported(text: str, label: str, rows: list[SpecRow],
                   catalog: dict[str, list[dict[str, str]]] | None,
                   catalog_sha256: str | None) -> list[str]:
    """Kiểm nội dung file đã xuất; trả về danh sách lỗi (rỗng = đạt)."""
    errors: list[str] = []
    document, parse_errors = parse_exported(text)
    if document is None:
        return [f"{label}: {error}" for error in parse_errors]

    if document.get("schema") != SCHEMA:
        errors.append(f"{label}: schema phải là {SCHEMA}, thấy {document.get('schema')!r}")
    for field in ("locale", "generated_by", "catalog_source", "catalog_sha256", "spec_source"):
        if not str(document.get(field, "")).strip():
            errors.append(f"{label}: thiếu trường `{field}`")
    sha = str(document.get("catalog_sha256", ""))
    if not re.fullmatch(r"[0-9a-f]{64}", sha):
        errors.append(f"{label}: catalog_sha256 không phải SHA-256 hex 64 ký tự")

    strings = document.get("strings")
    if not isinstance(strings, dict):
        errors.append(f"{label}: thiếu bảng `strings`")
        return errors

    by_key = {row.key: row for row in rows}
    for key in sorted(set(strings) - set(by_key)):
        errors.append(f"{label}: khóa `{key}` không có trong §2 của spec")
    for key in sorted(set(by_key) - set(strings)):
        errors.append(f"{label}: thiếu khóa `{key}` có trong §2 của spec")

    for key in sorted(strings):
        entry = strings[key]
        row = by_key.get(key)
        if not isinstance(entry, dict):
            errors.append(f"{label}: `{key}` không phải bảng")
            continue
        status = entry.get("status")
        if status not in STATUSES:
            errors.append(f"{label}: `{key}` có status lạ `{status!r}`")
            continue
        has_text = "text" in entry
        if status in TEXT_STATUSES:
            if not has_text:
                errors.append(f"{label}: `{key}` ({status}) phải có text")
            if not str(entry.get("catalog_id", "")).strip():
                errors.append(f"{label}: `{key}` ({status}) phải có catalog_id")
        else:
            if has_text:
                errors.append(f"{label}: `{key}` ({status}) không được có text")
            if not str(entry.get("reason", "")).strip():
                errors.append(f"{label}: `{key}` ({status}) phải có reason")
        if row is not None:
            if status != row.status:
                errors.append(
                    f"{label}: `{key}` có status `{status}`, §2 ghi `{row.status}`"
                )
            locator = row.locator
            if locator.startswith(CATALOG_PREFIX):
                expected_id = locator[len(CATALOG_PREFIX):]
                if entry.get("catalog_id") != expected_id:
                    errors.append(
                        f"{label}: `{key}` có catalog_id {entry.get('catalog_id')!r}, "
                        f"§2 ghi `{expected_id}`"
                    )
            elif entry.get("catalog_id") is not None:
                errors.append(f"{label}: `{key}` có catalog_id nhưng §2 không có locator catalog")
            if str(entry.get("note", "")) != row.note:
                errors.append(f"{label}: `{key}` có note khác §2")
            if catalog is not None and has_text:
                source = pick_catalog_row(catalog, str(entry.get("catalog_id", "")))
                if entry["text"] != source["vi"].strip():
                    errors.append(
                        f"{label}: `{key}`: text khác `catalog.vi` sau trim"
                    )
        if has_text:
            value = entry["text"]
            if not isinstance(value, str) or not value.strip():
                errors.append(f"{label}: `{key}`: text rỗng")
            else:
                if unicodedata.normalize("NFC", value) != value:
                    errors.append(f"{label}: `{key}`: text chưa ở dạng NFC")
                if value != value.strip():
                    errors.append(f"{label}: `{key}`: text còn khoảng trắng biên")
        if status == STATUS_IMAGE and not str(entry.get("image", "")).strip():
            errors.append(f"{label}: `{key}` (image-pending) phải có image")

    if catalog is not None and catalog_sha256 is not None and sha != catalog_sha256:
        errors.append(
            f"{label}: catalog_sha256 {sha} khác catalog hiện tại {catalog_sha256}"
        )
    return errors


def check_placeholder_groups(strings: dict) -> list[str]:
    """Placeholder của các khóa dùng chung một `catalog_id` phải khớp số lượng."""
    groups: dict[str, list[tuple[str, list[str]]]] = {}
    for key, entry in strings.items():
        if not isinstance(entry, dict):
            continue
        catalog_id = entry.get("catalog_id")
        text = entry.get("text")
        if not catalog_id or not isinstance(text, str):
            continue
        groups.setdefault(str(catalog_id), []).append((key, placeholder_tokens(text)))
    errors: list[str] = []
    for catalog_id, members in sorted(groups.items()):
        if len(members) < 2:
            continue
        reference_key, reference = members[0]
        for key, tokens in members[1:]:
            if sorted(tokens) != sorted(reference):
                errors.append(
                    f"catalog `{catalog_id}`: placeholder của `{key}` "
                    f"{sorted(tokens)} khác `{reference_key}` {sorted(reference)}"
                )
    return errors


def check_summary(rows: list[SpecRow], summary: dict[str, int]) -> list[str]:
    """Đối chiếu số đếm của §2 với phần tổng hợp ghi ở §1."""
    counts = {status: 0 for status in STATUSES}
    counts["total"] = len(rows)
    counts["catalog"] = 0
    for row in rows:
        counts[row.status] += 1
        if row.status in TEXT_STATUSES:
            counts["catalog"] += 1
    errors = []
    for name, value in sorted(summary.items()):
        if counts.get(name) != value:
            errors.append(
                f"UI_STRINGS_V1 §1 ghi `{name}` = {value}, §2 cho ra {counts.get(name)}"
            )
    return errors


# ------------------------------------------------------------------------ main


def report(entries: list[Entry]) -> str:
    counts = {status: 0 for status in STATUSES}
    for entry in entries:
        counts[entry.status] += 1
    with_text = counts[STATUS_TEXT] + counts[STATUS_REUSED]
    missing = len(entries) - with_text
    parts = [f"tổng {len(entries)}", f"có text {with_text}"]
    parts += [f"{status} {counts[status]}" for status in STATUSES]
    return f"Tổng hợp: {', '.join(parts)}; còn thiếu text {missing} khóa."


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="kiểm file đã xuất, không ghi lại")
    parser.add_argument("--spec", type=Path, default=SPEC_PATH, help="file đặc tả khóa V1")
    parser.add_argument("--catalog", type=Path, default=DEFAULT_CATALOG, help="catalog Việt hóa")
    parser.add_argument("--out", type=Path, default=OUT_PATH, help="file xuất")
    args = parser.parse_args()

    try:
        rows = parse_spec(args.spec)
        errors = check_summary(rows, spec_summary(args.spec))
        if errors:
            raise ExportError("\n".join(errors))

        catalog = None
        catalog_sha256 = None
        if args.catalog.is_file():
            catalog = read_catalog(args.catalog)
            catalog_sha256 = hashlib.sha256(args.catalog.read_bytes()).hexdigest()

        if args.check:
            if catalog is None:
                print(
                    f"Không có catalog tại {args.catalog}: chỉ kiểm schema và đối chiếu §2."
                )
            if not args.out.is_file():
                raise ExportError(f"thiếu file {args.out}")
            text = args.out.read_text(encoding="utf-8")
            errors = check_exported(text, str(args.out), rows, catalog, catalog_sha256)
            document, _ = parse_exported(text)
            if document is not None:
                errors += check_placeholder_groups(document.get("strings", {}))
            if errors:
                raise ExportError("\n".join(errors))
            print(f"OK: {args.out} khớp §2" + (" và catalog." if catalog else "."))
            return 0

        if catalog is None:
            raise ExportError(
                f"không có catalog tại {args.catalog}; cần catalog để xuất "
                "(bỏ --check để chỉ kiểm schema)"
            )
        entries = build_entries(rows, catalog)
        text = render(entries, catalog_sha256, str(args.catalog))
        # Tự kiểm nội dung vừa dựng trước khi ghi: xuất ra mà sai thì không ghi.
        errors = check_exported(text, str(args.out), rows, catalog, catalog_sha256)
        document, _ = parse_exported(text)
        if document is not None:
            errors += check_placeholder_groups(document.get("strings", {}))
        if errors:
            raise ExportError("\n".join(errors))
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text, encoding="utf-8")
        print(report(entries))
        print(f"catalog_sha256: {catalog_sha256}")
        print(f"Đã ghi {args.out}")
        return 0
    except ExportError as error:
        print(f"lỗi: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
