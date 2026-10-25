"""Controlled failure/success cases for the optional native package GUI probe."""
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import json
import gui_probe
import zlib
from gui_probe import capture, inspect_png


def png(width=2, height=2, uniform=False, raw=None):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw if raw is not None else b''.join(b'\x00' + b''.join(bytes([0 if uniform else x % 256, 0 if uniform else y % 256, 20, 255]) for x in range(width)) for y in range(height))))
            + chunk(b'IEND', b''))


class GuiProbeTests(unittest.TestCase):
    def test_valid_capture_png_and_corruption(self):
        with tempfile.TemporaryDirectory() as temp:
            p = Path(temp) / 'test.png'
            p.write_bytes(png())
            self.assertEqual(inspect_png(p)['width'], 2)
            p.write_bytes(png(uniform=True))
            with self.assertRaisesRegex(ValueError, 'uniform'):
                inspect_png(p)
            p.write_bytes(png()[:-1])
            with self.assertRaises(ValueError):
                inspect_png(p)
            bad = bytearray(png()); bad[30] ^= 1; p.write_bytes(bad)
            with self.assertRaisesRegex(ValueError, 'CRC'):
                inspect_png(p)

    def test_uniform_image_in_every_filter_is_rejected(self):
        # Known encoded constant RGBA pixels, not encoder logic copied from the decoder.
        full = bytes([10, 20, 30, 255])
        zero = bytes(4)
        half = bytes([5, 10, 15, 128])
        rows = [
            b'\x00' + full * 2 + b'\x00' + full * 2,
            b'\x01' + full + zero + b'\x01' + full + zero,
            b'\x02' + full * 2 + b'\x02' + zero * 2,
            b'\x03' + full + half + b'\x03' + half + zero,
            b'\x04' + full + zero + b'\x04' + zero * 2,
        ]
        with tempfile.TemporaryDirectory() as temp:
            p = Path(temp) / 'constant.png'
            for raw in rows:
                p.write_bytes(png(raw=raw))
                with self.assertRaisesRegex(ValueError, 'uniform'):
                    inspect_png(p)

    def test_empty_image_stream_is_not_valid_gui(self):
        with tempfile.TemporaryDirectory() as temp:
            p = Path(temp) / 'test.png'
            # Valid header/checksum alone must not count as an actual capture.
            p.write_bytes(png()[:33])
            with self.assertRaises(ValueError):
                inspect_png(p)

    def test_real_capture_shape_positive_and_wrong_dimensions(self):
        def fake(args, **kwargs):
            Path(args[-1]).write_bytes(png(800, 600))
            self.assertEqual(kwargs['timeout'], 1)
            self.assertIn('--windowed', args)
            self.shot = Path(args[-1])
            return subprocess.CompletedProcess(args, 0, b'captured', b'')
        self.assertEqual(capture(Path('/mock/boom-client'), 1, 1, fake)['status'], 'PASS')
        self.assertFalse(self.shot.exists(), 'capture must remove its temporary screenshot')
        observed = capture(Path('/mock/boom-client'), 2, 1, fake)
        self.assertEqual(observed['observation'], 'FRAMEBUFFER_SIZE_DIFFERS')
        self.assertEqual(observed['status'], 'FAIL')

    def test_main_two_launch_report_and_failure_exit(self):
        with tempfile.TemporaryDirectory() as temp:
            package = Path(temp) / 'package'
            binary = package / 'bin' / ('boom-client.exe' if gui_probe.os.name == 'nt' else 'boom-client')
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'controlled binary')
            out = Path(temp) / 'observations.json'
            with patch('sys.argv', ['gui_probe', '--package', str(package), '--out', str(out)]):
                with patch('gui_probe.capture', side_effect=[{'status': 'PASS'}, {'status': 'FAIL'}]) as call:
                    self.assertEqual(gui_probe.main(), 1)
                    self.assertEqual([x.args[1] for x in call.call_args_list], [1, 2])
            report = json.loads(out.read_text())
            self.assertEqual(report['status'], 'FAIL')
            self.assertFalse(report['screenshots_retained_or_uploaded'])
            self.assertIn('bin/' + binary.name, report['files'])
            self.assertEqual(len(report['captures']), 2)

    def test_process_exit_missing_png_and_timeout_keep_failure(self):
        exit_bad = lambda args, **kw: subprocess.CompletedProcess(args, 7, b'', b'unknown context failure')
        bad = capture(Path('/mock/client'), 1, 1, exit_bad)
        self.assertEqual((bad['exit_code'], bad['cause']), (7, 'unclassified'))
        empty = lambda args, **kw: subprocess.CompletedProcess(args, 0, b'', b'')
        self.assertEqual(capture(Path('/mock/client'), 1, 1, empty)['observation'], 'PNG_INVALID_OR_MISSING')
        def timeout(args, **kwargs):
            raise subprocess.TimeoutExpired(args, 1, output=b'partial stdout', stderr=b'partial stderr')
        timed = capture(Path('/mock/client'), 1, 1, timeout)
        self.assertEqual((timed['observation'], timed['stderr']), ('TIMEOUT', 'partial stderr'))


if __name__ == '__main__':
    unittest.main()
