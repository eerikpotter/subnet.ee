import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
import { localeHost } from './i18n-test-helpers.mjs';

const source = readFileSync(new URL('../portal/craft-updates.js', import.meta.url), 'utf8').replace(/^import .*;\n/, '');
const a = { app: 'wordcraft', version: '0.6.0', build: 'a'.repeat(64), bytes: 100, releaseNotes: null };
const b = { ...a, version: '0.7.0', build: 'b'.repeat(64), releaseNotes: { version: '0.7.0', url: 'https://github.com/storytold/wordcraft/releases/tag/v0.7.0', body: "## What's Changed\n* Published improvement\n* <script>unsafe()</script>" } };
const tick = () => new Promise(resolve => setImmediate(resolve));
class CustomEvent extends Event { constructor(type, options = {}) { super(type); this.detail = options.detail; } }

function host({ cached = a, latest = b, supportsCache = true } = {}) {
  let selected = cached, dirty = 'false', installError = false, beforeComplete;
  class Element extends EventTarget {
    hidden = false; children = []; textContent = ''; classList = { toggle() {} };
    replaceChildren() { this.children = []; }
    append(item) { this.children.push(item); }
    removeAttribute() {}
    setAttribute() {}
    showModal() { this.open = true; }
    close() { this.open = false; }
  }
  const ids = ['craft-editor','app-version','update-dialog','update-status','update-install','update-open','update-actions','update-progress','current-version','current-build','available-release','available-version','available-build','release-details','release-notes','release-source'];
  const elements = Object.fromEntries(ids.map(id => [id, new Element()]));
  const window = Object.assign(new EventTarget(), { isSecureContext: supportsCache, caches: {}, SubnetCraftHost: { dirtyState: () => dirty }, SubnetI18n: localeHost({ search: '?lang=en' }).i18n });
  const store = {
    selected: async () => selected, latest: async () => latest,
    complete: async () => { beforeComplete?.(); return true; },
    install: async release => { if (installError) throw Error('interrupted'); selected = release; return release; },
  };
  const navigator = {
    serviceWorker: { controller: { scriptURL: 'https://cloud.test/apps/craft-cache-sw.js', postMessage() {} }, register: async () => {} },
    locks: { request: async (_name, options, callback) => (callback || options)({}) },
  };
  const document = { getElementById: id => elements[id], createElement: () => new Element() };
  const location = { pathname: '/apps/wordcraft/', origin: 'https://cloud.test', search: '?lang=en&cloud_file=42', hash: '#page=2' };
  vm.runInNewContext(source, { window, document, navigator, location, URL, Event, CustomEvent, setTimeout, clearTimeout,
    APPS: ['wordcraft'], createPackageStore: () => store, launchURL: (release, query, hash) => `/cached/${release.build}${query}${hash}` });
  return { elements, window, get selected() { return selected; },
    setDirty(value) { dirty = value; }, failInstall() { installError = true; }, duringComplete(fn) { beforeComplete = fn; },
    async click(id) { elements[id].dispatchEvent(new Event('click')); await tick(); },
  };
}

test('checking and accepting an update never replaces the active editor; clean restart is explicit', async () => {
  const h = host(); await tick();
  assert.match(h.elements['craft-editor'].src, /cached\/aaaa/);
  assert.equal(h.elements['app-version'].textContent, 'Update available');
  await h.click('app-version');
  assert.equal(h.elements['release-notes'].children[0].textContent, 'Published improvement');
  assert.equal(h.elements['release-notes'].children[1].textContent, '<script>unsafe()</script>');
  assert.equal(h.elements['release-source'].href, b.releaseNotes.url);
  await h.click('update-install');
  assert.equal(h.selected, b);
  assert.match(h.elements['craft-editor'].src, /cached\/aaaa/);
  await h.click('update-open');
  assert.match(h.elements['craft-editor'].src, /cached\/bbbb.*cloud_file=42#page=2/);
  assert.equal(h.elements['update-dialog'].open, false);
});

test('dirty and unknown documents block a restart, including edits while a cache read is pending', async () => {
  for (const state of ['true', 'unknown', 'during-cache-read']) {
    const h = host(); await tick(); await h.click('app-version'); await h.click('update-install');
    if (state === 'during-cache-read') h.duringComplete(() => h.setDirty('true')); else h.setDirty(state);
    await h.click('update-open');
    assert.match(h.elements['craft-editor'].src, /cached\/aaaa/);
    assert.match(h.elements['update-status'].textContent, /Save or export/);
    assert.equal(h.elements['update-dialog'].open, true);
  }
});

test('failed download preserves the current editor and offers retry', async () => {
  const h = host(); await tick(); h.failInstall(); await h.click('update-install');
  assert.equal(h.selected, a);
  assert.match(h.elements['craft-editor'].src, /cached\/aaaa/);
  assert.match(h.elements['update-status'].textContent, /update failed/);
  assert.equal(h.elements['update-install'].disabled, false);
});

test('same-version builds remain distinguishable without inventing release notes', async () => {
  const h = host({ latest: { ...a, build: b.build } }); await tick(); await h.click('app-version');
  assert.equal(h.elements['app-version'].textContent, 'Update available');
  assert.equal(h.elements['current-version'].textContent, h.elements['available-version'].textContent);
  assert.notEqual(h.elements['current-build'].textContent, h.elements['available-build'].textContent);
  assert.equal(h.elements['release-notes'].children[0].textContent, 'No release notes available.');
  assert.equal(h.elements['release-source'].hidden, true);
});

test('notes published later for the same package appear without an app update', async () => {
  const h = host({ latest: { ...a, releaseNotes: { ...b.releaseNotes, version: a.version } } });
  await tick(); await h.click('app-version');
  assert.equal(h.elements['app-version'].textContent, 'v0.6.0');
  assert.equal(h.elements['release-notes'].children[0].textContent, 'Published improvement');
  assert.equal(h.elements['update-install'].hidden, true);
});

test('first launch installs latest; unsupported storage opens latest without promising version retention', async () => {
  const fresh = host({ cached: null }); await tick();
  assert.equal(fresh.selected, b);
  assert.match(fresh.elements['craft-editor'].src, /cached\/bbbb/);
  const fallback = host({ supportsCache: false }); await tick(); await fallback.click('app-version');
  assert.match(fallback.elements['craft-editor'].src, /^\.\/editor.html\?lang=en&cloud_file=42#page=2$/);
  assert.match(fallback.elements['update-status'].textContent, /cannot retain app versions/);
});
