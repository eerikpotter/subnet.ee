"""Credential custody and signed-management workflow; no real key or mail delivery."""
import argparse
import importlib.util
import json
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('configure_mail', Path(__file__).with_name('configure-mail.py'))
mail = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mail)
KEY = 're_' + 'dummy' * 6


class ConfigureMailTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        ids = self.root / '.icp/cache/mappings/local.ids.json'
        ids.parent.mkdir(parents=True)
        ids.write_text('{"cloud":"backend"}')
        self.settings = dict(controllers=['owner'], status_visibility='controllers',
                             snapshot_visibility='controllers', log_visibility='controllers',
                             environment_variables=[dict(name='UNRELATED', value='preserved')])
        self.options = argparse.Namespace(project=self.root, network=None, canister=None,
                                          identity='owner-keychain', status=False, disable=False,
                                          allow_cycleops_monitor=False)
        self.monitor = dict(id=mail.CYCLEOPS_MONITOR, controllers=[], module_hash='0x' + mail.CYCLEOPS_MODULE)
        self.temporary_paths = []
        self.fail_sync = False

    def cli(self, args, cwd):
        self.assertNotIn(KEY, str(args))
        if args[:2] == ['network', 'status']:
            return json.dumps(dict(managed=True, gateway_url='http://127.0.0.1:5000'))
        if args[:2] == ['identity', 'principal']:
            return 'owner\n'
        if args[:2] == ['canister', 'status']:
            self.assertEqual(args[2], mail.CYCLEOPS_MONITOR)
            return json.dumps(self.monitor)
        if args[:3] == ['canister', 'settings', 'show']:
            return json.dumps(self.settings)
        if args[:2] == ['canister', 'link']:
            return ''
        if args[:3] == ['canister', 'settings', 'sync']:
            self.assertIn('--identity', args)
            self.assertNotEqual(cwd, self.root)
            self.assertNotIn(self.root, cwd.parents)
            path = cwd / 'icp.yaml'
            self.temporary_paths.append(path)
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertEqual(stat.S_IMODE(cwd.stat().st_mode), 0o700)
            config = json.loads(path.read_text())
            if self.options.network:
                self.assertEqual(config['networks'][0]['root-key'], 'mainnet')
            if self.fail_sync:
                raise RuntimeError('simulated transport failure')
            values = {v['name']: v['value'] for v in self.settings['environment_variables']}
            # Match actual CLI semantics: sync merges, it never removes omitted vars.
            values.update(config['canisters'][0]['settings']['environment_variables'])
            self.settings['environment_variables'] = [dict(name=k, value=v) for k, v in values.items()]
            return ''
        if args[:3] == ['canister', 'settings', 'update']:
            self.assertIn('--identity', args)
            names = [args[i+1] for i, arg in enumerate(args) if arg == '--remove-environment-variable']
            self.settings['environment_variables'] = [v for v in self.settings['environment_variables'] if v['name'] not in names]
            return ''
        self.fail(f'Unexpected CLI operation: {args[:3]}')

    def configure(self):
        with patch.object(mail, 'run', side_effect=self.cli), \
             patch.object(mail, 'keychain_credential', return_value=KEY), \
             patch('builtins.print') as output:
            mail.configure(self.options)
            self.assertNotIn(KEY, str(output.call_args_list))

    def test_set_rotate_and_remove_preserve_unrelated_settings_and_erase_temporary_file(self):
        self.configure()
        self.assertTrue(all(not p.exists() for p in self.temporary_paths))
        self.assertEqual(len(self.settings['environment_variables']), 4)
        # Repeating is an upsert, without creating duplicate configuration records.
        self.configure()
        self.assertEqual(len(self.settings['environment_variables']), 4)
        self.options.disable = True
        self.configure()
        self.assertEqual(self.settings['environment_variables'], [dict(name='UNRELATED', value='preserved')])

    def test_fail_closed_for_other_controllers_and_nonprivate_visibility(self):
        for controllers in (['2vxsx-fae'], ['stranger'], ['owner', 'stranger']):
            with self.subTest(controllers=controllers), self.assertRaises(RuntimeError):
                mail.require_private_owner(dict(self.settings, controllers=controllers), 'owner')
        for name in ('status_visibility', 'snapshot_visibility', 'log_visibility'):
            for value in ('public', {'allowed_viewers': ['stranger']}, None):
                with self.subTest(name=name, value=value), self.assertRaises(RuntimeError):
                    mail.require_private_owner(dict(self.settings, **{name: value}), 'owner')

    def test_failed_provisioning_erases_temporary_secret(self):
        self.fail_sync = True
        with self.assertRaises(RuntimeError):
            self.configure()
        self.assertEqual(len(self.temporary_paths), 1)
        self.assertFalse(self.temporary_paths[0].exists())

    def test_explicit_immutable_monitor_preserves_existing_controllers(self):
        self.options.allow_cycleops_monitor = True
        self.settings['controllers'] = [mail.CYCLEOPS_MONITOR, 'owner']
        self.configure()
        self.assertEqual(self.settings['controllers'], [mail.CYCLEOPS_MONITOR, 'owner'])
        self.options.status = True
        self.configure()

    def test_monitor_exception_fails_closed_before_reading_secret(self):
        self.options.allow_cycleops_monitor = True
        self.settings['controllers'] = ['owner', mail.CYCLEOPS_MONITOR]
        valid = self.monitor.copy()
        for changes in ({'controllers': ['third-party']}, {'controllers': None},
                        {'module_hash': '0xunknown'}, {'id': 'another-monitor'}):
            self.monitor = dict(valid, **changes)
            with self.subTest(changes=changes), \
                 patch.object(mail, 'run', side_effect=self.cli), \
                 patch.object(mail, 'keychain_credential') as credential, \
                 self.assertRaises(RuntimeError):
                mail.configure(self.options)
            credential.assert_not_called()
        self.monitor = valid
        for controllers in (['owner', mail.CYCLEOPS_MONITOR, 'stranger'],
                            [mail.CYCLEOPS_MONITOR], ['owner']):
            with self.subTest(controllers=controllers), self.assertRaises(RuntimeError):
                mail.require_private_owner(dict(self.settings, controllers=controllers), 'owner', valid)

    def test_project_temporary_directory_is_rejected_before_writing_secret(self):
        with patch.object(mail.tempfile, 'tempdir', str(self.root)), self.assertRaises(RuntimeError):
            self.configure()
        self.assertEqual(self.temporary_paths, [])
        self.assertEqual(list(self.root.glob('subnet-mail-settings-*')), [])

    def test_remote_uses_pinned_mainnet_root_and_requires_explicit_canister(self):
        self.options.network = 'https://icp-api.io'
        with self.assertRaises(RuntimeError):
            self.configure()
        self.options.canister = 'explicit-backend'
        self.configure()
        self.options.network = 'http://untrusted.example'
        with self.assertRaises(RuntimeError):
            self.configure()

    def test_status_and_disable_do_not_read_keychain(self):
        with patch.object(mail, 'run', side_effect=self.cli), \
             patch.object(mail, 'keychain_credential', side_effect=AssertionError('Keychain read')), \
             patch('builtins.print'):
            self.options.status = True
            mail.configure(self.options)
            self.options.status = False
            self.options.disable = True
            mail.configure(self.options)


if __name__ == '__main__':
    unittest.main()
