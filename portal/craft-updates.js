import { APPS, createPackageStore, launchURL } from "/craft-cache.js";

const { t } = window.SubnetI18n;
const byId = id => document.getElementById(id);
const frame = byId("craft-editor");
const badge = byId("app-version");
const dialog = byId("update-dialog");
const status = byId("update-status");
const installButton = byId("update-install");
const openButton = byId("update-open");
const progress = byId("update-progress");
const app = location.pathname.match(/^\/apps\/([a-z]+)\/$/)?.[1];
const store = createPackageStore();
let current = null;
let available = null;
let prepared = null;
let caching = false;
let busy = false;
let launching = false;
let cacheNotice = false;
const leases = new Set();
const mb = bytes => (bytes / 1_000_000).toLocaleString(window.SubnetI18n.locale, { maximumFractionDigits: 1 });
const lock = fn => navigator.locks.request(`subnet-craft-${app}`, fn);
const storageError = error => ["QuotaExceededError", "SecurityError", "NotSupportedError", "InvalidStateError"].includes(error?.name);
const different = () => current && available && current.build !== available.build;

function render() {
  badge.disabled = !current;
  badge.textContent = different() ? t("Uuendus saadaval") : current ? `v${current.version}` : t("Versioon");
  badge.classList.toggle("has-update", Boolean(different()));
  byId("current-version").textContent = current ? `v${current.version}` : "—";
  byId("current-build").textContent = current ? t("Järk {build}", { build: current.build.slice(0, 8) }) : "";
  byId("available-release").hidden = !different();
  byId("available-version").textContent = available ? `v${available.version}` : "—";
  byId("available-build").textContent = available ? t("Järk {build}", { build: available.build.slice(0, 8) }) : "";
  const release = available || current; // Notes may be published later for the same build.
  byId("release-details").textContent = release ? t("Rakenduse maht {size} MB", { size: mb(release.bytes) }) : "";
  const notes = byId("release-notes");
  notes.replaceChildren();
  const source = byId("release-source");
  const official = release?.releaseNotes;
  source.hidden = !official;
  if (official) source.href = official.url;
  // An excerpt of the publisher's text, never generated from our commits.
  // Keep remote Markdown inert; the original formatting/full notes are one link away.
  const lines = official?.body.split(/\r?\n/).map(line => line.trim())
    .filter(line => line && !line.startsWith("#")).slice(0, 5) || [];
  for (const line of lines.length ? lines : [t("Väljalaskemärkmed pole saadaval.")]) {
    const item = document.createElement("p");
    const plain = line.replace(/^[-*]\s+/, "").replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
    item.textContent = plain.length > 300 ? plain.slice(0, 300) + "…" : plain;
    notes.append(item);
  }
  const ready = prepared?.build === available?.build && different();
  installButton.hidden = !different() || ready;
  installButton.disabled = busy;
  openButton.hidden = !ready;
  openButton.disabled = busy;
  byId("update-actions").hidden = !different();
}

// Only a click that starts and ends on the backdrop dismisses the dialog.
// Clicking its padding or dragging selected notes outside keeps it open.
let backdropPressed = false;
function onBackdrop(event) {
  if (event.target !== dialog) return false;
  const rect = dialog.getBoundingClientRect();
  return event.clientX < rect.left || event.clientX > rect.right
    || event.clientY < rect.top || event.clientY > rect.bottom;
}
dialog.addEventListener("pointerdown", event => { backdropPressed = onBackdrop(event); });
dialog.addEventListener("click", event => {
  if (backdropPressed && onBackdrop(event)) dialog.close();
  backdropPressed = false;
});
dialog.addEventListener("close", () => { backdropPressed = false; });

async function ensureWorker() {
  if (!window.isSecureContext || !navigator.serviceWorker || !window.caches || !navigator.locks) return false;
  if (navigator.serviceWorker.controller?.scriptURL === new URL("/apps/craft-cache-sw.js", location.origin).href) {
    // A cached launch doesn't wait for a worker update or a network check.
    navigator.serviceWorker.register("/apps/craft-cache-sw.js", { scope: "/apps/", type: "module", updateViaCache: "none" }).catch(() => {});
    return true;
  }
  await navigator.serviceWorker.register("/apps/craft-cache-sw.js", { scope: "/apps/", type: "module", updateViaCache: "none" });
  if (!navigator.serviceWorker.controller) {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => { navigator.serviceWorker.removeEventListener("controllerchange", changed); reject(new Error("Cache worker activation timed out")); }, 10000);
      const changed = () => {
        if (navigator.serviceWorker.controller) { clearTimeout(timer); navigator.serviceWorker.removeEventListener("controllerchange", changed); resolve(); }
      };
      navigator.serviceWorker.addEventListener("controllerchange", changed);
      changed();
    });
  }
  return true;
}

async function openRelease(release, useCache, protectDraft = false) {
  if (useCache) {
    if (!leases.has(release.build)) {
      // A live tab holds a shared lease; cleanup cannot remove its package,
      // including while an iframe navigation has not yet appeared in clients.
      await new Promise((resolve, reject) => {
        navigator.locks.request(`subnet-craft-live-${app}-${release.build}`, { mode: "shared" }, () => {
          leases.add(release.build);
          resolve();
          return new Promise(() => {}); // Released by the browser when this document closes.
        }).catch(reject);
      });
    }
    if (!await store.complete(app, release.build)) throw new Error("Cached package disappeared");
  }
  // The user can edit while cache/lock work is pending. Check again immediately
  // before changing the iframe, with no asynchronous gap after this check.
  if (protectDraft && window.SubnetCraftHost.dirtyState() !== "false") {
    status.textContent = t("Enne uue versiooni avamist salvesta või ekspordi oma töö.");
    return false;
  }
  current = release;
  window.dispatchEvent(new Event("craft-launch"));
  frame.src = useCache ? launchURL(release, location.search, location.hash)
    : `./editor.html${location.search}${location.hash}`;
  render();
  return true;
}

async function checkUpdate() {
  try {
    available = await store.latest(app);
    // An update accepted in another tab affects future launches, never this editor.
    if (caching) prepared = await store.selected(app);
    render();
  } catch { /* A release-check outage does not interrupt a cached editor. */ }
}

async function start() {
  if (launching || !APPS.includes(app)) return;
  launching = true;
  window.dispatchEvent(new Event("craft-launch"));
  try {
    try { caching = await ensureWorker(); } catch { caching = false; }
    let saved = null;
    if (caching) {
      try {
        saved = await lock(async () => {
          const selected = await store.selected(app);
          if (selected) await openRelease(selected, true);
          return selected;
        });
      } catch { caching = false; }
    }
    if (saved) {
      void checkUpdate();
    } else {
      const latest = await store.latest(app);
      available = latest;
      if (caching) {
        try {
          // Recheck under the lock: another tab may have installed while we waited.
          saved = await lock(async () => {
            const selected = await store.selected(app) || await store.install(latest, detail => {
              window.dispatchEvent(new CustomEvent("craft-package-progress", { detail }));
            });
            await openRelease(selected, true);
            return selected;
          });
        } catch (error) {
          if (!storageError(error)) throw error;
          caching = false;
        }
      }
      cacheNotice = !caching;
      if (!saved) await openRelease(latest, false);
      if (saved && saved.build !== latest.build) void checkUpdate();
    }
  } catch {
    window.dispatchEvent(new Event("craft-launch-error"));
  } finally { launching = false; }
}

badge.addEventListener("click", () => {
  render();
  if (!busy) {
    status.textContent = cacheNotice ? t("Brauser ei saa rakenduse versiooni säilitada. Avatakse uusim versioon.")
      : prepared?.build === available?.build && different() ? t("Uuendus on valmis. Järgmisel avamisel kasutatakse uut versiooni.") : "";
    void checkUpdate();
  }
  dialog.showModal();
});

installButton.addEventListener("click", async () => {
  if (busy || !different()) return;
  busy = true;
  const target = available;
  render();
  status.textContent = t("Laadin uuendust…");
  progress.hidden = !caching;
  progress.value = 0;
  try {
    if (caching) {
      prepared = await lock(() => store.install(target, ({ loaded, total }) => {
        progress.value = loaded / total * 100;
        status.textContent = t("Laadin uuendust… {percent}%", { percent: Math.floor(loaded / total * 100) });
      }));
    } else {
      // Storage-disabled browsers can still explicitly restart on the server release.
      prepared = target;
    }
    status.textContent = t("Uuendus on valmis. Järgmisel avamisel kasutatakse uut versiooni.");
  } catch {
    status.textContent = t("Uuendamine ebaõnnestus. Sinu praegune versioon jäi alles. Proovi uuesti.");
  } finally {
    busy = false;
    progress.hidden = true;
    render();
  }
});

openButton.addEventListener("click", async () => {
  if (!prepared || busy) return;
  if (window.SubnetCraftHost.dirtyState() !== "false") {
    status.textContent = t("Enne uue versiooni avamist salvesta või ekspordi oma töö.");
    return;
  }
  try {
    const open = async () => {
      if (window.SubnetCraftHost.dirtyState() !== "false") {
        status.textContent = t("Enne uue versiooni avamist salvesta või ekspordi oma töö.");
        return;
      }
      if (await openRelease(prepared, caching, true)) dialog.close();
    };
    if (caching) await lock(open); else await open();
  } catch {
    status.textContent = t("Vahemälu pole enam saadaval. Ava rakendus uuesti, et laadida uusim versioon.");
  }
});

frame.addEventListener("load", () => {
  if (caching) navigator.serviceWorker.controller?.postMessage({ type: "prune", app });
});
window.addEventListener("focus", () => { if (current && !busy) void checkUpdate(); });
window.SubnetCraftLaunch = { retry: start };
window.SubnetI18n.bind(dialog, render);
void start();
