"""Image build contracts: payloads, alpha, responsive discovery and cache identity."""
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image

from images import optimize
from localize import Page


class Tags(HTMLParser):
    def __init__(self, source):
        super().__init__()
        self.tags = []
        self.feed(source)

    def handle_starttag(self, name, attrs):
        self.tags.append((name, dict(attrs)))


class ImagesTest(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.stage = Path(temp.name)
        (self.stage / 'assets').mkdir()
        self.source = self.stage / 'assets/cloud.png'
        Image.new('RGBA', (100, 60), (120, 180, 240, 100)).save(self.source)
        self.template = ('<!doctype html><html><head></head><body>'
                         '<img src="/assets/cloud.png" data-image-widths="32,64,128" '
                         'data-image-preload sizes="100vw" width="100" height="60" '
                         'fetchpriority="high" alt="Pilv">'
                         '<img src="/assets/cloud.png" data-image-widths="32,64,128" '
                         'sizes="50vw" loading="lazy" alt="Pilv"></body></html>')

    def build(self):
        (self.stage / 'index.html').write_text(self.template)
        optimize(self.stage)
        return json.loads((self.stage / 'image-assets.json').read_text())

    def test_dimensions_alpha_original_and_preloads_in_both_locales(self):
        original = self.source.read_bytes()
        manifest = self.build()
        self.assertEqual(self.source.read_bytes(), original)
        self.assertEqual(sorted(item['width'] for item in manifest.values()), [32, 64, 100])
        self.assertEqual(len(list((self.stage / 'images').glob('*.webp'))), 3)
        for url, metadata in manifest.items():
            file = self.stage / url.lstrip('/')
            with Image.open(file) as image:
                self.assertEqual(image.format, 'WEBP')
                self.assertEqual(image.size, (metadata['width'], metadata['height']))
                self.assertEqual(image.getchannel('A').getextrema(), (100, 100))
            self.assertEqual(metadata['bytes'], file.stat().st_size)
            self.assertIn(hashlib.sha256(file.read_bytes()).hexdigest()[:16], file.name)
        for language in ('et', 'en'):
            rendered = Page((self.stage / 'index.html').read_text()).render({'Pilv': 'Cloud'}, language)
            tags = Tags(rendered).tags
            images = [attrs for name, attrs in tags if name == 'img']
            preloads = [attrs for name, attrs in tags if name == 'link']
            self.assertEqual(len(preloads), 1)
            self.assertEqual(preloads[0]['href'], images[0]['src'])
            self.assertEqual(preloads[0]['imagesrcset'], images[0]['srcset'])
            self.assertEqual(preloads[0]['imagesizes'], images[0]['sizes'])
            self.assertEqual(preloads[0]['fetchpriority'], 'high')
            self.assertEqual(images[1]['loading'], 'lazy')
            self.assertEqual(images[0]['alt'], 'Cloud' if language == 'en' else 'Pilv')
            self.assertNotIn('/assets/cloud.png', rendered)
            self.assertNotIn('data-image-', rendered)

    def test_identical_build_reuses_urls_and_changed_source_invalidates_them(self):
        first = self.build()
        self.assertEqual(self.build(), first)
        Image.new('RGBA', (100, 60), (0, 0, 0, 200)).save(self.source)
        self.assertTrue(set(first).isdisjoint(self.build()))

    def test_single_small_logo_and_missing_input_failure(self):
        self.template = self.template.replace('32,64,128', '32')
        manifest = self.build()
        self.assertEqual(len(manifest), 1)
        self.assertNotIn('srcset', (self.stage / 'index.html').read_text())
        self.source.unlink()
        with self.assertRaises(FileNotFoundError):
            self.build()

    def test_responsive_image_requires_layout_hint(self):
        self.template = self.template.replace('sizes="100vw"', '')
        with self.assertRaisesRegex(ValueError, 'needs sizes'):
            self.build()


if __name__ == '__main__':
    unittest.main()
