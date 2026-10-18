// Small DOM host for state-preservation tests; browser layout is checked separately.
export class Element extends EventTarget {
  constructor(tag = 'div', attrs = {}, children = []) {
    super();
    this.tagName = tag.toUpperCase(); this.nodeType = 1;
    this.attrs = new Map(Object.entries(attrs)); this.childNodes = children;
    for(const child of children) child.parentNode=this;
    this.classList = {
      contains: name => (this.className||'').split(/\s+/).includes(name),
      toggle: (name,force) => {
        const names=new Set((this.className||'').split(/\s+/).filter(Boolean));
        const active=force===undefined?!names.has(name):force;
        if(active)names.add(name);else names.delete(name);this.className=[...names].join(' ');return active;
      },
      add: name => this.classList.toggle(name,true),remove: name => this.classList.toggle(name,false),
    };
  }
  getAttribute(name) { return this.attrs.get(name) ?? null; }
  setAttribute(name, value) { this.attrs.set(name, String(value)); }
  removeAttribute(name) { this.attrs.delete(name); }
  get href() { return this.getAttribute('href'); }
  set href(value) { this.setAttribute('href', value); }
  get content() { return this.getAttribute('content'); }
  set content(value) { this.setAttribute('content', value); }
  get parentElement() { return this.parentNode?.nodeType===1 ? this.parentNode : null; }
  get children() { return this.childNodes.filter(node => node.nodeType === 1); }
  get textContent() { return this.childNodes.map(node => node.textContent).join(''); }
  set textContent(value) { this.childNodes = [{ nodeType: 3, textContent: value }]; }
  append(...nodes) { for(const node of nodes) this.insertBefore(node,null); }
  replaceChildren(...nodes) { for(const node of this.childNodes) node.parentNode=null; this.childNodes=[]; this.append(...nodes); }
  insertBefore(node,before) {
    if(node===before)return node;
    node.remove?.();
    const index=before ? this.childNodes.indexOf(before) : -1;
    this.childNodes.splice(index<0 ? this.childNodes.length : index,0,node);node.parentNode=this;return node;
  }
  remove() {
    if(!this.parentNode)return;
    const siblings=this.parentNode.childNodes,index=siblings.indexOf(this);
    if(index>=0)siblings.splice(index,1);this.parentNode=null;
  }
  replaceWith(node) { const parent=this.parentNode;if(parent){parent.insertBefore(node,this);this.remove();} }
  querySelectorAll(selector) { return this.children.flatMap(child=>[...(child.matches(selector) ? [child] : []),...child.querySelectorAll(selector)]); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  contains(node) { return node===this || this.children.some(child=>child.contains(node)); }
  matches(selector) {
    if(selector.includes(','))return selector.split(',').some(part=>this.matches(part.trim()));
    const tag = selector.match(/^\w+/)?.[0];
    if (tag && this.tagName.toLowerCase() !== tag) return false;
    for(const [,name] of selector.matchAll(/\.([\w-]+)/g))if(!this.classList.contains(name))return false;
    return [...selector.matchAll(/\[([\w-]+)(?:="([^"]*)")?\]/g)].every(([, key, value]) => this.attrs.has(key) && (value === undefined || this.attrs.get(key) === value));
  }
  closest(selector) { return this.matches(selector) ? this : this.parentNode?.closest?.(selector)||null; }
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
