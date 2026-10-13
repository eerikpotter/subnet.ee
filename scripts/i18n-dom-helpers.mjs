// Small DOM host for state-preservation tests; browser layout is checked separately.
export class Element extends EventTarget {
  constructor(tag = 'div', attrs = {}, children = []) {
    super();
    this.tagName = tag.toUpperCase(); this.nodeType = 1;
    this.attrs = new Map(Object.entries(attrs)); this.childNodes = children;
    this.classList = { toggle() {} };
  }
  getAttribute(name) { return this.attrs.get(name) ?? null; }
  setAttribute(name, value) { this.attrs.set(name, String(value)); }
  removeAttribute(name) { this.attrs.delete(name); }
  get href() { return this.getAttribute('href'); }
  set href(value) { this.setAttribute('href', value); }
  get content() { return this.getAttribute('content'); }
  set content(value) { this.setAttribute('content', value); }
  get children() { return this.childNodes.filter(node => node.nodeType === 1); }
  get textContent() { return this.childNodes.map(node => node.textContent).join(''); }
  set textContent(value) { this.childNodes = [{ nodeType: 3, textContent: value }]; }
  append(...nodes) { this.childNodes.push(...nodes); }
  replaceChildren(...nodes) { this.childNodes = nodes; }
  matches(selector) {
    const tag = selector.match(/^\w+/)?.[0];
    if (tag && this.tagName.toLowerCase() !== tag) return false;
    return [...selector.matchAll(/\[([\w-]+)(?:="([^"]*)")?\]/g)].every(([, key, value]) => this.attrs.has(key) && (value === undefined || this.attrs.get(key) === value));
  }
  closest(selector) { return this.matches(selector) ? this : null; }
}

export function domHost(nodes = []) {
  function all() {
    const visit = node => [node, ...node.children.flatMap(visit)];
    return nodes.flatMap(visit);
  }
  return Object.assign(new EventTarget(), {
    documentElement: {}, readyState: 'loading', hidden: false,
    querySelectorAll(selector) { return all().filter(node => node.matches(selector)); },
    querySelector(selector) { return this.querySelectorAll(selector)[0] || null; },
    getElementById(id) { return all().find(node => node.getAttribute('id') === id); },
    createElement(tag) { return new Element(tag); },
  });
}
