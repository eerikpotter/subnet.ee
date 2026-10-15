import { APPS, createPackageStore, parsePackageURL } from "/craft-cache.js";
const store = createPackageStore();
self.addEventListener("install", event => event.waitUntil(self.skipWaiting()));
self.addEventListener("activate", event => event.waitUntil(self.clients.claim()));
self.addEventListener("fetch", event => {
  const url = new URL(event.request.url);
  // Only virtual, version-pinned public app assets. Never intercept identity,
  // cloud reads/writes, the picker, release checks or the normal launcher.
  if (event.request.method !== "GET" || url.origin !== self.location.origin
      || !/^\/apps\/[a-z]+\/_cached\//.test(url.pathname)) return;
  event.respondWith(store.serve(url.href).then(response => response || new Response("Cached app is no longer available. Reopen the app to download the latest version.", {
    status: 404, headers: { "Content-Type": "text/plain", "Cache-Control": "no-store" },
  })));
});
self.addEventListener("message", event => {
  const app = event.data?.app;
  if (event.data?.type !== "prune" || !APPS.includes(app) || !self.navigator.locks) return;
  event.waitUntil(self.navigator.locks.request(`subnet-craft-${app}`, async () => {
    const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    const live = clients.map(client => parsePackageURL(client.url)).filter(value => value?.app === app).map(value => value.build);
    await store.prune(app, live, (build, remove) => self.navigator.locks.request(
      `subnet-craft-live-${app}-${build}`, { ifAvailable: true }, lease => lease ? remove() : undefined));
  }));
});
