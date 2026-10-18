// Public pages are prerendered; switching only updates their paired text/labels.
(() => {
  const key = 'subnet.language';
  const supported = value => value === 'et' || value === 'en';
  const publicPath = path => /^\/(?:en\/)?(?:(?:tooruum|pilveruum)\/)?$/.test(path) || /^\/(?:en\/)?pilveruum\/(?:file|folder)\/[0-9a-f-]{36}\/?$/i.test(path);
  const english = window.SubnetEnglish;
  const dynamic = new WeakMap();
  let language;
  function fromLocation() {
    const explicit = new URLSearchParams(window.location.search).get('lang');
    if (supported(explicit)) return explicit;
    // Each public URL has its own language, independent of browser preferences.
    if (publicPath(window.location.pathname)) return window.location.pathname.startsWith('/en/') ? 'en' : 'et';
    try {
      const saved = window.localStorage.getItem(key);
      if (supported(saved)) return saved;
    } catch { /* App links still work when storage is disabled. */ }
    return 'et';
  }
  function remember() {
    try { window.localStorage.setItem(key, language); } catch { /* Optional preference. */ }
  }
  function t(source, values = {}) {
    const text = language === 'en' && Object.hasOwn(english, source) ? english[source] : source;
    return text.replace(/\{(\w+)\}/g, (token, name) => Object.hasOwn(values, name) ? String(values[name]) : token);
  }
  function translate(source) {
    if (Object.hasOwn(english, source)) return t(source);
    for (const [pattern, template, name] of [
      [/^(\d+) rakendust$/, '{count} rakendust', 'count'],
      [/^(\d+) tööriista · üks loomeruum$/, '{count} tööriista · üks loomeruum', 'count'],
      [/^([A-Za-z]+Craft) käivitub$/i, '{name} käivitub', 'name'],
      [/^([A-Za-z]+Craft) tööala$/i, '{name} tööala', 'name'],
      [/^Ava ([A-Za-z]+Craft)$/i, 'Ava {name}', 'name'],
    ]) {
      const match = source.match(pattern);
      if (match) return t(template, { [name]: match[1] });
    }
    return source;
  }
  function href(value, targetLanguage = language) {
    const url = new URL(value, window.location.href);
    if (url.origin !== window.location.origin) return value;
    const choice = supported(targetLanguage) ? targetLanguage : language;
    if (publicPath(url.pathname)) {
      const base = url.pathname.replace(/^\/en\//, '/');
      url.pathname = choice === 'en' ? '/en' + base : base;
      url.searchParams.delete('lang');
    } else if (/^\/apps\/[a-z]+\/$/.test(url.pathname)) {
      url.searchParams.set('lang', choice);
    } else return value; // Never change editor assets or external URLs.
    return `${url.pathname}${url.search}${url.hash}`;
  }
  // Dynamic status/formatting stays tied to existing DOM and account state.
  function bind(element, render) {
    element.removeAttribute('data-i18n-text');
    element.setAttribute('data-i18n-dynamic', '');
    dynamic.set(element, render);
    render();
  }
  function text(element, source, values) {
    bind(element, () => { element.textContent = t(source, values); });
  }
  function apply() {
    document.documentElement.lang = language;
    const index = language === 'en' ? 1 : 0;
    for (const element of document.querySelectorAll('[data-i18n-text]')) {
      for (const [position, ...pair] of JSON.parse(element.getAttribute('data-i18n-text'))) {
        const node = element.childNodes[position];
        if (node?.nodeType === 3) node.textContent = pair[index];
      }
    }
    for (const element of document.querySelectorAll('[data-i18n-attrs]')) {
      for (const [name, pair] of Object.entries(JSON.parse(element.getAttribute('data-i18n-attrs')))) element.setAttribute(name, pair[index]);
    }
    for (const element of document.querySelectorAll('[data-i18n-dynamic]')) dynamic.get(element)?.();
    const ogLocale = document.querySelector('meta[property="og:locale"]');
    if (ogLocale) ogLocale.content = language === 'en' ? 'en_GB' : 'et_EE';
    const alternate = document.querySelector(`link[rel="alternate"][hreflang="${language}"]`);
    if (alternate) {
      const canonical = document.querySelector('link[rel="canonical"]');
      if (canonical) canonical.href = alternate.href;
      const ogURL = document.querySelector('meta[property="og:url"]');
      if (ogURL) ogURL.content = alternate.href;
    }
    for (const link of document.querySelectorAll('a[href]')) {
      const choice = link.getAttribute('data-language');
      const target = link.getAttribute('href');
      if (supported(choice)) {
        link.setAttribute('href', href(window.location.href, choice));
        if (choice === language) link.setAttribute('aria-current', 'true');
        else link.removeAttribute('aria-current');
      } else if (!target.startsWith('#')) link.setAttribute('href', href(target));
    }
  }
  function setLanguage(choice, history = 'push') {
    if (!supported(choice)) return;
    language = choice;
    remember();
    const target = href(window.location.href);
    const current = `${window.location.pathname}${window.location.search}${window.location.hash}`;
    if (history && target !== current) window.history[`${history}State`](window.history.state, '', target);
    apply();
    window.dispatchEvent(new Event('subnet:languagechange'));
  }
  language = fromLocation();
  remember();
  window.SubnetI18n = { t, href, translate, text, bind, setLanguage,
    get locale() { return language === 'en' ? 'en-GB' : 'et-EE'; },
    get language() { return language; },
  };
  document.documentElement.lang = language;
  function start() {
    // Old ?lang= links remain valid; normalize them without a network navigation.
    if (publicPath(window.location.pathname)) setLanguage(language, 'replace');
    else apply();
    document.addEventListener('click', event => {
      const link = event.target.closest('a[data-language]');
      if (!link || event.defaultPrevented || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
      event.preventDefault();
      setLanguage(link.getAttribute('data-language'));
    });
  }
  window.addEventListener('popstate', () => {
    const next = fromLocation();
    if (next !== language) setLanguage(next, false);
  });
  window.addEventListener('hashchange', () => {
    for (const link of document.querySelectorAll('a[data-language]')) link.setAttribute('href', href(window.location.href, link.getAttribute('data-language')));
  });
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true });
  else start();
})();
