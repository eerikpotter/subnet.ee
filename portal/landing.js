/* The public page loads only lightweight navigation; account/Wasm lives in /tooruum/. */
const oldWorkspaceHashes = new Set(['#failid', '#rakendused']);
function followLegacyLink() {
  if (oldWorkspaceHashes.has(location.hash)) location.replace(`/tooruum/${location.search}${location.hash}`);
}
followLegacyLink();
window.addEventListener('hashchange', followLegacyLink);

const menuToggle = document.querySelector('.menu-toggle');
const navigation = document.getElementById('main-nav');
function closeMenu() {
  navigation.classList.remove('is-open');
  menuToggle.setAttribute('aria-expanded', 'false');
  menuToggle.setAttribute('aria-label', 'Ava menüü');
}
menuToggle.addEventListener('click', () => {
  const open = menuToggle.getAttribute('aria-expanded') !== 'true';
  navigation.classList.toggle('is-open', open);
  menuToggle.setAttribute('aria-expanded', String(open));
  menuToggle.setAttribute('aria-label', open ? 'Sulge menüü' : 'Ava menüü');
});
navigation.addEventListener('click', (event) => { if (event.target.closest('a')) closeMenu(); });
document.addEventListener('click', (event) => { if (!event.target.closest('.site-header')) closeMenu(); });
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && menuToggle.getAttribute('aria-expanded') === 'true') {
    closeMenu();
    menuToggle.focus();
  }
});
window.matchMedia('(min-width: 861px)').addEventListener('change', closeMenu);

let catalog;
let catalogRequest;
const searchInput = document.getElementById('tool-search');
const searchResults = document.getElementById('search-results');
const searchStatus = document.getElementById('search-status');
const normalize = (value) => value.toLocaleLowerCase('et').normalize('NFD').replace(/[\u0300-\u036f]/g, '');
function renderSearch() {
  if (!catalog) return;
  const query = normalize(searchInput.value.trim());
  const matches = catalog.filter((app) => normalize(`${app.name} ${app.kind} ${app.description} ${app.formats}`).includes(query));
  searchResults.replaceChildren();
  searchStatus.textContent = matches.length ? `${matches.length} ${matches.length === 1 ? 'tööriist' : 'tööriista'}` : 'Ühtegi tööriista ei leitud. Proovi näiteks „pilt”, „PDF” või „heli”.';
  for (const app of matches) {
    const link = document.createElement('a');
    link.className = 'search-result';
    link.href = `/apps/${app.id}/`;
    const copy = document.createElement('span');
    const name = document.createElement('strong');
    name.textContent = app.name;
    const description = document.createElement('small');
    description.textContent = app.description;
    const arrow = document.createElement('span');
    arrow.setAttribute('aria-hidden', 'true');
    arrow.textContent = '↗';
    copy.append(name, description);
    link.append(copy, arrow);
    searchResults.append(link);
  }
}
async function loadSearch() {
  if (catalog) { renderSearch(); return; }
  if (catalogRequest) return catalogRequest;
  searchStatus.textContent = 'Laadin tööriistu…';
  catalogRequest = (async () => {
    try {
      const response = await fetch('/apps.json');
      if (!response.ok) throw new Error('Catalog unavailable');
      const data = await response.json();
      if (!Array.isArray(data.apps)) throw new Error('Invalid catalog');
      catalog = data.apps.filter((app) => typeof app.id === 'string' && /^[a-z]+$/.test(app.id));
      renderSearch();
    } catch {
      searchStatus.textContent = 'Otsingut ei õnnestunud laadida. Kõik tööriistad leiad tööruumist.';
    } finally {
      catalogRequest = null;
    }
  })();
  return catalogRequest;
}
searchInput.addEventListener('input', renderSearch);

function openDialog(dialog) {
  if (!(dialog instanceof HTMLDialogElement) || dialog.open) return;
  dialog.showModal();
  if (dialog.id === 'otsing') {
    searchInput.focus();
    loadSearch();
  }
}
for (const opener of document.querySelectorAll('[data-dialog]')) {
  opener.addEventListener('click', (event) => {
    event.preventDefault();
    openDialog(document.getElementById(opener.dataset.dialog));
  });
}
for (const dialog of document.querySelectorAll('dialog')) {
  dialog.querySelector('[data-close]').addEventListener('click', () => dialog.close());
  dialog.addEventListener('click', (event) => {
    const box = dialog.getBoundingClientRect();
    if (event.target === dialog && (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom)) dialog.close();
  });
}
// Informational links can also be opened directly or bookmarked.
function openHashDialog() {
  const name = location.hash.slice(1);
  if (name) openDialog(document.getElementById(name));
}
openHashDialog();
window.addEventListener('hashchange', openHashDialog);
