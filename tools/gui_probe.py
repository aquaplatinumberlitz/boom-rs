"""Bounded real-window probe of an unpacked package; JSON only, no game media upload.

A successful UI capture proves this runner created a framebuffer of the recorded
size. It does not prove OS focus, hardware requirements, FPS or release acceptance.
A failed capture records the observation; its product/environment cause is unknown.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import sys
import tempfile
import time
import zlib


def inspect_png(path):
    """Check all chunk CRCs, image stream and dimensions, not just a file suffix."""
    data = path.read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('PNG signature missing')
    pos, size, compressed, ended = 8, None, bytearray(), False
    while pos < len(data):
        if pos + 12 > len(data):
            raise ValueError('truncated PNG chunk')
        length = struct.unpack_from('>I', data, pos)[0]
        kind = data[pos + 4:pos + 8]
        end = pos + 8 + length
        if end + 4 > len(data):
            raise ValueError('truncated PNG payload')
        payload = data[pos + 8:end]
        if zlib.crc32(kind + payload) != struct.unpack_from('>I', data, end)[0]:
            raise ValueError('PNG CRC mismatch')
        if size is None:
            if kind != b'IHDR' or length != 13:
                raise ValueError('PNG IHDR missing')
            width, height, depth, color, compression, filtering, interlace = struct.unpack('>IIBBBBB', payload)
            if not (0 < width <= 8192 and 0 < height <= 8192):
                raise ValueError('PNG size outside probe bound')
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise ValueError('expected non-interlaced RGBA8 capture')
            size = (width, height)
        if kind == b'IDAT':
            compressed.extend(payload)
        if kind == b'IEND':
            if length or end + 4 != len(data):
                raise ValueError('invalid PNG end')
            ended = True
        pos = end + 4
    if not ended or size is None:
        raise ValueError('PNG end missing')
    decoder = zlib.decompressobj()
    expected = (size[0] * 4 + 1) * size[1]
    raw = decoder.decompress(compressed, expected + 1)
    if len(raw) != expected or not decoder.eof or decoder.unused_data:
        raise ValueError('PNG image stream size/end mismatch')
    stride = size[0] * 4 + 1
    if any(raw[i] > 4 for i in range(0, len(raw), stride)):
        raise ValueError('PNG filter invalid')
    # Reconstruct scanlines: a checksum-valid black/blank framebuffer is not a UI control.
    previous = bytearray(size[0] * 4)
    first_rgb, varied = None, False
    for row_start in range(0, len(raw), stride):
        mode = raw[row_start]
        row = bytearray(raw[row_start + 1:row_start + stride])
        for i in range(len(row)):
            left = row[i - 4] if i >= 4 else 0
            above = previous[i]
            upper_left = previous[i - 4] if i >= 4 else 0
            if mode == 1:
                predictor = left
            elif mode == 2:
                predictor = above
            elif mode == 3:
                predictor = (left + above) // 2
            elif mode == 4:
                p = left + above - upper_left
                dl, da, du = abs(p - left), abs(p - above), abs(p - upper_left)
                predictor = left if dl <= da and dl <= du else above if da <= du else upper_left
            else:
                predictor = 0
            row[i] = (row[i] + predictor) & 255
        for i in range(0, len(row), 4):
            rgb = row[i:i + 3]
            if first_rgb is None:
                first_rgb = rgb
            elif rgb != first_rgb:
                varied = True
        previous = row
    if not varied:
        raise ValueError('uniform framebuffer; UI positive control absent')
    return {'width': size[0], 'height': size[1], 'bytes': len(data),
            'rgb_nonuniform': True, 'sha256': hashlib.sha256(data).hexdigest()}


def text_tail(value):
    if isinstance(value, bytes):
        value = value.decode('utf-8', errors='replace')
    return (value or '')[-12000:]


def capture(binary, scale, timeout, run=subprocess.run):
    with tempfile.TemporaryDirectory(prefix='boom-gui-probe-') as temp:
        shot = Path(temp) / 'ui.png'
        args = [str(binary), '--ui-demo', '--ui-shot', 'login', '--windowed',
                '--scale', str(scale), '--snapshot', str(shot)]
        result = {'scale': scale, 'args': args, 'expected_size': [800 * scale, 600 * scale],
                  'status': 'FAIL', 'cause': 'unclassified'}
        started = time.monotonic()
        try:
            done = run(args, cwd=temp, capture_output=True, timeout=timeout, check=False)
            result.update(exit_code=done.returncode, stdout=text_tail(done.stdout), stderr=text_tail(done.stderr))
            if done.returncode != 0:
                result['observation'] = 'PROCESS_EXIT_NONZERO'
            else:
                try:
                    image = inspect_png(shot)
                    result['image'] = image
                    if [image['width'], image['height']] == result['expected_size']:
                        result.update(status='PASS', observation='FRAMEBUFFER_CAPTURE_VALID', cause=None)
                    else:
                        result['observation'] = 'FRAMEBUFFER_SIZE_DIFFERS'
                except (OSError, ValueError, zlib.error) as error:
                    result.update(observation='PNG_INVALID_OR_MISSING', error=str(error))
        except subprocess.TimeoutExpired as error:
            result.update(observation='TIMEOUT', stdout=text_tail(error.stdout), stderr=text_tail(error.stderr))
        except OSError as error:
            result.update(observation='PROCESS_START_FAILED', error=str(error))
        result['elapsed_seconds'] = time.monotonic() - started
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--timeout', type=float, default=25)
    args = parser.parse_args()
    if not 0 < args.timeout <= 60:
        parser.error('timeout must be in (0,60] seconds per capture')
    package = args.package.resolve()
    binary = package / 'bin' / ('boom-client.exe' if os.name == 'nt' else 'boom-client')
    report = {'schema': 1, 'os': platform.system(), 'machine': platform.machine(),
              'platform': platform.platform(), 'package': str(package),
              'source_head': os.environ.get('GITHUB_SHA'),
              'workflow_run_id': os.environ.get('GITHUB_RUN_ID'),
              'version': os.environ.get('BOOM_QA_VERSION'),
              'scope': 'Native UI window/framebuffer only; not focus/minHW/FPS/DAC/G6',
              'screenshots_retained_or_uploaded': False, 'files': {}, 'captures': []}
    for name in ['bin/' + binary.name, 'data/rules/v1-dev.toml', 'data/l10n/vi-v1.toml']:
        p = package / name
        if p.is_file():
            report['files'][name] = hashlib.sha256(p.read_bytes()).hexdigest()
    # Exactly two bounded observations; failures are retained, never retried.
    report['captures'] = [capture(binary, scale, args.timeout) for scale in [1, 2]]
    report['status'] = 'PASS' if all(c['status'] == 'PASS' for c in report['captures']) else 'FAIL'
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'status': report['status'], 'out': str(args.out)}))
    return 0 if report['status'] == 'PASS' else 1


if __name__ == '__main__':
    sys.exit(main())
