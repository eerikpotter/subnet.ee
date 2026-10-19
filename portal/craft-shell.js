import { workspaceReturn } from "./workspace-context.js";
const { t } = window.SubnetI18n;
// One host for every Craft. The editor publishes only its aggregate dirty state;
// document contents and account data stay in the editor.
const editor = document.getElementById("craft-editor");
const dialog = document.getElementById("leave-dialog");
const description = document.getElementById("leave-description");
const leaveButton = dialog.querySelector('[value="leave"]');
let destination = null;
let confirmed = false;
const returnPath = workspaceReturn(new URLSearchParams(window.location.search).get("workspace_return"), window.location.origin);
if (returnPath) document.querySelectorAll("[data-leave]").forEach(link => { link.href = returnPath; });
const documentName = document.getElementById("craft-document-name");
const saveStatus = document.getElementById("craft-save-status");
if (documentName) documentName.title = t("Muuda dokumendi nime");
documentName?.addEventListener("click", () => {
  if (documentName.disabled) return;
  try {
    editor.contentDocument?.documentElement.dispatchEvent(new editor.contentWindow.Event("subnet-rename-document"));
  } catch { /* A navigating editor will publish its capability after load. */ }
});
let statusObserver;
function observeEditor() {
  statusObserver?.disconnect();
  try {
    const root = editor.contentDocument?.documentElement;
    if (!root) return;
    // The explicit host confirmation covers child unload listeners too. Browser
    // reload/close remains guarded; we suppress nothing before confirmation.
    editor.contentWindow?.addEventListener("beforeunload", event => {
      if (confirmed) event.stopImmediatePropagation();
    }, {capture:true});
    const refresh = () => {
      let state;
      try { state = JSON.parse(root.getAttribute("data-subnet-document")); } catch {}
      if (documentName) {
        documentName.textContent = state?.version === 1 ? state.name || "" : "";
        documentName.disabled = state?.version !== 1 || state.canRename !== true;
      }
      if (saveStatus) saveStatus.textContent = state?.version === 1 ? state.status || "" : "";
    };
    refresh();
    statusObserver = new MutationObserver(refresh);
    statusObserver.observe(root, {attributes:true, attributeFilter:["data-subnet-document"]});
  } catch { /* The existing unknown-state leave guard remains authoritative. */ }
}
editor.addEventListener("load", observeEditor);

function dirtyState() {
  try {
    const state = editor.contentDocument?.documentElement.getAttribute("data-craft-dirty");
    if (state === "true" || state === "false") return state;
    // No editing is possible while the loading screen covers the inert frame.
    if (["loading", "error"].includes(editor.dataset?.loadState)) return "false";
    return state;
  } catch {
    return null;
  }
}

function beforeUnload(event) {
  if (!confirmed && dirtyState() !== "false") {
    event.preventDefault();
    event.returnValue = "";
  }
}

// Read synchronously when leaving, so a just-completed edit/save is respected.
// Unknown state is guarded too (old builds, startup or an unavailable editor).
window.addEventListener("beforeunload", beforeUnload);
window.addEventListener("pageshow", () => { confirmed = false; });
document.querySelectorAll("[data-leave]").forEach(link => {
  link.addEventListener("click", event => {
    // Opening the portal in another tab keeps the document alive.
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    if (dirtyState() === "false") return;
    event.preventDefault();
    destination = link.href;
    const dirty = dirtyState() === "true";
    description.textContent = dirty
      ? t("Sul on salvestamata muudatusi. Salvesta oma töö rakenduses või lahku muudatusi salvestamata.")
      : t("Enne lahkumist veendu, et oled oma töö salvestanud. Salvestamata muudatused lähevad lahkumisel kaotsi.");
    leaveButton.textContent = dirty ? t("Lahku salvestamata") : t("Lahku rakendusest");
    dialog.returnValue = "cancel";
    dialog.showModal();
  });
});

dialog.addEventListener("close", () => {
  if (dialog.returnValue === "leave" && destination) {
    confirmed = true;
    window.location.assign(destination);
  }
  destination = null;
});

// Keep launch references (cloud files, ?file=, renderer flags) and fragments.
// The editor stays beside its original assets, with its own complete viewport.
window.SubnetCraftHost = { dirtyState };
// The version manager owns launching/restarting; navigation keeps its existing
// synchronous draft guard regardless of which cached editor is running.
