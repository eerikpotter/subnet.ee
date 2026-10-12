import { readFileSync } from "node:fs";
import { createHash, webcrypto } from "node:crypto";
import vm from "node:vm";
import test from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("../portal/craft-wasm.js", import.meta.url), "utf8").replace("export async function", "async function");
// Valid Wasm with an empty custom section named x; changing x to y stays valid.
const bytes = Uint8Array.from([0, 97, 115, 109, 1, 0, 0, 0, 0, 2, 1, 120]);
const integrity = "sha384-" + createHash("sha384").update(bytes).digest("base64");

function host(fetchResponse, wasm = WebAssembly) {
  const reports = [];
  const requests = [];
  let clock = 0;
  let onReport = () => {};
  const context = vm.createContext({
    Response, ReadableStream, Uint8Array, WebAssembly: wasm, crypto: webcrypto, btoa,
    performance: { now: () => clock += 100 },
    document: { documentElement: { setAttribute(_, value) { const report = JSON.parse(value); reports.push(report); onReport(report); } } },
    fetch: async (...args) => { requests.push(args); return fetchResponse(); },
  });
  vm.runInContext(source, context);
  return {
    reports, requests,
    watch(fn) { onReport = fn; },
    load: (total = bytes.length, hash = integrity) => context.loadCraftWasm("https://example.test/app-v1.wasm", total, hash),
  };
}

test("reports decoded bytes before completion, ignores compressed Content-Length, and fetches once", async () => {
  let controller;
  const stream = new ReadableStream({ start(c) { controller = c; } });
  const h = host(() => new Response(stream, { headers: { "Content-Encoding": "br", "Content-Length": "3" } }));
  let halfway;
  const reachedHalf = new Promise(resolve => { halfway = resolve; });
  h.watch(p => { if (p.loaded === 6) halfway(); });
  let finished = false;
  const pending = h.load().then(module => { finished = true; return module; });
  controller.enqueue(bytes.slice(0, 6));
  await reachedHalf;
  assert.equal(finished, false);
  assert.equal(h.reports.at(-1).total, 12);
  controller.enqueue(bytes.slice(6));
  controller.close();
  const module = await pending;
  assert.ok(module instanceof WebAssembly.Module);
  assert.equal(h.requests.length, 1);
  assert.deepEqual(h.requests[0], ["https://example.test/app-v1.wasm"]);
  assert.deepEqual(h.reports.slice(-2).map(p => p.phase), ["verify", "starting"]);
  assert.equal(h.reports.at(-1).loaded, bytes.length);
});

test("a valid but tampered module fails integrity checking before it can be instantiated", async () => {
  const changed = bytes.slice();
  changed[11] = 121;
  const h = host(() => new Response(changed));
  await assert.rejects(h.load(), /integrity mismatch/);
  assert.equal(h.reports.at(-1).phase, "error");
  assert.equal(h.reports.some(p => p.phase === "starting"), false);
});

test("truncated and oversized packages cannot produce successful progress", async () => {
  const short = host(() => new Response(bytes.slice(0, -1)));
  await assert.rejects(short.load(), /Incomplete Wasm package/);
  const oversized = host(() => new Response(bytes));
  await assert.rejects(oversized.load(bytes.length - 1), /Unexpected Wasm package size/);
  for (const h of [short, oversized]) assert.equal(h.reports.at(-1).phase, "error");
});

test("HTTP failures and interrupted streams report an error", async () => {
  const missing = host(() => new Response("Missing", { status: 404 }));
  await assert.rejects(missing.load(), /404/);
  const broken = host(() => new Response(new ReadableStream({ start(c) { c.error(new Error("Offline")); } })));
  await assert.rejects(broken.load(), /Offline/);
  assert.equal(broken.reports.at(-1).phase, "error");
});

test("fallback compilation still verifies integrity and reports completion", async () => {
  const fallback = { compile: WebAssembly.compile.bind(WebAssembly) };
  const h = host(() => new Response(bytes), fallback);
  assert.ok(await h.load() instanceof WebAssembly.Module);
  assert.equal(h.reports.at(-1).phase, "starting");
});
