import init, { CloudPortal } from "__CLOUD_BINDINGS__";

const $ = (id) => document.getElementById(id);
let cloud;
let account = null;
let generation = 0;
let cursor = null;
let loading = false;
let restoring = false;
let pendingRefresh = false;
let registry = [];
let lastRefresh = 0;
let loginAttempt = 0;

function route() {
  const files = location.hash === "#failid";
  $("apps-view").hidden = files;
  $("failid").hidden = !files;
  $("page-name").textContent = files ? "Minu failid" : "Rakendused";
  for (const [id, active] of [["files-nav", files], ["apps-nav", !files]]) {
    $(id).classList.toggle("nav-active", active);
    if (active) $(id).setAttribute("aria-current", "page");
    else $(id).removeAttribute("aria-current");
  }
  if (files && cloud && Date.now() - lastRefresh > 5000) refresh();
}
function message(text, error = false) {
  $("account-message").textContent = text;
  $("account-message").hidden = !text;
  $("account-message").classList.toggle("is-error", error);
}
function size(bytes) {
  return new Intl.NumberFormat("et-EE", { maximumFractionDigits: 1 }).format(bytes / 1024 / 1024) + " MB";
}
function clearFiles(text) {
  cursor = null;
  $("file-list").replaceChildren();
  $("more-files").hidden = true;
  $("file-status").hidden = false;
  $("file-status").textContent = text;
}
function updateAccount() {
  const signedIn = !!account?.authenticated;
  $("sign-in").hidden = signedIn;
  $("sign-out").hidden = !signedIn;
  $("sign-in").disabled = !cloud || restoring;
  $("sign-out").disabled = restoring;
  $("refresh-files").disabled = !cloud || restoring || loading;
  $("account-status").textContent = signedIn ? "Minu konto" : "Sinu isiklik loomeruum";
  $("account-status").title = signedIn ? account.principal : "";
  $("storage-usage").textContent = signedIn ? `${size(account.usedBytes)} / ${size(account.quotaBytes)} kasutusel` : "Sinu failid on seotud sinu kontoga.";
}
function appendFile(file) {
  const row = document.createElement("article");
  row.className = "file-row";
  const icon = document.createElement("span");
  icon.className = "file-icon";
  icon.textContent = "Ph";
  icon.setAttribute("aria-hidden", "true");
  const detail = document.createElement("div");
  detail.className = "file-detail";
  const title = document.createElement("h2");
  title.textContent = file.name.endsWith(".pcraft") ? file.name : `${file.name}.pcraft`;
  const meta = document.createElement("p");
  meta.textContent = `${new Date(file.updatedMs).toLocaleString("et-EE", { dateStyle: "medium", timeStyle: "short" })} · ${file.versions} ${file.versions === 1 ? "versioon" : "versiooni"}`;
  detail.append(title, meta);
  const bytes = document.createElement("span");
  bytes.className = "file-size";
  bytes.textContent = size(file.bytes);
  row.append(icon, detail, bytes);
  const app = registry.find((app) => app.cloud && app.openFormats?.some((format) => format.id === file.formatId && format.version === file.formatVersion));
  if (app && /^[a-z]+$/.test(app.id)) {
    const link = document.createElement("a");
    link.className = "button";
    const params = new URLSearchParams({ cloud_service: "cloud", cloud_space: file.spaceId, cloud_file: file.id });
    link.href = `/apps/${app.id}/?${params}`;
    link.textContent = `Ava ${app.name}is →`;
    row.append(link);
  }
  $("file-list").append(row);
}
async function loadFiles(epoch, append = false) {
  if (!account?.authenticated || !cloud) return;
  loading = true;
  updateAccount();
  $("more-files").disabled = true;
  if (!append) clearFiles("Laadin sinu faile…");
  try {
    const page = await cloud.files(account.spaceId, append ? cursor : undefined);
    if (epoch !== generation || cloud.needs_refresh()) return;
    for (const file of page.files) appendFile(file);
    cursor = page.nextCursor;
    $("more-files").hidden = !cursor;
    $("file-status").hidden = $("file-list").children.length > 0;
    $("file-status").textContent = "Sinu esimene fail ootab loomist. Vali PhotoCraftis File → Sovereign Cloud → Save.";
  } catch (error) {
    if (epoch !== generation) return;
    $("file-status").hidden = false;
    $("file-status").textContent = "Failide laadimine ebaõnnestus. Proovi värskendada.";
    message(error.message || String(error), true);
  } finally {
    if (epoch === generation) {
      loading = false;
      $("more-files").disabled = false;
      updateAccount();
    }
  }
}
async function refresh() {
  if (!cloud) return;
  if (restoring) { pendingRefresh = true; return; }
  restoring = true;
  loading = false;
  const epoch = ++generation;
  account = null;
  clearFiles("Kontrollin sisselogimist…");
  updateAccount();
  try {
    const state = await cloud.restore();
    if (epoch !== generation) return;
    account = state;
    lastRefresh = Date.now();
    message("");
    if (state.authenticated) await loadFiles(epoch);
    else clearFiles("Logi sisse, et näha oma salvestatud faile.");
  } catch (error) {
    if (epoch !== generation) return;
    clearFiles("Kontoga ühendamine ei õnnestunud. Proovi uuesti.");
    message(error.message || String(error), true);
  } finally {
    restoring = false;
    updateAccount();
    if (pendingRefresh) { pendingRefresh = false; refresh(); }
  }
}
$("sign-in").addEventListener("click", async () => {
  if (!cloud) return;
  const attempt = ++loginAttempt;
  try {
    message("Lõpeta sisselogimine Internet Identity aknas.");
    $("sign-in").textContent = "Ava sisselogimine uuesti";
    await cloud.sign_in();
    if (attempt === loginAttempt) await refresh();
  } catch (error) {
    if (attempt === loginAttempt) message(error.message || String(error), true);
  } finally {
    if (attempt === loginAttempt) $("sign-in").textContent = "Logi sisse";
  }
});
$("sign-out").addEventListener("click", async () => {
  if (!cloud) return;
  ++generation;
  account = null;
  clearFiles("Login välja…");
  updateAccount();
  try { await cloud.sign_out(); } catch (error) { message(error.message || String(error), true); }
  await refresh();
});
$("refresh-files").addEventListener("click", refresh);
$("more-files").addEventListener("click", () => { if (!loading) loadFiles(generation, true); });
window.addEventListener("hashchange", route);
window.addEventListener("storage", (event) => {
  if (!event.key || ["ic-identity", "ic-delegation", "subnet.files.changed"].includes(event.key)) refresh();
});
window.addEventListener("focus", () => { if (Date.now() - lastRefresh > 2000) refresh(); });
document.addEventListener("visibilitychange", () => { if (!document.hidden) refresh(); });
setInterval(() => { if (!document.hidden && cloud?.needs_refresh()) refresh(); }, 1000);
route();
try {
  const response = await fetch("/apps.json");
  if (!response.ok) throw new Error("Rakenduste register ei ole kättesaadav.");
  registry = (await response.json()).apps;
  await init({ module_or_path: "__CLOUD_WASM__" });
  cloud = new CloudPortal();
  await refresh();
} catch (error) {
  $("account-status").textContent = "Konto pole kättesaadav";
  message(error.message || String(error), true);
}
