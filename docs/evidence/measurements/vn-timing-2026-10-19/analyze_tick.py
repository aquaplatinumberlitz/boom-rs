#!/usr/bin/env python3
"""Tính tốc độ bước từ tick-samples.txt (log sampler.py, 0.5 s/mẫu)."""
import re
from pathlib import Path

here = Path(__file__).resolve().parent
rows = []
for line in (here / "tick-samples.txt").read_text(encoding="utf-8").splitlines():
    m = re.match(r"S ([\d.]+) .*steps=(\d+)", line)
    if m:
        rows.append((float(m.group(1)), int(m.group(2))))

# tách run theo tiêu đề "## sampleN.txt"
runs, cur = {}, None
for line in (here / "tick-samples.txt").read_text(encoding="utf-8").splitlines():
    if line.startswith("## "):
        cur = line[3:].strip()
        runs[cur] = []
    else:
        m = re.match(r"S ([\d.]+) .*steps=(\d+)", line)
        if m and cur:
            runs[cur].append((float(m.group(1)), int(m.group(2))))

print("run,start_s,end_s,samples,steps_delta,steps_per_s,ms_per_step")
for name, rs in runs.items():
    t0, s0 = rs[0]
    t1, s1 = rs[-1]
    rate = (s1 - s0) / (t1 - t0)
    print(f"{name},{t0:.3f},{t1:.3f},{len(rs)},{s1-s0},{rate:.3f},{1000/rate:.2f}")
    # cửa sổ 2.5 s để thấy lượng tử mẫu
    start = 0.0
    while start < t1 - 0.1:
        seg = [r for r in rs if start <= r[0] <= start + 2.5]
        if len(seg) >= 2:
            r = (seg[-1][1] - seg[0][1]) / (seg[-1][0] - seg[0][0])
            print(f"{name},{start:.3f},{start+2.5:.3f},{len(seg)},{seg[-1][1]-seg[0][1]},{r:.3f},{1000/r:.2f}")
        start += 2.5
