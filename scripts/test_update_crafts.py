"""Offline Git integration tests: no build commands, networks or real app checkouts."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("updates", Path(__file__).with_name("update-crafts.py"))
updates = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updates)


class UpdatesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "upstream"
        self.source.mkdir()
        updates.git(self.source, "init", "-b", "main", "--template=")
        updates.git(self.source, "config", "user.name", "Test")
        updates.git(self.source, "config", "user.email", "test@example.invalid")
        (self.source / "document.txt").write_text("base\n")
        self.commit(self.source, "base")
        self.app = self.root / "apps/photocraft"
        self.app.parent.mkdir()
        updates.git(self.root, "clone", "--no-local", str(self.source), str(self.app), local=True)
        updates.git(self.app, "config", "user.name", "Test")
        updates.git(self.app, "config", "user.email", "test@example.invalid")
        (self.app / "integration.txt").write_text("our integration\n")
        self.base = self.commit(self.app, "fork integration")
        (self.source / "document.txt").write_text("upstream change\n")
        (self.source / "build.rs").write_text('compile_error!("must never execute");\n')
        self.target = self.commit(self.source, "upstream update")

    def commit(self, repo, message):
        updates.git(repo, "add", ".")
        updates.git(repo, "commit", "-m", message)
        return updates.out(repo, "rev-parse", "HEAD")

    def check(self):
        return updates.check_app(self.root, "photocraft", str(self.source))

    def test_review_and_promotion_preserve_fork_and_skip_hooks(self):
        marker = self.root / "hook-ran"
        hook = self.app / ".git/hooks/post-merge"
        hook.write_text(f"#!/bin/sh\ntouch '{marker}'\n")
        hook.chmod(0o755)
        plan = self.check()
        self.assertEqual(plan["incoming_commits"], 1)
        self.assertIn("build.rs", "\n".join(plan["review_paths"]))
        self.assertEqual(updates.out(self.app, "rev-parse", "HEAD"), self.base)
        review = updates.prepare(self.root, "photocraft")
        self.assertEqual(updates.out(self.app, "rev-parse", "HEAD"), self.base)
        merged = self.commit(review, "Reviewed upstream merge")
        updates.promote(self.root, "photocraft", merged)
        self.assertEqual((self.app / "integration.txt").read_text(), "our integration\n")
        self.assertEqual((self.app / "document.txt").read_text(), "upstream change\n")
        self.assertEqual(updates.out(self.app, "rev-parse", "backup/craft-update-" + self.base[:12]), self.base)
        self.assertFalse(marker.exists())

    def test_dirty_and_moving_heads_are_rejected(self):
        self.check()
        (self.app / "uncommitted.txt").write_text("keep me")
        with self.assertRaisesRegex(RuntimeError, "local changes"):
            updates.prepare(self.root, "photocraft")
        self.commit(self.app, "user work")
        with self.assertRaisesRegex(RuntimeError, "HEAD changed"):
            updates.prepare(self.root, "photocraft")

    def test_conflict_is_contained_and_cannot_be_promoted(self):
        (self.app / "document.txt").write_text("local change\n")
        self.commit(self.app, "conflicting local change")
        plan = self.check()
        self.assertIn("document.txt", plan["overlapping_paths"])
        review = updates.prepare(self.root, "photocraft")
        self.assertEqual(updates.out(review, "diff", "--name-only", "--diff-filter=U"), "document.txt")
        with self.assertRaisesRegex(RuntimeError, "local changes"):
            updates.promote(self.root, "photocraft", updates.out(review, "rev-parse", "HEAD"))
        self.assertEqual((self.app / "document.txt").read_text(), "local change\n")

    def test_wrong_sha_and_stale_app_cannot_be_promoted(self):
        self.check()
        review = updates.prepare(self.root, "photocraft")
        merged = self.commit(review, "reviewed")
        with self.assertRaisesRegex(RuntimeError, "approved SHA"):
            updates.promote(self.root, "photocraft", self.target)
        (self.app / "new.txt").write_text("later work")
        self.commit(self.app, "later work")
        with self.assertRaisesRegex(RuntimeError, "HEAD changed"):
            updates.promote(self.root, "photocraft", merged)

    def test_executable_config_is_rejected_without_execution(self):
        marker = self.root / "filter-ran"
        updates.git(self.app, "config", "filter.hostile.smudge", f"touch {marker}")
        with self.assertRaisesRegex(RuntimeError, "configuration"):
            self.check()
        self.assertFalse(marker.exists())


if __name__ == "__main__":
    unittest.main()
