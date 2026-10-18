#!/usr/bin/env python3
"""Provision canister-held Resend settings through the owner's signed ICP management call.

No secret command arguments or permanent project files. The credential is read
from macOS Keychain; the standard CLI reads a private, temporary settings file.
This command does not send email or change controllers.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
MAIL_NAMES = ('RESEND_API_KEY', 'SUBNET_MAIL_FROM', 'SUBNET_MAIL_ORIGIN')
SENDER = 'subnet.ee <info@subnet.ee>'


def run(arguments, cwd):
    # Explicit arguments own target/identity selection. Never inherit debug logging.
    env = {k: v for k, v in os.environ.items()
           if k not in ('ICP_PROJECT_ROOT', 'ICP_NETWORK', 'ICP_ENVIRONMENT', 'RUST_LOG', 'RUST_BACKTRACE')}
    result = subprocess.run(['icp', *arguments, '--project-root-override', str(cwd)],
                            cwd=cwd, env=env, capture_output=True, text=True)
    if result.returncode:
        # The CLI may echo settings in either stream. Do not expose that output.
        raise RuntimeError('ICP operation failed; credential-bearing output was suppressed. '
                           'Check the target and controller identity, then use --status.')
    return result.stdout


def require_private_owner(settings, principal):
    if principal == '2vxsx-fae' or settings.get('controllers') != [principal]:
        raise RuntimeError('The signing identity must be the sole private canister controller.')
    for name in ('status_visibility', 'snapshot_visibility', 'log_visibility'):
        if settings.get(name) != 'controllers':
            raise RuntimeError('Canister status, snapshots and logs must be controller-only.')


def keychain_credential():
    import keyring
    if keyring.get_keyring().__class__.__module__ != 'keyring.backends.macOS':
        raise RuntimeError('A native macOS Keychain backend is required.')
    key = keyring.get_password('ee.subnet.cloud.resend', 'info@subnet.ee')
    if not key or not re.fullmatch(r're_[A-Za-z0-9_-]{21,125}', key):
        raise RuntimeError('Resend credential is missing or invalid in macOS Keychain.')
    return key


def configure(options):
    project = options.project.resolve()
    if options.network:
        url = urlparse(options.network)
        if url.scheme != 'https' or not url.hostname or url.username or url.password or url.query or url.fragment:
            raise RuntimeError('A remote target requires an HTTPS gateway URL without credentials.')
        if not options.canister:
            raise RuntimeError('A remote target requires an explicit backend --canister ID.')
        gateway, root_key, canister = options.network, 'mainnet', options.canister
        origin = 'https://www.subnet.ee'
        target = ['--network', gateway, '--root-key', root_key]
    else:
        network = json.loads(run(['network', 'status', 'local', '--json'], project))
        url = urlparse(network['gateway_url'])
        if not network.get('managed') or url.hostname not in ('localhost', '127.0.0.1', '::1') or not url.port:
            raise RuntimeError('Expected a managed loopback local network.')
        gateway, root_key = network['gateway_url'], 'fetch'
        canister = json.loads((project / '.icp/cache/mappings/local.ids.json').read_text())['cloud']
        origin = f'http://frontend.local.localhost:{url.port}'
        target = ['--environment', 'local']
    identity = ['--identity', options.identity]

    def settings():
        return json.loads(run(['canister', 'settings', 'show', canister, *target, *identity, '--json'], project))

    before = settings()
    principal = run(['identity', 'principal', *identity], project).strip()
    require_private_owner(before, principal)
    if options.status:
        values = {v['name']: v['value'] for v in before.get('environment_variables', [])}
        print('Backend:', canister)
        print('Controller:', principal)
        print('Mail:', 'configured' if all(values.get(n) for n in MAIL_NAMES) else 'disabled or incomplete')
        return
    if options.disable:
        # settings sync MERGES maps: omission cannot remove a credential.
        arguments = ['canister', 'settings', 'update', canister, *target, *identity]
        for name in MAIL_NAMES:
            arguments.extend(['--remove-environment-variable', name])
        run(arguments, project)
        desired = {}
    else:
        desired = dict(zip(MAIL_NAMES, (keychain_credential(), SENDER, origin)))
        # CLI's file-based sync is the standard update_settings transport. Its
        # merge preserves unrelated settings; no shadow configuration is stored.
        with tempfile.TemporaryDirectory(prefix='subnet-mail-settings-') as name:
            temporary = Path(name).resolve()
            if any(root == temporary or root in temporary.parents for root in (ROOT, project)):
                raise RuntimeError('The system temporary directory must be outside the project.')
            config = {
                'canisters': [{'name': 'cloud', 'build': {'steps': [{'type': 'script', 'commands': ['true']}]},
                               'settings': {'environment_variables': desired}}],
                'networks': [{'name': 'mail', 'mode': 'connected', 'url': gateway, 'root-key': root_key}],
                'environments': [{'name': 'mail', 'network': 'mail', 'canisters': ['cloud']}],
            }
            fd = os.open(temporary / 'icp.yaml', os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, 'w') as output:
                json.dump(config, output)
            run(['canister', 'link', 'cloud', canister, '-e', 'mail'], temporary)
            run(['canister', 'settings', 'sync', 'cloud', '-e', 'mail', *identity], temporary)
    after = settings()
    require_private_owner(after, principal)
    values = {v['name']: v['value'] for v in after.get('environment_variables', [])}
    if any(values.get(n) != desired.get(n) for n in MAIL_NAMES):
        raise RuntimeError('Mail configuration verification failed; no values were printed.')
    print('Disabled backend mail:' if options.disable else 'Configured backend mail:', canister)
    print('No email was sent. The API key is never returned by the application.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--project', type=Path, default=ROOT)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument('--local', action='store_true', help='Use this project’s managed local backend')
    target.add_argument('--network', help='HTTPS gateway of the selected IC/Cloud Engine deployment')
    parser.add_argument('--canister', help='Explicit backend canister ID for a remote target')
    parser.add_argument('--identity', required=True, help='The sole controller’s ICP CLI identity')
    action = parser.add_mutually_exclusive_group()
    action.add_argument('--disable', action='store_true')
    action.add_argument('--status', action='store_true', help='Print only non-secret configuration status')
    options = parser.parse_args()
    try:
        configure(options)
    except RuntimeError as error:
        raise SystemExit(str(error)) from None
    except Exception:
        # Keychain/CLI/config exceptions must not dump settings or credentials.
        raise SystemExit('Mail configuration failed; sensitive diagnostics were suppressed.') from None


if __name__ == '__main__':
    main()
