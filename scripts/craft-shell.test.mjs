import { readFileSync } from "node:fs";
import vm from "node:vm";
import { localeHost } from "./i18n-test-helpers.mjs";
import test from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("../portal/craft-shell.js", import.meta.url), "utf8");

function host(initialState, search = "", hash = "", language = "et") {
  let state = initialState;
  const locations = [];
  const window = new EventTarget();
  window.SubnetI18n = localeHost({ search: `?lang=${language}` }).i18n;
  window.location = { search, hash, assign: target => locations.push(target) };
  const editor = { contentDocument: { documentElement: { getAttribute: () => state } } };
  const description = {};
  const leave = {};
  const dialog = new EventTarget();
  dialog.querySelector = () => leave;
  dialog.showModal = () => { dialog.open = true; };
  const home = Object.assign(new EventTarget(), { href: "https://subnet.ee/" });
  const files = Object.assign(new EventTarget(), { href: "https://subnet.ee/#failid" });
  const elements = { "craft-editor": editor, "leave-dialog": dialog, "leave-description": description };
  const document = { getElementById: id => elements[id], querySelectorAll: () => [home, files] };
  vm.runInNewContext(source, { window, document });
  return {
    editor, description, dialog, locations, window,
    setState(value) { state = value; },
    click(destination = "home", modifiers = {}) {
      const event = Object.assign(new Event("click", { cancelable: true }), { button: 0, ...modifiers });
      (destination === "home" ? home : files).dispatchEvent(event);
      return event.defaultPrevented;
    },
    answer(value) {
      dialog.returnValue = value;
      dialog.open = false;
      dialog.dispatchEvent(new Event("close"));
    },
    unload() {
      const event = new Event("beforeunload", { cancelable: true });
      // Browser BeforeUnloadEvent has a string-valued returnValue.
      Object.defineProperty(event, "returnValue", { value: "", writable: true });
      window.dispatchEvent(event);
      return event.defaultPrevented;
    },
  };
}

test("cloud file references, renderer flags and fragments reach the original editor", () => {
  const query = "?cloud_service=cloud&cloud_space=one&cloud_file=two&webgl";
  assert.equal(host("false", query, "#page=3").editor.src, `./editor.html${query}#page=3`);
});

test("clean editors allow both portal destinations and browser navigation", () => {
  const h = host("false");
  assert.equal(h.click(), false);
  assert.equal(h.click("files"), false);
  assert.equal(h.unload(), false);
});

test("cancel keeps unsaved work and confirmation follows the selected destination once", () => {
  const h = host("true");
  assert.equal(h.click(), true);
  assert.equal(h.dialog.open, true);
  h.answer("cancel");
  assert.deepEqual(h.locations, []);
  assert.equal(h.unload(), true);
  assert.equal(h.click("files"), true);
  h.answer("leave");
  assert.deepEqual(h.locations, ["https://subnet.ee/#failid"]);
  assert.equal(h.unload(), false, "no duplicate browser prompt after explicit confirmation");
});

test("fresh save/edit state is read at navigation time", () => {
  const h = host("true");
  h.setState("false");
  assert.equal(h.click(), false);
  assert.equal(h.unload(), false);
  h.setState("true");
  assert.equal(h.click(), true);
  assert.equal(h.unload(), true);
});

test("an unavailable or older editor never silently loses work", () => {
  for (const value of [null, undefined, "invalid"]) {
    const h = host(value);
    assert.equal(h.click(), true);
    assert.match(h.description.textContent, /veendu/);
    assert.equal(h.unload(), true);
  }
  const h = host("false");
  Object.defineProperty(h.editor, "contentDocument", { get() { throw new Error("Unavailable"); } });
  assert.equal(h.click(), true);
  assert.equal(h.unload(), true);
});

test("startup allows leaving, but a first unsaved edit is still guarded", () => {
  const h = host(null);
  h.editor.dataset = { loadState: "loading" };
  assert.equal(h.click(), false);
  assert.equal(h.unload(), false);
  h.editor.dataset.loadState = "error";
  assert.equal(h.click(), false);
  h.setState("true");
  assert.equal(h.click(), true);
  assert.equal(h.unload(), true);
});

test("opening another tab does not prompt or disable the existing document guard", () => {
  for (const modifiers of [{ metaKey: true }, { ctrlKey: true }, { shiftKey: true }, { button: 1 }]) {
    const h = host("true");
    assert.equal(h.click("files", modifiers), false);
    assert.equal(h.dialog.open, undefined);
    assert.equal(h.unload(), true);
  }
});

test("returning with browser history restores the guard and Escape cannot reuse confirmation", () => {
  const h = host("true");
  h.click();
  h.answer("leave");
  h.window.dispatchEvent(new Event("pageshow"));
  assert.equal(h.unload(), true);
  h.click("files");
  assert.equal(h.dialog.returnValue, "cancel");
  h.answer(h.dialog.returnValue);
  assert.deepEqual(h.locations, ["https://subnet.ee/"]);
});

test("English unsaved-work prompts keep the same cancellation and leave guard", () => {
  const h = host("true", "?lang=en&cloud_file=one", "#page=2", "en");
  const original = h.editor.src;
  h.click();
  assert.match(h.description.textContent, /You have unsaved changes/);
  h.answer("cancel");
  assert.equal(h.editor.src, original);
  assert.equal(h.unload(), true);
  assert.deepEqual(h.locations, []);
});
