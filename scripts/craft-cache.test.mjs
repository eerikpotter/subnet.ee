import { readFileSync } from "node:fs";
import { createHash, webcrypto } from "node:crypto";
import test from "node:test";
import assert from "node:assert/strict";

const code = readFileSync(new URL("../portal/craft-cache.js", import.meta.url), "utf8");
const { APPS, createPackageStore, launchURL, packageName, parsePackageURL, validateRelease } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
const origin = "https://cloud.test";
const sha = bytes => createHash("sha256").update(bytes).digest("hex");

test("every registered editor has an isolated package cache", () => {
  const registered = JSON.parse(readFileSync(new URL("../registry/apps.json", import.meta.url), "utf8"));
  assert.deepEqual([...APPS].sort(), registered.map(app => app.id).sort());
});

function fixture() {
  const caches = new Map();
  const storage = {
    has: async name => caches.has(name), keys: async () => [...caches.keys()],
    delete: async name => caches.delete(name),
    async open(name) {
      if (!caches.has(name)) caches.set(name, new Map());
      const records = caches.get(name);
      return { match: async key => records.get(key)?.clone(),
        put: async (key, value) => { records.set(key, value.clone()); },
        delete: async key => records.delete(key) };
    },
  };
  const server = new Map();
  const requests = [];
  const store = createPackageStore({ storage, origin, crypto: webcrypto, fetcher: async url => {
    requests.push(url);
    return server.get(url)?.clone() || new Response("missing", { status: 404 });
  } });
  function publish(version, app = "wordcraft") {
    // Replace the entire server package: previous binaries really disappear.
    for (const key of server.keys()) if (key.startsWith(`${origin}/apps/${app}/`)) server.delete(key);
    const files = { "editor.html": `<script src="./${version}.js"></script>`, [`${version}.js`]: `version ${version}`, [`${version}.wasm`]: `wasm ${version}` };
    const assets = {};
    for (const [path, content] of Object.entries(files)) {
      assets[path] = { bytes: Buffer.byteLength(content), sha256: sha(content) };
      server.set(`${origin}/apps/${app}/${path}`, new Response(content, { headers: { "Content-Type": path.endsWith(".html") ? "text/html" : "application/octet-stream", "Content-Security-Policy": "default-src 'self'", "Content-Encoding": "gzip" } }));
    }
    const release = { schema: 1, app, version, build: sha(app + version), releaseNotes: null, assets, bytes: Object.values(assets).reduce((sum, a) => sum + a.bytes, 0) };
    server.set(`${origin}/apps/${app}/release.json`, new Response(JSON.stringify(release)));
    return release;
  }
  return { store, storage, server, requests, publish };
}

test("A still opens after B replaces every server file; checking B does not select it", async () => {
  const f = fixture();
  const a = f.publish("1.0");
  await f.store.install(a);
  const b = f.publish("2.0");
  assert.equal((await f.store.latest("wordcraft")).build, b.build);
  assert.equal((await f.store.selected("wordcraft")).build, a.build);
  f.requests.length = 0;
  const old = await f.store.serve(origin + launchURL(a, "?cloud_file=42&lang=en", "#page=2"));
  assert.match(await old.text(), /1\.0\.js/);
  assert.deepEqual(f.requests, [], "old app is served entirely from its cache");
  assert.equal(old.headers.get("content-encoding"), null);
  assert.equal(old.headers.get("content-security-policy"), "default-src 'self'");
});

test("accepting B selects it only after complete verification and keeps A usable in open tabs", async () => {
  const f = fixture();
  const a = f.publish("1.0"); await f.store.install(a);
  const b = f.publish("2.0");
  const progress = [];
  await f.store.install(b, value => progress.push(value));
  assert.equal((await f.store.selected("wordcraft")).build, b.build);
  assert.equal(progress.at(-1).loaded, b.bytes);
  await f.store.prune("wordcraft", [a.build]);
  assert.ok(await f.store.complete("wordcraft", a.build));
  await f.store.prune("wordcraft", []);
  assert.equal(await f.store.complete("wordcraft", a.build), null);
  assert.ok(await f.store.complete("wordcraft", b.build));
});

test("a failed or corrupt update leaves the previous selection and removes staging bytes", async () => {
  for (const failure of ["missing", "corrupt", "short"]) {
    const f = fixture();
    const a = f.publish("1.0"); await f.store.install(a);
    const b = f.publish("2.0");
    const key = `${origin}/apps/wordcraft/2.0.wasm`;
    if (failure === "missing") f.server.delete(key);
    else f.server.set(key, new Response(failure === "short" ? "x" : "wasm XXXX"));
    await assert.rejects(f.store.install(b));
    assert.equal((await f.store.selected("wordcraft")).build, a.build);
    assert.equal(await f.storage.has(packageName("wordcraft", b.build)), false);
  }
});

test("cleared and partially evicted caches choose latest, never download old server assets", async () => {
  const f = fixture();
  const a = f.publish("1.0"); await f.store.install(a);
  const b = f.publish("2.0");
  const cache = await f.storage.open(packageName("wordcraft", a.build));
  await cache.delete(`${origin}/apps/wordcraft/_cached/${a.build}/1.0.js`);
  assert.equal(await f.store.selected("wordcraft"), null);
  await f.store.install(await f.store.latest("wordcraft"));
  assert.equal((await f.store.selected("wordcraft")).build, b.build);
  for (const name of await f.storage.keys()) await f.storage.delete(name);
  assert.equal(await f.store.selected("wordcraft"), null);
  assert.equal((await f.store.latest("wordcraft")).build, b.build);
});

test("cloud/private URLs and unknown cached assets are never served or fetched by the cache", async () => {
  const f = fixture(); const a = f.publish("1.0"); await f.store.install(a);
  f.requests.length = 0;
  for (const url of ["/api/v3/canister/x/call", "/pilveruum/?pick=open", "/apps/wordcraft/release.json", `/apps/wordcraft/_cached/${a.build}/missing.js`, `/apps/wordcraft/_cached/${a.build}/_complete.json`]) {
    assert.equal(await f.store.serve(origin + url), null);
  }
  assert.equal(await f.store.serve("https://elsewhere.test" + launchURL(a)), null);
  assert.deepEqual(f.requests, []);
});

test("packages and cleanup are isolated between all registered applications", async () => {
  const f = fixture();
  const apps = APPS;
  for (const app of apps) await f.store.install(f.publish("1.0", app));
  await f.store.install(f.publish("2.0", "wordcraft"));
  await f.store.prune("wordcraft", []);
  for (const app of apps) assert.equal((await f.store.selected(app)).version, app === "wordcraft" ? "2.0" : "1.0");
});

test("live leases protect a package even before its iframe appears in the worker clients", async () => {
  const f = fixture(); const a = f.publish("1.0"); await f.store.install(a);
  await f.store.install(f.publish("2.0"));
  await f.store.prune("wordcraft", [], async () => {});
  assert.ok(await f.store.complete("wordcraft", a.build));
});

test("manifest validation rejects path escapes, mixed apps, oversized and incomplete packages", () => {
  const f = fixture(); const a = f.publish("1.0");
  for (const path of ["../private", "/cloud.js", "a/../../cloud.js", "a%2fprivate", "a?x", "index.html"]) {
    const broken = structuredClone(a); broken.assets[path] = { sha256: sha(""), bytes: 0 };
    assert.throws(() => validateRelease(broken, "wordcraft"));
  }
  assert.throws(() => validateRelease(a, "photocraft"));
  assert.throws(() => validateRelease({ ...a, bytes: a.bytes + 1 }, "wordcraft"));
  assert.equal(parsePackageURL(origin + "/api/v3/call"), null);
});

test("cached launch keeps cloud references, renderer flags and fragment", () => {
  const f = fixture(); const a = f.publish("1.0");
  assert.equal(launchURL(a, "?cloud_file=42&webgl&lang=en", "#page=2"), `/apps/wordcraft/_cached/${a.build}/editor.html?cloud_file=42&webgl&lang=en#page=2`);
});

test("notes must belong to the exact app release and the official publisher", () => {
  const f = fixture(); const release = f.publish("1.0");
  const notes = { version: "1.0", publishedAt: "2026-10-15", url: "https://github.com/storytold/wordcraft/releases/tag/v1.0", body: "Published notes" };
  assert.equal(validateRelease({ ...release, releaseNotes: notes }, "wordcraft").releaseNotes, notes);
  for (const bad of [{ ...notes, version: "2.0" }, { ...notes, url: "javascript:alert(1)" }, { ...notes, url: "https://github.com/storytold/photocraft/releases/tag/v1.0" }]) {
    assert.throws(() => validateRelease({ ...release, releaseNotes: bad }, "wordcraft"));
  }
});
