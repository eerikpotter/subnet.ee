import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const translations = readFileSync(new URL('../portal/translations.js', import.meta.url), 'utf8');
const source = readFileSync(new URL('../portal/i18n.js', import.meta.url), 'utf8');

export function localeHost({ search = '', saved, blocked = false, pathname = '/', document: suppliedDocument } = {}) {
  const storage = new Map(saved ? [['subnet.language', saved]] : []);
  const window = Object.assign(new EventTarget(), {
    location: new URL(`https://subnet.ee${pathname}${search}`),
    localStorage: {
      getItem(key) { if (blocked) throw new Error('Storage blocked'); return storage.get(key); },
      setItem(key, value) { if (blocked) throw new Error('Storage blocked'); storage.set(key, value); },
    },
  });
  const history = [window.location.href];
  window.history = {
    state: { retained: true },
    pushState(state, _, value) { this.state = state; window.location = new URL(value, window.location); history.push(window.location.href); },
    replaceState(state, _, value) { this.state = state; window.location = new URL(value, window.location); history[history.length - 1] = window.location.href; },
    back() { if (history.length > 1) { history.pop(); window.location = new URL(history.at(-1)); window.dispatchEvent(new Event('popstate')); } },
  };
  const document = suppliedDocument || Object.assign(new EventTarget(), {
    documentElement: {}, readyState: 'loading', querySelectorAll() { return []; }, querySelector() { return null; },
  });
  const context = vm.createContext({ window, document, URL, URLSearchParams, Event });
  vm.runInContext(translations, context);
  vm.runInContext(source, context);
  return { i18n: window.SubnetI18n, storage, document, window, context, history,
    start() { document.dispatchEvent(new Event('DOMContentLoaded')); },
  };
}
