import { readFileSync } from "node:fs";
import vm from "node:vm";
import { localeHost } from "./i18n-test-helpers.mjs";
import test from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("../portal/craft-loading.js", import.meta.url), "utf8");

function host(language = "et") {
  let state = null;
  let download = null;
  let nextTimer = 0;
  const intervals = new Map();
  const timeouts = new Map();
  const window = new EventTarget();
  window.SubnetI18n = localeHost({ search: `?lang=${language}` }).i18n;
  window.location = { search: "?cloud_file=one&webgl", hash: "#page=2" };
  window.setInterval = fn => { intervals.set(++nextTimer, fn); return nextTimer; };
  window.setTimeout = fn => { timeouts.set(++nextTimer, fn); return nextTimer; };
  window.clearInterval = id => intervals.delete(id);
  window.clearTimeout = id => timeouts.delete(id);
  const frame = new EventTarget();
  frame.dataset = { loadState: "loading" };
  frame.contentDocument = { URL: "https://subnet.ee/apps/pdfcraft/editor.html", documentElement: { getAttribute: key => key === "data-craft-dirty" ? state : download } };
  frame.contentWindow = new EventTarget();
  frame.setAttribute = (name, value) => { frame[name] = value; };
  frame.removeAttribute = name => { delete frame[name]; };
  const ids = ["craft-loading", "loading-status", "loading-amount", "loading-progress", "loading-help", "loading-hint", "loading-retry"];
  const elements = Object.fromEntries(ids.map(id => {
    const element = Object.assign(new EventTarget(), { dataset: {}, style: { setProperty(k, v) { this[k] = v; }, removeProperty(k) { delete this[k]; } } });
    element.setAttribute = (k, v) => { element[k] = v; };
    element.removeAttribute = k => { delete element[k]; };
    return [id, element];
  }));
  elements["craft-editor"] = frame;
  let observer;
  class MutationObserver {
    constructor(fn) { this.notify = fn; this.active = true; observer = this; }
    observe() {}
    disconnect() { this.active = false; }
  }
  vm.runInNewContext(source, { window, document: { getElementById: id => elements[id] }, MutationObserver });
  return {
    frame, window, elements, intervals, timeouts,
    tick() { for (const fn of [...intervals.values()]) fn(); },
    slow() { for (const fn of [...timeouts.values()]) fn(); },
    started(value = "false") { state = value; if (observer?.active) observer.notify(); },
    download(value) { download = JSON.stringify(value); if (observer?.active) observer.notify(); },
    retry() { elements["loading-retry"].dispatchEvent(new Event("click")); },
  };
}

test("HTML load and Wasm initialization do not hide the loader before the first editor frame", () => {
  const h = host();
  h.frame.dispatchEvent(new Event("load"));
  assert.equal(h.frame.dataset.loadState, "loading");
  assert.equal(h.frame.inert, true);
  h.frame.contentWindow.wasmBindings = {};
  h.tick();
  assert.equal(h.elements["loading-status"].textContent, "Käivitan rakendust…");
  assert.equal(h.elements["craft-loading"].hidden, false);
  h.started();
  assert.equal(h.frame.dataset.loadState, "ready");
  assert.equal(h.frame.inert, false);
  assert.equal(h.frame["aria-hidden"], undefined);
  assert.equal(h.elements["craft-loading"].hidden, true);
  assert.equal(h.intervals.size + h.timeouts.size, 0);
});

test("a cached editor that starts before the observer attaches is revealed immediately", () => {
  const h = host();
  h.started("true");
  h.tick();
  assert.equal(h.frame.dataset.loadState, "ready");
});

test("a slow connection offers recovery while allowing startup to complete", () => {
  const h = host();
  h.tick();
  h.slow();
  assert.equal(h.elements["loading-help"].hidden, false);
  assert.equal(h.elements["loading-progress"].hidden, false);
  h.started();
  assert.equal(h.elements["craft-loading"].hidden, true);
});

test("a rejected bootstrap stops the indicator and retry preserves launch references", () => {
  const h = host();
  h.tick();
  h.frame.contentWindow.dispatchEvent(new Event("unhandledrejection"));
  assert.equal(h.frame.dataset.loadState, "error");
  assert.equal(h.elements["loading-help"].hidden, false);
  assert.equal(h.elements["loading-progress"].hidden, true);
  assert.equal(h.intervals.size + h.timeouts.size, 0);
  h.retry();
  assert.equal(h.frame.src, "./editor.html?cloud_file=one&webgl#page=2");
  assert.equal(h.frame.dataset.loadState, "loading");
  assert.equal(h.elements["loading-help"].hidden, true);
  h.started();
  h.tick();
  assert.equal(h.frame.dataset.loadState, "loading", "the old frame cannot finish a retry");
  h.frame.contentDocument = { ...h.frame.contentDocument };
  h.tick();
  assert.equal(h.frame.dataset.loadState, "ready");
});

test("a late error or retry cannot replace a running editor", () => {
  const h = host();
  h.started();
  h.tick();
  h.frame.contentWindow.dispatchEvent(new Event("unhandledrejection"));
  h.retry();
  assert.equal(h.frame.dataset.loadState, "ready");
  assert.equal(h.frame.src, undefined);
});

test("returning from browser history resumes a pending loader", () => {
  const h = host();
  h.window.dispatchEvent(new Event("pagehide"));
  assert.equal(h.intervals.size + h.timeouts.size, 0);
  h.window.dispatchEvent(new Event("pageshow"));
  assert.equal(h.intervals.size, 1);
  h.started();
  assert.equal(h.frame.dataset.loadState, "ready");
});

test("byte progress is determinate and 100% waits for verification and the first frame", () => {
  const h = host();
  h.tick();
  h.download({ phase: "download", loaded: 2_000_000, total: 10_000_000 });
  assert.equal(h.elements["loading-progress"]["aria-valuenow"], "20");
  assert.equal(h.elements["loading-progress"].style["--loading-percent"], "20%");
  assert.match(h.elements["loading-amount"].textContent, /20%/);
  h.download({ phase: "verify", loaded: 10_000_000, total: 10_000_000 });
  assert.equal(h.elements["loading-progress"]["aria-valuenow"], "100");
  assert.equal(h.elements["loading-status"].textContent, "Kontrollin rakendusepaketti…");
  assert.equal(h.elements["craft-loading"].hidden, false);
  h.download({ phase: "starting", loaded: 10_000_000, total: 10_000_000 });
  assert.equal(h.elements["loading-status"].textContent, "Käivitan rakendust…");
  h.started();
  assert.equal(h.elements["craft-loading"].hidden, true);
});

test("a failed package exposes recovery and a retry clears previous progress", () => {
  const h = host();
  h.tick();
  h.download({ phase: "download", loaded: 4, total: 8 });
  h.download({ phase: "error", loaded: 4, total: 8 });
  assert.equal(h.frame.dataset.loadState, "error");
  h.retry();
  assert.equal(h.elements["loading-progress"]["aria-valuenow"], undefined);
  assert.equal(h.elements["loading-amount"].textContent, "");
});

test("English loading reports real byte progress and translates failures", () => {
  const h = host("en");
  h.tick();
  h.download({ phase: "download", loaded: 2_000_000, total: 10_000_000 });
  assert.equal(h.elements["loading-status"].textContent, "Downloading application…");
  assert.equal(h.elements["loading-amount"].textContent, "2.0 / 10.0 MB · 20%");
  assert.match(h.elements["loading-progress"]["aria-valuetext"], /^Application package:/);
  h.download({ phase: "error", loaded: 2_000_000, total: 10_000_000 });
  assert.equal(h.elements["loading-status"].textContent, "The application could not be opened.");
});
