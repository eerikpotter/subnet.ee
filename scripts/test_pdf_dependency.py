"""Pin the single upstream modification and exhaustively check its lookup table."""
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1] / 'packages/sovereign-cloud-text/vendor/lopdf'

class PdfDependency(unittest.TestCase):
    def test_only_glyph_lookup_differs_from_upstream(self):
        hashes = json.loads((ROOT / 'upstream-sha256.json').read_text())
        changed = [name for name, digest in hashes.items()
                   if hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != digest]
        self.assertEqual(changed, ['src/encodings/glyphnames.rs'])

    def test_every_upstream_name_and_alias_has_the_same_sorted_mapping(self):
        source = (ROOT / 'src/encodings/glyphnames.rs').read_text()
        expected = {}
        for value, names in re.findall(r'(0x[0-9a-f]+) = \[([^\]]+)\];', source):
            for name in names.split(','):
                self.assertNotIn(name.strip(), expected)
                expected[name.strip()] = value
        actual = re.findall(r'\(b"([^"]+)", (0x[0-9a-f]+)\)', source)
        self.assertGreater(len(actual), 4000)
        self.assertEqual(actual, sorted(expected.items()))
        self.assertIn('GLYPH_NAMES.binary_search_by', source)

if __name__ == '__main__':
    unittest.main()
