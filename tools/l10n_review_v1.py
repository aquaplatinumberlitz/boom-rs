#!/usr/bin/env python3
"""Tạo trang HTML review cục bộ cho 126 khóa UI V1.

Nguồn (chỉ đọc, không sửa):

* `docs/specification/localization/UI_STRINGS_V1.md` §2: khóa, ngữ cảnh, locator,
  trạng thái nguồn và yêu cầu hiển thị;
* `/home/admin/boom-re-l10n/l10n/catalog.csv`: text `ko`/`vi` của các locator
  `catalog:<id>` (kể cả dòng `review`, để người duyệt thấy bản đang có);
* `/home/admin/boom-re-l10n/l10n/.cache/image-review/`: `manifest.json`,
  `images_review.csv` và ảnh giải mã cục bộ cho các dòng `image-pending`.

Cách chạy:

```sh
python3 tools/l10n_review_v1.py
xdg-open local/l10n-review-v1.html
```

Trang là **bản xem chỉ đọc**: không ghi quyết định duyệt, không tạo nguồn text
thứ hai. Nguồn chuẩn vẫn là §2 + `catalog.csv`; mọi text hiển thị lấy nguyên văn
từ catalog (không rút gọn, không thay ý). Output mặc định nằm trong `local/` đã
được Git bỏ qua.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import html
import json
import re
import sys
import unicodedata
from collections import Counter
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import l10n_export as export  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "docs/specification/localization/UI_STRINGS_V1.md"
CATALOG_PATH = Path("/home/admin/boom-re-l10n/l10n/catalog.csv")
IMAGE_REVIEW_DIR = Path("/home/admin/boom-re-l10n/l10n/.cache/image-review")
OUT_PATH = ROOT / "local/l10n-review-v1.html"

SECTION_RE = re.compile(r"^### (2\.\d+) (.+)$")
IMAGE_SPLIT_RE = re.compile(r"`\s*,\s*`\s*(?=ảnh\s)|,\s*(?=ảnh\s)")


@dataclass(frozen=True)
class ReviewRow:
    """Một dòng §2, giữ thêm mục/ngữ cảnh để hiển thị."""

    key: str
    section: str
    context: str
    locator: str
    status: str
    note: str


@dataclass
class CatalogLookup:
    """Kết quả tra một locator `catalog:<id>`."""

    catalog_id: str
    status: str | None = None
    ko: str | None = None
    vi: str | None = None
    error: str | None = None


@dataclass
class ImageLookup:
    """Một locator `ảnh <pack>:<entry>` và dữ liệu review cục bộ."""

    locator: str
    pack: str
    entry: str
    manifest: dict | None = None
    review_status: str | None = None
    review_notes: str | None = None
    error: str | None = None


def esc(value: object) -> str:
    """Escape mọi nội dung lấy từ spec/catalog trước khi đưa vào HTML."""
    return html.escape(str(value), quote=True)


def file_uri(path: Path) -> str:
    return path.resolve().as_uri()


def parse_review_rows(path: Path) -> list[ReviewRow]:
    """Đọc bảng §2, giữ tên mục và cột ngữ cảnh.

    Đối chiếu lại với `l10n_export.parse_spec` để hai tool không hiểu §2 khác nhau.
    """
    lines = path.read_text(encoding="utf-8").split("\n")
    start = end = None
    for index, line in enumerate(lines):
        if start is None and line.startswith("## 2. "):
            start = index
        elif start is not None and end is None and line.startswith("## 3. "):
            end = index
    if start is None or end is None:
        raise export.ExportError(f"{path}: không tìm thấy mục §2/§3 để đọc bảng khóa V1")

    section = ""
    rows: list[ReviewRow] = []
    for number, line in enumerate(lines[start:end], start=start + 1):
        heading = SECTION_RE.match(line)
        if heading:
            section = f"{heading.group(1)} {heading.group(2)}"
            continue
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 5:
            raise export.ExportError(
                f"{path}:{number + 1}: bảng §2 phải có 5 cột, thấy {len(cells)}"
            )
        key = cells[0].strip("`")
        if not export.KEY_RE.match(key):
            continue  # hàng tiêu đề và hàng đường kẻ
        status = cells[3].strip("`")
        if status not in export.STATUSES:
            raise export.ExportError(
                f"{path}:{number + 1}: khóa `{key}` có trạng thái lạ `{status}`"
            )
        rows.append(
            ReviewRow(
                key=key,
                section=section,
                context=cells[1],
                locator=cells[2].strip("`"),
                status=status,
                note=cells[4],
            )
        )

    canonical = {row.key: row for row in export.parse_spec(path)}
    seen: set[str] = set()
    for row in rows:
        if row.key in seen:
            raise export.ExportError(f"{path}: khóa V1 trùng `{row.key}`")
        seen.add(row.key)
        source = canonical.get(row.key)
        if source is None:
            raise export.ExportError(f"{path}: khóa `{row.key}` không có trong parse_spec")
        if (
            row.locator,
            row.status,
            row.note,
        ) != (source.locator, source.status, source.note):
            raise export.ExportError(
                f"{path}: khóa `{row.key}` đọc khác giữa parser review và l10n_export"
            )
    missing = sorted(set(canonical) - seen)
    if missing:
        raise export.ExportError(f"{path}: thiếu khóa khi đọc review: {', '.join(missing)}")
    if not section:
        raise export.ExportError(f"{path}: §2 không có mục con để lấy ngữ cảnh")
    return rows


def load_catalog(path: Path) -> tuple[dict[str, list[dict[str, str]]] | None, str | None, list[str]]:
    """Đọc catalog; thiếu file không làm tool dừng, chỉ thêm cảnh báo rõ."""
    warnings: list[str] = []
    if not path.is_file():
        warnings.append(
            f"Không thấy catalog tại {path}; trang vẫn liệt kê đủ khóa nhưng "
            "không có text KO/VI để đối chiếu."
        )
        return None, None, warnings
    try:
        with path.open(encoding="utf-8-sig", newline="") as stream:
            header = next(csv.reader(stream), None)
    except (OSError, UnicodeError, csv.Error) as error:
        warnings.append(f"Không đọc được header catalog {path}: {error}")
        return None, None, warnings
    required = {"id", "ko", "vi", "status"}
    if not header or not required <= set(header):
        warnings.append(
            f"Catalog {path} thiếu cột bắt buộc {sorted(required - set(header or []))}; "
            "không dùng catalog này."
        )
        return None, None, warnings
    try:
        catalog = export.read_catalog(path)
    except (OSError, UnicodeError, export.ExportError) as error:
        warnings.append(f"Không đọc được catalog {path}: {error}")
        return None, None, warnings
    sha256 = hashlib.sha256(path.read_bytes()).hexdigest()
    return catalog, sha256, warnings


def lookup_catalog(
    row: ReviewRow,
    catalog: dict[str, list[dict[str, str]]] | None,
    warnings: list[str],
) -> CatalogLookup | None:
    """Tra locator catalog; lỗi của một dòng được hiện trên trang, không crash."""
    if not row.locator.startswith(export.CATALOG_PREFIX):
        return None
    catalog_id = row.locator[len(export.CATALOG_PREFIX):]
    lookup = CatalogLookup(catalog_id=catalog_id)
    if catalog is None:
        lookup.error = "Không có catalog để tra locator này."
        return lookup
    try:
        source = export.pick_catalog_row(catalog, catalog_id)
    except (export.ExportError, KeyError) as error:
        lookup.error = f"Không tra được catalog `{catalog_id}`: {error}"
        warnings.append(f"`{row.key}`: {lookup.error}")
        return lookup
    try:
        lookup.status = source["status"]
        lookup.ko = source["ko"]
        lookup.vi = source["vi"]
    except KeyError as error:
        lookup.error = f"Catalog `{catalog_id}` thiếu cột {error}."
        warnings.append(f"`{row.key}`: {lookup.error}")
        return lookup
    expected = {
        export.STATUS_TEXT: "translated",
        export.STATUS_REUSED: "reused",
    }.get(row.status)
    if expected is not None and lookup.status != expected:
        warning = (
            f"`{row.key}`: §2 ghi `{row.status}` nhưng catalog `{catalog_id}` "
            f"có status `{lookup.status}`."
        )
        warnings.append(warning)
        lookup.error = warning
    return lookup


def split_image_locators(locator: str) -> list[str]:
    """Tách ô locator có nhiều ảnh (vd. `ảnh A`, `ảnh B`) thành từng locator."""
    parts = IMAGE_SPLIT_RE.split(locator)
    result: list[str] = []
    for part in parts:
        cleaned = part.strip().strip("`").strip()
        if not cleaned:
            continue
        if not cleaned.startswith(export.IMAGE_PREFIX):
            raise export.ExportError(f"locator ảnh lạ: `{cleaned}`")
        result.append(cleaned[len(export.IMAGE_PREFIX):].strip().strip("`").strip())
    return result


def load_image_manifest(path: Path, warnings: list[str]) -> dict[tuple[str, str], dict] | None:
    """Đọc manifest review ảnh; trả về map (pack, entry) -> mục."""
    if not path.is_file():
        warnings.append(
            f"Không thấy manifest ảnh tại {path}; dòng image-pending chỉ hiện locator."
        )
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        warnings.append(f"Không đọc được manifest ảnh {path}: {error}")
        return None
    if not isinstance(data, list):
        warnings.append(f"Manifest ảnh {path} không phải danh sách; bỏ qua liên kết ảnh.")
        return None
    by_ref: dict[tuple[str, str], dict] = {}
    for item in data:
        if not isinstance(item, dict):
            continue
        pack = item.get("pack")
        entry = item.get("entry")
        if isinstance(pack, str) and isinstance(entry, str):
            by_ref.setdefault((pack, entry), item)
    return by_ref


def load_image_review(path: Path, warnings: list[str]) -> dict[tuple[str, str], tuple[str, str]]:
    """Đọc trạng thái duyệt ảnh hiện có (chỉ để hiển thị, không quyết định gì)."""
    if not path.is_file():
        warnings.append(
            f"Không thấy {path}; không hiện được trạng thái duyệt ảnh hiện tại."
        )
        return {}
    try:
        with path.open(encoding="utf-8-sig", newline="") as stream:
            reader = csv.DictReader(stream)
            if not reader.fieldnames or not {"pack", "entry", "status"} <= set(reader.fieldnames):
                warnings.append(f"{path}: thiếu cột pack/entry/status; bỏ qua trạng thái ảnh.")
                return {}
            statuses: dict[tuple[str, str], tuple[str, str]] = {}
            for item in reader:
                pack = item.get("pack")
                entry = item.get("entry")
                if isinstance(pack, str) and isinstance(entry, str):
                    statuses[(pack, entry)] = (
                        item.get("status", "") or "",
                        item.get("notes", "") or "",
                    )
            return statuses
    except (OSError, csv.Error) as error:
        warnings.append(f"Không đọc được {path}: {error}")
        return {}


def lookup_images(
    row: ReviewRow,
    manifest: dict[tuple[str, str], dict] | None,
    review_statuses: dict[tuple[str, str], tuple[str, str]],
    warnings: list[str],
) -> list[ImageLookup]:
    if not row.locator.startswith(export.IMAGE_PREFIX):
        return []
    lookups: list[ImageLookup] = []
    for ref in split_image_locators(row.locator):
        if ":" not in ref:
            lookups.append(
                ImageLookup(locator=ref, pack="", entry=ref, error="locator ảnh thiếu `pack:entry`.")
            )
            warnings.append(f"`{row.key}`: locator ảnh thiếu `pack:entry`: `{ref}`.")
            continue
        pack, entry = ref.split(":", 1)
        item = manifest.get((pack, entry)) if manifest is not None else None
        status, notes = review_statuses.get((pack, entry), (None, None))
        lookup = ImageLookup(
            locator=ref,
            pack=pack,
            entry=entry,
            manifest=item,
            review_status=status,
            review_notes=notes,
        )
        if manifest is not None and item is None:
            lookup.error = f"Không thấy `{pack}:{entry}` trong manifest ảnh."
            warnings.append(f"`{row.key}`: {lookup.error}")
        if status is None and review_statuses:
            lookup.error = (lookup.error + " " if lookup.error else "") + (
                f"Không thấy `{pack}:{entry}` trong images_review.csv."
            )
        lookups.append(lookup)
    return lookups


def placeholder_html(lookup: CatalogLookup | None) -> str:
    """Hiện placeholder của KO/VI; lệch số lượng hoặc chưa NFC thì đánh dấu."""
    if lookup is None or lookup.ko is None or lookup.vi is None:
        return '<span class="muted">—</span>'
    ko_tokens = export.placeholder_tokens(lookup.ko.strip())
    vi_tokens = export.placeholder_tokens(lookup.vi.strip())
    parts = [
        '<div><span class="label">KO:</span> '
        + (" ".join(f"<code>{esc(token)}</code>" for token in ko_tokens) or '<span class="muted">không có</span>')
        + "</div>",
        '<div><span class="label">VI:</span> '
        + (" ".join(f"<code>{esc(token)}</code>" for token in vi_tokens) or '<span class="muted">không có</span>')
        + "</div>",
    ]
    if sorted(ko_tokens) != sorted(vi_tokens):
        parts.append('<div class="bad">Lệch placeholder giữa KO và VI</div>')
    if unicodedata.normalize("NFC", lookup.vi) != lookup.vi:
        parts.append('<div class="bad">VI chưa ở dạng NFC</div>')
    return "".join(parts)


def status_badge(status: str) -> str:
    labels = {
        export.STATUS_TEXT: "catalog-translated",
        export.STATUS_REUSED: "catalog-reused",
        export.STATUS_REVIEW: "review",
        export.STATUS_IMAGE: "image-pending",
        export.STATUS_UNKNOWN: "unknown",
    }
    return f'<span class="badge {esc(status)}">{esc(labels.get(status, status))}</span>'


def reason_text(row: ReviewRow) -> str:
    if row.status in export.TEXT_STATUSES:
        return "Đã có text chốt từ catalog."
    reason = export.REASONS.get(row.status, "")
    return f"<code>{esc(reason)}</code> — chưa dùng làm text đã chốt."


def catalog_cell(lookup: CatalogLookup | None) -> str:
    if lookup is None:
        return '<span class="muted">— (không phải locator catalog)</span>'
    parts = [f'<div><code>catalog:{esc(lookup.catalog_id)}</code></div>']
    if lookup.error:
        parts.append(f'<div class="bad">{esc(lookup.error)}</div>')
    if lookup.status:
        parts.append(f'<div class="small">catalog status: <code>{esc(lookup.status)}</code></div>')
    return "".join(parts)


def image_cell(
    lookups: list[ImageLookup],
    image_dir: Path,
    index_path: Path,
    manifest_path: Path,
) -> str:
    if not lookups:
        return '<span class="muted">—</span>'
    parts: list[str] = []
    if index_path.is_file():
        parts.append(
            f'<div><a href="{esc(file_uri(index_path))}">Mở công cụ duyệt ảnh cục bộ</a> '
            f'(tìm entry trong danh sách)</div>'
        )
    else:
        parts.append(f'<div class="bad">Không thấy công cụ duyệt ảnh tại {esc(index_path)}</div>')
    for lookup in lookups:
        parts.append('<div class="imgref">')
        parts.append(f'<div><code>ảnh {esc(lookup.locator)}</code></div>')
        if lookup.error:
            parts.append(f'<div class="bad">{esc(lookup.error)}</div>')
        if lookup.review_status is not None:
            parts.append(
                f'<div class="small">images_review.csv: '
                f'<code>{esc(lookup.review_status)}</code>'
                + (f' — {esc(lookup.review_notes)}' if lookup.review_notes else "")
                + "</div>"
            )
        elif lookup.review_status is None and lookup.manifest is not None:
            parts.append('<div class="small muted">Không thấy trạng thái trong images_review.csv.</div>')
        if lookup.manifest is None:
            parts.append(
                f'<div class="small muted">Không có mục trong {esc(manifest_path.name)}; '
                "không tự chọn ảnh thay thế.</div>"
            )
        else:
            for side, label in (("kr", "KR"), ("vn", "VN")):
                side_data = lookup.manifest.get(side)
                images = side_data.get("images", []) if isinstance(side_data, dict) else []
                links: list[str] = []
                for number, image in enumerate(images if isinstance(images, list) else [], start=1):
                    if not isinstance(image, dict):
                        continue
                    full = image.get("full")
                    if not isinstance(full, str):
                        continue
                    image_path = image_dir / full
                    if image_path.is_file():
                        links.append(
                            f'<a href="{esc(file_uri(image_path))}">ảnh {number}</a>'
                        )
                    else:
                        links.append(f'<span class="bad">thiếu file ảnh {number}</span>')
                if links:
                    parts.append(f'<div class="small">{esc(label)}: {" · ".join(links)}</div>')
                else:
                    parts.append(
                        f'<div class="small muted">{esc(label)}: không có ảnh trong manifest.</div>'
                    )
        parts.append("</div>")
    return "".join(parts)


def render_html(
    rows: list[ReviewRow],
    counts: Counter[str],
    summary: dict[str, int],
    catalog: dict[str, list[dict[str, str]]] | None,
    manifest: dict[tuple[str, str], dict] | None,
    image_statuses: dict[tuple[str, str], tuple[str, str]],
    catalog_path: Path,
    catalog_sha256: str | None,
    catalog_rows: int | None,
    image_dir: Path,
    warnings: list[str],
    out_path: Path,
) -> str:
    total = len(rows)
    with_text = counts[export.STATUS_TEXT] + counts[export.STATUS_REUSED]
    text_label = (
        "khóa có text từ catalog"
        if catalog is not None
        else "khóa thuộc trạng thái có text (catalog không đọc được)"
    )
    # Tra catalog/ảnh trước khi dựng banner để cảnh báo từng dòng cũng hiện ở đầu trang.
    resolved: list[tuple[ReviewRow, CatalogLookup | None, list[ImageLookup]]] = []
    for row in rows:
        resolved.append(
            (
                row,
                lookup_catalog(row, catalog, warnings),
                lookup_images(row, manifest, image_statuses, warnings),
            )
        )
    warning_html = (
        "<section class=\"warnings\" id=\"warnings\"><h2>Cảnh báo dữ liệu</h2><ul>"
        + "".join(f"<li>{esc(item)}</li>" for item in warnings)
        + "</ul></section>"
        if warnings
        else '<p class="ok">Không có cảnh báo dữ liệu.</p>'
    )
    summary_rows = "".join(
        f"<tr><th>{esc(status)}</th><td>{counts[status]}</td></tr>"
        for status in export.STATUSES
    )
    status_options = "".join(
        f'<option value="{esc(status)}">{esc(status)} ({counts[status]})</option>'
        for status in export.STATUSES
    )
    row_html: list[str] = []
    for row, catalog_lookup, images in resolved:
        search_data = f"{row.key} {row.section} {row.context}".lower()
        row_html.append(
            '<tr data-status="{status}" data-search="{search}">'.format(
                status=esc(row.status), search=esc(search_data)
            )
            + f'<td class="key"><code>{esc(row.key)}</code></td>'
            + f'<td><div class="small muted">{esc(row.section)}</div>{esc(row.context)}</td>'
            + f'<td>{status_badge(row.status)}<div class="small">{reason_text(row)}</div></td>'
            + f'<td class="locator">{catalog_cell(catalog_lookup)}</td>'
            + f'<td class="text-cell">{text_cell(catalog_lookup.ko if catalog_lookup else None, "en" if catalog_lookup and catalog_lookup.catalog_id.startswith("boom_rs_v1.") else "ko")}</td>'
            + f'<td class="text-cell">{text_cell(catalog_lookup.vi if catalog_lookup else None, "vi")}</td>'
            + f'<td class="placeholder">{placeholder_html(catalog_lookup)}</td>'
            + f'<td class="locator">{image_cell(images, image_dir, image_dir / "index.html", image_dir / "manifest.json")}</td>'
            + f'<td>{esc(row.note)}</td>'
            + "</tr>"
        )
    generated = datetime.now().astimezone().isoformat(timespec="seconds")
    sha_text = catalog_sha256 or "—"
    rows_text = f"{catalog_rows}" if catalog_rows is not None else "—"
    return f"""<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Review {total} khóa UI V1 — chỉ đọc</title>
<style>
:root {{ color-scheme: light dark; --line: #8884; --muted: #777; --bad: #b00020; --ok: #18794e; }}
* {{ box-sizing: border-box; }}
body {{ font: 14px/1.45 system-ui, "Segoe UI", sans-serif; margin: 0; padding: 18px; }}
h1 {{ font-size: 1.35rem; margin: 0 0 6px; }}
h2 {{ font-size: 1.05rem; margin: 18px 0 6px; }}
p {{ margin: 6px 0; }}
.wrap {{ max-width: 100%; overflow-x: auto; }}
.readonly {{ border: 2px solid var(--ok); padding: 8px 10px; border-radius: 6px; }}
.warnings {{ border: 2px solid var(--bad); padding: 8px 10px; border-radius: 6px; margin: 12px 0; }}
.warnings ul {{ margin: 4px 0 0 18px; padding: 0; }}
.ok {{ color: var(--ok); }}
.controls {{ display: flex; gap: 10px; flex-wrap: wrap; align-items: end; margin: 14px 0; }}
label {{ display: block; font-size: 0.85rem; color: var(--muted); }}
input, select {{ font: inherit; padding: 4px 6px; min-width: 14rem; }}
table {{ border-collapse: collapse; width: 100%; table-layout: auto; }}
th, td {{ border: 1px solid var(--line); padding: 6px; vertical-align: top; text-align: left; }}
thead th {{ position: sticky; top: 0; background: Canvas; z-index: 1; }}
td.key {{ white-space: nowrap; }}
td.text-cell {{ min-width: 16rem; max-width: 34rem; }}
.text {{ white-space: pre-wrap; overflow-wrap: anywhere; }}
td.locator {{ max-width: 30rem; overflow-wrap: anywhere; }}
td.placeholder {{ min-width: 12rem; }}
code {{ font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 0.92em; }}
.small {{ font-size: 0.85rem; }}
.muted {{ color: var(--muted); }}
.bad {{ color: var(--bad); }}
.badge {{ display: inline-block; padding: 1px 6px; border-radius: 10px; border: 1px solid currentColor; font-size: 0.8rem; white-space: nowrap; }}
.badge.catalog-translated, .badge.catalog-reused {{ color: var(--ok); }}
.badge.review {{ color: #9a6700; }}
.badge.image-pending {{ color: #6f42c1; }}
.badge.unknown {{ color: var(--bad); }}
.imgref {{ margin-top: 4px; padding-top: 4px; border-top: 1px dashed var(--line); }}
.label {{ color: var(--muted); }}
tr[hidden] {{ display: none; }}
footer {{ margin-top: 18px; color: var(--muted); font-size: 0.85rem; }}
</style>
</head>
<body>
<h1>Review {total} khóa UI V1 — bản xem chỉ đọc</h1>
<p class="readonly"><strong>Không phải nguồn quyết định duyệt.</strong> Trang chỉ đọc §2 của
<code>UI_STRINGS_V1.md</code> và catalog hiện có; không ghi/không approve, không tạo bản dịch mới.
Nguồn chuẩn vẫn là <code>catalog.csv</code> và <code>data/l10n/vi-v1.toml</code> (do tool sinh).</p>
<p>Tổng <strong>{total}</strong> khóa; <strong>{with_text}</strong> {esc(text_label)};
còn <strong>{total - with_text}</strong> khóa chưa dùng được text.</p>
{warning_html}
<section>
<h2>Số đếm theo trạng thái §2</h2>
<table style="max-width: 36rem"><thead><tr><th>Trạng thái</th><th>Số khóa</th></tr></thead>
<tbody>{summary_rows}<tr><th>Tổng</th><td>{total}</td></tr></tbody></table>
<p class="small">§1 ghi: tổng {esc(summary.get("total", "—"))}, catalog-translated
{esc(summary.get(export.STATUS_TEXT, "—"))}, catalog-reused {esc(summary.get(export.STATUS_REUSED, "—"))},
review {esc(summary.get(export.STATUS_REVIEW, "—"))}, image-pending {esc(summary.get(export.STATUS_IMAGE, "—"))},
unknown {esc(summary.get(export.STATUS_UNKNOWN, "—"))}.</p>
</section>
<section>
<h2>Metadata</h2>
<p class="small">Spec: <code>{esc(SPEC_PATH)}</code><br>
Catalog: <code>{esc(catalog_path)}</code> — {rows_text} dòng dữ liệu, SHA-256
<code>{esc(sha_text)}</code><br>
Ảnh review: <code>{esc(image_dir)}</code><br>
Output cục bộ: <code>{esc(out_path)}</code><br>
Sinh lúc: {esc(generated)} — output nằm trong thư mục đã Git bỏ qua, không commit.</p>
</section>
<section>
<h2>Bảng khóa</h2>
<div class="controls">
<div><label for="status">Lọc trạng thái</label>
<select id="status"><option value="">Tất cả ({total})</option>{status_options}</select></div>
<div><label for="search">Tìm khóa / ngữ cảnh</label>
<input id="search" type="search" placeholder="vd. lobby.create" autocomplete="off"></div>
<div id="count" class="small"></div>
</div>
<div class="wrap">
<table>
<thead><tr>
<th>Khóa V1</th><th>Ngữ cảnh</th><th>Trạng thái / lý do</th><th>Locator catalog</th>
<th>Nguồn (KR / DESIGN EN)</th><th>VI (nguồn)</th><th>Placeholder</th><th>Ảnh / công cụ review</th>
<th>Yêu cầu hiển thị / ghi chú §2</th>
</tr></thead>
<tbody>
{chr(10).join(row_html)}
</tbody>
</table>
</div>
</section>
<footer>Sinh bằng <code>python3 tools/l10n_review_v1.py</code>. Không sửa tay output;
không dùng trang này làm quyết định duyệt thứ hai.</footer>
<script>
(function () {{
  const rows = Array.from(document.querySelectorAll('tbody tr[data-status]'));
  const status = document.getElementById('status');
  const search = document.getElementById('search');
  const count = document.getElementById('count');
  function apply() {{
    const wanted = status.value;
    const query = search.value.trim().toLowerCase();
    let shown = 0;
    for (const row of rows) {{
      const ok = (!wanted || row.dataset.status === wanted)
        && (!query || row.dataset.search.includes(query));
      row.hidden = !ok;
      if (ok) shown += 1;
    }}
    count.textContent = shown + ' / ' + rows.length + ' dòng';
  }}
  status.addEventListener('change', apply);
  search.addEventListener('input', apply);
  apply();
}})();
</script>
</body>
</html>
"""


def text_cell(value: str | None, lang: str) -> str:
    if value is None:
        return '<span class="muted">—</span>'
    return f'<div class="text" lang="{esc(lang)}">{esc(value)}</div>'


def approved_images_html(image_manifest: dict | None, image_dir: Path) -> str:
    """Ảnh thật bundle sẽ dùng; tách khỏi các locator text dự phòng."""
    import tomllib
    path = ROOT / "data/assets/ui-vi-v1.toml"
    if not path.is_file():
        return ""
    decisions = tomllib.loads(path.read_text(encoding="utf-8"))
    base_ui = tomllib.loads((ROOT / "data/assets/ui-v1.toml").read_text(encoding="utf-8"))
    entries = {row["id"]: row.get("entry") for row in base_ui["elements"]}
    parts = ['<section id="approved-images"><h2>Ảnh Việt hóa đã duyệt</h2>',
             '<p>Chủ dự án duyệt ngày 2026-10-24. Đây là nguồn ảnh cho bundle Việt hóa; '
             'chữ catalog ở trên vẫn là dự phòng. Ảnh Win/Lose dùng nhãn Thắng/Thua.</p>']
    for item in decisions["images"]:
        paths = []
        if item["kind"] == "png":
            paths = [ROOT / "local/l10n-assets" / item["file"]]
        elif image_manifest is not None:
            logical = entries.get(item["id"], "")
            if logical:
                pack, entry = logical.split("/", 1)
                found = image_manifest.get((pack, entry), {})
                paths = [image_dir / image["full"] for image in (found.get("vn") or {}).get("images", [])]
        parts.append('<div><h3>' + esc(item["id"]) + '</h3><code>SHA256 ' + esc(item["sha256"]) + '</code><br>')
        for asset in paths:
            if asset.is_file():
                url = file_uri(asset)
                parts.append('<a href="' + esc(url) + '"><img style="max-width:300px;max-height:180px" src="' + esc(url) + '" alt="' + esc(item["id"]) + '"></a>')
            else:
                parts.append('<span class="muted">Chưa có file local: ' + esc(asset) + '</span>')
        parts.append('</div>')
    parts.append('</section>')
    return "".join(parts)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--spec", type=Path, default=SPEC_PATH, help="file UI_STRINGS_V1.md")
    parser.add_argument("--catalog", type=Path, default=CATALOG_PATH, help="catalog Việt hóa")
    parser.add_argument("--images", type=Path, default=IMAGE_REVIEW_DIR, help="thư mục review ảnh cục bộ")
    parser.add_argument("--image-decisions", type=Path,
                        default=Path("/home/admin/boom-re-l10n/l10n/images_review.csv"),
                        help="quyết định duyệt thật; nếu chưa có thì dùng mẫu CSV ở cache")
    parser.add_argument("--out", type=Path, default=OUT_PATH, help="file HTML cục bộ cần ghi")
    args = parser.parse_args()

    try:
        rows = parse_review_rows(args.spec)
        summary = export.spec_summary(args.spec)
        errors = export.check_summary(rows, summary)
        expected_summary = {"total", "catalog", *export.STATUSES}
        if expected_summary - set(summary):
            errors.append(
                f"{args.spec}: không đọc được đủ số đếm §1 "
                f"(thiếu {', '.join(sorted(expected_summary - set(summary)))}); "
                "kiểm lại câu chữ tổng hợp"
            )
        if errors:
            raise export.ExportError("\n".join(errors))

        warnings: list[str] = []
        catalog, catalog_sha256, catalog_warnings = load_catalog(args.catalog)
        warnings.extend(catalog_warnings)
        catalog_rows = sum(len(value) for value in catalog.values()) if catalog else None
        manifest = load_image_manifest(args.images / "manifest.json", warnings)
        decisions = args.image_decisions if args.image_decisions.is_file() else args.images / "images_review.csv"
        image_statuses = load_image_review(decisions, warnings)

        counts = Counter(row.status for row in rows)
        text = render_html(
            rows,
            counts,
            summary,
            catalog,
            manifest,
            image_statuses,
            args.catalog,
            catalog_sha256,
            catalog_rows,
            args.images,
            warnings,
            args.out,
        )
        text = text.replace("</footer>", "</footer>" + approved_images_html(manifest, args.images), 1)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text, encoding="utf-8")
    except (export.ExportError, OSError) as error:
        print(f"lỗi: {error}", file=sys.stderr)
        return 1

    print(
        f"Đã tạo {args.out}: {len(rows)} khóa; "
        + ", ".join(f"{status} {counts[status]}" for status in export.STATUSES)
        + f"; {counts[export.STATUS_TEXT] + counts[export.STATUS_REUSED]} khóa thuộc "
        "trạng thái có text."
    )
    if catalog_sha256:
        print(f"catalog: {args.catalog} SHA-256 {catalog_sha256}")
    if warnings:
        print("CẢNH BÁO:", file=sys.stderr)
        for warning in warnings:
            print(f"  - {warning}", file=sys.stderr)
        print("Trang vẫn được tạo nhưng thiếu dữ liệu đối chiếu; xem mục Cảnh báo trong trang.")
        return 1
    print(f"Mở bằng: xdg-open {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
