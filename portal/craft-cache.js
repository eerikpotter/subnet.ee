// Cache only public editor packages. Cloud requests and credentials never enter here.
export const APPS = ["photocraft", "wordcraft", "pdfcraft", "deckcraft", "gridcraft", "soundcraft", "cadcraft", "figcraft"];
export const PREFIX = "subnet-craft-v1-";
const SETTINGS = "subnet-craft-settings-v1";
const HASH = /^[a-f0-9]{64}$/;
const pathOK = path => typeof path === "string" && /^[A-Za-z0-9_./-]+$/.test(path)
  && !path.startsWith("/") && path.split("/").every(part => part && part !== "." && part !== "..");

export function validateRelease(release, app) {
  if (!APPS.includes(app) || release?.schema !== 1 || release.app !== app || !HASH.test(release.build)
      || typeof release.version !== "string" || release.version.length > 80
      || !release.assets || typeof release.assets !== "object") throw new Error("Invalid app release");
  const assets = Object.entries(release.assets);
  if (!assets.length || assets.length > 1000 || !release.assets["editor.html"]
      || !assets.some(([path]) => path.endsWith(".wasm"))) throw new Error("Incomplete app release");
  for (const [path, asset] of assets) {
    if (!pathOK(path) || ["index.html", "release.json", "_complete.json"].includes(path)
        || !HASH.test(asset?.sha256) || !Number.isSafeInteger(asset?.bytes) || asset.bytes < 0) {
      throw new Error("Invalid app asset");
    }
  }
  if (assets.reduce((sum, [, asset]) => sum + asset.bytes, 0) !== release.bytes || release.bytes > 512_000_000) {
    throw new Error("Invalid app package size");
  }
  const notes = release.releaseNotes;
  if (notes != null) {
    if (notes.version !== release.version || !/^\d{4}-\d{2}-\d{2}$/.test(notes.publishedAt)
        || notes.url !== `https://github.com/storytold/${app}/releases/tag/v${release.version}`
        || typeof notes.body !== "string" || notes.body.length > 200_000) {
      throw new Error("Invalid release notes");
    }
  }
  return release;
}

export function packagePath(app, build) { return `/apps/${app}/_cached/${build}/`; }
export function packageName(app, build) { return `${PREFIX}${app}-${build}`; }
export function parsePackageURL(value) {
  const url = new URL(value);
  const match = url.pathname.match(/^\/apps\/([a-z]+)\/_cached\/([a-f0-9]{64})\/(.+)$/);
  if (!match || !APPS.includes(match[1]) || !pathOK(match[3])) return null;
  return { app: match[1], build: match[2], path: match[3] };
}
export function launchURL(release, search = "", hash = "") {
  return `${packagePath(release.app, release.build)}editor.html${search}${hash}`;
}

export function createPackageStore({ storage = globalThis.caches, origin = globalThis.location?.origin,
  fetcher = globalThis.fetch, crypto = globalThis.crypto } = {}) {
  const absolute = path => new URL(path, origin).href;
  const pointer = app => absolute(`/apps/${app}/_selected.json`);
  const key = (app, build, path) => absolute(packagePath(app, build) + path);
  const json = value => new Response(JSON.stringify(value), { headers: { "Content-Type": "application/json" } });

  async function complete(app, build) {
    if (!APPS.includes(app) || !HASH.test(build) || !await storage.has(packageName(app, build))) return null;
    const cache = await storage.open(packageName(app, build));
    const record = await cache.match(key(app, build, "_complete.json"));
    if (!record) return null;
    let release;
    try { release = validateRelease(await record.json(), app); } catch { return null; }
    if (release.build !== build) return null;
    for (const path of Object.keys(release.assets)) {
      if (!await cache.match(key(app, build, path))) return null;
    }
    return release;
  }

  async function selected(app) {
    if (!APPS.includes(app) || !await storage.has(SETTINGS)) return null;
    const record = await (await storage.open(SETTINGS)).match(pointer(app));
    if (!record) return null;
    try { return await complete(app, (await record.json()).build); } catch { return null; }
  }

  async function latest(app) {
    if (!APPS.includes(app)) throw new Error("Unknown app");
    const response = await fetcher(absolute(`/apps/${app}/release.json`), { cache: "no-store" });
    if (!response.ok) throw new Error("Release unavailable");
    return validateRelease(await response.json(), app);
  }

  // Call while holding this app's Web Lock. Publish the pointer only after every
  // byte is verified; a failed update never replaces the selected working build.
  async function install(release, progress = () => {}) {
    validateRelease(release, release.app);
    const { app, build } = release;
    if (!await complete(app, build)) {
      await storage.delete(packageName(app, build));
      const cache = await storage.open(packageName(app, build));
      let loaded = 0;
      try {
        for (const [path, asset] of Object.entries(release.assets)) {
          const response = await fetcher(absolute(`/apps/${app}/${path}`), { cache: "no-cache" });
          if (!response.ok || !response.body) throw new Error("App asset unavailable");
          const reader = response.body.getReader();
          const bytes = new Uint8Array(asset.bytes);
          let offset = 0;
          try {
            while (true) {
              const { done, value } = await reader.read();
              if (done) break;
              if (offset + value.length > bytes.length) throw new Error("App asset size mismatch");
              bytes.set(value, offset);
              offset += value.length;
              loaded += value.length;
              progress({ loaded, total: release.bytes });
            }
          } finally { await reader.cancel().catch(() => {}); }
          const digest = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), b => b.toString(16).padStart(2, "0")).join("");
          if (offset !== asset.bytes || digest !== asset.sha256) throw new Error("App asset integrity mismatch");
          // Fetch exposes decoded bytes; don't replay compressed Content-Length/Encoding.
          const headers = new Headers(response.headers);
          for (const name of ["content-encoding", "content-length", "set-cookie", "ic-certificate", "ic-certificateexpression"]) headers.delete(name);
          headers.set("Content-Length", String(bytes.length));
          await cache.put(key(app, build, path), new Response(bytes, { headers }));
        }
        await cache.put(key(app, build, "_complete.json"), json(release));
      } catch (error) {
        await storage.delete(packageName(app, build)).catch(() => {});
        throw error;
      }
    }
    await (await storage.open(SETTINGS)).put(pointer(app), json({ build }));
    return release;
  }

  async function serve(value) {
    const parsed = parsePackageURL(value);
    if (!parsed || new URL(value).origin !== origin || parsed.path === "_complete.json") return null;
    const name = packageName(parsed.app, parsed.build);
    if (!await storage.has(name)) return null;
    const cache = await storage.open(name);
    if (!await cache.match(key(parsed.app, parsed.build, "_complete.json"))) return null;
    return await cache.match(key(parsed.app, parsed.build, parsed.path)) || null;
  }

  async function prune(app, liveBuilds, removeIfUnused = (_build, remove) => remove()) {
    const active = await selected(app);
    const keep = new Set(liveBuilds);
    if (active) keep.add(active.build);
    const prefix = `${PREFIX}${app}-`;
    for (const name of await storage.keys()) {
      if (name.startsWith(prefix) && !keep.has(name.slice(prefix.length))) {
        await removeIfUnused(name.slice(prefix.length), () => storage.delete(name));
      }
    }
  }
  return { complete, selected, latest, install, serve, prune };
}
