import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { domHost, Element } from './i18n-dom-helpers.mjs';

const source = readFileSync(new URL('../portal/workspace.js', import.meta.url), 'utf8');
function setup() {
  const nodes = Object.fromEntries(['workspace-menu', 'workspace-navigation', 'account-menu', 'workspace-content', 'workspace-skip'].map(id => [id, new Element('div', {id})]));
  const document = domHost(Object.values(nodes));
  document.body = {classList: {add() {}}};
  const summary = new Element('summary');
  nodes['account-menu'].append(summary);
  nodes['account-menu'].querySelector = () => summary;
  for (const node of [...Object.values(nodes), summary]) node.focus = () => { document.activeElement = node; };
  const window = new EventTarget(), desktop = new EventTarget();
  desktop.matches = false;
  let english = false;
  window.matchMedia = () => desktop;
  window.SubnetI18n = {t: text => english ? ({'Ava menüü':'Open menu', 'Sulge menüü':'Close menu'})[text] : text};
  vm.runInNewContext(source, {document, window});
  function event(target, type, values = {}) {
    const event = new Event(type);
    for (const [key, value] of Object.entries(values)) Object.defineProperty(event, key, {value});
    target.dispatchEvent(event);
  }
  return {document, desktop, nodes, summary, event, language() {english = true; event(window, 'subnet:languagechange');}};
}

test('mobile navigation preserves open state through translation and Escape restores focus', () => {
  const {nodes, document, event, language} = setup();
  nodes['account-menu'].open = true;
  event(nodes['workspace-menu'], 'click');
  assert.equal(nodes['workspace-menu'].getAttribute('aria-expanded'), 'true');
  assert.equal(nodes['workspace-navigation'].getAttribute('data-open'), '');
  assert.equal(nodes['account-menu'].open, false);
  language();
  assert.equal(nodes['workspace-menu'].getAttribute('aria-label'), 'Close menu');
  event(document, 'keydown', {key:'Escape'});
  assert.equal(nodes['workspace-menu'].getAttribute('aria-expanded'), 'false');
  assert.equal(nodes['workspace-navigation'].getAttribute('data-open'), null);
  assert.equal(document.activeElement, nodes['workspace-menu']);
});

test('route selection, outside interaction and desktop resizing close the mobile panel', () => {
  const {nodes, document, desktop, event} = setup();
  event(nodes['workspace-menu'], 'click');
  event(nodes['workspace-navigation'], 'click', {target:new Element('a', {href:'#failid'})});
  assert.equal(nodes['workspace-menu'].getAttribute('aria-expanded'), 'false');
  assert.equal(document.activeElement, nodes['workspace-content']);
  for (const type of ['click', 'focusin']) {
    event(nodes['workspace-menu'], 'click');
    event(document, type, {target:new Element('button')});
    assert.equal(nodes['workspace-menu'].getAttribute('aria-expanded'), 'false');
  }
  event(nodes['workspace-menu'], 'click');
  desktop.matches = true;
  event(desktop, 'change');
  assert.equal(nodes['workspace-menu'].getAttribute('aria-expanded'), 'false');
});

test('account popup closes on Escape with focus restored, and on outside interaction', () => {
  const {nodes, document, summary, event} = setup();
  nodes['account-menu'].open = true;
  event(document, 'keydown', {key:'Escape'});
  assert.equal(nodes['account-menu'].open, false);
  assert.equal(document.activeElement, summary);
  nodes['account-menu'].open = true;
  event(document, 'click', {target:new Element('button')});
  assert.equal(nodes['account-menu'].open, false);
});

test('skip link focuses the current view without changing its route', () => {
  const {nodes, document} = setup();
  const event = new Event('click', {cancelable:true});
  nodes['workspace-skip'].dispatchEvent(event);
  assert.equal(event.defaultPrevented, true);
  assert.equal(document.activeElement, nodes['workspace-content']);
});
