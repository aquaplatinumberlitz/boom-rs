#!/usr/bin/env python3
"""Dump mọi khối animation trong ObjectTable (ODT) của client Boom/Crazy Arcade.

Đầu vào: payload `fx/ObjectTable/ObjectTable`, lấy bằng
    boom-assets pack-extract <pack-dir> fx ObjectTable/ObjectTable ObjectTable.bin
Định dạng (R-094/R-251):
    u32 n; n × (u32 len, tên cp949, u32 offset)
    khối tại offset: u32 số state; state × (str nhãn, str entry, u32 k, k × (u32 frame, u32 thời lượng))
Thời lượng tính theo đơn vị 1/30 s (R-222); 0xFFFFFFFF = giữ vô hạn (R-223).

Dùng:  python3 -I odt_dump.py ObjectTable.bin out.json [lọc-tên ...]
"""
import json
import struct
import sys


def read_table(b):
    at = 0
    (n,) = struct.unpack_from("<I", b, at)
    at += 4
    names = []
    for _ in range(n):
        (ln,) = struct.unpack_from("<I", b, at)
        at += 4
        name = b[at:at + ln].decode("cp949", "replace")
        at += ln
        (off,) = struct.unpack_from("<I", b, at)
        at += 4
        names.append((name, off))
    return names


def read_block(b, off):
    a = off

    def u32():
        nonlocal a
        (v,) = struct.unpack_from("<I", b, a)
        a += 4
        return v

    def text():
        nonlocal a
        ln = u32()
        if ln == 0 or ln > 128:
            raise ValueError("chuỗi lạ")
        t = b[a:a + ln].decode("cp949", "replace")
        a += ln
        return t

    count = u32()
    if count == 0 or count > 500:
        raise ValueError("số state lạ")
    states = []
    for _ in range(count):
        label, entry, k = text(), text(), u32()
        if k > 2000:
            raise ValueError("số frame lạ")
        steps = [list(struct.unpack_from("<II", b, a + 8 * i)) for i in range(k)]
        a += 8 * k
        states.append({"label": label, "entry": entry, "steps": steps})
    return states


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    data = open(sys.argv[1], "rb").read()
    wanted = sys.argv[3:]
    out = {}
    for name, off in read_table(data):
        if wanted and not any(w in name for w in wanted):
            continue
        if off >= len(data):
            continue
        try:
            block = read_block(data, off)
        except (ValueError, struct.error):
            continue
        if block and all(s["steps"] for s in block):
            out[name] = block
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=1)
    print(f"{len(out)} khối animation → {sys.argv[2]}")


if __name__ == "__main__":
    main()
