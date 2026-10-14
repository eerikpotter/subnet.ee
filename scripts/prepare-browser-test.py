#!/usr/bin/env python3
"""Create opt-in, loopback-only browser fixtures; not included by assemble.py.

The two disposable signing keys authenticate normally to the replica. They test the
shared session/Files UI, not Internet Identity issuance or passkey handling.
Copy the two generated files from .local/browser-test to dist only for local QA,
sync frontend -e local, then reassemble/sync to remove them before handing off.
Never use this fixture on an origin holding a real user's session.
"""
import base64
import json
from pathlib import Path
import secrets

ROOT = Path(__file__).resolve().parents[1]
if __name__ == '__main__':
    out = ROOT / '.local/browser-test'
    out.mkdir(exist_ok=True, parents=True)
    keys = {name: base64.b64encode(secrets.token_bytes(32)).decode().rstrip('=') for name in ['A', 'B']}
    (out / 'browser-session-test.html').write_text('''<!doctype html><html lang="en"><meta charset="utf-8"><title>Disposable local session test</title><script type="module" src="/browser-session-test.js"></script><h1>Disposable local session test</h1><p>Only use on the isolated local development origin without a real user session.</p><button id="account-a">Use test account A</button><button id="account-b">Use test account B</button><button id="logout">Sign out test accounts</button><button id="damage">Test damaged session recovery</button><p id="status" role="status">No session change made.</p><a href="/pilveruum/#failid">Open My files</a></html>''')
    script = 'const keys = ' + json.dumps(keys) + ';\n' + '''
if (location.protocol !== 'http:' || !(location.hostname === 'localhost' || location.hostname.endsWith('.localhost'))) throw new Error('Local test fixture only');
function status(text) { document.getElementById('status').textContent = text; }
function login(name) {
  // A direct-signing identity is a valid IC caller. Empty delegation selects that
  // identity in the pinned client; it does not grant access to any other account.
  localStorage.setItem('ic-identity', keys[name]);
  localStorage.setItem('ic-delegation', JSON.stringify({public_key:[],delegations:[]}));
  status('Test account ' + name + ' selected.');
}
document.getElementById('account-a').onclick = () => login('A');
document.getElementById('account-b').onclick = () => login('B');
document.getElementById('logout').onclick = () => {
  localStorage.removeItem('ic-delegation'); localStorage.removeItem('ic-identity'); status('Test session cleared.');
};
document.getElementById('damage').onclick = () => {
  localStorage.setItem('ic-delegation', '{invalid'); status('Damaged session installed for recovery test.');
};
'''
    (out / 'browser-session-test.js').write_text(script)
    print('Disposable fixtures prepared in .local/browser-test (not assembled or deployed).')
