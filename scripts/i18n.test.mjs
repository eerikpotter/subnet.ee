import test from 'node:test';
import assert from 'node:assert/strict';
import { localeHost } from './i18n-test-helpers.mjs';
import { domHost, Element } from './i18n-dom-helpers.mjs';

test('public paths select their prerendered language; app wrappers remember preferences', () => {
  assert.equal(localeHost().i18n.language, 'et');
  assert.equal(localeHost({ saved: 'en' }).i18n.language, 'et');
  assert.equal(localeHost({ pathname: '/en/', saved: 'et' }).i18n.language, 'en');
  assert.equal(localeHost({ pathname: '/apps/photocraft/', saved: 'en' }).i18n.language, 'en');
  const selected = localeHost({ search: '?lang=et', saved: 'en' });
  assert.equal(selected.i18n.language, 'et');
  assert.equal(selected.storage.get('subnet.language'), 'et');
  assert.equal(localeHost({ search: '?lang=invalid', saved: 'invalid' }).i18n.language, 'et');
});

test('shareable English links work when storage is unavailable', () => {
  const { i18n, document } = localeHost({ search: '?lang=en', blocked: true });
  assert.equal(document.documentElement.lang, 'en');
  assert.equal(i18n.t('Eesti esimene'), 'Estonia’s first');
  assert.equal(i18n.translate('7 rakendust'), '7 applications');
  assert.equal(i18n.translate('GridCraft käivitub'), 'Starting GridCraft');
  assert.equal(i18n.translate('Ava PhotoCraft'), 'Open PhotoCraft');
  assert.equal(i18n.t('not translated'), 'not translated');
});

test('language links preserve file identity, renderer flags and fragments', () => {
  const { i18n } = localeHost({ search: '?lang=en' });
  const result = new URL(i18n.href('/apps/photocraft/?cloud_file=a%26b&cloud_space=c&webgl#page=3'), 'https://subnet.ee');
  assert.equal(result.searchParams.get('cloud_file'), 'a&b');
  assert.equal(result.searchParams.get('cloud_space'), 'c');
  assert.equal(result.searchParams.has('webgl'), true);
  assert.equal(result.searchParams.get('lang'), 'en');
  assert.equal(result.hash, '#page=3');
  assert.equal(i18n.href('/en/tooruum/?lang=en#failid', 'et'), '/tooruum/#failid');
  assert.equal(i18n.href('/tooruum/?demo=1#failid', 'en'), '/en/tooruum/?demo=1#failid');
});

test('localization never rewrites third-party links or editor assets', () => {
  const { i18n } = localeHost({ search: '?lang=en' });
  for (const link of ['https://github.com/eerikpotter/subnet.ee', 'mailto:contact@efti.org', '/apps/photocraft/editor.html', '/apps/photocraft/editor.wasm']) {
    assert.equal(i18n.href(link), link);
  }
});

test('interpolation preserves literal user values instead of interpreting replacement syntax', () => {
  const { i18n } = localeHost({ search: '?lang=en' });
  assert.equal(i18n.t('Ava rakenduses {name} →', { name: '<img>$&' }), 'Open in <img>$& →');
});

test('switching changes existing text, SVG labels, metadata and links without replacing DOM or navigating', () => {
  const first = { nodeType: 3, textContent: 'Eesti esimene' };
  const br = new Element('br');
  const heading = new Element('h1', { 'data-i18n-text': JSON.stringify([[0, 'Eesti esimene', 'Estonia’s first']]) }, [first, br]);
  const svg = new Element('text', { 'data-i18n-text': JSON.stringify([[0, 'Isiklik kontroll', 'Personal control']]) }, [{ nodeType: 3, textContent: 'Isiklik kontroll' }]);
  const meta = new Element('meta', { 'data-i18n-attrs': JSON.stringify({ content: ['Kirjeldus', 'Description'] }), content: 'Kirjeldus' });
  const canonical = new Element('link', { rel: 'canonical', href: 'https://subnet.ee/' });
  const et = new Element('link', { rel: 'alternate', hreflang: 'et', href: 'https://subnet.ee/' });
  const en = new Element('link', { rel: 'alternate', hreflang: 'en', href: 'https://subnet.ee/en/' });
  const switcher = new Element('a', { 'data-language': 'en', href: '/en/' });
  const workspace = new Element('a', { href: '/tooruum/#failid' });
  const host = localeHost({ search: '?demo=1#tehnoloogia', document: domHost([heading, svg, meta, canonical, et, en, switcher, workspace]) });
  host.start();
  const click = new Event('click', { cancelable: true });
  Object.defineProperties(click, { target: { value: switcher }, button: { value: 0 } });
  host.document.dispatchEvent(click);
  assert.equal(click.defaultPrevented, true);
  assert.equal(heading.childNodes[0], first);
  assert.equal(heading.childNodes[1], br);
  assert.equal(heading.textContent, 'Estonia’s first');
  assert.equal(svg.textContent, 'Personal control');
  assert.equal(meta.content, 'Description');
  assert.equal(canonical.href, 'https://subnet.ee/en/');
  assert.equal(workspace.href, '/en/tooruum/#failid');
  assert.equal(host.window.location.href, 'https://subnet.ee/en/?demo=1#tehnoloogia');
  assert.equal(host.window.history.state.retained, true);
  host.window.history.back();
  assert.equal(heading.textContent, 'Eesti esimene');
  assert.equal(canonical.href, 'https://subnet.ee/');
  assert.equal(workspace.href, '/tooruum/#failid');
  assert.equal(host.i18n.locale, 'et-EE');
});

test('dynamic loading/failure states stay current through repeated language changes', () => {
  const status = new Element('p', { 'data-i18n-text': JSON.stringify([[0, 'Algne', 'Initial']]) });
  const host = localeHost({ document: domHost([status]) });
  host.start();
  host.i18n.text(status, 'Laadin sinu faile…');
  host.i18n.setLanguage('en');
  assert.equal(status.textContent, 'Loading your files…');
  host.i18n.text(status, 'Failide laadimine ebaõnnestus. Proovi värskendada.');
  for (const language of ['et', 'en', 'et', 'en']) host.i18n.setLanguage(language);
  assert.equal(status.textContent, host.i18n.t('Failide laadimine ebaõnnestus. Proovi värskendada.'));
  assert.equal(status.getAttribute('data-i18n-text'), null);
});

test('legacy links normalize without adding history; modified clicks keep native link behavior', () => {
  const link = new Element('a', { 'data-language': 'et', href: '/' });
  const host = localeHost({ search: '?lang=en&demo=1#tehnoloogia', blocked: true, document: domHost([link]) });
  host.start();
  assert.equal(host.window.location.href, 'https://subnet.ee/en/?demo=1#tehnoloogia');
  assert.equal(host.history.length, 1);
  assert.equal(link.href, '/?demo=1#tehnoloogia');
  const click = new Event('click', { cancelable: true });
  Object.defineProperties(click, { target: { value: link }, button: { value: 0 }, ctrlKey: { value: true } });
  host.document.dispatchEvent(click);
  assert.equal(click.defaultPrevented, false);
  assert.equal(host.i18n.language, 'en');
});
