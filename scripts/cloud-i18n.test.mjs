import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { localeHost } from './i18n-test-helpers.mjs';
import { domHost, Element } from './i18n-dom-helpers.mjs';

test('a signed-in workspace keeps its loaded pages, cursor, file names and session when switching', async () => {
  const source = readFileSync(new URL('../portal/cloud.js', import.meta.url), 'utf8').replace(/^import .*;\n/, '');
  const nodes = ['apps-view', 'failid', 'page-name', 'files-nav', 'apps-nav', 'account-message', 'file-list', 'more-files', 'file-status', 'sign-in', 'sign-out', 'refresh-files', 'account-status', 'storage-usage'].map(id => new Element('div', { id }));
  const document = domHost(nodes);
  const host = localeHost({ pathname: '/tooruum/', search: '#failid', document });
  const calls = { registry: 0, init: 0, restore: 0, files: [] };
  const file = id => ({ id, spaceId: 'private-space', name: 'Ideed', formatId: 'grid', formatVersion: 1, bytes: 1572864, updatedMs: 1700000000000, versions: 2 });
  Object.assign(host.context, {
    location: host.window.location,
    setInterval() {},
    async fetch() { calls.registry++; return { ok: true, json: async () => ({ apps: [{ id: 'gridcraft', name: 'GridCraft', cloud: true, openFormats: [{ id: 'grid', version: 1 }] }] }) }; },
    async init() { calls.init++; },
    CloudPortal: class {
      async restore() { calls.restore++; return { authenticated: true, principal: 'owner', spaceId: 'private-space', usedBytes: 1572864, quotaBytes: 10485760 }; }
      needs_refresh() { return false; }
      async files(space, cursor) { calls.files.push([space, cursor]); return { files: [file(cursor ? 'second' : 'first')], nextCursor: cursor ? 'third-page' : 'second-page' }; }
    },
  });
  host.start();
  await vm.runInContext(`(async () => { ${source} })()`, host.context);
  document.getElementById('more-files').dispatchEvent(new Event('click'));
  await new Promise(resolve => setImmediate(resolve));
  const rows = [...document.getElementById('file-list').children];
  const before = JSON.stringify(calls);
  for (const language of ['en', 'et', 'en']) host.i18n.setLanguage(language);
  assert.equal(JSON.stringify(calls), before, 'language changes must not authenticate or fetch');
  assert.deepEqual(document.getElementById('file-list').children, rows);
  assert.equal(rows.length, 2);
  assert.equal(rows[0].children[1].children[0].textContent, 'Ideed', 'user filenames are not translated');
  assert.match(rows[0].textContent, /2 versions/);
  assert.equal(document.getElementById('account-status').textContent, 'My account');
  assert.equal(document.getElementById('sign-in').hidden, true);
  assert.equal(document.getElementById('page-name').textContent, 'My files');
  assert.equal(rows[0].children[3].href, '/apps/gridcraft/?cloud_service=cloud&cloud_space=private-space&cloud_file=first&lang=en');
  document.getElementById('more-files').dispatchEvent(new Event('click'));
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(calls.files.at(-1), ['private-space', 'third-page']);
});
