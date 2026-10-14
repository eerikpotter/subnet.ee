"""Deployment-boundary tests: no production IDs, network, or account credentials."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("discovery", Path(__file__).with_name("mcp-discovery.py"))
discovery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(discovery)


class DiscoveryTest(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.dist = Path(directory.name)
        # Public test principals, never used to contact a network.
        self.env = {"ICP_CLI_CID_CLOUD": "rrkah-fqaaa-aaaaa-aaaaq-cai",
                    "ICP_CLI_CID_FRONTEND": "ryjl3-tyaaa-aaaaa-aaaba-cai",
                    "ICP_CLI_NETWORK": "ic"}

    def test_network_switch_replaces_ids_and_removes_stale_identity(self):
        discovery.generate(self.dist, self.env)
        well_known = self.dist / ".well-known"
        self.assertEqual((well_known / "ii-derivation-origin").read_text(),
                         "https://ryjl3-tyaaa-aaaaa-aaaba-cai.icp.net\n")
        self.env.update(ICP_CLI_NETWORK="local", ICP_CLI_CID_CLOUD="r7inp-6aaaa-aaaaa-aaabq-cai")
        discovery.generate(self.dist, self.env)
        manifest = json.loads((well_known / "ic-architecture").read_text())
        self.assertEqual([x['id'] for x in manifest['canisters']], [self.env['ICP_CLI_CID_CLOUD']])
        self.assertFalse((well_known / "ii-derivation-origin").exists())

    def test_invalid_deployment_context_fails_before_writing(self):
        for value in ("", "2vxsx-fae", "aaaaa-aa", "rrkah-fqaaa-aaaaa-aaaar-cai", "${ICP_CLI_CID_CLOUD}"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                discovery.generate(self.dist, {**self.env, "ICP_CLI_CID_CLOUD": value})
        with self.assertRaises(ValueError):
            discovery.generate(self.dist, {**self.env, "ICP_CLI_NETWORK": "unknown"})
        with self.assertRaises(ValueError):
            discovery.generate(self.dist, {**self.env, "ICP_CLI_CID_CLOUD": self.env['ICP_CLI_CID_FRONTEND']})
        self.assertFalse((self.dist / ".well-known").exists())


if __name__ == "__main__":
    unittest.main()
