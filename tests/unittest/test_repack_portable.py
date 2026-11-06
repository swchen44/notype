import importlib.util
import pathlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('repack', pathlib.Path(__file__).resolve().parents[2] / 'scripts/release/repack-portable.py')
repack = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repack)

class PortableArchiveTests(unittest.TestCase):
    def test_corrupt_source_is_rejected_before_output_or_tool_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            archive = root / 'bad.zip'
            archive.write_bytes(b'not the pinned release')
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                repack.repack(archive, root / 'output', 'must-not-execute')
            self.assertFalse((root / 'output').exists())

    def test_inventory_detects_unicode_file_changes_and_extra_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            file = root / '中文.txt'
            file.write_text('原稿', encoding='utf-8')
            before = repack.inventory(root)
            file.write_text('修改', encoding='utf-8')
            self.assertNotEqual(repack.inventory(root), before)
            (root / 'extra').write_bytes(b'new')
            self.assertEqual(len(repack.inventory(root)), 2)

if __name__ == '__main__':
    unittest.main()
