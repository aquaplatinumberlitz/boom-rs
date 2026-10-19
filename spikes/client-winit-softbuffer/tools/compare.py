#!/usr/bin/env python3
"""So sánh hai snapshot PNG của spike client và in thống kê sai khác pixel.

Ảnh heatmap đầu ra chỉ chứa pixel sai khác (không chứa asset game), dùng làm bằng chứng
đo trong ADR 0010. Không phụ thuộc code Rust; cần Pillow + numpy.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--a", required=True, type=Path, help="snapshot tham chiếu")
    parser.add_argument("--b", required=True, type=Path, help="snapshot cần so")
    parser.add_argument("--hud", type=int, default=40, help="chiều cao HUD cần bỏ qua (px)")
    parser.add_argument("--out", type=Path, help="ghi heatmap PNG")
    parser.add_argument("--json", type=Path, help="ghi thống kê JSON")
    parser.add_argument(
        "--check-2x-from",
        type=Path,
        help="ảnh 1×; kiểm ảnh --b là nearest-neighbor expansion trực tiếp của ảnh này",
    )
    parser.add_argument(
        "--fail-different-gt",
        type=int,
        help="exit 2 nếu số pixel khác trong vùng scene vượt ngưỡng này",
    )
    parser.add_argument(
        "--fail-max-delta-gt",
        type=int,
        help="exit 2 nếu max delta kênh trong vùng scene vượt ngưỡng này",
    )
    args = parser.parse_args()

    a = Image.open(args.a).convert("RGB")
    b = Image.open(args.b).convert("RGB")
    if a.size != b.size:
        raise SystemExit(f"kích thước khác nhau: {a.size} vs {b.size}")
    width, height = a.size
    diff = np.abs(np.asarray(a, dtype=np.int16) - np.asarray(b, dtype=np.int16)).max(axis=2)

    hud = max(0, min(args.hud, height))
    scene = diff[hud:]
    scene_px = width * (height - hud)
    stats = {
        "size": [width, height],
        "hud_height": hud,
        "scene": {
            "pixels": scene_px,
            "different": int((scene > 0).sum()),
            "different_gt1": int((scene > 1).sum()),
            "different_gt5": int((scene > 5).sum()),
            "max_delta": int(scene.max()) if scene.size else 0,
            "pct": 100.0 * float((scene > 0).sum()) / scene_px if scene_px else 0.0,
        },
        "hud": {
            "pixels": width * hud,
            "different": int((diff[:hud] > 0).sum()),
            "different_gt1": int((diff[:hud] > 1).sum()),
            "max_delta": int(diff[:hud].max()) if hud else 0,
        },
    }

    failed = False
    if args.check_2x_from:
        small = Image.open(args.check_2x_from).convert("RGB")
        expected = np.repeat(np.repeat(np.asarray(small), 2, axis=0), 2, axis=1)
        actual = np.asarray(b)
        if actual.shape != expected.shape:
            raise SystemExit(
                f"kích thước expansion sai: {actual.shape} vs {expected.shape} "
                f"(1× {small.size})"
            )
        mismatch = int((actual != expected).any(axis=2).sum())
        stats["nearest_expansion_2x"] = {
            "source_1x": str(args.check_2x_from),
            "pixels": int(expected.shape[0] * expected.shape[1]),
            "mismatch": mismatch,
        }
        if mismatch:
            print(
                f"nearest expansion FAIL: {mismatch} pixel khác bản mở rộng 2× của "
                f"{args.check_2x_from}",
                file=sys.stderr,
            )
            failed = True
        else:
            print(f"nearest expansion PASS: {args.b} = 2× trực tiếp của {args.check_2x_from}")

    if args.fail_different_gt is not None and stats["scene"]["different"] > args.fail_different_gt:
        print(
            f"FAIL: {stats['scene']['different']} pixel khác > ngưỡng "
            f"{args.fail_different_gt}",
            file=sys.stderr,
        )
        failed = True
    if args.fail_max_delta_gt is not None and stats["scene"]["max_delta"] > args.fail_max_delta_gt:
        print(
            f"FAIL: max delta {stats['scene']['max_delta']} > ngưỡng "
            f"{args.fail_max_delta_gt}",
            file=sys.stderr,
        )
        failed = True

    print(json.dumps(stats, indent=2, ensure_ascii=False))

    if args.json:
        args.json.write_text(json.dumps(stats, indent=2, ensure_ascii=False) + "\n")

    if args.out:
        heat = np.zeros((height, width, 3), dtype=np.uint8)
        heat[diff > 0] = (110, 0, 0)
        heat[diff > 1] = (255, 96, 0)
        heat[diff > 5] = (255, 255, 0)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        Image.fromarray(heat).save(args.out)
        print(f"heatmap -> {args.out}")

    return 2 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
