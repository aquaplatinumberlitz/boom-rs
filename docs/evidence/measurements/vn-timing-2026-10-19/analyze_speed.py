#!/usr/bin/env python3
"""Đọc pos.csv của sampler2/3, đếm thay đổi x theo bước.

CẢNH BÁO: xok/yok trong các log này luôn false vì checksum chưa khớp
(sampler đọc blob[7:9], worker log ghi checksum ở +8). Vì vậy px/tick dưới
đây chỉ là quan sát thô, không phải giá trị đã xác thực.
"""
import re
import sys

path = sys.argv[1]
rows = []
for line in open(path, encoding="utf-8"):
    m = re.match(r"^S ([\d.]+) steps=(\d+).*? x=(-?\d+) y=(-?\d+) xok=(\w+) yok=(\w+)", line)
    if m:
        rows.append((float(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4)),
                     m.group(5).lower() == "true", m.group(6).lower() == "true"))

print("samples", len(rows), "xok_true", sum(r[4] for r in rows), "yok_true", sum(r[5] for r in rows))
c5 = c6 = other = 0
for a, b in zip(rows, rows[1:]):
    if a[2] == b[2] or a[1] == b[1]:
        continue
    ratio = (b[2] - a[2]) / (b[1] - a[1])
    if abs(ratio) == 5:
        c5 += 1
    elif abs(ratio) == 6:
        c6 += 1
    elif abs(b[2] - a[2]) < 100:
        other += 1
print("changes_5px_per_tick", c5, "changes_6px_per_tick", c6, "other_plausible", other)
print("t_s,steps,dx,dsteps,px_per_tick")
for a, b in zip(rows, rows[1:]):
    if a[2] != b[2]:
        dx, ds = b[2] - a[2], b[1] - a[1]
        if abs(dx) < 100 and ds:
            print(f"{b[0]:.3f},{b[1]},{dx},{ds},{dx/ds:.3f}")
