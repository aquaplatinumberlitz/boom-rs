#!/usr/bin/env python3
"""Fixture pixel tổng hợp cho spike client — không dùng asset game.

Sinh sprite bằng code (pattern màu + alpha), một scene JSON dùng được bởi cả hai renderer,
và ảnh expected 1×/2× do một compositor tham chiếu độc lập (Pillow/numpy) tính.

Quy ước:
- pivot bất đối xứng: anchor của ba sprite khác nhau và khác tâm;
- 3 lớp chồng theo z-order (object sort theo y trong cả hai renderer);
- alpha bán trong suốt 64/96/128/160/200/255;
- additive: hai spike hiện quy đổi additive→alpha ở prepare-assets, không có blend additive
  per-draw, nên fixture chỉ kiểm straight alpha. Không có pixel additive riêng.

Expected dùng công thức chuẩn:
  out = (src_rgb * a + dst_rgb * (255 - a) + 127) // 255
và vị trí object:
  x = map_origin_x + cell_x*tile - anchor_x
  y = map_origin_y + cell_y*tile + tile - (anchor_y + 1)
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image

LOGIC = (600, 560)
TILE = 40
HUD = 40
MAP_ORIGIN = (0, 40)
BACKGROUND = (16, 18, 24)
SPRITE_W, SPRITE_H = 100, 80

# Ba anchor bất đối xứng, khác tâm (50, 40).
SPRITES = {
    "layer_a": {"anchor": [12, 37]},
    "layer_b": {"anchor": [50, 58]},
    "layer_c": {"anchor": [88, 60]},
    "transparent": {"anchor": [0, 0], "w": 1, "h": 1},
}
# cell (x, y); y tăng dần = thứ tự vẽ sau (z-order trên).
OBJECTS = [
    ("layer_a", 1, 1),
    ("layer_b", 2, 2),
    ("layer_c", 3, 3),
]


def make_sprites() -> dict[str, np.ndarray]:
    yy, xx = np.mgrid[0:SPRITE_H, 0:SPRITE_W]
    a = np.zeros((SPRITE_H, SPRITE_W, 4), dtype=np.uint8)
    a[..., 0] = (20 + xx * 2) % 256
    a[..., 1] = (40 + yy * 3) % 256
    a[..., 2] = (140 + (xx + yy)) % 256
    a[..., 3] = 255
    # Lỗ trong để thấy nền/lớp dưới.
    a[10:30, 20:40] = 0

    b = np.zeros((SPRITE_H, SPRITE_W, 4), dtype=np.uint8)
    b[..., 0] = (180 + xx) % 256
    b[..., 1] = (30 + yy * 2) % 256
    b[..., 2] = 60
    b_alpha = np.full((SPRITE_H, SPRITE_W), 64, dtype=np.uint8)
    b_alpha[:, 25:50] = 128
    b_alpha[:, 50:75] = 200
    b_alpha[:, 75:] = 255
    b_alpha[0:8, :] = 96
    b[..., 3] = b_alpha

    c = np.zeros((SPRITE_H, SPRITE_W, 4), dtype=np.uint8)
    c[..., 0] = 40
    c[..., 1] = (120 + yy * 2) % 256
    c[..., 2] = (200 + xx) % 256
    c_alpha = np.zeros((SPRITE_H, SPRITE_W), dtype=np.uint8)
    c_alpha[15:65, 10:90] = 160
    c_alpha[25:55, 25:75] = 220
    c_alpha[35:45, 35:65] = 255
    c[..., 3] = c_alpha

    transparent = np.zeros((1, 1, 4), dtype=np.uint8)
    return {
        "layer_a": a,
        "layer_b": b,
        "layer_c": c,
        "transparent": transparent,
    }


def object_position(cell_x: int, cell_y: int, anchor: list[int]) -> tuple[int, int]:
    x = MAP_ORIGIN[0] + cell_x * TILE - anchor[0]
    y = MAP_ORIGIN[1] + cell_y * TILE + TILE - (anchor[1] + 1)
    return x, y


def blend_region(dst: np.ndarray, src: np.ndarray, x: int, y: int) -> None:
    """Composite RGBA src lên dst RGB tại (x, y), công thức straight alpha chuẩn."""
    height, width = dst.shape[:2]
    sh, sw = src.shape[:2]
    x0, y0 = max(0, x), max(0, y)
    x1, y1 = min(width, x + sw), min(height, y + sh)
    if x1 <= x0 or y1 <= y0:
        return
    sx0, sy0 = x0 - x, y0 - y
    piece = src[sy0 : sy0 + (y1 - y0), sx0 : sx0 + (x1 - x0)]
    alpha = piece[..., 3:4].astype(np.int32)
    rgb = piece[..., :3].astype(np.int32)
    region = dst[y0:y1, x0:x1].astype(np.int32)
    blended = (rgb * alpha + region * (255 - alpha) + 127) // 255
    dst[y0:y1, x0:x1] = blended.astype(np.uint8)


def expected_image(sprites: dict[str, np.ndarray]) -> np.ndarray:
    canvas = np.zeros((LOGIC[1], LOGIC[0], 3), dtype=np.uint8)
    canvas[:, :] = BACKGROUND
    # Renderer sort object theo y; fixture dùng y tăng dần nên thứ tự là OBJECTS.
    for key, cell_x, cell_y in sorted(OBJECTS, key=lambda item: item[2]):
        x, y = object_position(cell_x, cell_y, SPRITES[key]["anchor"])
        blend_region(canvas, sprites[key], x, y)
    return canvas


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, default=Path("local/pixel-fixture"))
    args = parser.parse_args()
    out = args.out
    (out / "sprites").mkdir(parents=True, exist_ok=True)

    sprites = make_sprites()
    sprite_meta = {}
    for key, image in sprites.items():
        rel = f"sprites/{key}.png"
        Image.fromarray(image, "RGBA").save(out / rel)
        sprite_meta[key] = {
            "file": rel,
            "w": int(image.shape[1]),
            "h": int(image.shape[0]),
            "anchor": SPRITES[key]["anchor"],
        }

    scene = {
        "schema": 1,
        "tile_size": TILE,
        "logic_size": list(LOGIC),
        "map_origin": list(MAP_ORIGIN),
        "background": list(BACKGROUND),
        "hud_height": HUD,
        "sprites": sprite_meta,
        "map": {
            "width": 4,
            "height": 4,
            "ground": ["transparent"] * 16,
            "objects": [
                {"x": cell_x, "y": cell_y, "sprite": key}
                for key, cell_x, cell_y in OBJECTS
            ],
        },
        "characters": {"right": ["transparent"]},
        "bomb": [],
        "explosion": [],
        "character_cell": [0, 0],
        "character_dir": "right",
        "bomb_cell": [0, 0],
        "explosion_cell": [0, 0],
        "audio": {"bgm": "audio/bgm.ogg", "sfx": "audio/sfx.ogg"},
        "text": "",
    }
    (out / "scene.json").write_text(
        json.dumps(scene, indent=2, ensure_ascii=False) + "\n"
    )

    expected = expected_image(sprites)
    Image.fromarray(expected, "RGB").save(out / "expected-1x.png")
    expanded = np.repeat(np.repeat(expected, 2, axis=0), 2, axis=1)
    Image.fromarray(expanded, "RGB").save(out / "expected-2x.png")

    manifest = {
        "scene": str(out / "scene.json"),
        "expected_1x": str(out / "expected-1x.png"),
        "expected_2x": str(out / "expected-2x.png"),
        "logic_size": list(LOGIC),
        "objects": [
            {
                "sprite": key,
                "cell": [cell_x, cell_y],
                "anchor": SPRITES[key]["anchor"],
                "position_1x": list(object_position(cell_x, cell_y, SPRITES[key]["anchor"])),
            }
            for key, cell_x, cell_y in OBJECTS
        ],
        "alpha_values": [64, 96, 128, 160, 200, 255],
        "additive": "không có per-draw; spike quy đổi additive→alpha trước khi render",
        "blend_formula": "out = (src*a + dst*(255-a) + 127) // 255",
    }
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    )
    print(
        f"fixture: {out} — 4 sprite, 3 lớp, expected 1×/2×; "
        f"anchor {[SPRITES[k]['anchor'] for k in ('layer_a','layer_b','layer_c')]}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
