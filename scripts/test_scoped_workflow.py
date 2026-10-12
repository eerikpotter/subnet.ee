"""Offline workflow boundaries: tiny fake artifacts, no Cargo/network/deployment."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import assemble
import build
import dev

spec = importlib.util.spec_from_file_location("sdk_sync", Path(__file__).with_name("sync-sdk.py"))
sdk_sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sdk_sync)


class ScopedWorkflowTest(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.apps = [dict(id=app, name=app, symbol=app[0], kind="Test", description="Test", formats="Test")
                     for app in ("photocraft", "gridcraft")]
        self.put("registry/apps.json", json.dumps(self.apps))
        for name, content in {
            "index.html": '<a href="/tooruum/">Ava tööruum</a><!-- LANDING_CARDS -->',
            "tooruum/index.html": "<!-- APP_CARDS --> __APP_COUNT__ tööriista __APP_COUNT_PADDED__",
            "craft-shell.html": '<iframe id="craft-editor" data-leave="guard">__APP_NAME__</iframe>',
            "cloud.js": "__CLOUD_BINDINGS__ __CLOUD_WASM__", "tokens.css": "tokens",
            "craft-shell.css": "shell", "craft-shell.js": "shell", "craft-loading.js": "loader", "craft-wasm.js": "wasm-loader", "style.css": "style",
        }.items():
            self.put("portal/" + name, content)
        for extension in ("js", "wasm"):
            suffix = "_bg.wasm" if extension == "wasm" else ".js"
            self.put(".local/portal-runtime/sovereign-cloud-web-old" + suffix, "portal")
        for app in self.apps:
            self.put(f"apps/{app['id']}/dist/web/index.html", "<script>const wasm = await init({ module_or_path: './editor.wasm' });</script><link rel=\"preload\" as=\"fetch\" href=\"./editor.wasm\">")
            self.put(f"apps/{app['id']}/dist/web/editor.js", app['id'])
            self.put(f"apps/{app['id']}/dist/web/editor.wasm", app['id'])
        with patch.object(assemble.subprocess, "check_output", return_value="original\n"):
            self.assemble("all")

    def put(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def assemble(self, *targets):
        with contextlib.redirect_stdout(io.StringIO()):
            assemble.assemble(self.root, targets or ("portal",))

    def manifest(self):
        return json.loads((self.root / "dist/apps.json").read_text())

    def test_portal_works_without_any_editor_checkouts_or_project_commands(self):
        original = self.manifest()['apps']
        shutil.rmtree(self.root / "apps")
        shutil.rmtree(self.root / ".local/portal-runtime")
        self.put("portal/style.css", "new style")
        with patch.object(assemble.subprocess, "run", side_effect=AssertionError("project command")), \
             patch.object(assemble.subprocess, "check_output", side_effect=AssertionError("editor inspection")):
            self.assemble()
        self.assertEqual((self.root / "dist/style.css").read_text(), "new style")
        landing = (self.root / "dist/index.html").read_text()
        workspace = (self.root / "dist/tooruum/index.html").read_text()
        self.assertIn('href="/tooruum/"', landing)
        self.assertIn('href="/apps/photocraft/"', landing)
        self.assertNotIn('<!-- LANDING_CARDS -->', landing)
        self.assertNotIn('<!-- APP_CARDS -->', workspace)
        self.assertNotIn('__APP_COUNT', workspace)
        self.assertIn('2 tööriista 02', workspace)
        for app in self.apps:
            self.assertIn(f'href="/apps/{app["id"]}/"', workspace)
        self.assertEqual(self.manifest()['apps'], original)
        self.assertEqual((self.root / "dist/apps/gridcraft/editor.wasm").read_text(), "gridcraft")

    def test_one_app_replaces_only_its_artifact_and_keeps_portal_runtime(self):
        old_grid = self.manifest()['apps'][1]
        shutil.rmtree(self.root / "apps/gridcraft")
        self.put("apps/photocraft/dist/web/editor.wasm", "new photo")
        self.put(".local/portal-runtime/sovereign-cloud-web-old_bg.wasm", "unselected portal change")
        def git_output(command, cwd, text):
            self.assertEqual(cwd, self.root / "apps/photocraft")
            return "new-commit\n" if "rev-parse" in command else ""
        with patch.object(assemble.subprocess, "check_output", side_effect=git_output):
            self.assemble("photocraft")
        self.assertEqual((self.root / "dist/apps/photocraft/editor.wasm").read_text(), "new photo")
        self.assertEqual(self.manifest()['apps'][1], old_grid)
        self.assertEqual((self.root / "dist/cloud/sovereign-cloud-web-old_bg.wasm").read_text(), "portal")
        self.assertEqual(self.manifest()['apps'][0]['sourceCommit'], "new-commit")

    def test_failed_selected_build_preserves_existing_site(self):
        before = (self.root / "dist/apps.json").read_bytes()
        (self.root / "apps/photocraft/dist/web/editor.wasm").unlink()
        with self.assertRaisesRegex(RuntimeError, "Missing complete photocraft"):
            self.assemble("photocraft")
        self.assertEqual((self.root / "dist/apps.json").read_bytes(), before)

    def test_bootstrap_progress_is_idempotent_and_preserves_editor_payloads(self):
        folder = self.root / "dist/apps/photocraft"
        original = (folder / "editor.html").read_text()
        self.assertIn('import { loadCraftWasm } from "/craft-wasm.js";', original)
        self.assertIn('"./editor.wasm"', original)
        self.assertIn("sha384-", original)
        self.assertNotIn('rel="preload"', original)
        assemble.instrument_editor(folder)
        self.assertEqual((folder / "editor.html").read_text(), original)
        self.assertEqual((folder / "editor.wasm").read_text(), "photocraft")
        self.assertEqual((folder / "editor.js").read_text(), "photocraft")

    def test_unknown_bootstrap_fails_without_replacing_the_deployed_bundle(self):
        before = (self.root / "dist/apps.json").read_bytes()
        self.put("apps/photocraft/dist/web/index.html", "<script>unknown_loader()</script>")
        with patch.object(assemble.subprocess, "check_output", return_value="new\n"), self.assertRaisesRegex(RuntimeError, "Unrecognized Trunk bootstrap"):
            self.assemble("photocraft")
        self.assertEqual((self.root / "dist/apps.json").read_bytes(), before)

    def test_default_dev_syncs_frontend_only_and_app_build_is_scoped(self):
        self.put(".icp/cache/mappings/local.ids.json", '{"frontend":"front", "cloud":"cloud"}')
        for argv, expected in (([], "portal"), (["photocraft", "--build"], "photocraft")):
            with self.subTest(argv=argv), patch.object(dev, "ROOT", self.root), \
                 patch.object(dev, "run") as run, \
                 patch.object(dev.subprocess, "run") as status, \
                 patch.object(dev.subprocess, "check_output", return_value=b'{"gateway_url":"http://localhost:1234"}'), \
                 contextlib.redirect_stdout(io.StringIO()):
                status.return_value.returncode = 0
                dev.main(argv)
                calls = [call.args for call in run.call_args_list]
                self.assertIn((dev.sys.executable, "scripts/assemble.py", expected), calls)
                self.assertIn(("icp", "sync", "frontend", "-e", "local"), calls)
                self.assertFalse(any("deploy" in call or "all" in call for call in calls))
                builds = [call for call in calls if "scripts/build.py" in call]
                self.assertEqual(builds, [(dev.sys.executable, "scripts/build.py", expected)] if argv else [])

    def test_build_checks_only_selected_pinned_sdk(self):
        self.put("apps/photocraft/Cargo.toml", "")
        self.put("apps/photocraft/apps/photocraft-web/index.html", '<link rel="rust">')
        with patch.object(build, "ROOT", self.root), patch.object(build.shutil, "which", return_value="trunk"), \
             patch.object(build.subprocess, "run") as run, contextlib.redirect_stdout(io.StringIO()):
            build.build("photocraft")
            commands = [call.args[0] for call in run.call_args_list]
            self.assertEqual(len(commands), 2)
            self.assertEqual(commands[0][-2:], ["photocraft", "--check-pinned"])
            self.assertEqual(commands[1][0], "trunk")

    def test_pinned_sdk_does_not_require_canonical_but_detects_edits(self):
        path = "apps/photocraft/vendor/sovereign-cloud/"
        self.put(path + "Cargo.toml", "pinned")
        self.put(path + "snapshot.json", json.dumps({"Cargo.toml": hashlib.sha256(b"pinned").hexdigest()}))
        with patch.object(sdk_sync, "ROOT", self.root), contextlib.redirect_stdout(io.StringIO()):
            sdk_sync.sync_package("photocraft", "sovereign-cloud", pinned=True)
            self.put(path + "Cargo.toml", "unexpected edit")
            with self.assertRaisesRegex(SystemExit, "Pinned SDK snapshot differs"):
                sdk_sync.sync_package("photocraft", "sovereign-cloud", pinned=True)


if __name__ == "__main__":
    unittest.main()
