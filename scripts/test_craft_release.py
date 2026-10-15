import json
import tempfile
import unittest
from pathlib import Path
from craft_release import write_release, official_notes


class CraftReleaseTest(unittest.TestCase):
    def test_only_matching_published_upstream_notes_are_used(self):
        response = {'draft': False, 'tag_name': 'v0.7.0', 'html_url': 'https://github.com/storytold/wordcraft/releases/tag/v0.7.0', 'published_at': '2026-10-14T12:00:00Z', 'body': '## Changes\n* An official improvement'}
        notes = official_notes('wordcraft', '0.7.0', response)
        self.assertEqual(notes['body'], response['body'])
        for invalid in ({**response, 'draft': True}, {**response, 'published_at': None}, {**response, 'tag_name': 'v0.8.0'}, {**response, 'html_url': 'https://untrusted.test/'}):
            with self.assertRaises(ValueError):
                official_notes('wordcraft', '0.7.0', invalid)
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            (folder / 'editor.html').write_text('editor')
            (folder / 'editor.wasm').write_bytes(b'wasm')
            app = {'id': 'wordcraft', 'appVersion': '0.6.0'}
            release = write_release(folder, app, {'apps': {'wordcraft': notes}})
            self.assertIsNone(release['releaseNotes'])
            app['appVersion'] = '0.7.0'
            release = write_release(folder, app, {'apps': {'wordcraft': notes}})
            self.assertEqual(release['releaseNotes'], notes)
            self.assertNotEqual(write_release(folder, {**app, 'appVersion': '0.8.0'}, {'apps': {}})['build'], release['build'])
            self.assertEqual(write_release(folder, app, {'apps': {}})['build'], release['build'], 'notes do not change the editor build')
