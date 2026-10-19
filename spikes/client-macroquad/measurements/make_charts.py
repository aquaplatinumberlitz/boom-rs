#!/usr/bin/env python3
"""Sinh ảnh đo cho ADR 0010 từ snapshot + metrics.json (không chứa asset game)."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
FONT_PATH = "/usr/share/fonts/TTF/DejaVuSans.ttf"
BG = (16, 20, 26)
FG = (226, 235, 242)
MUTED = (150, 164, 176)
GREEN = (76, 175, 80)
BLUE = (33, 150, 243)


def font(size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(FONT_PATH, size)


def bar_chart(metrics: dict, out: Path) -> None:
    image = Image.new("RGB", (960, 540), BG)
    draw = ImageDraw.Draw(image)
    draw.text((32, 24), "Spike client stack — chi phí build (Linux, release sạch)", fill=FG, font=font(22))
    draw.text(
        (32, 56),
        "Ryzen 5 3600 · Rust 1.99 · target sạch, registry đã warm · thấp hơn là tốt hơn",
        fill=MUTED,
        font=font(14),
    )

    linux = metrics["linux_release"]
    pairs = [
        ("macroquad", linux["macroquad"]["build_s"], GREEN),
        ("winit+softbuffer", linux["winit_softbuffer"]["build_s"], BLUE),
    ]
    max_build = 40.0
    base_y = 430
    draw.text((70, 92), "Thời gian build (giây)", fill=FG, font=font(16))
    for index, (label, value, color) in enumerate(pairs):
        x = 110 + index * 170
        height = int((value / max_build) * 280)
        draw.rectangle([x, base_y - height, x + 110, base_y], fill=color)
        draw.text((x + 12, base_y - height - 24), f"{value:.2f}s", fill=FG, font=font(16))
        draw.text((x, base_y + 10), label, fill=MUTED, font=font(13))
    draw.line([80, base_y, 440, base_y], fill=MUTED, width=1)

    sizes = [
        ("macroquad", linux["macroquad"]["binary_b"] / (1024 * 1024), GREEN),
        ("winit+softbuffer", linux["winit_softbuffer"]["binary_b"] / (1024 * 1024), BLUE),
    ]
    max_size = 7.0
    draw.text((530, 92), "Kích thước binary (MiB)", fill=FG, font=font(16))
    for index, (label, value, color) in enumerate(sizes):
        x = 570 + index * 170
        height = int((value / max_size) * 280)
        draw.rectangle([x, base_y - height, x + 110, base_y], fill=color)
        draw.text((x + 14, base_y - height - 24), f"{value:.2f} MiB", fill=FG, font=font(16))
        draw.text((x, base_y + 10), label, fill=MUTED, font=font(13))
    draw.line([540, base_y, 900, base_y], fill=MUTED, width=1)

    draw.text(
        (32, 500),
        "Nguồn: spikes/client-macroquad/measurements/metrics.json — chỉ số đo 2026-10-19.",
        fill=MUTED,
        font=font(12),
    )
    image.save(out)


def diff_chart(metrics: dict, out: Path) -> None:
    macro = np.asarray(Image.open(ROOT / "local/snap-macroquad-1x.png").convert("RGB"), dtype=np.int16)
    cpu = np.asarray(Image.open(ROOT / "local/snap-winit-1x.png").convert("RGB"), dtype=np.int16)
    diff = np.abs(macro - cpu).max(axis=2)
    scene = diff[40:]
    heat = np.zeros((scene.shape[0], scene.shape[1], 3), dtype=np.uint8)
    heat[scene > 0] = (110, 0, 0)
    heat[scene > 1] = (255, 96, 0)
    heat[scene > 5] = (255, 255, 0)

    canvas = Image.new("RGB", (600, 600), BG)
    canvas.paste(Image.fromarray(heat), (0, 60))
    draw = ImageDraw.Draw(canvas)
    stats = metrics["pixel"]["scene_1x"]
    draw.text(
        (12, 12),
        f"Sai khác pixel scene 1× — {stats['different']}/{stats['pixels']} "
        f"({stats['pct']:.3f}%), max {stats['max_delta']}/255",
        fill=FG,
        font=font(15),
    )
    draw.text((12, 36), "Đỏ: Δ1 · Cam: Δ2–5 · Vàng: Δ>5 · toàn bộ ở khói nổ bán trong suốt", fill=MUTED, font=font(12))
    canvas.save(out)


def main() -> None:
    metrics = json.loads((HERE / "metrics.json").read_text())
    bar_chart(metrics, HERE / "build-size.png")
    diff_chart(metrics, HERE / "pixel-diff-scene-1x.png")
    print(f"wrote {HERE / 'build-size.png'} và {HERE / 'pixel-diff-scene-1x.png'}")


if __name__ == "__main__":
    main()
