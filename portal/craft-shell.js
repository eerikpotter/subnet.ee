const { t } = window.SubnetI18n;
// One host for every Craft. The editor publishes only its aggregate dirty state;
// document contents and account data stay in the editor.
const editor = document.getElementById("craft-editor");
const dialog = document.getElementById("leave-dialog");
const description = document.getElementById("leave-description");
const leaveButton = dialog.querySelector('[value="leave"]');
let destination = null;
let confirmed = false;

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
editor.src = `./editor.html${window.location.search}${window.location.hash}`;
