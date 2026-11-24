import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('../portal/site-entry.js', import.meta.url), 'utf8');
function entry(hostname) {
  const appended = [], removed = [], content = [], listeners = [];
  const templates = Object.fromEntries(['head', 'body'].map(name => [`subnet-cloud-${name}`, {
    content: { cloneNode: () => ({ template: name }) }, remove: () => removed.push(name),
  }]));
  const document = {
    documentElement: { dataset: {} },
    querySelector: () => ({ content: 'https://cloud.subnet.ee' }),
    createElement: tag => ({ tag }),
    getElementById: id => templates[id],
    head: { append: node => appended.push(node), querySelectorAll: () => [{ remove: () => removed.push('public-head') }] },
    body: { replaceChildren: node => content.push(node.template) },
    addEventListener: (event, callback) => listeners.push(callback),
  };
  vm.runInNewContext(source, { document, location: { hostname }, URL });
  assert.equal(appended.some(node => node.tag === 'script'), false, 'no application starts before the DOM is ready');
  listeners[0]();
  return { document, appended, removed, content, scripts: appended.filter(node => node.tag === 'script').map(node => node.src) };
}

test('the cloud domain and development gateway activate only the workspace before app startup', () => {
  for (const host of ['cloud.subnet.ee', 'frontend.local.localhost', 'vigkv-fqaaa-aaaac-bgeha-cai.icp.net']) {
    const result = entry(host);
    assert.equal(result.document.documentElement.dataset.subnetSite, 'cloud');
    assert.deepEqual(result.content, ['body']);
    assert.deepEqual(result.scripts, ['/workspace.js', '/cloud.js']);
    assert.ok(result.removed.includes('public-head'));
    assert.equal(result.appended.find(node => node.src === '/cloud.js').type, 'module');
  }
});

test('the public site keeps its prerendered body and never starts the cloud client', () => {
  for (const host of ['www.subnet.ee', 'subnet.ee', 'localhost', 'cloud.subnet.ee.example.org']) {
    const result = entry(host);
    assert.equal(result.document.documentElement.dataset.subnetSite, 'public');
    assert.deepEqual(result.content, []);
    assert.deepEqual(result.removed, ['head', 'body']);
    assert.deepEqual(result.scripts, ['/landing.js', '/agent-workflow.js']);
  }
});
