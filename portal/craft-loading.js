const { t } = window.SubnetI18n;
// Track the package's decoded bytes, then wait for the editor's first frame.
// The bootstrap publishes progress; this shell never fetches a second copy.
const frame = document.getElementById("craft-editor");
const loading = document.getElementById("craft-loading");
const status = document.getElementById("loading-status");
const progress = document.getElementById("loading-progress");
const amount = document.getElementById("loading-amount");
const help = document.getElementById("loading-help");
const hint = document.getElementById("loading-hint");
const retry = document.getElementById("loading-retry");
let watchedDocument = null;
let replacedDocument = null;
let observer = null;
let detachErrors = () => {};
let poll = null;
let slow = null;

function stopWatching() {
  window.clearInterval(poll);
  window.clearTimeout(slow);
  observer?.disconnect();
  detachErrors();
}

function ready() {
  frame.dataset.loadState = "ready";
  stopWatching();
  frame.inert = false;
  frame.removeAttribute("aria-hidden");
  loading.hidden = true;
}

function failed() {
  if (frame.dataset.loadState !== "loading") return;
  // A late error from a running app must not cover it or enable a destructive retry.
  if (hasStarted()) { ready(); return; }
  frame.dataset.loadState = "error";
  loading.dataset.state = "error";
  progress.hidden = true;
  status.textContent = t("Rakendust ei õnnestunud avada.");
  hint.textContent = t("Proovi uuesti. Kui tõrge kordub, saad ülamenüüst tööruumi tagasi minna.");
  help.hidden = false;
  stopWatching();
}

function hasStarted() {
  try {
    const state = frame.contentDocument?.documentElement?.getAttribute("data-craft-dirty");
    return state === "true" || state === "false";
  } catch {
    return false;
  }
}

function showDownload(doc) {
  const raw = doc.documentElement.getAttribute("data-craft-download");
  if (!raw) return;
  let download;
  try { download = JSON.parse(raw); } catch { return; }
  showProgress(download);
}

function showProgress(download) {
  const { phase, loaded, total } = download;
  if (!Number.isSafeInteger(total) || total <= 0 || !Number.isSafeInteger(loaded) || loaded < 0 || loaded > total) return;
  if (phase === "error") { failed(); return; }
  if (!["download", "verify", "starting"].includes(phase)) return;
  const percent = Math.floor(loaded / total * 100);
  const mb = value => (value / 1_000_000).toLocaleString(window.SubnetI18n.locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
  const label = `${mb(loaded)} / ${mb(total)} MB · ${percent}%`;
  progress.setAttribute("data-determinate", "");
  progress.setAttribute("aria-valuemin", "0");
  progress.setAttribute("aria-valuemax", "100");
  progress.setAttribute("aria-valuenow", String(percent));
  progress.setAttribute("aria-valuetext", t("Rakendusepakett: {amount}", { amount: label }));
  progress.style.setProperty("--loading-percent", `${loaded / total * 100}%`);
  if (amount.textContent !== label) amount.textContent = label;
  const message = phase === "download" ? t("Laadin rakendusepaketti…")
    : phase === "verify" ? t("Kontrollin rakendusepaketti…") : t("Käivitan rakendust…");
  if (status.textContent !== message) status.textContent = message;
}

function inspect() {
  if (frame.dataset.loadState !== "loading") return;
  try {
    const doc = frame.contentDocument;
    if (!doc || doc.URL === "about:blank" || doc === replacedDocument) return;
    if (hasStarted()) { ready(); return; }
    if (doc !== watchedDocument) {
      observer?.disconnect();
      detachErrors();
      watchedDocument = doc;
      observer = new MutationObserver(inspect);
      observer.observe(doc.documentElement, { attributes: true, attributeFilter: ["data-craft-dirty", "data-craft-download"] });
      const win = frame.contentWindow;
      // Broken images/fonts do not prevent the editor from starting.
      const onError = event => {
        if (event.target === win || event.target?.tagName === "SCRIPT") failed();
      };
      win.addEventListener("error", onError, true);
      win.addEventListener("unhandledrejection", failed);
      detachErrors = () => {
        win.removeEventListener("error", onError, true);
        win.removeEventListener("unhandledrejection", failed);
      };
    }
    showDownload(doc);
    if (frame.dataset.loadState === "loading" && frame.contentWindow.wasmBindings) status.textContent = t("Käivitan rakendust…");
  } catch {
    failed();
  }
}

function startWatching() {
  stopWatching();
  watchedDocument = null;
  observer = null;
  frame.dataset.loadState = "loading";
  frame.inert = true;
  frame.setAttribute("aria-hidden", "true");
  loading.hidden = false;
  loading.dataset.state = "loading";
  progress.hidden = false;
  for (const attribute of ["data-determinate", "aria-valuemin", "aria-valuemax", "aria-valuenow", "aria-valuetext"]) progress.removeAttribute(attribute);
  progress.style.removeProperty("--loading-percent");
  amount.textContent = "";
  help.hidden = true;
  status.textContent = t("Laadin rakendust…");
  hint.textContent = t("Esimene avamine võib võtta veidi kauem. Kui laadimine ei lõpe, proovi uuesti.");
  poll = window.setInterval(inspect, 100);
  // A slow connection is not a failure: keep watching and allow it to finish.
  slow = window.setTimeout(() => { help.hidden = false; }, 20000);
}

retry.addEventListener("click", () => {
  // A first frame may have arrived just before this click.
  try { if (hasStarted()) { ready(); return; } } catch { /* failed frame */ }
  if (window.SubnetCraftLaunch) { window.SubnetCraftLaunch.retry(); return; }
  replacedDocument = frame.contentDocument;
  frame.src = `./editor.html${window.location.search}${window.location.hash}`;
  startWatching();
});
window.addEventListener("craft-launch", () => {
  replacedDocument = frame.contentDocument;
  startWatching();
});
window.addEventListener("craft-package-progress", event => {
  if (frame.dataset.loadState === "loading") showProgress({ ...event.detail, phase: "download" });
});
window.addEventListener("craft-launch-error", failed);
frame.addEventListener("load", inspect);
window.addEventListener("pagehide", stopWatching);
window.addEventListener("pageshow", () => {
  if (frame.dataset.loadState === "loading") { startWatching(); inspect(); }
});
startWatching();
